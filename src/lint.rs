//! `upskill lint` — validate SSOT files against the format spec.
//!
//! Per [ADR-0004](../../docs/adr/0004-cli-surface.md), `lint` is an
//! author command: it runs in source-registry trees only and refuses to
//! run inside a consumer project (detected by `.upskill-lock.json` at
//! the lint root). Each rule has a stable ID so a future
//! `--strict-rule` could opt into individual rules; today `--strict`
//! is global (warnings → errors).
//!
//! Rule set (initial — see format-spec for the canonical statements):
//!
//! | ID                | Severity | Source             |
//! | ----------------- | -------- | ------------------ |
//! | `frontmatter`     | error    | §3                 |
//! | `name-matches-dir`| error    | §2.1               |
//! | `body-h1`         | warning  | §5.1               |
//! | `fence-lang`      | warning  | §5.2               |
//! | `body-format`     | warning  | §3.8 (dprint canonical body) |
//! | `directive`       | error    | §6.3 (any imbalance, unknown client, nesting) |
//! | `name-collision`  | error    | cross-file (bundle vs item name) |
//!
//! Discovery: a path argument is either a file (lint that file) or a
//! directory (walk for `<root>/<name>/{RULE,SKILL,AGENT}.md` per
//! format-spec §2.1, plus any `*.bundle.yaml`). With no paths, the
//! current directory is used.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

use crate::generate::Client;
use crate::generate::directives;
use crate::model::{Agent, Rule, Skill};
use crate::parse::frontmatter;

/// Severity of a [`Finding`]. `--strict` promotes every `Warning` to
/// `Error` at the report level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// One issue found in one file.
#[derive(Debug, Clone)]
pub struct Finding {
    pub rule_id: &'static str,
    pub severity: Severity,
    pub path: PathBuf,
    /// Line number when known (1-indexed). `None` for whole-file
    /// findings.
    pub line: Option<usize>,
    pub message: String,
}

/// Lint result for one or more files.
#[derive(Debug, Default, Clone)]
pub struct LintReport {
    pub findings: Vec<Finding>,
    pub files_checked: usize,
}

