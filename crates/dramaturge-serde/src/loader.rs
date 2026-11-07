//! YAML loader for dialogue graphs.
//!
//! This module provides functionality to load dialogue graphs from YAML files.
//! It uses serde-friendly wrapper types that can be converted to the core model types.
//!
//! # Keys, not numbers
//!
//! Every reference in the YAML format is a stable author key: node ids,
//! option ids, option targets, the entry node, speakers and relationship
//! endpoints. [`YamlDialogueGraph::to_model`] interns each key into a
//! [`SymbolTable`](model::SymbolTable) and returns it next to the graph, so
//! diagnostics and save games can show keys instead of numbers.
//!
//! ```yaml
//! version: "2.0"
//! entry_node: mara.intro
//! characters:
//!   player: { name: "Player" }
//!   mara: { name: "Mara Vey" }
//! nodes:
//!   - id: mara.intro
//!     speaker: mara
//!     text: "Hello."
//!     options:
//!       - id: mara.intro.bye
//!         text: "Goodbye."
//!         outcomes:
//!           - { type: add_relationship, source: player, target: mara, axis: trust, delta: 5 }
//!         target: end
//! ```
//!
//! Numeric ids from the 1.0 format (`id: 2`, `source: 1`, character `id:`)
//! were deprecated in 0.2 and are an error since 0.3 (ADR 0001). A number
//! in quotes (`target: "2"`) is a key and still loads.

use dramaturge_model as model;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// A dialogue graph loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDialogueGraph {
    /// Schema version for compatibility.
    pub version: String,
    /// Named characters referenced in the dialogue, by key.
    ///
    /// Sorted by key so that interned entity ids are deterministic.
    pub characters: BTreeMap<String, YamlCharacter>,
    /// All nodes in the graph.
    pub nodes: Vec<YamlNode>,
    /// Entry point node key.
    pub entry_node: YamlKey,
}

/// A stable author key such as `dlg.mara.intro`.
///
/// A YAML number (`id: 2`) is an error: numeric ids from the 1.0 format
/// were removed in 0.3 (ADR 0001). Write the number in quotes (`id: "2"`)
/// to use it as a key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct YamlKey(pub String);

impl YamlKey {
    /// Returns the key text.
    pub fn as_key(&self) -> &str {
        &self.0
    }
}

impl From<&str> for YamlKey {
    fn from(key: &str) -> Self {
        YamlKey(key.to_string())
    }
}

impl From<String> for YamlKey {
    fn from(key: String) -> Self {
        YamlKey(key)
    }
}

/// The error message for a numeric id in YAML.
const NUMERIC_ID_REMOVED: &str = "numeric ids were removed in 0.3 (ADR 0001); \
     use a key, or write the number in quotes";

impl<'de> Deserialize<'de> for YamlKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;

        impl serde::de::Visitor<'_> for KeyVisitor {
            type Value = YamlKey;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a key string")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<YamlKey, E> {
                Ok(YamlKey::from(v))
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<YamlKey, E> {
                Ok(YamlKey(v))
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<YamlKey, E> {
                Err(E::custom(format!("numeric id {v}: {NUMERIC_ID_REMOVED}")))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<YamlKey, E> {
                Err(E::custom(format!("numeric id {v}: {NUMERIC_ID_REMOVED}")))
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<YamlKey, E> {
                Err(E::custom(format!("numeric id {v}: {NUMERIC_ID_REMOVED}")))
            }
        }

        deserializer.deserialize_any(KeyVisitor)
    }
}

/// A character definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawYamlCharacter")]
pub struct YamlCharacter {
    /// Character name for display.
    pub name: String,
}

/// The YAML shape of a character. The removed `id` field fails while the
/// parser reads its value, so the error keeps the line and column.
#[derive(Deserialize)]
struct RawYamlCharacter {
    #[serde(default)]
    #[allow(dead_code)]
    id: Option<RemovedCharacterId>,
    name: String,
}

impl From<RawYamlCharacter> for YamlCharacter {
    fn from(raw: RawYamlCharacter) -> Self {
        YamlCharacter { name: raw.name }
    }
}

/// A value that no YAML can give: the old numeric `id` of a character.
struct RemovedCharacterId;

impl<'de> Deserialize<'de> for RemovedCharacterId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Reject;
        impl serde::de::Visitor<'_> for Reject {
            type Value = RemovedCharacterId;
            // Every `visit_*` keeps its default, which rejects the value with
            // this text at the value's position.
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    f,
                    "no character `id`: {NUMERIC_ID_REMOVED}; the map key is the id"
                )
            }
        }
        deserializer.deserialize_any(Reject)
    }
}

