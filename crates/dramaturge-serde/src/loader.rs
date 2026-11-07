//! YAML loader for dialogues.
//!
//! The YAML shape is the dialogue shape of `dramaturge-syntax` (decision
//! D3: one dialogue format, the shape that loremaster's editor uses). The
//! loader reads YAML into a [`Dialogue`] and lowers it with
//! [`dramaturge_syntax::lower_plain`].
//!
//! ```yaml
//! with: mara
//! start: [again, intro]
//! nodes:
//!   intro:
//!     say: "Hello, traveller."
//!     set: { met_mara: true }
//!     options:
//!       - say: "Who are you?"
//!         go: names
//!         relation: { trust: 1 }
//!       - say: "Goodbye."
//!         go: end
//!   again:
//!     when: met_mara
//!     say: "You again."
//!     options:
//!       - { say: "Goodbye.", go: end }
//!   names:
//!     say: "They call me Mara."
//!     lines:
//!       - { speaker: player, say: "Mara. I will remember." }
//!     next: end
//! ```
//!
//! See the `dramaturge-syntax` documentation for every field, and
//! `docs/migration-0.3-yaml.md` for the change from the 0.2 shape.
//!
//! # Keys, not numbers
//!
//! Every reference is a stable key: node keys, option ids, targets, the
//! start nodes and speakers. The lowering interns each key into a
//! [`SymbolTable`](dramaturge_model::SymbolTable) and returns it next to the graph.
//! The key `player` is the player ([`SymbolTable::player`](dramaturge_model::SymbolTable::player)).
//! Numeric ids from the 1.0 format were removed in 0.3 (ADR 0001); a
//! number in quotes (`go: "2"`) is a key.
//!
//! # Host payloads
//!
//! [`from_yaml`] is generic over the condition, text and effect payloads,
//! so a host with its own payloads reads the same shape and lowers it with
//! its own [`Lower`](dramaturge_syntax::Lower) hooks. [`load_yaml`] uses
//! the defaults of plain dramaturge.

pub use dramaturge_syntax::{Dialogue, Issue, Loaded as LoadedDialogue, LowerError, ParseError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt;

/// A YAML syntax or shape error, with its position in the input.
///
/// Wraps the YAML backend's error so that the backend can change without
/// changing this crate's public API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlError {
    message: String,
    line: Option<u64>,
    column: Option<u64>,
}

impl YamlError {
    /// The error message, without position.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The 1-based line of the error, if known.
    pub fn line(&self) -> Option<u64> {
        self.line
    }

    /// The 1-based column of the error, if known.
    pub fn column(&self) -> Option<u64> {
        self.column
    }
}

impl std::fmt::Display for YamlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(column)) => {
                write!(f, "line {line}, column {column}: {}", self.message)
            }
            (Some(line), None) => write!(f, "line {line}: {}", self.message),
            _ => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for YamlError {}

impl From<serde_saphyr::Error> for YamlError {
    fn from(e: serde_saphyr::Error) -> Self {
        let location = e.location();
        Self {
            message: e.to_string(),
            line: location.map(|l| l.line()),
            column: location.map(|l| l.column()),
        }
    }
}

/// Reads a dialogue from YAML.
///
/// `C`, `X` and `E` are the condition, text and effect payloads. For plain
/// dramaturge, use `from_yaml::<String, Text, Effect>` (the defaults of
/// [`Dialogue`]): `let d: Dialogue = from_yaml(yaml)?;`.
pub fn from_yaml<C, X, E>(yaml: &str) -> Result<Dialogue<C, X, E>, YamlError>
where
    C: DeserializeOwned,
    X: DeserializeOwned,
    E: DeserializeOwned,
{
    serde_saphyr::from_str(yaml).map_err(YamlError::from)
}

/// Writes a dialogue as YAML.
pub fn to_yaml<C: Serialize, X: Serialize, E: Serialize>(
    dialogue: &Dialogue<C, X, E>,
) -> Result<String, YamlError> {
    serde_saphyr::to_string(dialogue).map_err(|e| YamlError {
        message: e.to_string(),
        line: None,
        column: None,
    })
}

