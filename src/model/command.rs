//! Command items (§3.9).

use crate::model::common::{License, Metadata, SchemaVersion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    #[serde(default)]
    pub schema: SchemaVersion,
    pub name: String,
    pub description: String,
    #[serde(
        default,
        rename = "argument-hint",
        skip_serializing_if = "Option::is_none"
    )]
    pub argument_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parses_name_description_and_argument_hint() {
        let command: Command = serde_yaml_ng::from_str(
            "name: release-notes\ndescription: Write the notes\nargument-hint: '[version]'\n",
        )
        .unwrap();
        assert_eq!(command.name, "release-notes");
        assert_eq!(command.description, "Write the notes");
        assert_eq!(command.argument_hint.as_deref(), Some("[version]"));
        assert!(command.ignore.is_empty());
    }

    #[test]
    fn command_refuses_an_unknown_field() {
        let error = serde_yaml_ng::from_str::<Command>(
            "name: release-notes\ndescription: Write the notes\naudience: [claude]\n",
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("unknown field `audience`"),
            "{error}"
        );
    }
}
