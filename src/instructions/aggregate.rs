//! Pure editing of Markdown instruction contributions.

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contribution {
    pub id: String,
    pub markdown: String,
}

impl Contribution {
    pub fn new(id: impl Into<String>, markdown: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            markdown: markdown.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedContribution {
    pub id: String,
    pub content_sha256: String,
    pub leading_newlines: u8,
    pub preceding_newlines: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateEdit {
    pub content: String,
    pub owned: Vec<OwnedContribution>,
}

fn content_hash(content: &str) -> String {
    Sha256::digest(content.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_contribution(contribution: &Contribution) -> Result<()> {
    if contribution.id.is_empty()
        || !contribution
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        bail!("invalid instruction contribution id: {}", contribution.id);
    }
    if !contribution.markdown.ends_with('\n') {
        bail!("instruction contribution must end with a newline");
    }
    if contribution.markdown.contains("<!-- upskill:") {
        bail!("instruction contribution contains a managed marker");
    }
    Ok(())
}

fn append_contribution(content: &mut String, contribution: Contribution) -> OwnedContribution {
    let preceding_newlines = content
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\n')
        .count();
    let leading_newlines = if content.is_empty() {
        0
    } else if content.ends_with('\n') {
        1
    } else {
        2
    };
    for _ in 0..leading_newlines {
        content.push('\n');
    }
    content.push_str(&format!("<!-- upskill:begin {} -->\n", contribution.id));
    content.push_str(&contribution.markdown);
    content.push_str(&format!("<!-- upskill:end {} -->\n", contribution.id));
    OwnedContribution {
        id: contribution.id,
        content_sha256: content_hash(&contribution.markdown),
        leading_newlines,
        preceding_newlines,
    }
}

struct OwnedSpan<'a> {
    record: &'a OwnedContribution,
    start: usize,
    begin: usize,
    end_marker_start: usize,
    end: usize,
}

fn find_owned_span<'a>(existing: &str, record: &'a OwnedContribution) -> Result<OwnedSpan<'a>> {
    let begin_marker = format!("<!-- upskill:begin {} -->\n", record.id);
    let end_marker = format!("<!-- upskill:end {} -->\n", record.id);
    let begin = existing.find(&begin_marker).ok_or_else(|| {
        anyhow::anyhow!("owned instruction contribution {} is missing", record.id)
    })?;
    if begin > 0 && existing.as_bytes()[begin - 1] != b'\n' {
        bail!(
            "owned instruction contribution {} has an inline marker",
            record.id
        );
    }
    if existing[begin + begin_marker.len()..].contains(&begin_marker) {
        bail!("duplicate instruction contribution {}", record.id);
    }
    let content_start = begin + begin_marker.len();
    let relative_end = existing[content_start..].find(&end_marker).ok_or_else(|| {
        anyhow::anyhow!(
            "owned instruction contribution {} has no end marker",
            record.id
        )
    })?;
    let content_end = content_start + relative_end;
    if content_hash(&existing[content_start..content_end]) != record.content_sha256 {
        bail!("owned instruction contribution {} was edited", record.id);
    }
    let start = begin
        .checked_sub(usize::from(record.leading_newlines))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "owned instruction contribution {} has a changed separator",
                record.id
            )
        })?;
    if !existing.as_bytes()[start..begin]
        .iter()
        .all(|byte| *byte == b'\n')
    {
        bail!(
            "owned instruction contribution {} has a changed separator",
            record.id
        );
    }
    let preceding_newlines = existing[..start]
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\n')
        .count();
    if record.leading_newlines > 0 && preceding_newlines != record.preceding_newlines {
        bail!(
            "owned instruction contribution {} has a changed separator",
            record.id
        );
    }
    Ok(OwnedSpan {
        record,
        start,
        begin,
        end_marker_start: content_end,
        end: content_end + end_marker.len(),
    })
}

pub fn reconcile(
    existing: &str,
    prior: &[OwnedContribution],
    desired: &[Contribution],
) -> Result<AggregateEdit> {
    let mut desired_by_id = BTreeMap::new();
    for contribution in desired {
        validate_contribution(contribution)?;
        if desired_by_id
            .insert(contribution.id.clone(), contribution.clone())
            .is_some()
        {
            bail!(
                "duplicate desired instruction contribution {}",
                contribution.id
            );
        }
    }
    let mut spans = prior
        .iter()
        .map(|record| find_owned_span(existing, record))
        .collect::<Result<Vec<_>>>()?;
    for (position, _) in existing.match_indices("<!-- upskill:") {
        if !spans
            .iter()
            .any(|span| position == span.begin || position == span.end_marker_start)
        {
            bail!("unowned or malformed instruction contribution marker");
        }
    }
    spans.sort_by_key(|span| span.start);
    let mut content = String::new();
    let mut owned = Vec::new();
    let mut cursor = 0;
    for span in spans {
        if span.start < cursor {
            bail!("overlapping instruction contributions");
        }
        content.push_str(&existing[cursor..span.start]);
        if let Some(contribution) = desired_by_id.remove(&span.record.id) {
            let preceding_newlines = content
                .bytes()
                .rev()
                .take_while(|byte| *byte == b'\n')
                .count();
            content.push_str(&existing[span.start..span.begin]);
            content.push_str(&format!("<!-- upskill:begin {} -->\n", contribution.id));
            content.push_str(&contribution.markdown);
            content.push_str(&format!("<!-- upskill:end {} -->\n", contribution.id));
            owned.push(OwnedContribution {
                id: contribution.id,
                content_sha256: content_hash(&contribution.markdown),
                leading_newlines: span.record.leading_newlines,
                preceding_newlines,
            });
        }
        cursor = span.end;
    }
    content.push_str(&existing[cursor..]);
    for contribution in desired_by_id.into_values() {
        owned.push(append_contribution(&mut content, contribution));
    }
    owned.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(AggregateEdit { content, owned })
}
