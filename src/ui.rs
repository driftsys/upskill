use anyhow::{Context, Result};
use std::io::{self, IsTerminal, Write};

pub fn interactive_skill_selection_enabled() -> bool {
    std::env::var_os("UPSKILL_FORCE_INTERACTIVE").is_some() || io::stdin().is_terminal()
}

pub fn prompt_for_skill_selection(default_skill: &str) -> Result<Vec<String>> {
    print!(
        "select skills (comma-separated, empty for '{}'): ",
        default_skill
    );
    io::stdout().flush().context("failed to flush prompt")?;

    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .context("failed to read selected skills")?;

    let parsed: Vec<String> = answer
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect();

    if parsed.is_empty() {
        return Ok(vec![default_skill.to_string()]);
    }

    Ok(parsed)
}

pub fn should_prompt_for_confirmation(yes: bool) -> bool {
    !yes && io::stdin().is_terminal()
}

pub fn confirm_removal(skill: &str) -> bool {
    if io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() {
        print!("\u{1b}[33mremove skill '{}' ? [y/N]:\u{1b}[0m ", skill);
    } else {
        print!("remove skill '{}' ? [y/N]: ", skill);
    }
    if io::stdout().flush().is_err() {
        return false;
    }

    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_err() {
        return false;
    }

    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

pub fn print_selected_skills(skills: &[String], implicit_selection: bool) {
    if skills.is_empty() || implicit_selection {
        return;
    }

    println!("skills: {}", skills.join(","));
}
