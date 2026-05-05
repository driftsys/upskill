//! Output styling: a small universal palette plus the
//! [clig.dev](https://clig.dev) disable chain.
//!
//! Disable signals (highest precedence first):
//!
//! 1. `--no-color` flag — explicit user request.
//! 2. `NO_COLOR` env var (any non-empty value).
//! 3. `UPSKILL_NO_COLOR` env var (app-specific override).
//! 4. `TERM=dumb`.
//! 5. `!isatty(target stream)` — handled by `colored`'s default behaviour.
//!
//! `FORCE_COLOR` (or `CLICOLOR_FORCE`) re-enables color even when piped.

use std::sync::atomic::{AtomicBool, Ordering};

use colored::ColoredString;
use colored::Colorize;
use colored::control;

static QUIET: AtomicBool = AtomicBool::new(false);

/// Enable or disable quiet mode. Set once from `main()` after parsing
/// `--quiet`. Read by every `print_*` site to short-circuit informational
/// stdout. Errors on stderr are unaffected.
pub fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::SeqCst);
}

/// True when `--quiet` was passed (or `set_quiet(true)` was called).
pub fn is_quiet() -> bool {
    QUIET.load(Ordering::SeqCst)
}

/// Apply the disable chain at startup. Honors `--no-color` from the CLI plus
/// the env-var signals listed in the module docs. Should be called once,
/// before any styled output.
pub fn init(no_color_flag: bool) {
    if no_color_flag || env_is_set("NO_COLOR") || env_is_set("UPSKILL_NO_COLOR") || term_is_dumb() {
        control::set_override(false);
    } else if env_is_set("FORCE_COLOR") || env_is_set("CLICOLOR_FORCE") {
        control::set_override(true);
    }
    // Otherwise let `colored` auto-detect (respects isatty per stream).
}

fn env_is_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty())
}

fn term_is_dumb() -> bool {
    std::env::var("TERM").is_ok_and(|t| t == "dumb")
}

// --- palette helpers ------------------------------------------------------
//
// Universal palette per epic #105 §B1:
//   red     = error / missing / drift
//   yellow  = warning / would-change / pending
//   green   = success / created / ✓
//   gray    = secondary info (sources, paths, descriptions)
//   bold    = primary identifier (names, paths)

/// Bold red — for the "error:" label and similar fatal signals.
pub fn error_label(text: &str) -> ColoredString {
    text.red().bold()
}

/// Yellow — warnings, would-change in dry-run.
pub fn warn(text: &str) -> ColoredString {
    text.yellow()
}

/// Green — success, created, ✓.
pub fn success(text: &str) -> ColoredString {
    text.green()
}

/// Bold — primary identifier (names, paths).
pub fn name(text: &str) -> ColoredString {
    text.bold()
}

/// Dim — secondary info (sources, descriptions).
pub fn dim(text: &str) -> ColoredString {
    text.dimmed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let _lock = ENV_LOCK.lock().unwrap();
        let originals: Vec<_> = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var(k).ok()))
            .collect();
        for (k, v) in vars {
            // SAFETY: tests are serialised by ENV_LOCK so no concurrent mutation.
            unsafe {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
        f();
        for (k, original) in &originals {
            // SAFETY: tests are serialised by ENV_LOCK so no concurrent mutation.
            unsafe {
                match original {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    fn rendered(text: ColoredString) -> String {
        format!("{text}")
    }

    fn has_ansi(s: &str) -> bool {
        s.contains('\x1b')
    }

    #[test]
    fn no_color_flag_disables() {
        with_env(
            &[
                ("NO_COLOR", None),
                ("UPSKILL_NO_COLOR", None),
                ("TERM", Some("xterm")),
                ("FORCE_COLOR", None),
                ("CLICOLOR_FORCE", None),
            ],
            || {
                init(true);
                assert!(!has_ansi(&rendered(error_label("error"))));
                control::unset_override();
            },
        );
    }

    #[test]
    fn no_color_env_disables() {
        with_env(
            &[
                ("NO_COLOR", Some("1")),
                ("UPSKILL_NO_COLOR", None),
                ("TERM", Some("xterm")),
                ("FORCE_COLOR", None),
                ("CLICOLOR_FORCE", None),
            ],
            || {
                init(false);
                assert!(!has_ansi(&rendered(error_label("error"))));
                control::unset_override();
            },
        );
    }

    #[test]
    fn upskill_no_color_disables() {
        with_env(
            &[
                ("NO_COLOR", None),
                ("UPSKILL_NO_COLOR", Some("1")),
                ("TERM", Some("xterm")),
                ("FORCE_COLOR", None),
                ("CLICOLOR_FORCE", None),
            ],
            || {
                init(false);
                assert!(!has_ansi(&rendered(error_label("error"))));
                control::unset_override();
            },
        );
    }

    #[test]
    fn term_dumb_disables() {
        with_env(
            &[
                ("NO_COLOR", None),
                ("UPSKILL_NO_COLOR", None),
                ("TERM", Some("dumb")),
                ("FORCE_COLOR", None),
                ("CLICOLOR_FORCE", None),
            ],
            || {
                init(false);
                assert!(!has_ansi(&rendered(error_label("error"))));
                control::unset_override();
            },
        );
    }

    #[test]
    fn force_color_re_enables_when_piped() {
        with_env(
            &[
                ("NO_COLOR", None),
                ("UPSKILL_NO_COLOR", None),
                ("TERM", Some("xterm")),
                ("FORCE_COLOR", Some("1")),
                ("CLICOLOR_FORCE", None),
            ],
            || {
                init(false);
                // assert_cmd-style runs are non-TTY; FORCE_COLOR must override that.
                assert!(has_ansi(&rendered(error_label("error"))));
                control::unset_override();
            },
        );
    }

    #[test]
    fn empty_no_color_does_not_disable() {
        // Per the NO_COLOR spec: the var must be non-empty to disable.
        with_env(
            &[
                ("NO_COLOR", Some("")),
                ("UPSKILL_NO_COLOR", None),
                ("TERM", Some("xterm")),
                ("FORCE_COLOR", Some("1")),
                ("CLICOLOR_FORCE", None),
            ],
            || {
                init(false);
                assert!(has_ansi(&rendered(error_label("error"))));
                control::unset_override();
            },
        );
    }
}
