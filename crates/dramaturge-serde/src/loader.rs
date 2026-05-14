//! YAML loader for dialogue graphs.
//!
//! This module provides functionality to load dialogue graphs from YAML files.
//! It uses serde-friendly wrapper types that can be converted to the core model types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use dramaturge_model as model;

/// A dialogue graph loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDialogueGraph {
    /// Schema version for compatibility.
    pub version: String,
    /// Named characters referenced in the dialogue.
    pub characters: HashMap<String, YamlCharacter>,
    /// All nodes in the graph.
    pub nodes: Vec<YamlNode>,
    /// Entry point node ID.
    pub entry_node: u64,
}

/// A character definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlCharacter {
    /// Character ID (numeric).
    pub id: u64,
    /// Character name for display.
    pub name: String,
}

/// A dialogue node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlNode {
    /// Unique node ID.
    pub id: u64,
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
    /// Unique option ID.
    pub id: u64,
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
    /// Target node ID or special value "end".
    pub target: String,
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
    /// Check if player has a skill.
    #[serde(rename = "has_skill")]
    HasSkill {
        /// Skill identifier.
        skill: String,
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
        /// Source entity ID.
        source: u64,
        /// Target entity ID.
        target: u64,
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

    /// Convert to a model DialogueGraph.
    pub fn to_model(&self) -> Result<model::DialogueGraph, String> {
        use model::*;

        let mut graph = DialogueGraph::new();
        graph.entry_node = Some(DialogueNodeId::new(self.entry_node));

        // Build character lookup
        let mut character_ids: HashMap<String, CharacterId> = HashMap::new();
        for (key, character) in &self.characters {
            let entity_id = EntityId::new(character.id);
            character_ids.insert(key.clone(), CharacterId::new(entity_id));
        }

        // Convert nodes
        for yaml_node in &self.nodes {
            let speaker = character_ids
                .get(&yaml_node.speaker)
                .copied()
                .ok_or_else(|| format!("Unknown speaker: {}", yaml_node.speaker))?;

            let mut node =
                DialogueNode::new(yaml_node.id, speaker, DialogText::inline(&yaml_node.text));

            // Convert options
            for yaml_option in &yaml_node.options {
                let target = if yaml_option.target == "end" {
                    DialogTarget::End
                } else {
                    let target_id: u64 = yaml_option
                        .target
                        .parse()
                        .map_err(|_| format!("Invalid target: {}", yaml_option.target))?;
                    DialogTarget::Node(DialogueNodeId::new(target_id))
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
                    yaml_option.id,
                    DialogText::inline(&option_text),
                    target,
                    DialogueNodeId::new(yaml_node.id),
                );

                // Convert navigation hint
                if let Some(hint) = &yaml_option.navigation_hint {
                    option.navigation_hint = Some(hint.to_model(&yaml_option.go_to_target)?);
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

                    // If this tag requires a trait, add a HasSkill condition to the option
                    if tag.requires_trait {
                        option
                            .conditions
                            .push(model::Condition::has_skill(&*tag.key));
                    }

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
                        .push(yaml_condition.to_model(&character_ids)?);
                }

                // Convert outcomes
                for yaml_outcome in &yaml_option.outcomes {
                    option.outcomes.push(yaml_outcome.to_model(&character_ids)?);
                }

                node.options.push(option);
            }

            // Convert outcomes_on_enter
            for yaml_outcome in &yaml_node.outcomes {
                node.outcomes_on_enter
                    .push(yaml_outcome.to_model(&character_ids)?);
            }

            graph.nodes.push(node);
        }

        Ok(graph)
    }
}

impl YamlCondition {
    /// Convert to a model Condition.
    fn to_model(
        &self,
        _character_ids: &HashMap<String, model::CharacterId>,
    ) -> Result<model::Condition, String> {
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
            YamlCondition::HasSkill { skill } => Ok(Condition::HasSkill {
                skill: skill.clone(),
            }),
        }
    }
}

impl YamlOutcome {
    /// Convert to a model Outcome.
    fn to_model(
        &self,
        _character_ids: &HashMap<String, model::CharacterId>,
    ) -> Result<model::Outcome, String> {
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
                source: EntityId::new(*source),
                target: EntityId::new(*target),
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

    #[test]
    fn test_parse_simple_yaml() {
        let yaml = r#"
version: "1.0"
entry_node: 1
characters:
  mara:
    id: 2
    name: "Mara Vey"
nodes:
  - id: 1
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: 1
        text: "Tell me about the shipment."
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        assert_eq!(graph.version, "1.0");
        assert_eq!(graph.entry_node, 1);
        assert_eq!(graph.nodes.len(), 1);

        // Test conversion to model
        let model_graph = graph.to_model().expect("Should convert");
        assert_eq!(model_graph.nodes.len(), 1);
        assert!(model_graph.entry_node.is_some());
    }

    #[test]
    fn test_goto_without_target_fails() {
        let yaml = r#"
version: "1.0"
entry_node: 1
characters:
  mara:
    id: 2
    name: "Mara Vey"
nodes:
  - id: 1
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: 1
        text: "Go somewhere."
        navigation_hint: go_to
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        let result = graph.to_model();
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("go_to_target"));
    }

    #[test]
    fn test_goto_with_target_succeeds() {
        let yaml = r#"
version: "1.0"
entry_node: 1
characters:
  mara:
    id: 2
    name: "Mara Vey"
nodes:
  - id: 1
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: 1
        text: "Go to the ship."
        navigation_hint: go_to
        go_to_target: "the ship"
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        let model_graph = graph.to_model().expect("Should convert with go_to_target");
        assert_eq!(model_graph.nodes.len(), 1);
        let option = &model_graph.nodes[0].options[0];
        assert!(matches!(
            option.navigation_hint,
            Some(model::NavigationHint::GoTo(ref target)) if target == "the ship"
        ));
    }

    #[test]
    fn test_skill_tag_adds_has_skill_condition() {
        let yaml = r#"
version: "1.0"
entry_node: 1
characters:
  mara:
    id: 2
    name: "Mara Vey"
nodes:
  - id: 1
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: 1
        text: "Treat the wound."
        tags:
          - { axis: "skill", key: "medic" }
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        let model_graph = graph.to_model().expect("Should convert");
        let option = &model_graph.nodes[0].options[0];

        // Check that the tag has requires_trait set
        assert_eq!(option.tags.len(), 1);
        assert!(option.tags[0].requires_trait);

        // Check that a HasSkill condition was added
        assert_eq!(option.conditions.len(), 1);
        assert!(matches!(
            option.conditions[0],
            model::Condition::HasSkill { ref skill } if skill == "medic"
        ));
    }

    #[test]
    fn test_non_skill_tag_without_requires_trait_does_not_add_condition() {
        let yaml = r#"
version: "1.0"
entry_node: 1
characters:
  mara:
    id: 2
    name: "Mara Vey"
nodes:
  - id: 1
    speaker: mara
    text: "Hello, traveler."
    options:
      - id: 1
        text: "Be nice."
        tags:
          - { axis: "moral", key: "paragon" }
        target: "end"
"#;

        let graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        let model_graph = graph.to_model().expect("Should convert");
        let option = &model_graph.nodes[0].options[0];

        // Check that the tag does not require trait
        assert!(!option.tags[0].requires_trait);

        // Check that no HasSkill condition was added
        assert!(option.conditions.is_empty());
    }
}