impl LintReport {
    /// True when the report contains an error finding (taking the
    /// strict promotion the lint runner already applied into account —
    /// the runner mutates findings in place when `strict`).
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

/// Run lint over the given paths. With an empty `paths` slice, lints
/// the current working directory.
///
/// In strict mode, every warning finding is promoted to error before
/// the report is returned, so callers only have to check
/// [`LintReport::has_errors`].
pub fn lint(paths: &[PathBuf], strict: bool) -> Result<LintReport> {
    let owned_cwd: Vec<PathBuf>;
    let roots: &[PathBuf] = if paths.is_empty() {
        owned_cwd = vec![std::env::current_dir().context("get current directory")?];
        &owned_cwd
    } else {
        paths
    };

    for root in roots {
        if is_consumer_project(root) {
            return Err(anyhow!(
                "{}: refusing to lint — `.upskill-lock.json` indicates this is a consumer project, \
                 not a source registry. Run `upskill lint` inside the SSOT tree instead.",
                root.display()
            ));
        }
    }

    let mut report = LintReport::default();
    for root in roots {
        for file in discover(root)? {
            report.files_checked += 1;
            check_file(&file, &mut report.findings)?;
        }
    }

    // Cross-file checks: bundle-item name collisions.
    for root in roots {
        if root.is_dir() {
            check_bundle_item_name_collisions(root, &mut report.findings)?;
        }
    }

    if strict {
        for finding in &mut report.findings {
            if finding.severity == Severity::Warning {
                finding.severity = Severity::Error;
            }
        }
    }

    Ok(report)
}

/// True when `root/.upskill-lock.json` (or `root` itself, if it is a
/// file in such a directory) exists. Used by author commands (`lint`,
/// `fmt`, `registry build`) to refuse running inside consumer projects.
pub(crate) fn is_consumer_project(root: &Path) -> bool {
    let dir = if root.is_dir() {
        root.to_path_buf()
    } else if let Some(parent) = root.parent() {
        parent.to_path_buf()
    } else {
        return false;
    };
    dir.join(crate::lockfile::LOCKFILE_NAME).is_file()
}

/// Inputs to [`check_file`] — the kind drives both the dir-name
/// validator and the frontmatter parser to use.
#[derive(Debug, Clone, Copy)]
enum FileKind {
    Skill,
    Rule,
    Agent,
    Bundle,
}

impl FileKind {
    fn from_filename(name: &str) -> Option<Self> {
        match name {
            "SKILL.md" => Some(Self::Skill),
            "RULE.md" => Some(Self::Rule),
            "AGENT.md" => Some(Self::Agent),
            n if n.ends_with(crate::parse::bundle::BUNDLE_SUFFIX) => Some(Self::Bundle),
            _ => None,
        }
    }
}

/// Walk `root` (file or directory) and return every entrypoint file
/// the lint should inspect. SSOT items are detected by the
/// `<root>/<name>/<KIND>.md` layout per format-spec §2.1; bundles by
/// the `*.bundle.yaml` suffix. Shared with `crate::fmt`.
///
/// When `root` is itself an item directory (directly contains
/// `RULE.md`/`SKILL.md`/`AGENT.md`), those files are collected
/// directly — fixing the silent false-pass described in issue #159.
pub(crate) fn discover(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if root.is_file() {
        if let Some(name) = root.file_name().and_then(|n| n.to_str())
            && FileKind::from_filename(name).is_some()
        {
            out.push(root.to_path_buf());
        }
        return Ok(out);
    }

    if !root.is_dir() {
        return Err(anyhow!(
            "{}: not a file or directory — nothing to lint",
            root.display()
        ));
    }

    // Check if `root` itself is an item directory (contains entrypoint
    // files directly). This handles `upskill lint skills/my-rule/`.
    discover_self_entrypoints(root, &mut out);

    // If root is an item directory we already collected its entrypoints
    // above; still walk subdirectories so a mixed layout works (e.g. a
    // registry root that also happens to contain a bundle).
    discover_items(root, &mut out)?;
    walk_bundles(root, &mut out)?;
    Ok(out)
}

/// Collect entrypoint files (`RULE.md`, `SKILL.md`, `AGENT.md`) that
/// exist directly inside `dir`. This handles the case where `dir` is
/// itself an item directory rather than a registry root.
fn discover_self_entrypoints(dir: &Path, out: &mut Vec<PathBuf>) {
    for entrypoint in ["RULE.md", "SKILL.md", "AGENT.md"] {
        let path = dir.join(entrypoint);
        if path.is_file() {
            out.push(path);
        }
    }
}

/// Walk one level deep under `root`, collecting every `RULE.md`,
/// `SKILL.md`, and `AGENT.md` found in a subdirectory. Hidden
/// directories are skipped. A single item directory may contribute
/// more than one entrypoint when it holds multiple co-located kinds
/// (format-spec §2.1).
fn discover_items(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)
        .with_context(|| format!("read {}", root.display()))?
        .flatten()
    {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }
        for entrypoint in ["RULE.md", "SKILL.md", "AGENT.md"] {
            let entry_path = path.join(entrypoint);
            if entry_path.is_file() {
                out.push(entry_path);
            }
        }
    }
    Ok(())
}

/// Recurse into `root` looking for `*.bundle.yaml` files. Skips hidden
/// directories (`.git`, `.upskill-…`, etc.) so the scan stays cheap.
fn walk_bundles(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(root).with_context(|| format!("read {}", root.display()))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            walk_bundles(&path, out)?;
        } else if path.is_file()
            && let Some(filename) = path.file_name().and_then(|n| n.to_str())
            && filename.ends_with(crate::parse::bundle::BUNDLE_SUFFIX)
        {
            out.push(path);
        }
    }
    Ok(())
}

/// Run every applicable rule against `file` and append findings to
/// `out`.
fn check_file(file: &Path, out: &mut Vec<Finding>) -> Result<()> {
    let raw = fs::read_to_string(file).with_context(|| format!("read {}", file.display()))?;
    let kind = file
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(FileKind::from_filename)
        .ok_or_else(|| anyhow!("{}: unknown entrypoint filename", file.display()))?;

    let body = match parse_kind(&raw, kind) {
        Ok((parsed_name, body)) => {
            check_name_matches_dir(file, parsed_name.as_deref(), kind, out);
            body
        }
        Err(err) => {
            out.push(Finding {
                rule_id: "frontmatter",
                severity: Severity::Error,
                path: file.to_path_buf(),
                line: None,
                message: format!("{:#}", err),
            });
            // Without a parse, skip body-only rules — they would just
            // double-report on the same file.
            return Ok(());
        }
    };

    check_body_h1(file, body, out);
    check_fence_lang(file, body, out);
    check_directives(file, body, out);
    let frontmatter = frontmatter::split(&raw).map(|(fm, _)| fm).unwrap_or("");
    check_body_format(file, frontmatter, body, out);
    Ok(())
}

