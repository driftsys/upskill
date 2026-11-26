//! SSOT layout introspection: detecting item directories, bundle files,
//! and registry roots on disk.
//!
//! The pipeline accepts several on-disk layouts (per format-spec §2.1):
//! a flat `<root>/<item>/ENTRY.md`, a categorised
//! `<root>/<category>/<item>/ENTRY.md`, or a bundle file pointing into
//! either. The helpers in this module classify paths without touching
//! the parser — they only check filesystem structure.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

use super::ItemKind;
use crate::parse::frontmatter;

const ENTRYPOINTS: [&str; 5] = ["RULE.md", "SKILL.md", "AGENT.md", "COMMAND.md", "HOOK.md"];

/// Effective identity name for an item (§2.1). Skills always take the
/// folder name (Agent Skills standard mandates name == dir); rules/agents
/// take their frontmatter name when present, else the folder name.
pub(crate) fn effective_name(
    kind: ItemKind,
    frontmatter_name: Option<&str>,
    folder: &str,
) -> String {
    match kind {
        ItemKind::Skill => folder.to_string(),
        ItemKind::Rule | ItemKind::Agent => frontmatter_name.unwrap_or(folder).to_string(),
    }
}

/// Probe an entrypoint's frontmatter `name` cheaply, falling back to the
/// folder name on any read/parse failure (validation belongs to lint, not
/// discovery). Keeps `(kind, name)` identity consistent everywhere.
pub(super) fn probe_effective_name(entry_path: &Path, kind: ItemKind, folder: &str) -> String {
    if kind == ItemKind::Skill {
        return folder.to_string();
    }
    match fs::read_to_string(entry_path) {
        Ok(raw) => name_from_content(&raw, folder),
        Err(_) => folder.to_string(),
    }
}

/// The frontmatter `name` of the entrypoint `raw`, or `folder` when there is
/// none or the frontmatter does not parse.
fn name_from_content(raw: &str, folder: &str) -> String {
    name_from_content_strict(raw, folder).unwrap_or_else(|_| folder.to_string())
}

/// As [`name_from_content`], but a frontmatter that does not parse is an
/// error.
fn name_from_content_strict(raw: &str, folder: &str) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct NameProbe {
        #[serde(default)]
        name: Option<String>,
    }
    let (probe, _) = frontmatter::parse::<NameProbe>(raw)?;
    Ok(probe.name.unwrap_or_else(|| folder.to_string()))
}

/// Returns true when the directory contains at least one SSOT entrypoint
/// file (`RULE.md`, `SKILL.md`, `AGENT.md`, `COMMAND.md`, or `HOOK.md`).
pub(super) fn is_item_dir(path: &Path) -> bool {
    ENTRYPOINTS.iter().any(|entry| path.join(entry).is_file())
}

/// Refuse an `add` source directory that is itself an item directory (issues
/// #472 and #488). Discovery reads the items in the child and grandchild
/// directories of a source, not in the source directory itself, so such a
/// source installs no item of its own. The directory is not refused when it
/// has an item in a child or grandchild directory, when it holds a
/// `*.bundle.yaml` file, or when a name in `names` matches a bundle in it:
/// then the source still has something to install.
///
/// For a local source, when `inside_output` returns true for the kind and
/// the folder name or the effective name, the directory is inside an output
/// path of the run: it is an installed copy, and the error says to add the
/// item from its source registry. Otherwise the error names the command that
/// adds the item from the parent directory.
///
/// For a git source (issue #577), `source` is the fetched directory: the
/// subfolder, or the repository root when there is no subfolder or the
/// subfolder normalises to the root. For a subfolder, the error names the
/// command that adds the item from the parent subfolder of the same git
/// source. For the repository root, the error names no command. The caller
/// runs this check after `prepare_install`, so with item names given that
/// match nothing, `add` fails before it with the "no matching items or
/// bundles" error.
pub(super) fn refuse_item_dir_source(
    source: &Path,
    names: &[String],
    source_kind: ItemDirSource<'_>,
) -> Result<()> {
    if !is_item_dir(source)
        || !scan_source_items(source)?.is_empty()
        || holds_bundle_file(source)
        || names
            .iter()
            .any(|n| find_bundle_by_name(source, n).is_some())
    {
        return Ok(());
    }
    let source = normalize_lexically(source);
    let shown = match &source_kind {
        ItemDirSource::Local { .. } => source.display().to_string(),
        ItemDirSource::Git(git) => git.typed.clone(),
    };
    if source.join("COMMAND.md").is_file()
        && !["RULE.md", "SKILL.md", "AGENT.md"]
            .iter()
            .any(|entry| source.join(entry).is_file())
    {
        anyhow::bail!(
            "source {shown} is a command item directory (it holds COMMAND.md); \
             `upskill add` does not install command items. Use `upskill pack` to build a plugin that holds it"
        );
    }
    if source.join("HOOK.md").is_file()
        && !["RULE.md", "SKILL.md", "AGENT.md", "COMMAND.md"]
            .iter()
            .any(|entry| source.join(entry).is_file())
    {
        anyhow::bail!(
            "source {shown} is a hook item directory (it holds HOOK.md); \
             `upskill add` does not install hook items. Use `upskill pack` to build a plugin that holds it"
        );
    }
    let (entry, kind) = if source.join("SKILL.md").is_file() {
        ("SKILL.md", ItemKind::Skill)
    } else if source.join("RULE.md").is_file() {
        ("RULE.md", ItemKind::Rule)
    } else {
        ("AGENT.md", ItemKind::Agent)
    };
    if let ItemDirSource::Git(git) = &source_kind {
        if git.subfolder.is_empty() {
            anyhow::bail!(
                "source {shown}: the repository root is one item directory (it holds {entry}). \
                 upskill installs items from the directory that contains the item directories"
            );
        }
    }
    let head = format!(
        "source {shown} is an item directory (it holds {entry}); a source must be the directory \
         that contains the item directories."
    );
    let (Some(parent), Some(folder)) =
        (source.parent(), source.file_name().and_then(|n| n.to_str()))
    else {
        anyhow::bail!("{head} The file system root has no parent directory to add it from");
    };
    let name = probe_effective_name(&source.join(entry), kind, folder);
    let parent_arg = match source_kind {
        ItemDirSource::Local { inside_output } => {
            if inside_output(kind, folder) || inside_output(kind, &name) {
                anyhow::bail!(
                    "{head} {} is an output path that upskill installs, not a source. Add the \
                     item from the source registry that it was installed from",
                    source.display()
                );
            }
            parent.display().to_string()
        }
        ItemDirSource::Git(git) => git.with_parent_subfolder(),
    };
    anyhow::bail!(
        "{head} Use instead: upskill add {} {}",
        shell_quote(&parent_arg),
        shell_quote(&name)
    )
}