/// The result of converting a [`YamlDialogueGraph`] into the model.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedDialogue {
    /// The dialogue graph.
    pub graph: model::DialogueGraph,
    /// Keys for every entity, node and option id in the graph.
    pub symbols: model::SymbolTable,
    /// Non-fatal messages about the input.
    pub warnings: Vec<String>,
}

/// A dialogue node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlNode {
    /// Unique node key.
    pub id: YamlKey,
    /// Character key who speaks.
    pub speaker: String,
    /// Dialogue text: a string, or a list of fragments.
    pub text: YamlText,
    /// Outcomes applied when node is entered.
    #[serde(default)]
    pub outcomes: Vec<YamlOutcome>,
    /// Dialogue options.
    #[serde(default)]
    pub options: Vec<YamlOption>,
}

/// A dialogue option.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlOption {
    /// Unique option key.
    pub id: YamlKey,
    /// Option text: a string, or a list of fragments. Defaults to the
    /// navigation hint's text.
    #[serde(default)]
    pub text: Option<YamlText>,
    /// Tags categorizing this option.
    #[serde(default)]
    pub tags: Vec<YamlTag>,
    /// Explicit intentions.
    #[serde(default)]
    pub intentions_explicit: Vec<String>,
    /// Implicit intentions.
    #[serde(default)]
    pub intentions_implicit: Vec<String>,
    /// Conditions for availability.
    #[serde(default)]
    pub conditions: Vec<YamlCondition>,
    /// Outcomes when chosen.
    #[serde(default)]
    pub outcomes: Vec<YamlOutcome>,
    /// Target node key, or the special value `end`.
    pub target: YamlKey,
    /// Navigation hint for this option.
    #[serde(default)]
    pub navigation_hint: Option<YamlNavigationHint>,
    /// Target name for GoTo navigation hint.
    #[serde(default)]
    pub go_to_target: Option<String>,
}

/// Navigation hint for dialogue options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YamlNavigationHint {
    /// Continue to next node.
    Continue,
    /// Leave the conversation.
    Leave,
    /// Go to a specific target.
    GoTo,
    /// End the conversation.
    End,
    /// Return to previous menu.
    Return,
    /// Custom navigation label.
    Custom(String),
}

impl YamlNavigationHint {
    /// Convert to a model NavigationHint.
    fn to_model(&self, go_to_target: &Option<String>) -> Result<model::NavigationHint, String> {
        use model::NavigationHint;
        match self {
            YamlNavigationHint::Continue => Ok(NavigationHint::Continue),
            YamlNavigationHint::Leave => Ok(NavigationHint::Leave),
            YamlNavigationHint::GoTo => {
                let target = go_to_target
                    .as_ref()
                    .ok_or("GoTo navigation hint requires 'go_to_target' to be specified")?;
                Ok(NavigationHint::GoTo(target.clone()))
            }
            YamlNavigationHint::End => Ok(NavigationHint::End),
            YamlNavigationHint::Return => Ok(NavigationHint::Return),
            YamlNavigationHint::Custom(label) => Ok(NavigationHint::Custom(label.clone())),
        }
    }

    /// Returns the default display text for this navigation hint.
    fn default_text(&self, go_to_target: &Option<String>) -> String {
        match self {
            YamlNavigationHint::Continue => "Continue...".to_string(),
            YamlNavigationHint::Leave => "Leave...".to_string(),
            YamlNavigationHint::GoTo => format!(
                "Go to {}...",
                go_to_target.clone().unwrap_or_else(|| "Target".to_string())
            ),
            YamlNavigationHint::End => "End conversation".to_string(),
            YamlNavigationHint::Return => "Return...".to_string(),
            YamlNavigationHint::Custom(label) => label.clone(),
        }
    }
}

/// Dialogue text: a plain string, or a list of fragments.
///
/// ```yaml
/// text:
///   - "Mara looks up."
///   - when: { type: fact_at_least, fact: trust, value: 3 }
///     text: "She smiles."
///   - mode: cycle
///     variants: ["\"Again?\"", "\"You.\""]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlText {
    /// Plain inline text.
    Plain(String),
    /// Fragment list.
    Fragments(Vec<YamlFragment>),
}

/// One entry of a fragment list: a string (always shown) or a map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlFragment {
    /// Text that is always shown.
    Plain(String),
    /// A fragment with condition, mode and variants.
    Full(YamlFragmentSpec),
}

