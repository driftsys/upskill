//! `.gitignore`-style subtractive filtering of an item's supporting
//! resources (format-spec §2.4). Hand-rolled to avoid a glob dependency
//! (protects the ~3 MB binary target). Supports `*` (any run of
//! non-`/` chars), `**` (any run including `/`), `?` (one non-`/` char),
//! and literal segments. A pattern with no `/` matches the basename at any
//! depth; a pattern with a `/` matches the full relative path. A trailing
//! `/**` (or a bare directory-name pattern) matches everything under that
//! directory.

use std::path::PathBuf;

/// Drop every resource (path relative to the item dir) that matches any
/// `ignore` glob. Returns the kept resources, order preserved.
pub(super) fn filter_ignored(resources: Vec<PathBuf>, patterns: &[String]) -> Vec<PathBuf> {
    if patterns.is_empty() {
        return resources;
    }
    resources
        .into_iter()
        .filter(|rel| {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            !patterns.iter().any(|p| matches_pattern(&rel_str, p))
        })
        .collect()
}

fn matches_pattern(path: &str, pattern: &str) -> bool {
    let pattern = pattern.trim_end_matches('/');
    // A literal pattern with no slash and no glob metacharacters is a
    // directory-name shorthand: it matches that basename at any depth AND
    // every path nested under such a directory (`fixtures` drops
    // `fixtures/data.json`).
    if !pattern.contains('/') {
        let base = path.rsplit('/').next().unwrap_or(path);
        if glob_match(base, pattern) {
            return true;
        }
        if !pattern.contains(['*', '?']) && path.split('/').any(|seg| seg == pattern) {
            return true;
        }
        return false;
    }
    // Directory-prefix shorthand: `scripts/**` ignores the whole subtree.
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    glob_match(path, pattern)
}

/// Match `text` against a glob `pat` where `*`/`?` do not cross `/` and
/// `**` does. Recursive backtracking — patterns are short and few.
fn glob_match(text: &str, pat: &str) -> bool {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = pat.chars().collect();
    match_at(&t, 0, &p, 0)
}

/// Match `t[ti..]` against pattern `p[pi..]`. `*`/`?` stay within a path segment; `**` spans `/`.
fn match_at(t: &[char], ti: usize, p: &[char], pi: usize) -> bool {
    if pi == p.len() {
        return ti == t.len();
    }
    match p[pi] {
        '*' if pi + 1 < p.len() && p[pi + 1] == '*' => {
            // `**` — consume any chars including `/`.
            let mut k = ti;
            loop {
                if match_at(t, k, p, pi + 2) {
                    return true;
                }
                if k == t.len() {
                    return false;
                }
                k += 1;
            }
        }
        '*' => {
            let mut k = ti;
            loop {
                if match_at(t, k, p, pi + 1) {
                    return true;
                }
                if k == t.len() || t[k] == '/' {
                    return false;
                }
                k += 1;
            }
        }
        '?' => ti < t.len() && t[ti] != '/' && match_at(t, ti + 1, p, pi + 1),
        c => ti < t.len() && t[ti] == c && match_at(t, ti + 1, p, pi + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn paths(v: &[&str]) -> Vec<PathBuf> {
        v.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn empty_patterns_keep_everything() {
        let r = paths(&["scripts/gate.sh", "refs/p.md"]);
        assert_eq!(filter_ignored(r.clone(), &[]), r);
    }

    #[test]
    fn double_star_prefix_drops_subtree() {
        let kept = filter_ignored(
            paths(&["scripts/gate.sh", "scripts/lib/x.sh", "refs/p.md"]),
            &["scripts/**".to_string()],
        );
        assert_eq!(kept, paths(&["refs/p.md"]));
    }

    #[test]
    fn bare_name_matches_basename_at_any_depth() {
        let kept = filter_ignored(paths(&["a/b/notes.log", "keep.md"]), &["*.log".to_string()]);
        assert_eq!(kept, paths(&["keep.md"]));
    }

    #[test]
    fn single_star_does_not_cross_slash() {
        let kept = filter_ignored(paths(&["a/x.txt", "a/b/x.txt"]), &["a/*.txt".to_string()]);
        assert_eq!(kept, paths(&["a/b/x.txt"]));
    }

    #[test]
    fn bare_dir_name_ignores_subtree() {
        let kept = filter_ignored(
            paths(&["fixtures/data.json", "main.md"]),
            &["fixtures".to_string()],
        );
        assert_eq!(kept, paths(&["main.md"]));
    }

    #[test]
    fn question_mark_matches_single_char() {
        assert_eq!(
            filter_ignored(paths(&["a.md", "ab.md"]), &["?.md".to_string()]),
            paths(&["ab.md"])
        );
    }

    #[test]
    fn non_matching_pattern_keeps_file() {
        assert_eq!(
            filter_ignored(paths(&["keep.md"]), &["*.log".to_string()]),
            paths(&["keep.md"])
        );
    }
}
