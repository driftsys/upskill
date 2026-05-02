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
//! | `directive`       | error    | §6.3 (any imbalance, unknown client, nesting) |
//!
//! Discovery: a path argument is either a file (lint that file) or a
//! directory (walk for `skills/*/SKILL.md`, `rules/*/RULE.md`,
//! `agents/*/AGENT.md`, plus any `*.bundle.md`). With no paths, the
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
/// file in such a directory) exists. Shared with `crate::fmt` so both
/// author commands refuse running inside consumer projects.
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
            n if n.ends_with(".bundle.md") => Some(Self::Bundle),
            _ => None,
        }
    }
}

/// Walk `root` (file or directory) and return every entrypoint file
/// the lint should inspect. SSOT items are detected by the
/// `<kind>s/<name>/<KIND>.md` layout per format-spec §2.1; bundles by
/// the `*.bundle.md` suffix. Shared with `crate::fmt`.
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

    for sub in ["skills", "rules", "agents"] {
        let kind_root = root.join(sub);
        if !kind_root.is_dir() {
            continue;
        }
        let entry = match sub {
            "skills" => "SKILL.md",
            "rules" => "RULE.md",
            "agents" => "AGENT.md",
            _ => unreachable!(),
        };
        for item in fs::read_dir(&kind_root)
            .with_context(|| format!("read {}", kind_root.display()))?
            .flatten()
        {
            let path = item.path();
            if !path.is_dir() {
                continue;
            }
            let entry_path = path.join(entry);
            if entry_path.is_file() {
                out.push(entry_path);
            }
        }
    }

    walk_bundles(root, &mut out)?;
    Ok(out)
}

/// Recurse into `root` looking for `*.bundle.md` files. Skips hidden
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
            check_name_matches_dir(file, &parsed_name, kind, out);
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
    Ok(())
}

/// Parse frontmatter for the kind, returning `(name, body)`.
fn parse_kind(raw: &str, kind: FileKind) -> Result<(String, &str)> {
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
            let (item, body) = frontmatter::parse::<crate::model::Bundle>(raw)?;
            Ok((item.name, body))
        }
    }
}

fn check_name_matches_dir(file: &Path, name: &str, kind: FileKind, out: &mut Vec<Finding>) {
    // Bundles match the filename stem (`<name>.bundle.md`); items
    // match the parent directory name.
    match kind {
        FileKind::Bundle => {
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
        _ => {
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
                    message: format!("frontmatter `name: {name}` does not match directory `{dir}`"),
                });
            }
        }
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
            &tmp.path().join("skills/clean/SKILL.md"),
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
            &tmp.path().join("skills/has-h1/SKILL.md"),
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
            &tmp.path().join("skills/no-lang/SKILL.md"),
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
            &tmp.path().join("skills/in-fence/SKILL.md"),
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
            &tmp.path().join("skills/has-h1/SKILL.md"),
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
        write(
            &tmp.path().join("skills/foo/SKILL.md"),
            &skill("bar", "\n## ok\n"),
        );
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
            &tmp.path().join("skills/dangling/SKILL.md"),
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
    fn lint_flags_bundle_filename_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let body = concat!(
            "---\n",
            "schema: 1\n",
            "name: foo\n",
            "description: filename stem says baseline.\n",
            "items:\n",
            "  rules: []\n",
            "  skills: []\n",
            "  agents: []\n",
            "---\n",
            "## ok\n",
        );
        write(&tmp.path().join("bundles/baseline.bundle.md"), body);
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
}