/// A fragment written as a map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YamlFragmentSpec {
    /// Stable progress key.
    #[serde(default)]
    pub key: Option<String>,
    /// Show only if this holds.
    #[serde(default)]
    pub when: Option<YamlCondition>,
    /// Display mode (default `always`).
    #[serde(default)]
    pub mode: YamlFragmentMode,
    /// Single text (for `always`, `first_time`, `once`).
    #[serde(default)]
    pub text: Option<YamlText>,
    /// Alternatives (for `cycle`, `shuffle`, `sticky`).
    #[serde(default)]
    pub variants: Vec<YamlText>,
    /// Replace the text of earlier fragments.
    #[serde(default)]
    pub replaces: bool,
}

/// Fragment display mode.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum YamlFragmentMode {
    #[default]
    Always,
    FirstTime,
    Once,
    Cycle,
    Shuffle,
    Sticky,
}

impl YamlText {
    fn to_model(&self, ctx: &mut ConvertContext) -> Result<model::DialogText, String> {
        match self {
            YamlText::Plain(s) => Ok(model::DialogText::inline(s)),
            YamlText::Fragments(fs) => fs
                .iter()
                .enumerate()
                .map(|(i, f)| f.to_model(ctx).map_err(|e| format!("fragment {i}: {e}")))
                .collect::<Result<_, _>>()
                .map(model::DialogText::Fragments),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            YamlText::Plain(s) => s.is_empty(),
            YamlText::Fragments(fs) => fs.is_empty(),
        }
    }
}

impl YamlFragment {
    fn to_model(&self, ctx: &mut ConvertContext) -> Result<model::TextFragment, String> {
        use model::{FragmentMode, TextFragment};
        let spec = match self {
            YamlFragment::Plain(s) => {
                return Ok(TextFragment::always(model::DialogText::inline(s)));
            }
            YamlFragment::Full(spec) => spec,
        };
        let mut variants = Vec::new();
        if let Some(text) = &spec.text {
            variants.push(text.to_model(ctx)?);
        }
        for v in &spec.variants {
            variants.push(v.to_model(ctx)?);
        }
        if variants.is_empty() {
            return Err("fragment needs `text` or `variants`".to_string());
        }
        let mode = match spec.mode {
            YamlFragmentMode::Always => FragmentMode::Always,
            YamlFragmentMode::FirstTime => FragmentMode::FirstTime,
            YamlFragmentMode::Once => FragmentMode::Once,
            YamlFragmentMode::Cycle => FragmentMode::Cycle,
            YamlFragmentMode::Shuffle => FragmentMode::Shuffle,
            YamlFragmentMode::Sticky => FragmentMode::Sticky,
        };
        Ok(TextFragment {
            key: spec.key.as_deref().map(Into::into),
            variants,
            condition: spec.when.as_ref().map(|c| c.to_model(ctx)).transpose()?,
            mode,
            replaces: spec.replaces,
        })
    }
}

/// A tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlTag {
    /// Tag axis/category.
    pub axis: String,
    /// Tag key/value.
    pub key: String,
    /// Optional display prefix for rendering (e.g., "[Medic]").
    #[serde(default)]
    pub display_prefix: Option<String>,
    /// Whether this tag requires the player to have the trait.
    #[serde(default)]
    pub requires_trait: Option<bool>,
}

/// A condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum YamlCondition {
    /// Always true.
    #[serde(rename = "always")]
    Always,
    /// Check if a fact exists.
    #[serde(rename = "fact_exists")]
    FactExists {
        /// Fact scope (world, player, etc).
        #[serde(default = "default_fact_scope")]
        scope: String,
        /// Fact ID.
        fact: String,
    },
    /// Check that an integer fact is at least a value.
    #[serde(rename = "fact_at_least")]
    FactAtLeast {
        /// Fact scope.
        #[serde(default = "default_fact_scope")]
        scope: String,
        /// Fact ID.
        fact: String,
        /// Minimum value.
        value: i64,
    },
    /// Check that an integer fact is at most a value.
    #[serde(rename = "fact_at_most")]
    FactAtMost {
        /// Fact scope.
        #[serde(default = "default_fact_scope")]
        scope: String,
        /// Fact ID.
        fact: String,
        /// Maximum value.
        value: i64,
    },
    /// Check if a fact equals a value.
    #[serde(rename = "fact_equals")]
    FactEquals {
        /// Fact scope.
        #[serde(default = "default_fact_scope")]
        scope: String,
        /// Fact ID.
        fact: String,
        /// Expected value.
        value: YamlValue,
    },
    /// Check that an entity has the memory with the given key.
    #[serde(rename = "has_memory")]
    HasMemory {
        /// Character key of the entity that owns the memory.
        owner: YamlKey,
        /// Memory key.
        id: YamlKey,
    },
}

fn default_fact_scope() -> String {
    "world".to_string()
}

/// A value in a condition or outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlValue {
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i64),
    /// String value.
    String(String),
}