/// An error of [`load_yaml`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LoadError {
    /// The YAML does not parse or does not have the dialogue shape.
    Yaml(YamlError),
    /// The dialogue does not lower: a key, target or condition error.
    Lower(LowerError<ParseError>),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Yaml(e) => e.fmt(f),
            LoadError::Lower(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for LoadError {}

/// Reads a dialogue from YAML and lowers it to a
/// [`DialogueGraph`](dramaturge_model::DialogueGraph) with a new symbol table.
///
/// The [`LoadedDialogue`] has the graph, the symbol table and the warnings
/// of the check (for example a node with no options and no `next`).
pub fn load_yaml(yaml: &str) -> Result<LoadedDialogue, LoadError> {
    let dialogue: Dialogue = from_yaml(yaml).map_err(LoadError::Yaml)?;
    dramaturge_syntax::lower_plain(&dialogue).map_err(LoadError::Lower)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{assert, check};
    use dramaturge_model as model;
    use dramaturge_syntax::{IssueKind, Place};

    const EXCHANGE: &str = r#"
with: mara
start: meet
nodes:
  meet:
    say: "Sit, if you like."
    lines:
      - { speaker: player, say: "Who are you?" }
      - speaker: mara
        say:
          - text: "They call me Mara."
          - { text: "You again.", mode: once, key: again }
    next: names
  names:
    say: "And you?"
    next: end
"#;

    #[test]
    fn test_load_lines_and_next() {
        let loaded = load_yaml(EXCHANGE).expect("loads");
        check!(loaded.warnings.is_empty());
        let mara = model::CharacterId::new(loaded.symbols.entity("mara").unwrap());
        let player = model::CharacterId::new(
            loaded
                .symbols
                .entity(model::SymbolTable::PLAYER_KEY)
                .unwrap(),
        );
        let names = loaded.symbols.node("names").unwrap();
        let meet = &loaded.graph.nodes[0];
        let speakers: Vec<_> = meet.all_lines().map(|(speaker, _)| speaker).collect();
        check!(speakers == vec![mara, player, mara]);
        check!(meet.lines[0].text == model::DialogText::inline("Who are you?"));
        check!(let model::DialogText::Fragments(_) = &meet.lines[1].text);
        check!(meet.next == Some(model::DialogTarget::Node(names)));
        check!(loaded.graph.nodes[1].next == Some(model::DialogTarget::End));
    }

    #[test]
    fn test_round_trip() {
        let d: Dialogue = from_yaml(EXCHANGE).unwrap();
        let yaml = to_yaml(&d).unwrap();
        let again: Dialogue = from_yaml(&yaml).unwrap();
        check!(again == d, "{yaml}");
    }

    #[test]
    fn test_unknown_target_names_the_place() {
        let yaml = EXCHANGE.replace("next: names", "next: nmaes");
        let err = load_yaml(&yaml).unwrap_err();
        assert!(let LoadError::Lower(LowerError::Check(issues)) = &err);
        check!(issues[0].kind == IssueKind::UnknownTarget);
        check!(issues[0].place == Place::Next(0));
        check!(err.to_string() == "nodes.meet.next: no node `nmaes`");
    }

    #[test]
    fn test_node_without_a_way_on_warns() {
        let yaml = EXCHANGE.replace("    next: end\n", "");
        let loaded = load_yaml(&yaml).unwrap();
        check!(loaded.warnings.len() == 1);
        check!(loaded.warnings[0].kind == IssueKind::NoWayOn);
    }

    #[test]
    fn test_numeric_ids_are_errors() {
        for (from, to) in [
            ("next: names", "next: 2"),
            ("start: meet", "start: [meet, 2]"),
            ("start: meet", "start: 1"),
        ] {
            let err = load_yaml(&EXCHANGE.replace(from, to)).unwrap_err();
            assert!(let LoadError::Yaml(e) = &err);
            check!(e.message().contains("numeric id"), "{e}");
            check!(e.message().contains("removed in 0.3"), "{e}");
            check!(e.line().is_some(), "{e}");
        }
        let quoted = EXCHANGE
            .replace("  names:", "  \"2\":")
            .replace("next: names", "next: \"2\"");
        let loaded = load_yaml(&quoted).unwrap();
        check!(loaded.symbols.node("2").is_some());
    }

    #[test]
    fn test_yaml_error_has_line_and_column() {
        let err = from_yaml::<String, dramaturge_syntax::Text, dramaturge_syntax::Effect>(
            "with: mara\nstart: [a\nnodes: {}\n",
        )
        .unwrap_err();
        check!(err.line().is_some(), "{err}");
        check!(err.to_string().starts_with("line "), "{err}");
    }

    #[test]
    fn test_unknown_field_is_an_error() {
        let yaml = EXCHANGE.replace("    next: names", "    nxet: names");
        let err = load_yaml(&yaml).unwrap_err();
        assert!(let LoadError::Yaml(e) = &err);
        check!(e.message().contains("nxet"), "{e}");
    }
}
