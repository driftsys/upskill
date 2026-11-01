use anyhow::{Result, bail, ensure};
use serde_json::{Map, Value};
use toml_edit::{ImDocument, Item, Table};

use super::{Children, Member, Node};

pub(super) fn parse(source: &str) -> Result<Node> {
    let document = ImDocument::parse(source)?;
    table(source, document.as_table(), true)
}
fn table(source: &str, table: &Table, root: bool) -> Result<Node> {
    let span = if root {
        0..source.len()
    } else {
        table.span().unwrap_or(0..0)
    };
    let insert = if root {
        Some(if source.starts_with('\u{feff}') { 3 } else { 0 })
    } else if !table.is_implicit() && !table.is_dotted() {
        Some(
            source[span.start..]
                .find('\n')
                .map_or(source.len(), |i| span.start + i + 1),
        )
    } else {
        None
    };
    let mut members = Vec::new();
    for (name, item) in table.iter() {
        let (key, _) = table.get_key_value(name).unwrap();
        let key_span = key.span().unwrap_or(0..0);
        let node = match item {
            Item::Table(child) => self::table(source, child, false)?,
            Item::Value(value) => value_node(value)?,
            _ => Node {
                span: item.span().unwrap_or(0..0),
                value: None,
                children: Children::Scalar,
                insert: None,
                is_table: false,
            },
        };
        members.push(Member {
            key: name.to_owned(),
            separator: key_span.end..node.span.start,
            key_span,
            node,
        });
    }
    let value = members
        .iter()
        .map(|m| Some((m.key.clone(), m.node.value.clone()?)))
        .collect::<Option<Map<String, Value>>>()
        .map(Value::Object);
    Ok(Node {
        span,
        value,
        children: Children::Object(members),
        insert,
        is_table: true,
    })
}
fn value_node(value: &toml_edit::Value) -> Result<Node> {
    let span = value.span().unwrap_or(0..0);
    let (semantic, children) = match value {
        toml_edit::Value::String(v) => (Some(Value::String(v.value().clone())), Children::Scalar),
        toml_edit::Value::Integer(v) => (Some((*v.value()).into()), Children::Scalar),
        toml_edit::Value::Float(v) => (
            serde_json::Number::from_f64(*v.value()).map(Value::Number),
            Children::Scalar,
        ),
        toml_edit::Value::Boolean(v) => (Some((*v.value()).into()), Children::Scalar),
        toml_edit::Value::Datetime(_) => (None, Children::Scalar),
        toml_edit::Value::Array(array) => {
            let nodes = array.iter().map(value_node).collect::<Result<Vec<_>>>()?;
            let value = nodes
                .iter()
                .map(|n| n.value.clone())
                .collect::<Option<Vec<_>>>()
                .map(Value::Array);
            (value, Children::Array(nodes))
        }
        toml_edit::Value::InlineTable(table) => {
            let mut members = Vec::new();
            for (name, value) in table.iter() {
                let (key, _) = table.get_key_value(name).unwrap();
                let key_span = key.span().unwrap_or(0..0);
                let node = value_node(value)?;
                members.push(Member {
                    key: name.to_owned(),
                    separator: key_span.end..node.span.start,
                    key_span,
                    node,
                });
            }
            let semantic = members
                .iter()
                .map(|m| Some((m.key.clone(), m.node.value.clone()?)))
                .collect::<Option<Map<_, _>>>()
                .map(Value::Object);
            (semantic, Children::Object(members))
        }
    };
    Ok(Node {
        span,
        value: semantic,
        children,
        insert: None,
        is_table: false,
    })
}