/// An outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum YamlOutcome {
    /// Set a fact.
    #[serde(rename = "set_fact")]
    SetFact {
        /// Fact scope.
        #[serde(default = "default_fact_scope")]
        scope: String,
        /// Fact ID.
        fact: String,
        /// Fact value.
        value: YamlValue,
    },
    /// Add to an integer fact. A missing fact starts at 0.
    #[serde(rename = "add_fact")]
    AddFact {
        /// Fact scope.
        #[serde(default = "default_fact_scope")]
        scope: String,
        /// Fact ID.
        fact: String,
        /// Amount to add (positive or negative).
        delta: i64,
    },
    /// Add to a relationship.
    #[serde(rename = "add_relationship")]
    AddRelationship {
        /// Source entity key (a character key).
        source: YamlKey,
        /// Target entity key (a character key).
        target: YamlKey,
        /// Relationship axis.
        axis: String,
        /// Amount to add.
        delta: i32,
    },
}

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

impl YamlDialogueGraph {
    /// Load a dialogue graph from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, YamlError> {
        serde_saphyr::from_str(yaml).map_err(YamlError::from)
    }

    /// Convert to a model [`DialogueGraph`](model::DialogueGraph).
    ///
    /// Interns every key into a [`SymbolTable`](model::SymbolTable). Load
    /// errors name the offending keys.
    pub fn to_model(&self) -> Result<LoadedDialogue, String> {
        use model::*;

        let mut ctx = ConvertContext::default();

        // Characters are interned first, in key order, so their entity ids
        // do not depend on where they are first referenced.
        for key in self.characters.keys() {
            let entity_id = ctx.symbols.intern_entity(key);
            ctx.characters
                .insert(key.clone(), CharacterId::new(entity_id));
        }

        let mut graph = DialogueGraph::new();
        graph.entry_node = Some(ctx.node(&self.entry_node));

        for yaml_node in &self.nodes {
            let node_id = ctx.node(&yaml_node.id);
            let node_key = yaml_node.id.as_key();
            let speaker = ctx
                .characters
                .get(&yaml_node.speaker)
                .copied()
                .ok_or_else(|| {
                    format!("node '{node_key}': unknown speaker '{}'", yaml_node.speaker)
                })?;

            let text = yaml_node
                .text
                .to_model(&mut ctx)
                .map_err(|msg| format!("node '{node_key}': text: {msg}"))?;
            let mut node = DialogueNode::new(node_id.0, speaker, text);

            for yaml_option in &yaml_node.options {
                let option_id = ctx.option(&yaml_option.id);
                let option_key = yaml_option.id.as_key();
                let at = |msg: String| format!("option '{option_key}' in node '{node_key}': {msg}");

                let target = match yaml_option.target.as_key() {
                    "end" => DialogTarget::End,
                    _ => DialogTarget::Node(ctx.node(&yaml_option.target)),
                };

                // Determine the text for the option: use explicit text, or default from navigation hint
                let option_text = match &yaml_option.text {
                    Some(text) if !text.is_empty() => text.to_model(&mut ctx).map_err(at)?,
                    _ => DialogText::inline(
                        yaml_option
                            .navigation_hint
                            .as_ref()
                            .map(|h| h.default_text(&yaml_option.go_to_target))
                            .unwrap_or_else(|| "...".to_string()),
                    ),
                };

                let mut option = DialogueOption::new(option_id.0, option_text, target, node_id);

                // Convert navigation hint
                if let Some(hint) = &yaml_option.navigation_hint {
                    option.navigation_hint =
                        Some(hint.to_model(&yaml_option.go_to_target).map_err(at)?);
                }

                // Convert tags
                for yaml_tag in &yaml_option.tags {
                    let mut tag =
                        model::CustomDialogTag::new(yaml_tag.axis.as_str(), yaml_tag.key.as_str());

                    // Apply display prefix if provided
                    if let Some(prefix) = &yaml_tag.display_prefix {
                        tag.display_prefix = Some(std::sync::Arc::from(prefix.as_str()));
                    }

                    // Apply requires_trait: use explicit value if provided, otherwise default
                    // based on axis convention (skill axis defaults to true)
                    tag.requires_trait = yaml_tag
                        .requires_trait
                        .unwrap_or_else(|| yaml_tag.axis.as_str() == "skill");

                    option.tags.push(tag);
                }

                // Convert intentions
                for intention in &yaml_option.intentions_explicit {
                    option.intentions.explicit.push(model::Intention::Custom(
                        std::sync::Arc::from(intention.as_str()),
                    ));
                }
                for intention in &yaml_option.intentions_implicit {
                    option.intentions.implicit.push(model::Intention::Custom(
                        std::sync::Arc::from(intention.as_str()),
                    ));
                }

                // Convert conditions
                for yaml_condition in &yaml_option.conditions {
                    option
                        .conditions
                        .push(yaml_condition.to_model(&mut ctx).map_err(at)?);
                }

                // Convert outcomes
                for yaml_outcome in &yaml_option.outcomes {
                    option
                        .outcomes
                        .push(yaml_outcome.to_model(&mut ctx).map_err(at)?);
                }

                node.options.push(option);
            }

            // Convert outcomes_on_enter
            for yaml_outcome in &yaml_node.outcomes {
                node.outcomes_on_enter.push(
                    yaml_outcome
                        .to_model(&mut ctx)
                        .map_err(|msg| format!("node '{node_key}': {msg}"))?,
                );
            }

            graph.nodes.push(node);
        }

        Ok(LoadedDialogue {
            graph,
            symbols: ctx.symbols,
            warnings: ctx.warnings,
        })
    }
}

