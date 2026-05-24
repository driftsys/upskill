//! Shared test harness utilities.
//!
//! Every integration test that invokes `upskill` MUST use [`upskill_cmd`] (or
//! manually set `HOME`) to prevent writes to the developer's real home
//! directory. See <https://github.com/driftsys/upskill/issues/193>.

use assert_cmd::Command;
use std::path::Path;

/// Returns a `Command` for the `upskill` binary with `HOME` pointed at
/// `fake_home` (which the caller should create inside a `tempfile::TempDir`).
///
/// This is the **belt-and-suspenders** defense: even if a test forgets to
/// create a `.git/` marker in its working directory, the worst case is a
/// write into the tempdir's fake home — never the developer's real `$HOME`.
pub fn upskill_cmd(fake_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("upskill").unwrap();
    cmd.env("HOME", fake_home);
    // Also override USERPROFILE for Windows-compat code paths.
    cmd.env("USERPROFILE", fake_home);
    cmd
}