pub(super) fn encode(value: &Value) -> Result<String> {
    Ok(match value {
        Value::Null => bail!("TOML has no null value"),
        Value::Bool(_) | Value::Number(_) | Value::String(_) => serde_json::to_string(value)?,
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(encode)
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        ),
        Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(k, v)| Ok(format!("{} = {}", serde_json::to_string(k)?, encode(v)?)))
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        ),
    })
}
pub(super) fn insert(source: &str, root: &Node, path: &[String], value: &Value) -> Result<String> {
    let mut table = root;
    let mut consumed = 0;
    // An explicit table has a reliable header anchor. Implicit/dotted tables
    // instead use a dotted assignment relative to the nearest explicit table.
    for (i, key) in path[..path.len() - 1].iter().enumerate() {
        let Some(child) = root.lookup(&path[..i + 1])? else {
            break;
        };
        if child.insert.is_some() {
            table = child;
            consumed = i + 1;
        } else if matches!(child.children, Children::Scalar | Children::Array(_)) {
            bail!("configuration parent is not a table: {key}");
        }
    }
    let anchor = table
        .insert
        .ok_or_else(|| anyhow::anyhow!("no TOML insertion anchor"))?;
    let name = path[consumed..]
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join(".");
    let prefix =
        if anchor > 0 && &source[..anchor] != "\u{feff}" && !source[..anchor].ends_with('\n') {
            "\n"
        } else {
            ""
        };
    let text = format!("{prefix}{name} = {}\n", encode(value)?);
    Ok(super::splice(source, vec![(anchor..anchor, text)]))
}
pub(super) fn insert_array(source: &str, node: &Node, value: &Value) -> Result<String> {
    let Children::Array(nodes) = &node.children else {
        bail!("configuration selector is not an array");
    };
    let close = node.span.end - 1;
    let tail = nodes.last().map_or(node.span.start + 1, |n| n.span.end)..close;
    let comma = separator(source, tail).is_some();
    let prefix = if nodes.is_empty() || comma { "" } else { "," };
    let suffix = if comma { "," } else { "" };
    Ok(super::splice(
        source,
        vec![(close..close, format!("{prefix}{}{suffix}", encode(value)?))],
    ))
}
fn separator(source: &str, range: std::ops::Range<usize>) -> Option<usize> {
    let mut comment = false;
    for i in range {
        let byte = source.as_bytes()[i];
        if byte == b'\n' {
            comment = false;
        }
        if !comment && byte == b',' {
            return Some(i);
        }
        if byte == b'#' {
            comment = true;
        }
    }
    None
}
pub(super) fn remove(
    source: &str,
    parent: &Node,
    node: &Node,
    member: Option<&Member>,
) -> Result<String> {
    if parent.is_table {
        let member = member.ok_or_else(|| anyhow::anyhow!("TOML assignment has no key"))?;
        ensure!(
            !node.is_table,
            "removing TOML table selectors is unsupported"
        );
        let line = source[..member.key_span.start]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        let line = if line == 0 && source.starts_with('\u{feff}') {
            3
        } else {
            line
        };
        let start = line
            + source[line..member.key_span.start]
                .bytes()
                .take_while(u8::is_ascii_whitespace)
                .count();
        return Ok(super::splice(
            source,
            vec![(start..node.span.end, String::new())],
        ));
    }
    ensure!(
        !parent.span.is_empty() && !node.span.is_empty(),
        "selected implicit inline TOML table has no editable token span"
    );
    let (start, end) = member.map_or((node.span.start, node.span.end), |m| {
        (m.key_span.start, node.span.end)
    });
    let mut edits = vec![(start..end, String::new())];
    let after = separator(source, end..parent.span.end - 1);
    if let Some(i) = after {
        edits.push((i..i + 1, String::new()));
    } else {
        let siblings = match &parent.children {
            Children::Array(nodes) => nodes.iter().map(|n| n.span.end).collect::<Vec<_>>(),
            Children::Object(members) => members.iter().map(|m| m.node.span.end).collect(),
            Children::Scalar => Vec::new(),
        };
        let previous = siblings
            .into_iter()
            .filter(|e| *e <= start)
            .max()
            .unwrap_or(parent.span.start + 1);
        if let Some(i) = separator(source, previous..start) {
            edits.push((i..i + 1, String::new()));
        }
    }
    Ok(super::splice(source, edits))
}