/// Mutable state shared by the conversion of one graph.
#[derive(Default)]
struct ConvertContext {
    symbols: model::SymbolTable,
    characters: HashMap<String, model::CharacterId>,
    warnings: Vec<String>,
}

impl ConvertContext {
    fn node(&mut self, key: &YamlKey) -> model::DialogueNodeId {
        self.symbols.intern_node(key.as_key())
    }

    fn option(&mut self, key: &YamlKey) -> model::DialogueOptionId {
        self.symbols.intern_option(key.as_key())
    }

    fn entity(&mut self, key: &YamlKey) -> Result<model::EntityId, String> {
        let key = key.as_key();
        self.characters
            .get(key)
            .map(|c| c.0)
            .ok_or_else(|| format!("unknown entity '{key}'"))
    }
}

impl YamlCondition {
    /// Convert to a model Condition.
    fn to_model(&self, ctx: &mut ConvertContext) -> Result<model::Condition, String> {
        use model::*;

        match self {
            YamlCondition::Always => Ok(Condition::Always),
            YamlCondition::FactExists { scope, fact } => {
                let fact_scope = parse_fact_scope(scope)?;
                Ok(Condition::FactExists {
                    scope: fact_scope,
                    fact_id: FactId::new(fact.as_str()),
                })
            }
            YamlCondition::FactAtLeast { scope, fact, value } => Ok(Condition::FactAtLeast {
                scope: parse_fact_scope(scope)?,
                fact_id: FactId::new(fact.as_str()),
                value: *value,
            }),
            YamlCondition::FactAtMost { scope, fact, value } => Ok(Condition::FactAtMost {
                scope: parse_fact_scope(scope)?,
                fact_id: FactId::new(fact.as_str()),
                value: *value,
            }),
            YamlCondition::FactEquals { scope, fact, value } => {
                let fact_scope = parse_fact_scope(scope)?;
                let fact_value = match value {
                    YamlValue::Bool(b) => FactValue::Bool(*b),
                    YamlValue::Int(i) => FactValue::Int(*i),
                    YamlValue::String(s) => FactValue::Text(s.clone()),
                };
                Ok(Condition::FactEquals {
                    scope: fact_scope,
                    fact_id: FactId::new(fact.as_str()),
                    value: fact_value,
                })
            }
            YamlCondition::HasMemory { owner, id } => Ok(Condition::HasMemory {
                owner: ctx.entity(owner)?,
                query: MemoryQuery::by_id(ctx.symbols.intern_memory(id.as_key())),
            }),
        }
    }
}

impl YamlOutcome {
    /// Convert to a model Outcome.
    fn to_model(&self, ctx: &mut ConvertContext) -> Result<model::Outcome, String> {
        use model::*;

        match self {
            YamlOutcome::SetFact { scope, fact, value } => {
                let fact_scope = parse_fact_scope(scope)?;
                let fact_value = match value {
                    YamlValue::Bool(b) => FactValue::Bool(*b),
                    YamlValue::Int(i) => FactValue::Int(*i),
                    YamlValue::String(s) => FactValue::Text(s.clone()),
                };

                Ok(Outcome::SetFact {
                    scope: fact_scope,
                    fact: Fact::new(fact.as_str(), fact_value),
                })
            }
            YamlOutcome::AddFact { scope, fact, delta } => Ok(Outcome::AddFact {
                scope: parse_fact_scope(scope)?,
                fact_id: FactId::new(fact.as_str()),
                delta: *delta,
            }),
            YamlOutcome::AddRelationship {
                source,
                target,
                axis,
                delta,
            } => Ok(Outcome::AddRelationship {
                source: ctx.entity(source)?,
                target: ctx.entity(target)?,
                axis: RelationshipAxisId::new(axis.as_str()),
                delta: *delta,
            }),
        }
    }
}

