//! dprint-plugin-markdown wrapper (§7.5).
//!
//! Default config leaves `text_wrap` at `Maintain` so prose is not
//! re-wrapped. The formatter is invoked at the end of every render so
//! generated output is idempotent.

use anyhow::Result;
use dprint_plugin_markdown::configuration::{Configuration, ConfigurationBuilder};
use dprint_plugin_markdown::format_text;

/// Format markdown using the project's default dprint config.
///
/// `Ok(None)` from dprint means input was already canonical — return
/// it unchanged.
pub fn format_markdown(text: &str) -> Result<String> {
    format_with(text, &default_config())
}

/// Format markdown with an explicit dprint config.
pub fn format_with(text: &str, config: &Configuration) -> Result<String> {
    let result = format_text(text, config, |_lang, _code, _line| Ok(None))
        .map_err(|e| anyhow::anyhow!("dprint formatting failed: {}", e))?;
    Ok(result.unwrap_or_else(|| text.to_string()))
}

/// Default config — leaves `text_wrap` at `Maintain` per §7.5.
pub fn default_config() -> Configuration {
    ConfigurationBuilder::new().build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dprint_plugin_markdown::configuration::TextWrap;

    #[test]
    fn empty_input_clean() {
        let r = format_markdown("").expect("format empty");
        assert_eq!(r, "");
    }

    #[test]
    fn multiline_inline_code_in_list_does_not_panic() {
        // Regression: an inline code span that spans a hard line break inside
        // a list item (valid CommonMark) tripped a dprint-core debug assert
        // ("Found a newline in the string") in debug builds. Must not panic;
        // the span content must survive.
        let md = "1. returns `STATUS: available |\n   unavailable | refused` ok.\n";
        let out = format_markdown(md).expect("must format without panicking");
        assert!(out.contains("STATUS: available"), "code span lost: {out:?}");
    }

    #[test]
    fn already_canonical_idempotent() {
        let canonical = "## Heading\n\nA paragraph.\n";
        let pass1 = format_markdown(canonical).unwrap();
        let pass2 = format_markdown(&pass1).unwrap();
        assert_eq!(pass1, pass2, "format must be idempotent");
    }

    #[test]
    fn wrap_width_config_takes_effect() {
        let prose = "## H\n\nA single line of prose that is definitely longer than forty characters wide and will need to wrap.\n";
        let cfg = ConfigurationBuilder::new()
            .line_width(40)
            .text_wrap(TextWrap::Always)
            .build();
        let out = format_with(prose, &cfg).unwrap();
        let max = out
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        assert!(
            max <= 40,
            "longest line should be <= 40 with line_width=40, got {max}:\n{out}"
        );
    }
}