/// The kind of `add` source that [`refuse_item_dir_source`] checks.
pub(super) enum ItemDirSource<'a> {
    /// A local directory. `inside_output` tells whether the item with that
    /// kind and name has its output path at the directory.
    Local {
        inside_output: &'a dyn Fn(ItemKind, &str) -> bool,
    },
    /// A git source; the checked directory is its fetched subfolder or root.
    Git(GitSourceForm),
}

/// A git source in the form that `add` accepts: `owner/repo` for GitHub,
/// the URL otherwise, with its ref.
pub(super) struct GitSourceForm {
    /// The source as `add` accepts it, with its subfolder as given.
    typed: String,
    /// The source without its subfolder, with its ref.
    base: String,
    /// The subfolder after lexical normalisation: no empty, `.` or `..`
    /// component. Empty for the repository root.
    subfolder: Vec<String>,
}

impl GitSourceForm {
    /// `None` for a local path.
    pub(super) fn of(source: &crate::source::InstallSource) -> Option<Self> {
        use crate::source::InstallSource;
        let (mut base, git_ref, sub) = match source {
            InstallSource::Github(r) => (
                format!("{}/{}", r.owner, r.name),
                r.git_ref.as_deref(),
                r.subfolder.as_deref(),
            ),
            InstallSource::Git(r) => (r.url.clone(), r.git_ref.as_deref(), r.subfolder.as_deref()),
            InstallSource::LocalPath(_) => return None,
        };
        if let Some(g) = git_ref {
            base.push('@');
            base.push_str(g);
        }
        let typed = match sub {
            Some(s) => format!("{base}:{s}"),
            None => base.clone(),
        };
        let mut subfolder: Vec<String> = Vec::new();
        for part in sub.unwrap_or("").split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    subfolder.pop();
                }
                other => subfolder.push(other.to_string()),
            }
        }
        Some(Self {
            typed,
            base,
            subfolder,
        })
    }

    /// The source with the parent of its normalised subfolder; no subfolder
    /// when the parent is the repository root.
    fn with_parent_subfolder(&self) -> String {
        let parent = &self.subfolder[..self.subfolder.len().saturating_sub(1)];
        if parent.is_empty() {
            self.base.clone()
        } else {
            format!("{}:{}", self.base, parent.join("/"))
        }
    }
}

/// True when `dir` holds a file whose name ends in `.bundle.yaml`.
fn holds_bundle_file(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| entries.flatten().any(|e| is_bundle_file(&e.path())))
}

/// Remove `.` and `..` components without reading the file system.
pub(crate) fn normalize_lexically(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push(component);
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Quote `word` for a POSIX shell with single quotes when it holds a
/// character other than a letter, a digit or one of `_-./:@%+=,~` (on
/// Windows also `\\`, the path separator).
pub(super) fn shell_quote(word: &str) -> String {
    let plain = !word.is_empty()
        && word.chars().all(|c| {
            c.is_ascii_alphanumeric() || "_-./:@%+=,~".contains(c) || (cfg!(windows) && c == '\\')
        });
    if plain {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

pub(super) fn is_bundle_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(crate::parse::bundle::BUNDLE_SUFFIX))
}

/// Search `root` recursively for a file named `<name>.bundle.yaml`.
/// Skips hidden directories. Returns the first match or `None`.
pub(super) fn find_bundle_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    let target_filename = format!("{}{}", name, crate::parse::bundle::BUNDLE_SUFFIX);
    find_bundle_recursive(root, &target_filename)
}

/// As [`find_bundle_by_name`], but a directory that cannot be read is an
/// error (a missing one is not), so that a bundle is reported absent only
/// after a complete search.
pub(super) fn try_find_bundle_by_name(root: &Path, name: &str) -> Result<Option<PathBuf>> {
    let target = format!("{}{}", name, crate::parse::bundle::BUNDLE_SUFFIX);
    try_find_bundle_recursive(root, &target)
}

fn try_find_bundle_recursive(dir: &Path, target: &str) -> Result<Option<PathBuf>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("read_dir {}", dir.display())),
    };
    for entry in entries {
        let entry = entry.with_context(|| format!("read_dir {}", dir.display()))?;
        let entry_name = entry.file_name();
        let name_str = entry_name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        // `file_type()` does not follow symlinks, so a symlink loop ends here;
        // its error is returned, not read as "not a file".
        let file_type = entry
            .file_type()
            .with_context(|| format!("read the type of {}", path.display()))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_file() && name_str == target {
            return Ok(Some(path));
        }
        if file_type.is_dir() {
            if let Some(found) = try_find_bundle_recursive(&path, target)? {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

fn find_bundle_recursive(dir: &Path, target: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let entry_name = entry.file_name();
        let name_str = entry_name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_file() && name_str == target {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_bundle_recursive(&path, target) {
                return Some(found);
            }
        }
    }
    None
}

/// True when `root` holds at least one `*.bundle.yaml` file outside hidden
/// directories. Symbolic links are not followed or counted.
pub(super) fn has_any_bundle_file(root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            return false;
        }
        // `file_type()` does not follow symlinks, so a symlink loop ends here.
        match entry.file_type() {
            Ok(ft) if ft.is_dir() => has_any_bundle_file(&entry.path()),
            Ok(ft) if ft.is_symlink() => false,
            _ => name.ends_with(crate::parse::bundle::BUNDLE_SUFFIX),
        }
    })
}

