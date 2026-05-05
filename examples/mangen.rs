//! Generate roff man pages for `upskill` and every subcommand.
//!
//! Run with `cargo run --example mangen --release` (or `just man`).
//! Writes one `.1` file per command into `target/man/`:
//!
//! ```text
//! target/man/upskill.1
//! target/man/upskill-add.1
//! target/man/upskill-doctor.1
//! ...
//! ```
//!
//! Install system-wide with e.g.
//! `sudo cp target/man/*.1 /usr/local/share/man/man1/`.

use std::io;
use std::path::{Path, PathBuf};

use clap::{Command, CommandFactory};
use clap_mangen::Man;
use upskill::cli::Cli;

fn main() -> io::Result<()> {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/man");
    std::fs::create_dir_all(&out_dir)?;
    let root = Cli::command();
    let written = render_recursive(&root, "upskill", &out_dir)?;
    println!("wrote {written} man page(s) to {}", out_dir.display());
    Ok(())
}

fn render_recursive(cmd: &Command, name: &str, out_dir: &Path) -> io::Result<usize> {
    let mut buffer = Vec::new();
    Man::new(cmd.clone())
        .title(name.to_uppercase())
        .render(&mut buffer)?;
    std::fs::write(out_dir.join(format!("{name}.1")), buffer)?;
    let mut count = 1;
    for sub in cmd.get_subcommands() {
        let sub_name = format!("{name}-{}", sub.get_name());
        count += render_recursive(sub, &sub_name, out_dir)?;
    }
    Ok(count)
}
