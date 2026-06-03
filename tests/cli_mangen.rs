//! ATDD for man-page generation (#117).
//!
//! Renders the clap command tree through `clap_mangen` and asserts the
//! produced roff contains the binary name, a SYNOPSIS section, and one
//! page per declared subcommand. Pure in-memory check — no disk I/O —
//! so CI doesn't depend on `target/man/` layout. The
//! `examples/mangen.rs` binary uses the same `Cli::command()` entrypoint
//! to write the actual files.

use clap::CommandFactory;
use clap_mangen::Man;

use upskill::cli::Cli;

fn render(cmd: clap::Command) -> String {
    let mut buffer = Vec::new();
    Man::new(cmd).render(&mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[test]
fn root_man_page_has_synopsis_and_name() {
    let page = render(Cli::command());
    assert!(page.contains("upskill"), "missing binary name: {page}");
    assert!(page.contains("SYNOPSIS"), "missing SYNOPSIS section");
    assert!(page.contains("DESCRIPTION"), "missing DESCRIPTION section");
}

#[test]
fn every_subcommand_renders_a_man_page() {
    let root = Cli::command();
    let names: Vec<String> = root
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();

    // Sanity: the CLI ships a fixed number of commands. If this drifts,
    // update the assert and the docs.
    assert_eq!(
        names.len(),
        11,
        "expected 11 subcommands, got {}: {names:?}",
        names.len()
    );

    for sub in root.get_subcommands() {
        let page = render(sub.clone());
        let name = sub.get_name();
        assert!(
            page.contains("SYNOPSIS"),
            "{name} man page missing SYNOPSIS"
        );
        assert!(page.contains(name), "{name} man page missing its own name");
    }
}

#[test]
fn root_man_page_documents_quiet_flag() {
    // Smoke check that global flags surface in the rendered roff —
    // `--quiet` was just added, so its presence proves the generator
    // tracks the live `Cli` definition rather than a stale snapshot.
    let page = render(Cli::command());
    // roff escapes hyphens: `--quiet` is rendered as `\-\-quiet`.
    assert!(
        page.contains(r"\-\-quiet"),
        "expected --quiet in root man page, got:\n{page}"
    );
}