/// Parse a fact scope string.
fn parse_fact_scope(scope: &str) -> Result<model::FactScope, String> {
    match scope {
        "world" => Ok(model::FactScope::World),
        "player" => Ok(model::FactScope::Player),
        _ => Err(format!("Unknown fact scope: {}", scope)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn test_parse_simple_yaml() {
        let yaml = r#"
version: "2.0"
entry_node: mara.intro
characters:
  mara:
    name: "Mara Vey"
nodes:
  - id: mara.intro
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: mara.intro.first
        text: "Tell me about the shipment."
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        check!(graph.version == "2.0");
        check!(graph.entry_node == YamlKey::from("mara.intro"));
        check!(graph.nodes.len() == 1);

        // Test conversion to model
        let loaded = graph.to_model().expect("Should convert");
        check!(loaded.graph.nodes.len() == 1);
        let entry = loaded.graph.entry_node.expect("entry node");
        check!(loaded.symbols.node_key(entry) == Some("mara.intro"));
        check!(
            loaded
                .symbols
                .option_key(loaded.graph.nodes[0].options[0].id)
                == Some("mara.intro.first")
        );
        check!(loaded.warnings.is_empty());
    }

    #[test]
    fn test_goto_without_target_fails() {
        let yaml = r#"
version: "2.0"
entry_node: mara.intro
characters:
  mara:
    name: "Mara Vey"
nodes:
  - id: mara.intro
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: mara.intro.first
        text: "Go somewhere."
        navigation_hint: go_to
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        let result = graph.to_model();
        check!(result.is_err());
        let err_msg = result.unwrap_err();
        check!(err_msg.contains("go_to_target"));
    }

    #[test]
    fn test_goto_with_target_succeeds() {
        let yaml = r#"
version: "2.0"
entry_node: mara.intro
characters:
  mara:
    name: "Mara Vey"
nodes:
  - id: mara.intro
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: mara.intro.first
        text: "Go to the ship."
        navigation_hint: go_to
        go_to_target: "the ship"
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        let model_graph = graph
            .to_model()
            .expect("Should convert with go_to_target")
            .graph;
        check!(model_graph.nodes.len() == 1);
        let option = &model_graph.nodes[0].options[0];
        check!(matches!(
            option.navigation_hint,
            Some(model::NavigationHint::GoTo(ref target)) if target == "the ship"
        ));
    }

    #[test]
    fn test_skill_tag_sets_requires_trait_but_no_auto_condition() {
        let yaml = r#"
version: "2.0"
entry_node: mara.intro
characters:
  mara:
    name: "Mara Vey"
nodes:
  - id: mara.intro
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: mara.intro.first
        text: "Treat the wound."
        tags:
          - { axis: "skill", key: "medic" }
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        let model_graph = graph.to_model().expect("Should convert").graph;
        let option = &model_graph.nodes[0].options[0];

        // Check that the tag has requires_trait set (by convention for skill axis)
        check!(option.tags.len() == 1);
        check!(option.tags[0].requires_trait);

        // Check that NO HasSkill condition was automatically added
        // (Users must now explicitly add fact-based conditions)
        check!(option.conditions.is_empty());
    }

    #[test]
    fn test_non_skill_tag_without_requires_trait_does_not_add_condition() {
        let yaml = r#"
version: "2.0"
entry_node: mara.intro
characters:
  mara:
    name: "Mara Vey"
nodes:
  - id: mara.intro
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: mara.intro.first
        text: "Be nice."
        tags:
          - { axis: "moral", key: "paragon" }
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        let model_graph = graph.to_model().expect("Should convert").graph;
        let option = &model_graph.nodes[0].options[0];

        // Check that the tag does not require trait
        check!(!option.tags[0].requires_trait);

        // Check that no HasSkill condition was added
        check!(option.conditions.is_empty());
    }

    #[test]
    fn test_targets_and_relationships_use_keys() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  player:
    name: "Player"
  mara:
    name: "Mara Vey"
nodes:
  - id: intro
    speaker: mara
    text: "Hello."
    options:
      - id: intro.comfort
        text: "Easy."
        outcomes:
          - { type: add_relationship, source: player, target: mara, axis: trust, delta: 5 }
        target: confession
  - id: confession
    speaker: mara
    text: "Fine."
"#;
        let loaded = YamlDialogueGraph::from_yaml(yaml)
            .unwrap()
            .to_model()
            .unwrap();
        let option = &loaded.graph.nodes[0].options[0];
        let model::DialogTarget::Node(target) = option.target else {
            panic!("expected node target");
        };
        check!(target == loaded.graph.nodes[1].id);
        check!(loaded.symbols.node_key(target) == Some("confession"));

        let model::Outcome::AddRelationship { source, target, .. } = &option.outcomes[0] else {
            panic!("expected relationship outcome");
        };
        check!(loaded.symbols.entity_key(*source) == Some("player"));
        check!(loaded.symbols.entity_key(*target) == Some("mara"));
        check!(loaded.graph.nodes[0].speaker.0 == loaded.symbols.entity("mara").unwrap());
    }

    #[test]
    fn test_unknown_entity_names_key() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  mara:
    name: "Mara Vey"
nodes:
  - id: intro
    speaker: mara
    text: "Hello."
    options:
      - id: intro.bye
        outcomes:
          - { type: add_relationship, source: nobody, target: mara, axis: trust, delta: 5 }
        target: end
"#;
        let err = YamlDialogueGraph::from_yaml(yaml)
            .unwrap()
            .to_model()
            .unwrap_err();
        check!(err.contains("'intro.bye'"), "{err}");
        check!(err.contains("'nobody'"), "{err}");
    }

    fn numeric_id_error(yaml: &str) -> YamlError {
        YamlDialogueGraph::from_yaml(yaml).expect_err("numeric ids must not load")
    }

    #[test]
    fn test_numeric_node_id_is_an_error() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  mara:
    name: "Mara Vey"
nodes:
  - id: 1
    speaker: mara
    text: "Hello."
"#;
        let err = numeric_id_error(yaml);
        check!(err.message().contains("numeric id 1"), "{err}");
        check!(err.message().contains("removed in 0.3"), "{err}");
        check!(err.line() == Some(8), "{err}");
    }

    #[test]
    fn test_numeric_entry_target_and_option_ids_are_errors() {
        let entry = "version: \"2.0\"\nentry_node: 1\ncharacters: {}\nnodes: []\n";
        check!(numeric_id_error(entry).message().contains("numeric id 1"));

        let option = r#"
version: "2.0"
entry_node: intro
characters:
  mara: { name: "Mara Vey" }
nodes:
  - id: intro
    speaker: mara
    text: "Hello."
    options:
      - id: 3
        target: end
"#;
        check!(numeric_id_error(option).message().contains("numeric id 3"));

        let target = option
            .replace("id: 3", "id: intro.bye")
            .replace("target: end", "target: 2");
        check!(numeric_id_error(&target).message().contains("numeric id 2"));

        // A float is a number too.
        let float = option.replace("id: 3", "id: 3.5");
        check!(
            numeric_id_error(&float)
                .message()
                .contains("numeric id 3.5")
        );
    }

    #[test]
    fn test_numeric_relationship_endpoint_is_an_error() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  mara: { name: "Mara Vey" }