/// Parse the entrypoint for the kind, returning `(name, body)`. Items
/// are frontmatter+body; bundles are pure YAML with no body (§2.2,
/// ADR-0007), so the bundle arm returns an empty body so the
/// body-rules become no-ops.
fn parse_kind(raw: &str, kind: FileKind) -> Result<(Option<String>, &str)> {
    match kind {
        FileKind::Skill => {
            let (item, body) = frontmatter::parse::<Skill>(raw)?;
            Ok((item.name, body))
        }
        FileKind::Rule => {
            let (item, body) = frontmatter::parse::<Rule>(raw)?;
            Ok((item.name, body))
        }
        FileKind::Agent => {
            let (item, body) = frontmatter::parse::<Agent>(raw)?;
            Ok((item.name, body))
        }
        FileKind::Bundle => {
            let bundle: crate::model::Bundle = serde_yaml_ng::from_str(raw)?;
            bundle
                .validate_mcps()
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok((Some(bundle.name), ""))
        }
    }
}

/// Identity-name validation (§2.1, ADR-0006).
///
/// - Bundle: when `name` is present, the filename stem MUST equal it.
/// - Skill: the Agent Skills standard mandates `name == dir`. When a
///   `name` is present it MUST equal the directory; when absent the
///   directory name is used and there is nothing to check.
/// - Rule / Agent: identity is layout-independent — the frontmatter
///   `name` may diverge from the folder, so no check is performed.
fn check_name_matches_dir(file: &Path, name: Option<&str>, kind: FileKind, out: &mut Vec<Finding>) {
    match kind {
        FileKind::Bundle => {
            let Some(name) = name else {
                return;
            };
            let Some(stem) = file
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(crate::parse::bundle::BUNDLE_SUFFIX))
            else {
                return;
            };
            if stem != name {
                out.push(Finding {
                    rule_id: "name-matches-dir",
                    severity: Severity::Error,
                    path: file.to_path_buf(),
                    line: None,
                    message: format!(
                        "frontmatter `name: {name}` does not match filename stem `{stem}`"
                    ),
                });
            }
        }
        FileKind::Skill => {
            let Some(name) = name else {
                return;
            };
            let Some(dir) = file
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
            else {
                return;
            };
            if dir != name {
                out.push(Finding {
                    rule_id: "name-matches-dir",
                    severity: Severity::Error,
                    path: file.to_path_buf(),
                    line: None,
                    message: format!(
                        "skill `name: {name}` must equal directory `{dir}` (Agent Skills standard)"
                    ),
                });
            }
        }
        FileKind::Rule | FileKind::Agent => {}
    }
}

/// Flag any line that starts with `# ` (an H1) outside of fenced code
/// blocks. Per format-spec §5.1, generated output prepends an H1 from
/// the item name; an H1 in the body would produce two H1s.
fn check_body_h1(file: &Path, body: &str, out: &mut Vec<Finding>) {
    let mut in_fence = false;
    for (i, line) in body.split_inclusive('\n').enumerate() {
        let stripped = line.trim_end_matches(['\r', '\n']);
        if stripped.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if stripped.starts_with("# ") || stripped == "#" {
            out.push(Finding {
                rule_id: "body-h1",
                severity: Severity::Warning,
                path: file.to_path_buf(),
                line: Some(i + 1),
                message: "body MUST NOT contain H1 — generators derive it from `name`".into(),
            });
        }
    }
}

/// Flag fenced code blocks whose opening fence has no language hint.
/// Per format-spec §5.2, every fence MUST have a language tag (use
/// `text` when no language fits).
fn check_fence_lang(file: &Path, body: &str, out: &mut Vec<Finding>) {
    let mut in_fence = false;
    for (i, line) in body.split_inclusive('\n').enumerate() {
        let stripped = line.trim_end_matches(['\r', '\n']);
        let trimmed = stripped.trim_start();
        if !trimmed.starts_with("```") {
            continue;
        }
        if in_fence {
            in_fence = false;
            continue;
        }
        in_fence = true;
        let after_fence = trimmed.trim_start_matches('`').trim();
        if after_fence.is_empty() {
            out.push(Finding {
                rule_id: "fence-lang",
                severity: Severity::Warning,
                path: file.to_path_buf(),
                line: Some(i + 1),
                message: "fenced code block missing language hint (use `text` if none fits)".into(),
            });
        }
    }
}

