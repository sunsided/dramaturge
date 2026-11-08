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
//!
//! # Text pools
//!
//! [`load_pool_yaml`] reads a text pool (`id`, `pick`, `one_of`; see
//! `dramaturge_syntax::shape`) into a
//! [`Pool<DialogTypes>`](dramaturge_model::Pool) for the selection core
//! (`dramaturge_runtime::text`). A dialogue line says a named pool with
//! `say: { text: <pool id> }`.
//!
//! ```rust
//! use dramaturge_model::{Pick, Pools, SymbolTable};
//! use dramaturge_serde::loader::load_pool_yaml;
//!
//! let yaml = r#"
//! id: keeper_remark
//! pick: best
//! one_of:
//!   - "Mind your step."
//!   - when: room == "kitchen"
//!     text: "Do not touch the stove."
//! "#;
//! let mut symbols = SymbolTable::new();
//! let pool = load_pool_yaml(yaml, &mut symbols).unwrap();
//! assert_eq!(pool.pick, Pick::Best);
//! let pools = Pools::new().with(pool);
//! assert!(pools.get("keeper_remark").is_some());
//! ```

pub use dramaturge_syntax::{
    Dialogue, Issue, Loaded as LoadedDialogue, LowerError, ParseError, TextPool,
};
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
/// `C`, `X`, `E` and `M` are the condition, text, effect and note
/// payloads. For plain dramaturge, use the defaults of [`Dialogue`]:
/// `let d: Dialogue = from_yaml(yaml)?;`. A host with its own note type
/// (for example loremaster's `Note`) sets `M`.
pub fn from_yaml<C, X, E, M>(yaml: &str) -> Result<Dialogue<C, X, E, M>, YamlError>
where
    C: DeserializeOwned,
    X: DeserializeOwned,
    E: DeserializeOwned,
    M: DeserializeOwned,
{
    serde_saphyr::from_str(yaml).map_err(YamlError::from)
}

/// Writes a dialogue as YAML.
pub fn to_yaml<C: Serialize, X: Serialize, E: Serialize, M: Serialize>(
    dialogue: &Dialogue<C, X, E, M>,
) -> Result<String, YamlError> {
    serde_saphyr::to_string(dialogue).map_err(|e| YamlError {
        message: e.to_string(),
        line: None,
        column: None,
    })
}

/// An error of [`load_yaml`] and [`load_pool_yaml`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LoadError {
    /// The YAML does not parse or does not have the dialogue shape.
    Yaml(YamlError),
    /// The dialogue does not lower: a key, target or condition error.
    Lower(LowerError<ParseError>),
    /// A candidate key of the pool would share progress with another
    /// candidate (see [`Pool::bad_key`](dramaturge_model::Pool::bad_key)).
    BadKey(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Yaml(e) => e.fmt(f),
            LoadError::Lower(e) => e.fmt(f),
            LoadError::BadKey(key) => write!(
                f,
                "candidate key `{key}` is empty, digits only, has a `/` or is not unique in its pool"
            ),
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

/// Reads a text pool from YAML.
///
/// `C` and `X` are the condition and text payloads. For plain dramaturge,
/// use the defaults: `let p: TextPool = pool_from_yaml(yaml)?;`.
pub fn pool_from_yaml<C, X>(yaml: &str) -> Result<TextPool<C, X>, YamlError>
where
    C: DeserializeOwned,
    X: DeserializeOwned,
{
    serde_saphyr::from_str(yaml).map_err(YamlError::from)
}

/// Writes a text pool as YAML.
pub fn pool_to_yaml<C: Serialize, X: Serialize>(
    pool: &TextPool<C, X>,
) -> Result<String, YamlError> {
    serde_saphyr::to_string(pool).map_err(|e| YamlError {
        message: e.to_string(),
        line: None,
        column: None,
    })
}

/// Reads a text pool from YAML and lowers it to a
/// [`Pool<DialogTypes>`](dramaturge_model::Pool).
///
/// Speaker keys and the actors of `seen_by(..)` go into `symbols`, so
/// pass the symbol table of the dialogues that use the pool. On an error,
/// `symbols` is unchanged. The pool needs an `id`, and its candidate keys
/// must pass [`Pool::bad_key`](dramaturge_model::Pool::bad_key).
pub fn load_pool_yaml(
    yaml: &str,
    symbols: &mut dramaturge_model::SymbolTable,
) -> Result<dramaturge_model::Pool<dramaturge_model::DialogTypes>, LoadError> {
    let pool: TextPool = pool_from_yaml(yaml).map_err(LoadError::Yaml)?;
    // Lower into a copy: lowering interns keys before it can fail.
    let mut scratch = symbols.clone();
    let pool =
        dramaturge_syntax::lower_pool_plain(&pool, &mut scratch).map_err(LoadError::Lower)?;
    if let Some(key) = pool.bad_key() {
        return Err(LoadError::BadKey(key.to_string()));
    }
    *symbols = scratch;
    Ok(pool)
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
        let err = from_yaml::<String, dramaturge_syntax::Text, dramaturge_syntax::Effect, String>(
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

    #[test]
    fn test_host_note_type() {
        #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
        struct Note {
            text: String,
            #[serde(default)]
            todo: bool,
        }
        let yaml = EXCHANGE.replace(
            "with: mara\n",
            "with: mara\nnotes:\n  - { text: \"Check the pacing.\", todo: true }\n",
        );
        let d: Dialogue<String, dramaturge_syntax::Text, dramaturge_syntax::Effect, Note> =
            from_yaml(&yaml).unwrap();
        check!(d.notes[0].todo);
        let again: Dialogue<String, dramaturge_syntax::Text, dramaturge_syntax::Effect, Note> =
            from_yaml(&to_yaml(&d).unwrap()).unwrap();
        check!(again == d);
    }
}