nodes:
  - id: intro
    speaker: mara
    text: "Hello."
    options:
      - id: intro.bye
        outcomes:
          - { type: add_relationship, source: 1, target: mara, axis: trust, delta: 5 }
        target: end
"#;
        check!(numeric_id_error(yaml).message().contains("numeric id 1"));
    }

    #[test]
    fn test_numeric_character_id_is_an_error() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  mara:
    id: 2
    name: "Mara Vey"
nodes: []
"#;
        let err = numeric_id_error(yaml);
        check!(err.message().contains("character `id`"), "{err}");
        check!(err.message().contains("the map key is the id"), "{err}");
        // The error points at the value of `id`.
        check!(err.line() == Some(6), "{err}");
    }

    #[test]
    fn test_quoted_numbers_are_keys() {
        let yaml = r#"
version: "2.0"
entry_node: "1"
characters:
  mara: { name: "Mara Vey" }
nodes:
  - id: "1"
    speaker: mara
    text: "Hello."
    options:
      - id: "1"
        target: "2"
  - id: "2"
    speaker: mara
    text: "Bye."
"#;
        let loaded = YamlDialogueGraph::from_yaml(yaml)
            .unwrap()
            .to_model()
            .unwrap();
        let model::DialogTarget::Node(target) = loaded.graph.nodes[0].options[0].target else {
            panic!("expected node target");
        };
        check!(target == loaded.graph.nodes[1].id);
        check!(loaded.symbols.node_key(target) == Some("2"));
        check!(loaded.warnings.is_empty());
    }

    #[test]
    fn test_add_fact_outcome() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  mara: { name: "Mara Vey" }
nodes:
  - id: intro
    speaker: mara
    text: "Hello."
    outcomes:
      - { type: add_fact, fact: visits, delta: 1 }
    options:
      - id: intro.pay
        outcomes:
          - { type: add_fact, scope: player, fact: gold, delta: -5 }
        target: end
