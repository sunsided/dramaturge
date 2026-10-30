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
//! are still accepted for one release. Each use adds a deprecation message
//! to [`LoadedDialogue::warnings`].

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

/// A stable author key, or a deprecated numeric id from the 1.0 format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlKey {
    /// A readable key such as `dlg.mara.intro`.
    Key(String),
    /// A numeric id (deprecated; accepted for one release).
    Number(u64),
}

impl YamlKey {
    /// Returns the key text. Numbers become their decimal string.
    pub fn as_key(&self) -> std::borrow::Cow<'_, str> {
        match self {
            YamlKey::Key(key) => std::borrow::Cow::Borrowed(key),
            YamlKey::Number(n) => std::borrow::Cow::Owned(n.to_string()),
        }
    }

    fn is_number(&self) -> bool {
        matches!(self, YamlKey::Number(_))
    }
}

impl From<&str> for YamlKey {
    fn from(key: &str) -> Self {
        YamlKey::Key(key.to_string())
    }
}

/// A character definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlCharacter {
    /// Legacy numeric character id (deprecated; the map key is the id).
    ///
    /// Only used to resolve numeric relationship endpoints from 1.0 files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    /// Character name for display.
    pub name: String,
}

/// The result of converting a [`YamlDialogueGraph`] into the model.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedDialogue {
    /// The dialogue graph.
    pub graph: model::DialogueGraph,
    /// Keys for every entity, node and option id in the graph.
    pub symbols: model::SymbolTable,
    /// Non-fatal messages, such as deprecated numeric ids.
    pub warnings: Vec<String>,
}

/// A dialogue node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlNode {
    /// Unique node key.
    pub id: YamlKey,
    /// Character key who speaks.
    pub speaker: String,
    /// Dialogue text.
    pub text: String,
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
    /// Option text.
    #[serde(default)]
    pub text: String,
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

impl YamlDialogueGraph {
    /// Load a dialogue graph from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
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
        for (key, character) in &self.characters {
            let entity_id = ctx.symbols.intern_entity(key);
            ctx.characters
                .insert(key.clone(), CharacterId::new(entity_id));
            if let Some(legacy) = character.id {
                ctx.warnings.push(format!(
                    "character '{key}': numeric `id: {legacy}` is deprecated; the map key is the id"
                ));
                ctx.legacy_entities.insert(legacy, entity_id);
            }
        }

        let mut graph = DialogueGraph::new();
        graph.entry_node = Some(ctx.node(&self.entry_node, "entry_node"));

        for yaml_node in &self.nodes {
            let node_id = ctx.node(&yaml_node.id, "node id");
            let node_key = yaml_node.id.as_key();
            let speaker = ctx
                .characters
                .get(&yaml_node.speaker)
                .copied()
                .ok_or_else(|| {
                    format!("node '{node_key}': unknown speaker '{}'", yaml_node.speaker)
                })?;

            let mut node =
                DialogueNode::new(node_id.0, speaker, DialogText::inline(&yaml_node.text));

            for yaml_option in &yaml_node.options {
                let option_id = ctx.option(&yaml_option.id);
                let option_key = yaml_option.id.as_key();
                let at = |msg: String| format!("option '{option_key}' in node '{node_key}': {msg}");

                let target = match &yaml_option.target {
                    YamlKey::Key(key) if key == "end" => DialogTarget::End,
                    other => DialogTarget::Node(ctx.node(other, "target")),
                };

                // Determine the text for the option: use explicit text, or default from navigation hint
                let option_text = if yaml_option.text.is_empty() {
                    yaml_option
                        .navigation_hint
                        .as_ref()
                        .map(|h| h.default_text(&yaml_option.go_to_target))
                        .unwrap_or_else(|| "...".to_string())
                } else {
                    yaml_option.text.clone()
                };

                let mut option = DialogueOption::new(
                    option_id.0,
                    DialogText::inline(&option_text),
                    target,
                    node_id,
                );

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
                        .push(yaml_condition.to_model().map_err(at)?);
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
    legacy_entities: HashMap<u64, model::EntityId>,
    warnings: Vec<String>,
}

impl ConvertContext {
    fn node(&mut self, key: &YamlKey, what: &str) -> model::DialogueNodeId {
        if key.is_number() {
            self.warnings.push(format!(
                "{what} {}: numeric node ids are deprecated; use a key",
                key.as_key()
            ));
        }
        self.symbols.intern_node(&key.as_key())
    }

    fn option(&mut self, key: &YamlKey) -> model::DialogueOptionId {
        if key.is_number() {
            self.warnings.push(format!(
                "option id {}: numeric option ids are deprecated; use a key",
                key.as_key()
            ));
        }
        self.symbols.intern_option(&key.as_key())
    }

    fn entity(&mut self, key: &YamlKey) -> Result<model::EntityId, String> {
        match key {
            YamlKey::Key(key) => self
                .characters
                .get(key)
                .map(|c| c.0)
                .ok_or_else(|| format!("unknown entity '{key}'")),
            YamlKey::Number(n) => {
                let id = self
                    .legacy_entities
                    .get(n)
                    .copied()
                    .ok_or_else(|| format!("unknown numeric entity id {n}"))?;
                self.warnings.push(format!(
                    "entity {n}: numeric entity references are deprecated; use the character key"
                ));
                Ok(id)
            }
        }
    }
}

impl YamlCondition {
    /// Convert to a model Condition.
    fn to_model(&self) -> Result<model::Condition, String> {
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
                    fact: Fact {
                        id: FactId::new(fact.as_str()),
                        value: fact_value,
                    },
                })
            }
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

    #[test]
    fn test_legacy_numeric_ids_load_with_warnings() {
        let yaml = r#"
version: "1.0"
entry_node: 1
characters:
  player:
    id: 1
    name: "Player"
  mara:
    id: 2
    name: "Mara Vey"
nodes:
  - id: 1
    speaker: mara
    text: "Hello."
    options:
      - id: 1
        outcomes:
          - { type: add_relationship, source: 1, target: 2, axis: trust, delta: 5 }
        target: "2"
  - id: 2
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
        let model::Outcome::AddRelationship { source, .. } =
            &loaded.graph.nodes[0].options[0].outcomes[0]
        else {
            panic!("expected relationship outcome");
        };
        check!(loaded.symbols.entity_key(*source) == Some("player"));
        check!(!loaded.warnings.is_empty());
        check!(loaded.warnings.iter().all(|w| w.contains("deprecated")));
    }
}
