//! Conditional content directives (§6).
//!
//! Parses `<!-- @client:X -->` ... `<!-- @endclient -->` blocks and
//! emits a body with non-matching blocks stripped. Per §6.3:
//!
//! - non-matching block: strip delimiters AND content; collapse
//!   surrounding blanks so we don't produce MD012 violations.
//! - matching block: strip delimiter lines, keep content.
//! - validate balanced opens/closes.
//! - reject nested directives.
//! - reject unknown client identifiers.

use crate::generate::Client;
use anyhow::{Result, bail};

/// Process `@client:` directives in `body` for `target`.
pub fn process(body: &str, target: Client) -> Result<String> {
    let mut out = String::with_capacity(body.len());
    let mut state = State::Outside;
    let mut open_lineno = 0usize;

    for (i, line) in body.split_inclusive('\n').enumerate() {
        let lineno = i + 1;
        let line_trimmed = line.trim_end_matches(['\r', '\n']);

        if let Some(client_list) = parse_open(line_trimmed)? {
            match state {
                State::Outside => {
                    state = State::InBlock {
                        keep: client_list.matches(target),
                    };
                    open_lineno = lineno;
                }
                State::InBlock { .. } => {
                    bail!(
                        "line {lineno}: nested `@client:` directives are not allowed (outer opened at line {open_lineno})"
                    );
                }
            }
            continue;
        }

        if is_close(line_trimmed) {
            match state {
                State::InBlock { .. } => state = State::Outside,
                State::Outside => {
                    bail!("line {lineno}: `@endclient` without matching `@client:` open");
                }
            }
            continue;
        }

        match state {
            State::Outside => out.push_str(line),
            State::InBlock { keep: true } => out.push_str(line),
            State::InBlock { keep: false } => {}
        }
    }

    if !matches!(state, State::Outside) {
        bail!("unclosed `@client:` directive opened at line {open_lineno}");
    }

    Ok(collapse_blank_runs(&out))
}

#[derive(Debug, Clone, Copy)]
enum State {
    Outside,
    InBlock { keep: bool },
}

#[derive(Debug)]
struct ClientList {
    negated: bool,
    clients: Vec<Client>,
}

impl ClientList {
    fn matches(&self, target: Client) -> bool {
        let in_list = self.clients.contains(&target);
        if self.negated { !in_list } else { in_list }
    }
}

fn parse_open(line: &str) -> Result<Option<ClientList>> {
    let trimmed = line.trim();
    let Some(inner) = trimmed
        .strip_prefix("<!--")
        .and_then(|s| s.strip_suffix("-->"))
    else {
        return Ok(None);
    };
    let inner = inner.trim();
    let Some(after) = inner.strip_prefix("@client:") else {
        return Ok(None);
    };
    let after = after.trim();
    let (negated, list) = match after.strip_prefix('!') {
        Some(rest) => (true, rest.trim()),
        None => (false, after),
    };
    let mut clients = Vec::new();
    for tok in list.split(',') {
        let name = tok.trim();
        if name.is_empty() {
            bail!("empty client identifier in `@client:` directive");
        }
        clients.push(name.parse::<Client>()?);
    }
    Ok(Some(ClientList { negated, clients }))
}

fn is_close(line: &str) -> bool {
    let trimmed = line.trim();
    let Some(inner) = trimmed
        .strip_prefix("<!--")
        .and_then(|s| s.strip_suffix("-->"))
    else {
        return false;
    };
    inner.trim() == "@endclient"
}

/// Collapse runs of 2+ blank lines into a single blank line.
fn collapse_blank_runs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_blank = false;
    for line in s.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let is_blank = body.is_empty();
        if is_blank && prev_blank {
            continue;
        }
        out.push_str(line);
        prev_blank = is_blank;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_kept_for_listed_client() {
        let body = "before\n<!-- @client:claude -->\ninner\n<!-- @endclient -->\nafter\n";
        let claude = process(body, Client::Claude).unwrap();
        assert!(claude.contains("inner"), "claude should keep:\n{claude}");
        assert!(claude.contains("before") && claude.contains("after"));
    }

    #[test]
    fn positive_stripped_for_other_clients() {
        let body = "before\n<!-- @client:claude -->\ninner\n<!-- @endclient -->\nafter\n";
        let copilot = process(body, Client::Copilot).unwrap();
        let opencode = process(body, Client::OpenCode).unwrap();
        assert!(!copilot.contains("inner"), "copilot should strip");
        assert!(!opencode.contains("inner"), "opencode should strip");
    }

    #[test]
    fn negative_strips_only_for_listed_client() {
        let body = "before\n<!-- @client:!opencode -->\ninner\n<!-- @endclient -->\nafter\n";
        let claude = process(body, Client::Claude).unwrap();
        let copilot = process(body, Client::Copilot).unwrap();
        let opencode = process(body, Client::OpenCode).unwrap();
        assert!(claude.contains("inner"), "claude should keep");
        assert!(copilot.contains("inner"), "copilot should keep");
        assert!(!opencode.contains("inner"), "opencode should strip");
    }

    #[test]
    fn unbalanced_unclosed_errors() {
        let body = "<!-- @client:claude -->\nunclosed\n";
        let err = process(body, Client::Claude).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("unclosed"), "got: {msg}");
    }

    #[test]
    fn unbalanced_close_without_open_errors() {
        let body = "<!-- @endclient -->\n";
        let err = process(body, Client::Claude).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("@endclient"), "got: {msg}");
    }

    #[test]
    fn nested_directive_errors() {
        let body = concat!(
            "<!-- @client:claude -->\n",
            "outer\n",
            "<!-- @client:copilot -->\n",
            "inner\n",
            "<!-- @endclient -->\n",
            "<!-- @endclient -->\n"
        );
        let err = process(body, Client::Claude).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("nested"), "got: {msg}");
    }

    #[test]
    fn unknown_client_identifier_errors() {
        let body = "<!-- @client:foo -->\nx\n<!-- @endclient -->\n";
        let err = process(body, Client::Claude).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("unknown client"), "got: {msg}");
    }

    #[test]
    fn surrounding_blank_lines_collapsed_on_strip() {
        let body = "para 1.\n\n<!-- @client:opencode -->\nopencode-only.\n<!-- @endclient -->\n\npara 2.\n";
        let result = process(body, Client::Claude).unwrap();
        assert!(
            !result.contains("\n\n\n"),
            "no triple-newline allowed; got:\n{result:?}"
        );
        assert!(result.contains("para 1."));
        assert!(result.contains("para 2."));
    }
}