"#;
        let loaded = YamlDialogueGraph::from_yaml(yaml)
            .unwrap()
            .to_model()
            .unwrap();
        let node = &loaded.graph.nodes[0];
        check!(
            node.outcomes_on_enter[0]
                == model::Outcome::AddFact {
                    scope: model::FactScope::World,
                    fact_id: model::FactId::new("visits"),
                    delta: 1,
                }
        );
        check!(
            node.options[0].outcomes[0]
                == model::Outcome::AddFact {
                    scope: model::FactScope::Player,
                    fact_id: model::FactId::new("gold"),
                    delta: -5,
                }
        );
    }

    #[test]
    fn test_has_memory_condition() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  mara: { name: "Mara Vey" }
nodes:
  - id: intro
    speaker: mara
    text:
      - when: { type: has_memory, owner: mara, id: mara.saw_the_seal }
        text: "You were there."
    options:
      - id: intro.ask
        conditions:
          - { type: has_memory, owner: mara, id: mara.saw_the_seal }
        target: end
"#;
        let loaded = YamlDialogueGraph::from_yaml(yaml)
            .unwrap()
            .to_model()
            .unwrap();
        let memory = loaded.symbols.memory("mara.saw_the_seal").unwrap();
        let expected = model::Condition::HasMemory {
            owner: loaded.symbols.entity("mara").unwrap(),
            query: model::MemoryQuery::by_id(memory),
        };
        let node = &loaded.graph.nodes[0];
        check!(node.options[0].conditions[0] == expected);
        let model::DialogText::Fragments(fragments) = &node.text else {
            panic!("expected fragments");
        };
        check!(fragments[0].condition.as_ref() == Some(&expected));
    }

    #[test]
    fn test_has_memory_unknown_owner_fails() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  mara: { name: "Mara Vey" }
nodes:
  - id: intro
    speaker: mara
    text: "Hello."
    options:
      - id: intro.ask
        conditions:
          - { type: has_memory, owner: nobody, id: m }
        target: end
"#;
        let err = YamlDialogueGraph::from_yaml(yaml)
            .unwrap()
            .to_model()
            .unwrap_err();
        check!(err.contains("'nobody'"), "{err}");
    }

    #[test]
    fn test_fragment_lists() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  mara:
    name: "Mara Vey"
nodes:
  - id: intro
    speaker: mara
    text:
      - "Mara looks up."
      - when: { type: fact_at_least, fact: trust, value: 3 }
        text: "She smiles."
      - key: greeting
        mode: cycle
        variants: ["\"Again?\"", "\"You.\""]
    options:
      - id: intro.bye
        text: ["Leave."]
        target: end
"#;
        let loaded = YamlDialogueGraph::from_yaml(yaml)
            .unwrap()
            .to_model()
            .unwrap();
        let model::DialogText::Fragments(fragments) = &loaded.graph.nodes[0].text else {
            panic!("expected fragments");
        };
        check!(fragments.len() == 3);
        check!(fragments[0].mode == model::FragmentMode::Always);
        check!(let Some(model::Condition::FactAtLeast { value: 3, .. }) = &fragments[1].condition);
        check!(fragments[2].mode == model::FragmentMode::Cycle);
        check!(fragments[2].key.as_deref() == Some("greeting"));
        check!(fragments[2].variants.len() == 2);
        check!(let model::DialogText::Fragments(_) = &loaded.graph.nodes[0].options[0].text);
    }

    #[test]
    fn test_fragment_without_text_fails() {
        let yaml = r#"
version: "2.0"
entry_node: intro
characters:
  mara:
    name: "Mara Vey"
nodes:
  - id: intro
    speaker: mara
    text:
      - mode: cycle
"#;
        let err = YamlDialogueGraph::from_yaml(yaml)
            .unwrap()
            .to_model()
            .unwrap_err();
        check!(err.contains("'intro'"), "{err}");
        check!(err.contains("fragment 0"), "{err}");
    }

    #[test]
    fn test_yaml_error_has_line_and_column() {
        let yaml = "version: \"2.0\"\nentry_node: intro\ncharacters: {}\nnodes:\n  - id: intro\n    speaker: [unclosed\n";
        let err = YamlDialogueGraph::from_yaml(yaml).unwrap_err();
        check!(err.line().is_some(), "{err}");
        check!(err.column().is_some(), "{err}");
        check!(err.to_string().starts_with("line "), "{err}");
    }

    #[test]
    fn test_type_error_has_line() {
        let yaml = "version: \"2.0\"\nentry_node: intro\ncharacters:\n  mara:\n    name: [1, 2]\nnodes: []\n";
        let err = YamlDialogueGraph::from_yaml(yaml).unwrap_err();
        check!(err.line() == Some(5), "{err}");
    }
}