/// Check whether a source directory contains any item (skill, rule, or
/// agent) with the given name. An item exists when a subdirectory named
/// `name` contains at least one of `SKILL.md`, `RULE.md`, or `AGENT.md`.
pub(super) fn has_matching_items(source: &Path, name: &str) -> bool {
    let item_dir = source.join(name);
    if !item_dir.is_dir() {
        return false;
    }
    item_dir.join("SKILL.md").is_file()
        || item_dir.join("RULE.md").is_file()
        || item_dir.join("AGENT.md").is_file()
}

/// Scan a source directory for item (kind, name) pairs without generating output.
/// A `source` that is not a directory (a bundle file) has no items. A
/// directory that cannot be listed is an error, and so is a kind directory
/// in it that cannot be listed, or a category directory that cannot be
/// listed when `source` is a kind directory (see [`read_category_dir`]).
pub(super) fn scan_source_items(source: &Path) -> Result<Vec<(ItemKind, String)>> {
    scan_items(source, false)
}

/// As [`scan_source_items`], but any directory that cannot be read is an
/// error (a missing one is not). `update` uses it, because an item that the
/// scan misses is removed as no longer in the source.
pub(super) fn scan_source_items_strict(source: &Path) -> Result<Vec<(ItemKind, String)>> {
    scan_items(source, true)
}

/// `path.is_dir()`; with `strict`, a metadata error other than not-found is
/// returned instead of read as "not a directory".
fn check_dir(path: &Path, strict: bool) -> Result<bool> {
    if !strict {
        return Ok(path.is_dir());
    }
    match fs::metadata(path) {
        Ok(m) => Ok(m.is_dir()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("inspect {}", path.display())),
    }
}

/// As [`check_dir`], for a regular file.
fn check_file(path: &Path, strict: bool) -> Result<bool> {
    if !strict {
        return Ok(path.is_file());
    }
    match fs::metadata(path) {
        Ok(m) => Ok(m.is_file()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("inspect {}", path.display())),
    }
}

/// [`is_item_dir`], strict as [`check_dir`].
///
/// With `strict`, every entrypoint is checked, not only up to the first one
/// that exists: a metadata error of a later entrypoint (for example a
/// symlink loop that replaced `SKILL.md`) must not be hidden by an earlier
/// readable one.
fn has_entrypoint(path: &Path, strict: bool) -> Result<bool> {
    let mut found = false;
    for entry in ENTRYPOINTS {
        if check_file(&path.join(entry), strict)? {
            found = true;
            if !strict {
                break;
            }
        }
    }
    Ok(found)
}

/// [`probe_effective_name`]; with `strict`, a read error of the entrypoint is
/// returned instead of falling back to the folder name.
fn probe_name(entry_path: &Path, kind: ItemKind, folder: &str, strict: bool) -> Result<String> {
    if !strict || kind == ItemKind::Skill {
        return Ok(probe_effective_name(entry_path, kind, folder));
    }
    let raw =
        fs::read_to_string(entry_path).with_context(|| format!("read {}", entry_path.display()))?;
    name_from_content_strict(&raw, folder)
        .with_context(|| format!("parse {}", entry_path.display()))
}

