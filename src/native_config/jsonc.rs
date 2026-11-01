use std::ops::Range;

use anyhow::{Result, bail, ensure};
use serde_json::Value;

use super::{Children, Member, Node};

fn string_end(bytes: &[u8], start: usize) -> Result<usize> {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return Ok(i + 1),
            _ => i += 1,
        }
    }
    bail!("unterminated JSON string")
}

pub(super) fn masked(source: &str) -> Result<String> {
    let mut bytes = comments_only(source)?.into_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            i = string_end(&bytes, i)?;
            continue;
        }
        if bytes[i] == b',' {
            let previous = bytes[..i].iter().rfind(|b| !b.is_ascii_whitespace());
            let next = bytes[i + 1..].iter().find(|b| !b.is_ascii_whitespace());
            if matches!(next, Some(b']' | b'}'))
                && previous.is_some_and(|b| !matches!(b, b'[' | b'{' | b',' | b':'))
            {
                bytes[i] = b' ';
            }
        }
        i += 1;
    }
    Ok(String::from_utf8(bytes)?)
}

pub(super) fn parse(source: &str) -> Result<Node> {
    let masked = masked(source)?;
    let value = crate::parse::json::parse_unique(&masked)?;
    ensure!(value.is_object(), "configuration root must be an object");
    let mut position = 0;
    recover(&masked, &value, &mut position)
}
fn skip(source: &str, position: &mut usize) {
    while source
        .as_bytes()
        .get(*position)
        .is_some_and(u8::is_ascii_whitespace)
    {
        *position += 1;
    }
}
fn recover(source: &str, value: &Value, position: &mut usize) -> Result<Node> {
    skip(source, position);
    let start = *position;
    let children = match value {
        Value::Object(values) => {
            *position += 1;
            skip(source, position);
            let mut members = Vec::new();
            while source.as_bytes()[*position] != b'}' {
                let key_start = *position;
                let end = string_end(source.as_bytes(), key_start)?;
                let key = serde_json::from_str::<String>(&source[key_start..end])?;
                *position = end;
                skip(source, position);
                let colon = *position..*position + 1;
                *position += 1;
                let node = recover(source, &values[&key], position)?;
                members.push(Member {
                    key,
                    key_span: key_start..end,
                    separator: colon,
                    node,
                });
                skip(source, position);
                if source.as_bytes()[*position] == b',' {
                    *position += 1;
                    skip(source, position);
                }
            }
            *position += 1;
            Children::Object(members)
        }
        Value::Array(values) => {
            *position += 1;
            let mut nodes = Vec::new();
            for value in values {
                nodes.push(recover(source, value, position)?);
                skip(source, position);
                if source.as_bytes()[*position] == b',' {
                    *position += 1;
                }
            }
            skip(source, position);
            *position += 1;
            Children::Array(nodes)
        }
        Value::String(_) => {
            *position = string_end(source.as_bytes(), *position)?;
            Children::Scalar
        }
        _ => {
            while source
                .as_bytes()
                .get(*position)
                .is_some_and(|b| !b.is_ascii_whitespace() && !matches!(b, b',' | b']' | b'}'))
            {
                *position += 1;
            }
            Children::Scalar
        }
    };
    Ok(Node {
        span: start..*position,
        value: Some(value.clone()),
        children,
        insert: None,
        is_table: false,
    })
}

pub(super) fn insert(
    source: &str,
    container: &Node,
    key: Option<&str>,
    value: &Value,
) -> Result<String> {
    let close = container.span.end - 1;
    let masked = comments_only(source)?;
    let previous = masked[..close]
        .bytes()
        .rfind(|b| !b.is_ascii_whitespace())
        .unwrap();
    let mut text = String::new();
    if !matches!(previous, b'{' | b'[' | b',') {
        text.push(',');
    }
    if let Some(key) = key {
        text.push_str(&serde_json::to_string(key)?);
        text.push(':');
    }
    text.push_str(&serde_json::to_string(value)?);
    // Preserve an existing trailing separator after the newly inserted token.
    if previous == b',' {
        text.push(',');
    }
    Ok(super::splice(source, vec![(close..close, text)]))
}
fn comments_only(source: &str) -> Result<String> {
    // Mask comments at equal byte offsets; preserve actual separator tokens.
    let mut bytes = source.as_bytes().to_vec();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            i = string_end(&bytes, i)?;
            continue;
        }
        if bytes[i..].starts_with(b"//") {
            while i < bytes.len() && !matches!(bytes[i], b'\r' | b'\n') {
                bytes[i] = b' ';
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && &bytes[i..i + 2] != b"*/" {
                i += 1;
            }
            ensure!(i + 1 < bytes.len(), "unterminated JSONC comment");
            i += 2;
            for b in &mut bytes[start..i] {
                if !matches!(*b, b'\r' | b'\n') {
                    *b = b' ';
                }
            }
        } else {
            i += 1;
        }
    }
    Ok(String::from_utf8(bytes)?)
}
pub(super) fn remove(
    source: &str,
    parent: &Node,
    selected: &Node,
    member: Option<&Member>,
) -> Result<String> {
    let masked = comments_only(source)?;
    let mut edits = vec![(selected.span.clone(), String::new())];
    let start = if let Some(member) = member {
        edits.push((member.key_span.clone(), String::new()));
        edits.push((member.separator.clone(), String::new()));
        member.key_span.start
    } else {
        selected.span.start
    };
    let after = masked[selected.span.end..parent.span.end - 1]
        .bytes()
        .position(|b| !b.is_ascii_whitespace())
        .map(|i| selected.span.end + i);
    if after.is_some_and(|i| masked.as_bytes()[i] == b',') {
        let i = after.unwrap();
        edits.push((i..i + 1, String::new()));
    } else if let Some(i) = masked[parent.span.start + 1..start]
        .bytes()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|i| parent.span.start + 1 + i)
    {
        if masked.as_bytes()[i] == b',' {
            edits.push((i..i + 1, String::new()));
        }
    }
    Ok(super::splice(source, edits))
}

pub(super) fn replace(source: &str, span: Range<usize>, value: &Value) -> Result<String> {
    Ok(super::splice(
        source,
        vec![(span, serde_json::to_string(value)?)],
    ))
}