/// Run the directive parser once over the body and surface any error
/// (unbalanced, nested, unknown client identifier) as a single
/// `directive` error finding. The parser already covers all three
/// per format-spec §6.3.
fn check_directives(file: &Path, body: &str, out: &mut Vec<Finding>) {
    if let Err(err) = directives::process(body, Client::Claude) {
        out.push(Finding {
            rule_id: "directive",
            severity: Severity::Error,
            path: file.to_path_buf(),
            line: None,
            message: format!("{:#}", err),
        });
    }
}

/// Compare the body against its canonical dprint-formatted form. A
/// mismatch means the author has not run `upskill fmt`. Uses the same
/// `fmt::canonical_body` helper that `fmt` writes with, so checker and
/// fixer never disagree. The on-disk frontmatter is passed through dprint
/// as an opaque prefix, so this is independent of frontmatter key order
/// (which `lint` does not check — only `fmt` reorders).
fn check_body_format(file: &Path, frontmatter: &str, body: &str, out: &mut Vec<Finding>) {
    let Ok(canonical) = crate::fmt::canonical_body(frontmatter, body) else {
        // A dprint failure is rare and surfaced by `fmt` itself; don't
        // double-report it here.
        return;
    };
    if canonical != body {
        out.push(Finding {
            rule_id: "body-format",
            severity: Severity::Warning,
            path: file.to_path_buf(),
            line: None,
            message: "body is not formatted — run `upskill fmt`".into(),
        });
    }
}

/// Cross-file lint: detect when a bundle's `name:` collides with an item
/// directory name in the same registry.
fn check_bundle_item_name_collisions(root: &Path, out: &mut Vec<Finding>) -> Result<()> {
    // Collect item names (directories containing SKILL.md/RULE.md/AGENT.md).
    let mut item_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dirname = entry.file_name().to_string_lossy().to_string();
            if dirname.starts_with('.') {
                continue;
            }
            if path.join("SKILL.md").is_file()
                || path.join("RULE.md").is_file()
                || path.join("AGENT.md").is_file()
            {
                item_names.insert(dirname);
            }
        }
    }

    if item_names.is_empty() {
        return Ok(());
    }

    // Discover bundles and check for collisions.
    let mut bundle_files: Vec<PathBuf> = Vec::new();
    walk_bundles(root, &mut bundle_files)?;

    for bundle_path in bundle_files {
        let raw = fs::read_to_string(&bundle_path)
            .with_context(|| format!("read {}", bundle_path.display()))?;
        if let Some(name) = extract_bundle_name(&raw)
            && item_names.contains(&name)
        {
            let item_kind = detect_item_kind(root, &name);
            out.push(Finding {
                rule_id: "name-collision",
                severity: Severity::Error,
                path: bundle_path,
                line: None,
                message: format!(
                    "bundle name '{}' collides with {} '{}'",
                    name, item_kind, name
                ),
            });
        }
    }

    Ok(())
}

/// Extract the `name:` field from a bundle YAML without full parse.
fn extract_bundle_name(raw: &str) -> Option<String> {
    let parsed: Result<crate::model::Bundle, _> = serde_yaml_ng::from_str(raw);
    parsed.ok().map(|b| b.name)
}