fn scan_items(source: &Path, strict: bool) -> Result<Vec<(ItemKind, String)>> {
    let mut items = Vec::new();
    if !check_dir(source, strict)? {
        return Ok(items);
    }
    let entries = fs::read_dir(source).with_context(|| format!("read_dir {}", source.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("read_dir {}", source.display()))?;
        let path = entry.path();
        if !check_dir(&path, strict)? {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if has_entrypoint(&path, strict)? {
            if check_file(&path.join("SKILL.md"), strict)? {
                items.push((ItemKind::Skill, name.to_string()));
            }
            if check_file(&path.join("RULE.md"), strict)? {
                items.push((
                    ItemKind::Rule,
                    probe_name(&path.join("RULE.md"), ItemKind::Rule, name, strict)?,
                ));
            }
            if check_file(&path.join("AGENT.md"), strict)? {
                items.push((
                    ItemKind::Agent,
                    probe_name(&path.join("AGENT.md"), ItemKind::Agent, name, strict)?,
                ));
            }
        } else {
            // Category subdir: source/<category>/<item>/ENTRY.md
            if let Some(sub_entries) = read_category_dir(&path, source, strict)? {
                for sub_entry in sub_entries {
                    let sub_entry =
                        sub_entry.with_context(|| format!("read_dir {}", path.display()))?;
                    let sub_path = sub_entry.path();
                    if !check_dir(&sub_path, strict)? {
                        continue;
                    }
                    let Some(sub_name) = sub_path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if sub_name.starts_with('.') {
                        continue;
                    }
                    if check_file(&sub_path.join("SKILL.md"), strict)? {
                        items.push((ItemKind::Skill, sub_name.to_string()));
                    }
                    if check_file(&sub_path.join("RULE.md"), strict)? {
                        items.push((
                            ItemKind::Rule,
                            probe_name(
                                &sub_path.join("RULE.md"),
                                ItemKind::Rule,
                                sub_name,
                                strict,
                            )?,
                        ));
                    }
                    if check_file(&sub_path.join("AGENT.md"), strict)? {
                        items.push((
                            ItemKind::Agent,
                            probe_name(
                                &sub_path.join("AGENT.md"),
                                ItemKind::Agent,
                                sub_name,
                                strict,
                            )?,
                        ));
                    }
                }
            }
        }
    }
    Ok(items)
}

/// Walk up from `bundle_path`'s parent until a directory is found that
/// looks like an SSOT root — a directory whose direct children include
/// at least one item directory (containing `RULE.md`, `SKILL.md`, `AGENT.md`,
/// `COMMAND.md`, or `HOOK.md`) or another bundle file. Falls back to the bundle's
/// parent directory if no such ancestor exists, so a flat layout
/// (bundle and items in the same dir) still works. The walk fails, instead
/// of continuing, when a directory on it holds a kind directory that cannot
/// be listed (see [`has_ssot_layout`]).
pub(super) fn find_registry_root(bundle_path: &Path) -> Result<PathBuf> {
    let parent = bundle_path
        .parent()
        .ok_or_else(|| anyhow!("bundle path {} has no parent", bundle_path.display()))?;
    let mut cursor = parent;
    loop {
        if has_ssot_layout(cursor)? {
            return Ok(cursor.to_path_buf());
        }
        match cursor.parent() {
            Some(p) => cursor = p,
            None => break,
        }
    }
    Ok(parent.to_path_buf())
}

/// Whether `dir` holds an item directory as a child or grandchild. A `dir`
/// that cannot be opened is not an SSOT root (`find_registry_root` walks up
/// through directories that may be unreadable). Once `dir` is open, every
/// child is checked before the result is returned, so the result does not
/// depend on the listing order: an entry that cannot be read, a kind
/// directory that cannot be listed, or a category directory of a kind
/// directory `dir` that cannot be listed is an error (see
/// [`read_category_dir`]).
pub(super) fn has_ssot_layout(dir: &Path) -> Result<bool> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(false);
    };
    let mut found = false;
    for entry in entries {
        let entry = entry.with_context(|| format!("read_dir {}", dir.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Direct item check: dir/<item>/ENTRY.md
        if is_item_dir(&path) {
            found = true;
            continue;
        }
        // Grandchild check: dir/<category>/<item>/ENTRY.md
        // Handles sibling layouts where items live in subdirectories
        // (e.g. `skills/<item>/RULE.md` alongside `bundles/`).
        if let Some(sub_entries) = read_category_dir(&path, dir, false)? {
            for sub_entry in sub_entries {
                let sub_entry =
                    sub_entry.with_context(|| format!("read_dir {}", path.display()))?;
                let sub_path = sub_entry.path();
                if sub_path.is_dir() && is_item_dir(&sub_path) {
                    found = true;
                }
            }
        }
    }
    Ok(found)
}

/// Detect which entrypoint file (SKILL.md, RULE.md, AGENT.md) exists for
/// an item, for use in error messages.
pub(super) fn detect_item_entrypoint(source: &Path, name: &str) -> &'static str {
    let dir = source.join(name);
    if dir.join("SKILL.md").is_file() {
        "SKILL.md"
    } else if dir.join("RULE.md").is_file() {
        "RULE.md"
    } else if dir.join("AGENT.md").is_file() {
        "AGENT.md"
    } else {
        "SKILL.md" // fallback for error message
    }
}

/// Read the entries of `dir`, a directory under `parent` that is not an
/// item directory. A read error is returned when `dir` is a kind directory
/// (`skills/`, `rules/`, `agents/`), or a category directory of a `parent`
/// that is itself a kind directory. Any other unreadable directory holds no
/// items by convention: `Ok(None)`.
/// With `strict`, an error other than not-found is returned for every
/// directory.
fn read_category_dir(dir: &Path, parent: &Path, strict: bool) -> Result<Option<fs::ReadDir>> {
    match fs::read_dir(dir) {
        Ok(entries) => Ok(Some(entries)),
        Err(e) if strict && e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) if !strict && !is_kind_dir(dir) && !is_kind_dir(parent) => Ok(None),
        Err(e) => Err(e).with_context(|| format!("read_dir {}", dir.display())),
    }
}

/// Whether the last component of `path` is `skills`, `rules` or `agents`,
/// the kind directory names of the recommended layout (conventions.md).
fn is_kind_dir(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|n| ["skills", "rules", "agents"].iter().any(|k| n == *k))
}

/// Iterate `(name, dir)` for every immediate subdirectory of `kind_root`.
/// Returns an empty iterator when the kind root does not exist (treating
/// "no items of this kind" as a non-error). A kind directory under
/// `kind_root` that cannot be listed is an error, and so is a category
/// directory that cannot be listed when `kind_root` is a kind directory
/// (see [`is_kind_dir`]); other unreadable directories are skipped.
pub(super) fn iter_item_dirs(kind_root: &Path) -> Result<Vec<(String, PathBuf)>> {
    walk_item_dirs(kind_root, false)
}

/// The same traversal as [`iter_item_dirs`], hidden directories included,
/// with a fallible check of every directory, child and entrypoint: every
/// error except not-found is returned. `update` runs it before it decides
/// that a recorded item is no longer in its source, so that a directory that
/// the lenient walk cannot inspect never makes an item look absent.
pub(super) fn iter_item_dirs_strict(kind_root: &Path) -> Result<Vec<(String, PathBuf)>> {
    walk_item_dirs(kind_root, true)
}

fn walk_item_dirs(kind_root: &Path, strict: bool) -> Result<Vec<(String, PathBuf)>> {
    let exists = if strict {
        match fs::metadata(kind_root) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => {
                return Err(e).with_context(|| format!("inspect {}", kind_root.display()));
            }
        }
    } else {
        kind_root.exists()
    };
    if !exists {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in
        fs::read_dir(kind_root).with_context(|| format!("read_dir {}", kind_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !check_dir(&path, strict)? {
            continue;
        }
        if has_entrypoint(&path, strict)? {
            // Direct item: kind_root/<item>/ENTRY.md
            let name = entry
                .file_name()
                .to_str()
                .map(str::to_owned)
                .with_context(|| format!("non-UTF8 name in {}", kind_root.display()))?;
            out.push((name, path));
        } else {
            // Category subdir: kind_root/<category>/<item>/ENTRY.md
            // Descend one level to find items in subdirectories (format-spec §2.2).
            let Some(sub_entries) = read_category_dir(&path, kind_root, strict)? else {
                continue;
            };
            for sub_entry in sub_entries {
                let sub_entry =
                    sub_entry.with_context(|| format!("read_dir {}", path.display()))?;
                let sub_path = sub_entry.path();
                if check_dir(&sub_path, strict)? && has_entrypoint(&sub_path, strict)? {
                    let name = sub_entry
                        .file_name()
                        .to_str()
                        .map(str::to_owned)
                        .with_context(|| format!("non-UTF8 name in {}", path.display()))?;
                    out.push((name, sub_path));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// The first item directory named `name` that holds a `COMMAND.md`.
pub(super) fn find_command_dir(root: &Path, name: &str) -> Result<Option<PathBuf>> {
    Ok(iter_item_dirs(root)?
        .into_iter()
        .find(|(folder, dir)| folder == name && dir.join("COMMAND.md").is_file())
        .map(|(_, dir)| dir))
}

/// The first item directory whose effective hook name is `name` and that
/// holds a `HOOK.md`. Hooks use the same optional name rule as rules.
pub(super) fn find_hook_dir(root: &Path, name: &str) -> Result<Option<PathBuf>> {
    Ok(iter_item_dirs(root)?
        .into_iter()
        .find(|(folder, dir)| {
            let entry = dir.join("HOOK.md");
            entry.is_file() && probe_effective_name(&entry, ItemKind::Rule, folder) == name
        })
        .map(|(_, dir)| dir))
}

/// Relative paths (from `dir`) of every file under an item directory that
/// is not an entrypoint (`SKILL.md`, `RULE.md`, `AGENT.md`, `COMMAND.md`, or
/// `HOOK.md`). A file such as `SKILL.claude.md` is an ordinary resource
/// (format-spec §2.3).
/// Recursive; sub-directory structure is preserved in the returned relative paths.
/// Sorted for deterministic output.
pub(super) fn iter_item_resources(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = walk_files(dir)
        .into_iter()
        .filter(|path| {
            // Entrypoints only ever sit at the top level of an item
            // directory; a nested file with the same name is content.
            let top_level = path.parent() == Some(dir);
            let fname = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            !(top_level && is_entrypoint(fname))
        })
        .filter_map(|path| path.strip_prefix(dir).ok().map(Path::to_path_buf))
        .collect();
    out.sort();
    out
}

/// Best-effort recursive discovery of non-directory, non-symlink entries.
/// This can include special entries rather than only regular files. Directory
/// enumeration, iterator and file-type errors are silently skipped. Observed
/// symlinks are skipped before recursion; this does not qualify containment or
/// concurrent replacement. Order is filesystem-dependent; callers sort when
/// needed. Shared by [`iter_item_resources`] and `hash::hash_item_dir`.
pub(super) fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_files_into(dir, &mut out);
    out
}

fn walk_files_into(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        // `file_type()` does not follow symlinks.
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            walk_files_into(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// True when `fname` is one of the entrypoint file names
/// (`SKILL.md`, `RULE.md`, `AGENT.md`, `COMMAND.md`, `HOOK.md`).
fn is_entrypoint(fname: &str) -> bool {
    ENTRYPOINTS.contains(&fname)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_hook_dir_matches_the_frontmatter_name_or_the_folder() {
        let tmp = tempfile::tempdir().unwrap();
        for rel in ["flat", "category/nested"] {
            let dir = tmp.path().join(rel);
            fs::create_dir_all(&dir).unwrap();
            let folder = dir.file_name().unwrap().to_str().unwrap().to_owned();
            fs::write(
                dir.join("HOOK.md"),
                "---\ndescription: Test hook\nevents: { session-start: null }\n---\nText.\n",
            )
            .unwrap();
            assert_eq!(
                find_hook_dir(tmp.path(), &folder).unwrap(),
                Some(dir.clone())
            );
            let effective = format!("other-{folder}");
            fs::write(
                dir.join("HOOK.md"),
                format!("---\nname: {effective}\ndescription: Test hook\nevents: {{ session-start: null }}\n---\nText.\n"),
            )
            .unwrap();
            assert_eq!(find_hook_dir(tmp.path(), &effective).unwrap(), Some(dir));
            assert_eq!(find_hook_dir(tmp.path(), &folder).unwrap(), None);
        }
        let skill = tmp.path().join("skill");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "Skill").unwrap();
        assert_eq!(find_hook_dir(tmp.path(), "skill").unwrap(), None);
        assert_eq!(find_hook_dir(tmp.path(), "missing").unwrap(), None);
        let broken = tmp.path().join("broken");
        fs::create_dir_all(&broken).unwrap();
        fs::write(broken.join("HOOK.md"), "---\nname: [\n---\n").unwrap();
        assert_eq!(
            find_hook_dir(tmp.path(), "broken").unwrap(),
            Some(broken.clone())
        );
        fs::write(broken.join("HOOK.md"), [0xff]).unwrap();
        assert_eq!(find_hook_dir(tmp.path(), "broken").unwrap(), Some(broken));
    }

    #[test]
    fn is_item_dir_is_true_for_a_hook_only_directory() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_item_dir(tmp.path()));
        fs::write(tmp.path().join("HOOK.md"), "Hook").unwrap();
        assert!(is_item_dir(tmp.path()));
    }

    #[test]
    fn iter_item_resources_excludes_hook_md() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("nested")).unwrap();
        for name in ["HOOK.md", "SKILL.md", "template.md", "nested/HOOK.md"] {
            fs::write(tmp.path().join(name), "Content").unwrap();
        }
        assert_eq!(
            iter_item_resources(tmp.path()),
            vec![
                PathBuf::from("nested/HOOK.md"),
                PathBuf::from("template.md")
            ]
        );
    }

    #[test]
    fn find_command_dir_finds_flat_and_category_layouts() {
        let tmp = tempfile::tempdir().unwrap();
        for rel in ["flat", "category/nested"] {
            let dir = tmp.path().join(rel);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("COMMAND.md"), "Command").unwrap();
            let name = dir.file_name().unwrap().to_str().unwrap();
            assert_eq!(find_command_dir(tmp.path(), name).unwrap(), Some(dir));
        }
        let skill = tmp.path().join("skill");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "Skill").unwrap();
        assert_eq!(find_command_dir(tmp.path(), "skill").unwrap(), None);
        assert_eq!(find_command_dir(tmp.path(), "missing").unwrap(), None);
    }

    #[test]
    fn is_item_dir_is_true_for_a_command_only_directory() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_item_dir(tmp.path()));
        fs::write(tmp.path().join("COMMAND.md"), "Command").unwrap();
        assert!(is_item_dir(tmp.path()));
    }

    #[test]
    fn iter_item_resources_excludes_command_md() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("nested")).unwrap();
        for name in ["COMMAND.md", "RULE.md", "template.md", "nested/COMMAND.md"] {
            fs::write(tmp.path().join(name), "Content").unwrap();
        }
        assert_eq!(
            iter_item_resources(tmp.path()),
            vec![
                PathBuf::from("nested/COMMAND.md"),
                PathBuf::from("template.md")
            ]
        );
    }

    #[test]
    fn find_bundle_by_name_finds_nested_bundle_file() {
        let tmp = tempfile::tempdir().unwrap();
        let bundles_dir = tmp.path().join("bundles");
        std::fs::create_dir_all(&bundles_dir).unwrap();
        std::fs::write(
            bundles_dir.join("baseline.bundle.yaml"),
            "schema: 1\nname: baseline\ndescription: test\nitems:\n  rules: []\n",
        )
        .unwrap();

        let result = find_bundle_by_name(tmp.path(), "baseline");
        assert_eq!(result, Some(bundles_dir.join("baseline.bundle.yaml")));
    }

    #[test]
    fn find_bundle_by_name_returns_none_when_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("skills/foo")).unwrap();
        std::fs::write(
            tmp.path().join("skills/foo/SKILL.md"),
            "---\nschema: 1\nname: foo\n---\n# body\n",
        )
        .unwrap();

        let result = find_bundle_by_name(tmp.path(), "foo");
        assert!(result.is_none());
    }

    #[test]
    fn find_bundle_by_name_skips_hidden_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let hidden = tmp.path().join(".hidden");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(
            hidden.join("secret.bundle.yaml"),
            "schema: 1\nname: secret\ndescription: x\nitems:\n  rules: []\n",
        )
        .unwrap();

        let result = find_bundle_by_name(tmp.path(), "secret");
        assert!(result.is_none());
    }

    #[test]
    fn has_matching_items_true_when_skill_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("code-review");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nschema: 1\nname: code-review\n---\n# body\n",
        )
        .unwrap();

        assert!(has_matching_items(tmp.path(), "code-review"));
    }

    #[test]
    fn has_matching_items_false_when_no_item_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("other")).unwrap();

        assert!(!has_matching_items(tmp.path(), "nonexistent"));
    }

    #[test]
    fn has_matching_items_true_for_rules_and_agents() {
        let tmp = tempfile::tempdir().unwrap();
        let rule_dir = tmp.path().join("my-rule");
        std::fs::create_dir_all(&rule_dir).unwrap();
        std::fs::write(
            rule_dir.join("RULE.md"),
            "---\nschema: 1\nname: my-rule\n---\n# body\n",
        )
        .unwrap();

        assert!(has_matching_items(tmp.path(), "my-rule"));
    }

    #[test]
    fn has_ssot_layout_detects_direct_children() {
        // Flat layout: root/<item>/RULE.md
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("my-rule");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("RULE.md"), "").unwrap();

        assert!(has_ssot_layout(tmp.path()).unwrap());
    }

    #[test]
    fn has_ssot_layout_detects_grandchild_entrypoints() {
        // Sibling layout: root/skills/<item>/RULE.md
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("skills/my-rule");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("RULE.md"), "").unwrap();

        assert!(
            has_ssot_layout(tmp.path()).unwrap(),
            "has_ssot_layout must detect items nested one level deeper"
        );
    }

    #[test]
    fn has_ssot_layout_returns_false_for_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!has_ssot_layout(tmp.path()).unwrap());
    }

    #[test]
    fn find_registry_root_returns_parent_for_sibling_layout() {
        // registry/bundles/x.bundle.yaml + registry/skills/<item>/RULE.md
        // → find_registry_root must return registry/
        let tmp = tempfile::tempdir().unwrap();
        let registry = tmp.path().join("registry");
        std::fs::create_dir_all(registry.join("bundles")).unwrap();
        std::fs::create_dir_all(registry.join("skills/my-rule")).unwrap();
        std::fs::write(registry.join("skills/my-rule/RULE.md"), "").unwrap();
        let bundle = registry.join("bundles/test.bundle.yaml");
        std::fs::write(&bundle, "").unwrap();

        let root = find_registry_root(&bundle).unwrap();
        assert_eq!(root, registry);
    }

    #[test]
    fn iter_item_dirs_finds_items_in_category_subdirs() {
        // registry/skills/<item>/SKILL.md should be discovered
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("skills/my-skill");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("SKILL.md"), "").unwrap();

        let dirs = iter_item_dirs(tmp.path()).unwrap();
        let names: Vec<&str> = dirs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"my-skill"),
            "iter_item_dirs must find items in category subdirectories: {names:?}"
        );
    }

    #[test]
    fn iter_item_dirs_still_finds_direct_children() {
        // Flat layout: root/<item>/RULE.md must still work
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("my-rule");
        std::fs::create_dir_all(&item).unwrap();
        std::fs::write(item.join("RULE.md"), "").unwrap();

        let dirs = iter_item_dirs(tmp.path()).unwrap();
        let names: Vec<&str> = dirs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"my-rule"),
            "iter_item_dirs must still find direct item children: {names:?}"
        );
    }

    #[test]
    fn iter_item_resources_excludes_only_entrypoints() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let mut expected = Vec::new();
        for stem in ["SKILL", "RULE", "AGENT"] {
            // Top-level entrypoints are excluded.
            std::fs::write(dir.join(format!("{stem}.md")), "x").unwrap();
            // Every <KIND>.<client>.md name, known or unknown client, is a
            // resource.
            for client in ["claude", "copilot", "opencode", "cursor"] {
                let fname = format!("{stem}.{client}.md");
                std::fs::write(dir.join(&fname), "x").unwrap();
                expected.push(std::path::PathBuf::from(fname));
            }
        }
        std::fs::write(dir.join("notes.claude.md"), "x").unwrap();
        expected.push(std::path::PathBuf::from("notes.claude.md"));
        // A nested file with an entrypoint name is content.
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/gate.sh"), "x").unwrap();
        std::fs::write(dir.join("scripts/SKILL.md"), "x").unwrap();
        expected.push(std::path::PathBuf::from("scripts/SKILL.md"));
        expected.push(std::path::PathBuf::from("scripts/gate.sh"));
        expected.sort();

        assert_eq!(
            iter_item_resources(dir),
            expected,
            "only top-level entrypoints are excluded"
        );
    }

    #[test]
    fn iter_item_resources_empty_when_only_entrypoint() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("SKILL.md"), "x").unwrap();
        assert!(iter_item_resources(tmp.path()).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn iter_item_resources_does_not_loop_on_symlink_cycle() {
        // A directory symlink pointing back at the item root forms a cycle;
        // walking it with a symlink-following `is_dir()` would recurse until
        // the stack overflows. The walk must terminate and skip the symlink.
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join("SKILL.md"), "x").unwrap();
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/gate.sh"), "x").unwrap();
        std::os::unix::fs::symlink(dir, dir.join("scripts/loop")).unwrap();

        let res = iter_item_resources(dir); // must terminate
        assert!(res.contains(&std::path::PathBuf::from("scripts/gate.sh")));
        assert!(
            !res.iter().any(|p| p.to_string_lossy().contains("loop")),
            "the symlinked directory must be skipped, not traversed: {res:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn has_any_bundle_file_ends_on_a_symlink_loop_and_ignores_links() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::os::unix::fs::symlink(dir, dir.join("sub/loop")).unwrap();
        assert!(!has_any_bundle_file(dir));
        std::fs::write(dir.join("sub/x.bundle.yaml"), "x").unwrap();
        assert!(has_any_bundle_file(dir));
    }

    #[cfg(unix)]
    #[test]
    fn try_find_bundle_by_name_ends_on_a_symlink_loop() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::os::unix::fs::symlink(dir, dir.join("sub/loop")).unwrap();
        assert!(try_find_bundle_by_name(dir, "x").unwrap().is_none());
        std::fs::write(dir.join("sub/x.bundle.yaml"), "x").unwrap();
        assert!(try_find_bundle_by_name(dir, "x").unwrap().is_some());
    }

    #[test]
    fn name_from_content_reads_the_frontmatter_name_or_the_folder() {
        assert_eq!(
            name_from_content("---\nname: policy\n---\nB\n", "folder"),
            "policy"
        );
        assert_eq!(
            name_from_content("---\ndescription: d\n---\nB\n", "folder"),
            "folder"
        );
        assert_eq!(name_from_content("no frontmatter", "folder"), "folder");
        // The strict form reports a frontmatter that does not parse.
        assert!(name_from_content_strict("---\nname: [x\n---\nB\n", "folder").is_err());
        assert!(name_from_content_strict("no frontmatter", "folder").is_err());
        assert_eq!(
            name_from_content_strict("---\nname: policy\n---\nB\n", "folder").unwrap(),
            "policy"
        );
    }

    /// `update` decides "no longer in source" from the strict scan, and plans
    /// the install from `iter_item_dirs`. Both walks must find the same item
    /// directories in every supported layout; a change to one walk must fail
    /// here.
    #[test]
    fn strict_scan_and_plan_walk_find_the_same_items() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let put = |rel: &str, file: &str| {
            let dir = root.join(rel);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(file), "---\nschema: 1\ndescription: d\n---\nB\n").unwrap();
        };
        put("root-skill", "SKILL.md");
        put("root-rule", "RULE.md");
        put("root-agent", "AGENT.md");
        put("cat/in-cat", "SKILL.md");
        put("skills/in-kind", "SKILL.md");
        put("rules/rule-in-kind", "RULE.md");
        put("agents/agent-in-kind", "AGENT.md");
        put("skills/deep/too-deep", "SKILL.md");
        put(".hidden/h", "SKILL.md");
        put("cat/.hidden-item", "SKILL.md");
        let mut scanned: Vec<String> = scan_source_items_strict(root)
            .unwrap()
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        let lenient = iter_item_dirs(root).unwrap();
        // The strict walk is the same traversal, so it finds the same
        // directories, hidden ones included.
        assert_eq!(iter_item_dirs_strict(root).unwrap(), lenient);
        assert!(lenient.iter().any(|(name, _)| name == "h"));
        // The scan skips hidden directories; apart from those it agrees with
        // the plan walk.
        let mut planned: Vec<String> = lenient
            .iter()
            .filter(|(_, dir)| {
                !dir.strip_prefix(root)
                    .unwrap()
                    .components()
                    .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
            })
            .map(|(name, _)| name.clone())
            .collect();
        scanned.sort();
        scanned.dedup();
        planned.sort();
        planned.dedup();
        assert_eq!(scanned, planned);
        assert!(scanned.contains(&"in-cat".to_string()));
        assert!(scanned.contains(&"root-rule".to_string()));
    }

    /// Makes `dir` unreadable; `None` when running as root, where the
    /// permission has no effect. The guard restores the mode on drop.
    #[cfg(unix)]
    fn unreadable(dir: &Path) -> Option<impl Drop + '_> {
        use std::os::unix::fs::PermissionsExt;
        struct Restore<'a>(&'a Path);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(self.0, std::fs::Permissions::from_mode(0o755));
            }
        }
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o000)).unwrap();
        let guard = Restore(dir);
        if std::fs::read_dir(dir).is_ok() {
            eprintln!("skipped: running as root, the permission has no effect");
            return None;
        }
        Some(guard)
    }

    #[cfg(unix)]
    #[test]
    fn find_registry_root_fails_on_an_unreadable_kind_directory() {
        // Issue #610: without the error, the walk continues to the
        // ancestors and returns a wrong root.
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("reg");
        std::fs::create_dir_all(reg.join("skills/one")).unwrap();
        std::fs::write(reg.join("skills/one/SKILL.md"), "").unwrap();
        let bundle = reg.join("bundles/b.bundle.yaml");
        std::fs::create_dir_all(bundle.parent().unwrap()).unwrap();
        std::fs::write(&bundle, "").unwrap();
        let kind_dir = reg.join("skills");
        let Some(_guard) = unreadable(&kind_dir) else {
            return;
        };
        let err = find_registry_root(&bundle).unwrap_err();
        assert!(
            format!("{err:#}").contains(&kind_dir.display().to_string()),
            "{err:#}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn has_ssot_layout_skips_an_unreadable_directory_that_is_not_a_kind_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        let Some(_guard) = unreadable(&docs) else {
            return;
        };
        assert!(!has_ssot_layout(tmp.path()).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn has_ssot_layout_is_false_for_an_unreadable_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("locked");
        std::fs::create_dir_all(&dir).unwrap();
        let Some(_guard) = unreadable(&dir) else {
            return;
        };
        assert!(!has_ssot_layout(&dir).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn scan_source_items_fails_on_an_unreadable_kind_directory() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("skills/one")).unwrap();
        std::fs::write(tmp.path().join("skills/one/SKILL.md"), "").unwrap();
        let kind_dir = tmp.path().join("skills");
        let Some(_guard) = unreadable(&kind_dir) else {
            return;
        };
        let err = scan_source_items(tmp.path()).unwrap_err();
        assert!(
            format!("{err:#}").contains(&kind_dir.display().to_string()),
            "{err:#}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn scan_source_items_skips_an_unreadable_directory_that_is_not_a_kind_directory() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("skills/one")).unwrap();
        std::fs::write(tmp.path().join("skills/one/SKILL.md"), "").unwrap();
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        let Some(_guard) = unreadable(&docs) else {
            return;
        };
        let items = scan_source_items(tmp.path()).unwrap();
        assert_eq!(items, vec![(ItemKind::Skill, "one".to_string())]);
    }

    #[test]
    fn scan_source_items_of_a_bundle_file_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = tmp.path().join("b.bundle.yaml");
        std::fs::write(&bundle, "").unwrap();
        assert!(scan_source_items(&bundle).unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn has_ssot_layout_fails_on_an_unreadable_kind_directory_next_to_an_item() {
        // The item is found whatever the listing order; the unreadable kind
        // directory is still an error.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("one")).unwrap();
        std::fs::write(tmp.path().join("one/SKILL.md"), "").unwrap();
        let kind_dir = tmp.path().join("skills");
        std::fs::create_dir_all(&kind_dir).unwrap();
        let Some(_guard) = unreadable(&kind_dir) else {
            return;
        };
        assert!(has_ssot_layout(tmp.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn has_ssot_layout_fails_on_an_unreadable_category_of_a_kind_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let kind_dir = tmp.path().join("skills");
        let category = kind_dir.join("category");
        std::fs::create_dir_all(category.join("one")).unwrap();
        std::fs::write(category.join("one/SKILL.md"), "").unwrap();
        let Some(_guard) = unreadable(&category) else {
            return;
        };
        let err = has_ssot_layout(&kind_dir).unwrap_err();
        assert!(
            format!("{err:#}").contains(&category.display().to_string()),
            "{err:#}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn scan_source_items_fails_on_an_unreadable_category_of_a_kind_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let kind_dir = tmp.path().join("skills");
        let category = kind_dir.join("category");
        std::fs::create_dir_all(category.join("one")).unwrap();
        std::fs::write(category.join("one/SKILL.md"), "").unwrap();
        let Some(_guard) = unreadable(&category) else {
            return;
        };
        let err = scan_source_items(&kind_dir).unwrap_err();
        assert!(
            format!("{err:#}").contains(&category.display().to_string()),
            "{err:#}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn scan_source_items_fails_on_an_unreadable_source() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("reg");
        std::fs::create_dir_all(&source).unwrap();
        let Some(_guard) = unreadable(&source) else {
            return;
        };
        let err = scan_source_items(&source).unwrap_err();
        assert!(
            format!("{err:#}").contains(&source.display().to_string()),
            "{err:#}"
        );
    }
}