/// Detect what kind of item a name corresponds to, for error messages.
fn detect_item_kind(root: &Path, name: &str) -> &'static str {
    let dir = root.join(name);
    if dir.join("SKILL.md").is_file() {
        "skill"
    } else if dir.join("RULE.md").is_file() {
        "rule"
    } else if dir.join("AGENT.md").is_file() {
        "agent"
    } else {
        "item"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn skill(name: &str, body: &str) -> String {
        format!(
            "---\nschema: 1\nname: {name}\ndescription: a test fixture for the lint module.\n---\n{body}"
        )
    }

    #[test]
    fn lint_clean_skill_yields_no_findings() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("clean/SKILL.md"),
            &skill("clean", "\n## Body\n\nText.\n"),
        );
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        assert!(
            report.findings.is_empty(),
            "unexpected findings: {:?}",
            report.findings
        );
        assert_eq!(report.files_checked, 1);
    }

    #[test]
    fn lint_detects_h1_in_body() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("has-h1/SKILL.md"),
            &skill("has-h1", "\n# Forbidden\n\n## ok\n"),
        );
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        let f = report.findings.iter().find(|f| f.rule_id == "body-h1");
        assert!(
            f.is_some(),
            "missing body-h1 finding: {:?}",
            report.findings
        );
        assert_eq!(f.unwrap().severity, Severity::Warning);
    }

    #[test]
    fn lint_detects_fence_without_language() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("no-lang/SKILL.md"),
            &skill("no-lang", "\n## ok\n\n```\nplain\n```\n"),
        );
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        let f = report.findings.iter().find(|f| f.rule_id == "fence-lang");
        assert!(
            f.is_some(),
            "missing fence-lang finding: {:?}",
            report.findings
        );
    }

    #[test]
    fn lint_skips_h1_inside_fence() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("in-fence/SKILL.md"),
            &skill(
                "in-fence",
                "\n## ok\n\n```sh\n# real comment in shell\n```\n",
            ),
        );
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        assert!(
            report.findings.iter().all(|f| f.rule_id != "body-h1"),
            "false-positive body-h1 inside fence: {:?}",
            report.findings
        );
    }

    #[test]
    fn lint_strict_promotes_warnings_to_errors() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("has-h1/SKILL.md"),
            &skill("has-h1", "\n# bad\n"),
        );
        let report = lint(&[tmp.path().to_path_buf()], true).unwrap();
        assert!(report.has_errors());
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.severity == Severity::Error)
        );
    }

    #[test]
    fn lint_flags_name_directory_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        write(&tmp.path().join("foo/SKILL.md"), &skill("bar", "\n## ok\n"));
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "name-matches-dir");
        assert!(
            f.is_some(),
            "missing name-matches-dir finding: {:?}",
            report.findings
        );
        assert_eq!(f.unwrap().severity, Severity::Error);
    }

    #[test]
    fn lint_flags_unbalanced_directive() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("dangling/SKILL.md"),
            &skill(
                "dangling",
                "\n## ok\n\n<!-- @client:claude -->\nnever closed\n",
            ),
        );
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        let f = report.findings.iter().find(|f| f.rule_id == "directive");
        assert!(
            f.is_some(),
            "missing directive finding: {:?}",
            report.findings
        );
    }

    #[test]
    fn lint_item_directory_discovers_entrypoint() {
        // Issue #159: pointing lint at a single item directory
        // (e.g., `skills/karpathy-guidelines/`) should discover the
        // entrypoint file inside it, not report "0 file(s) checked".
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("my-rule/RULE.md"),
            "---\nschema: 1\nname: my-rule\ndescription: a test rule.\n---\n\n## Body\n\nText.\n",
        );
        // Lint the item directory directly (not the parent registry root).
        let item_dir = tmp.path().join("my-rule");
        let report = lint(&[item_dir], false).unwrap();
        assert_eq!(
            report.files_checked, 1,
            "expected 1 file checked when pointing at an item directory; got {}",
            report.files_checked
        );
    }

    #[test]
    fn lint_item_directory_reports_findings() {
        // Issue #159: lint should actually validate the content when
        // given an item directory, not silently pass.
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("bad-rule/RULE.md"),
            "---\nschema: 1\nname: bad-rule\ndescription: has lint issues.\n---\n\n# Forbidden H1\n",
        );
        let item_dir = tmp.path().join("bad-rule");
        let report = lint(&[item_dir], false).unwrap();
        assert_eq!(report.files_checked, 1);
        let f = report.findings.iter().find(|f| f.rule_id == "body-h1");
        assert!(
            f.is_some(),
            "expected body-h1 finding when linting an item directory; got: {:?}",
            report.findings
        );
    }

    #[test]
    fn lint_item_directory_with_multiple_entrypoints() {
        // Format-spec §2.1: an item directory may hold multiple
        // co-located entrypoints. All should be discovered.
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("multi/SKILL.md"),
            &skill("multi", "\n## ok\n"),
        );
        write(
            &tmp.path().join("multi/AGENT.md"),
            "---\nschema: 1\nname: multi\ndescription: co-located agent.\n---\n\n## ok\n",
        );
        let item_dir = tmp.path().join("multi");
        let report = lint(&[item_dir], false).unwrap();
        assert_eq!(
            report.files_checked, 2,
            "expected 2 files checked for co-located item directory; got {}",
            report.files_checked
        );
    }

    #[test]
    fn lint_refuses_consumer_project() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join(".upskill-lock.json"),
            r#"{"schema":2,"items":[]}"#,
        )
        .unwrap();
        let err = lint(&[tmp.path().to_path_buf()], false).expect_err("must refuse");
        assert!(format!("{err:#}").contains("consumer project"));
    }

    #[test]
    fn lint_skill_name_mismatch_is_flagged_but_rule_divergence_is_silent() {
        // §2.1, ADR-0006: a skill's `name:` MUST equal its directory
        // (Agent Skills standard). Rules and agents have
        // layout-independent identity — their `name:` may diverge from
        // the folder, which is legal and silent.
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("security/SKILL.md"),
            concat!(
                "---\n",
                "schema: 1\n",
                "name: wrong-name\n",
                "description: skill name diverges from the directory.\n",
                "---\n",
                "\n## ok\n",
            ),
        );
        write(
            &tmp.path().join("stuff/RULE.md"),
            concat!(
                "---\n",
                "schema: 1\n",
                "name: security-baseline\n",
                "description: rule name diverges from the directory.\n",
                "---\n",
                "\n## ok\n",
            ),
        );
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        // The skill (mismatch) is flagged; the rule (mismatch) is not.
        let skill_finding = report.findings.iter().find(|f| {
            f.rule_id == "name-matches-dir"
                && f.path.file_name().and_then(|n| n.to_str()) == Some("SKILL.md")
        });
        assert!(
            skill_finding.is_some(),
            "expected skill name mismatch to be flagged: {:?}",
            report.findings
        );
        assert!(
            skill_finding
                .unwrap()
                .message
                .contains("Agent Skills standard"),
            "skill mismatch message should reference the standard: {:?}",
            skill_finding
        );
        let rule_finding = report.findings.iter().find(|f| {
            f.rule_id == "name-matches-dir"
                && f.path.file_name().and_then(|n| n.to_str()) == Some("RULE.md")
        });
        assert!(
            rule_finding.is_none(),
            "rule name divergence must be silent: {:?}",
            report.findings
        );
    }

    #[test]
    fn lint_flags_bundle_filename_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let body = concat!(
            "schema: 1\n",
            "name: foo\n",
            "description: filename stem says baseline.\n",
            "items:\n",
            "  rules: []\n",
            "  skills: []\n",
            "  agents: []\n",
        );
        write(&tmp.path().join("bundles/baseline.bundle.yaml"), body);
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "name-matches-dir");
        assert!(
            f.is_some(),
            "missing name-matches-dir finding for bundle: {:?}",
            report.findings
        );
    }

    #[test]
    fn lint_flags_unformatted_body() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("dirty/SKILL.md");
        write(&item, &skill("dirty", "\n## Body\n\n* one\n* two\n"));
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "body-format")
            .expect("expected a body-format finding");
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn lint_clean_body_has_no_body_format_finding() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("clean/SKILL.md");
        write(&item, &skill("clean", "\n## Body\n\n- one\n- two\n"));
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        assert!(
            report.findings.iter().all(|f| f.rule_id != "body-format"),
            "unexpected body-format finding: {:?}",
            report.findings
        );
    }

    #[test]
    fn lint_strict_promotes_body_format_to_error() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("dirty/SKILL.md");
        write(&item, &skill("dirty", "\n## Body\n\n* one\n* two\n"));
        let report = lint(&[tmp.path().to_path_buf()], true).unwrap();
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "body-format")
            .expect("expected a body-format finding");
        assert_eq!(f.severity, Severity::Error);
    }

    #[test]
    fn lint_flags_invalid_mcps_transport_in_bundle() {
        // Regression: lint's bundle path must call validate_mcps so an
        // unsupported transport type is surfaced as a frontmatter error,
        // not silently accepted.
        let tmp = tempfile::tempdir().unwrap();
        let body = concat!(
            "schema: 1\n",
            "name: bad-mcp\n",
            "description: bundle with an invalid MCP transport type.\n",
            "items:\n",
            "  skills: []\n",
            "mcps:\n",
            "  my-server:\n",
            "    remote:\n",
            "      type: ftp\n",
            "      url: https://example.com/mcp\n",
        );
        write(&tmp.path().join("bad-mcp.bundle.yaml"), body);
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        let f = report.findings.iter().find(|f| f.rule_id == "frontmatter");
        assert!(
            f.is_some(),
            "expected frontmatter error for invalid mcps transport; got: {:?}",
            report.findings
        );
        assert!(
            f.unwrap().message.contains("ftp"),
            "error message should mention the bad transport type; got: {:?}",
            report.findings
        );
    }
}
