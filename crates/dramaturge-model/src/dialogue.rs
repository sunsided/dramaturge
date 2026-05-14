//! Dialogue graph — nodes, options, and branching conversation structure.
//!
//! This module defines the core types for representing branching dialogue.
//! A [`DialogueGraph`] contains a set of [`DialogueNode`]s, each of which
//! presents [`DialogueOption`]s that lead to other nodes or terminate
//! the conversation. Nodes and options can have conditions and outcomes
//! attached.

use std::sync::Arc;

use crate::character::CharacterId;
use crate::condition::Condition;
use crate::intent::DialogIntentions;
use crate::outcome::Outcome;
use crate::tag::DialogTagKind;

/// Unique numeric identifier for a dialogue node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DialogueNodeId(pub u64);

impl DialogueNodeId {
    /// Creates a new dialogue node identifier from the given `u64` value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique numeric identifier for a dialogue option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DialogueOptionId(pub u64);

impl DialogueOptionId {
    /// Creates a new dialogue option identifier from the given `u64` value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Text content that can be inline or localized.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DialogText {
    /// Direct string text.
    Inline(String),
    /// Key into a localization table.
    LocalizedKey(Arc<str>),
}

impl DialogText {
    /// Creates inline text from the given string-like value.
    pub fn inline(text: impl Into<String>) -> Self {
        Self::Inline(text.into())
    }

    /// Creates a localized text reference from the given key.
    pub fn localized(key: impl Into<Arc<str>>) -> Self {
        Self::LocalizedKey(key.into())
    }
}

/// Where a dialogue option leads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DialogTarget {
    /// Transitions to another dialogue node.
    Node(DialogueNodeId),
    /// Ends the conversation entirely.
    End,
    /// Returns to the calling context (e.g., from a sub-dialogue).
    Return,
    /// Game-specific custom target.
    Custom(Arc<str>),
}

/// Navigation hint for dialogue options.
///
/// Provides typed navigation cues that can be rendered as UI hints
/// (e.g., "[Continue]", "[Leave]", "[End]"). These replace hardcoded
/// prefixes in dialogue text with semantic, localizable navigation markers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NavigationHint {
    /// Continue to the next node (→ Continue / [Continue]).
    Continue,
    /// Leave the current conversation (← Leave / [Leave]).
    Leave,
    /// Go to a specific character or location (→ Go to {name} / [Go to {name}]).
    GoTo(String),
    /// End the conversation (✓ End / [End]).
    End,
    /// Return to a previous menu or caller (↩ Return / [Return]).
    Return,
    /// Custom navigation label.
    Custom(String),
}

impl NavigationHint {
    /// Returns the default display text for this navigation hint.
    pub fn default_text(&self) -> String {
        match self {
            NavigationHint::Continue => "Continue...".to_string(),
            NavigationHint::Leave => "Leave...".to_string(),
            NavigationHint::GoTo(target) => format!("Go to {target}"),
            NavigationHint::End => "End conversation".to_string(),
            NavigationHint::Return => "Return...".to_string(),
            NavigationHint::Custom(label) => label.clone(),
        }
    }

    /// Returns the display prefix for this navigation hint (e.g., "[Continue]").
    pub fn display_prefix(&self) -> String {
        match self {
            NavigationHint::Continue => "[Continue]".to_string(),
            NavigationHint::Leave => "[Leave]".to_string(),
            NavigationHint::GoTo(target) => format!("[Go to {target}]"),
            NavigationHint::End => "[End]".to_string(),
            NavigationHint::Return => "[Return]".to_string(),
            NavigationHint::Custom(label) => format!("[{label}]"),
        }
    }
}

impl DialogTarget {
    /// Creates a target pointing to the given node ID.
    pub fn node(id: u64) -> Self {
        Self::Node(DialogueNodeId::new(id))
    }

    /// Creates an end-of-conversation target.
    pub fn end() -> Self {
        Self::End
    }

    /// Creates a return-to-caller target.
    pub fn r#return() -> Self {
        Self::Return
    }

    /// Creates a custom target with the given identifier.
    pub fn custom(target: impl Into<Arc<str>>) -> Self {
        Self::Custom(target.into())
    }
}

/// A single step in a dialogue graph where a character speaks.
///
/// Each node represents a moment in the conversation: a character says
/// something (via `text`), and optionally presents options to continue.
/// Nodes can have conditions that gate their entry and outcomes that
/// apply when the node is entered.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DialogueNode {
    /// Unique identifier for this node.
    pub id: DialogueNodeId,
    /// The character who speaks in this node.
    pub speaker: CharacterId,
    /// The spoken text (inline or localized).
    pub text: DialogText,
    /// Conditions that must be satisfied for this node to be entered.
    pub conditions: Vec<Condition>,
    /// Outcomes that apply when this node is entered.
    pub outcomes_on_enter: Vec<Outcome>,
    /// Branching options presented to the player after this node.
    pub options: Vec<DialogueOption>,
}

impl DialogueNode {
    /// Creates a node with the given ID, speaker, and text.
    ///
    /// Conditions, outcomes, and options all start empty.
    pub fn new(id: u64, speaker: CharacterId, text: DialogText) -> Self {
        Self {
            id: DialogueNodeId::new(id),
            speaker,
            text,
            conditions: Vec::new(),
            outcomes_on_enter: Vec::new(),
            options: Vec::new(),
        }
    }
}

/// A branching choice presented to the player.
///
/// Options are attached to [`DialogueNode`]s and represent the possible
/// responses or actions available to the player. Each option can have
/// tags, intentions, conditions, and outcomes.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DialogueOption {
    /// Unique identifier for this option.
    pub id: DialogueOptionId,
    /// The text displayed for this option.
    pub text: DialogText,
    /// Tags categorizing this option.
    pub tags: Vec<DialogTagKind>,
    /// Intentions behind this option.
    pub intentions: DialogIntentions,
    /// Conditions that must be satisfied for this option to be available.
    pub conditions: Vec<Condition>,
    /// Outcomes that apply when this option is chosen.
    pub outcomes: Vec<Outcome>,
    /// Where this option leads when chosen.
    pub target: DialogTarget,
    /// The node this option belongs to.
    pub source_node_id: DialogueNodeId,
    /// Optional navigation hint (e.g., Continue, Leave, End).
    #[cfg_attr(feature = "serde", serde(default))]
    pub navigation_hint: Option<NavigationHint>,
}

impl DialogueOption {
    /// Creates an option with the given ID, text, target, and source node.
    ///
    /// Tags, intentions, conditions, outcomes, and navigation hint all start empty.
    pub fn new(
        id: u64,
        text: DialogText,
        target: DialogTarget,
        source_node_id: DialogueNodeId,
    ) -> Self {
        Self {
            id: DialogueOptionId::new(id),
            text,
            tags: Vec::new(),
            intentions: DialogIntentions::new(),
            conditions: Vec::new(),
            outcomes: Vec::new(),
            target,
            source_node_id,
            navigation_hint: None,
        }
    }

    /// Sets the navigation hint for this option.
    pub fn with_navigation_hint(mut self, hint: NavigationHint) -> Self {
        self.navigation_hint = Some(hint);
        self
    }
}

/// Root container for a complete dialogue tree.
///
/// Holds all nodes and optionally designates an entry point for
/// traversal.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DialogueGraph {
    /// All nodes in the graph.
    pub nodes: Vec<DialogueNode>,
    /// The starting node for traversal, if set.
    pub entry_node: Option<DialogueNodeId>,
}

impl DialogueGraph {
    /// Creates an empty graph with no nodes and no entry point.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            entry_node: None,
        }
    }
}

impl Default for DialogueGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn dialogue_node_id_construction() {
        let id = DialogueNodeId::new(1);
        check!(id.0 == 1);
    }

    #[test]
    fn dialogue_node_id_is_copy() {
        let a = DialogueNodeId::new(1);
        let b = a;
        check!(a == b);
    }

    #[test]
    fn dialogue_option_id_construction() {
        let id = DialogueOptionId::new(2);
        check!(id.0 == 2);
    }

    #[test]
    fn dialog_text_variants() {
        let inline = DialogText::inline("Hello");
        let localized = DialogText::localized("greeting_key");
        check!(let DialogText::Inline(_) = inline);
        check!(let DialogText::LocalizedKey(_) = localized);
    }

    #[test]
    fn dialog_target_variants() {
        let node = DialogTarget::node(1);
        let end = DialogTarget::end();
        let ret = DialogTarget::r#return();
        let custom = DialogTarget::custom("custom_target");
        check!(let DialogTarget::Node(_) = node);
        check!(let DialogTarget::End = end);
        check!(let DialogTarget::Return = ret);
        check!(let DialogTarget::Custom(_) = custom);
    }

    #[test]
    fn dialogue_node_construction() {
        let speaker = CharacterId::new(crate::character::EntityId::new(1));
        let text = DialogText::inline("Hello, traveller.");
        let node = DialogueNode::new(1, speaker, text);
        check!(node.id.0 == 1);
        check!(node.conditions.is_empty());
        check!(node.outcomes_on_enter.is_empty());
        check!(node.options.is_empty());
    }

    #[test]
    fn dialogue_option_construction() {
        let text = DialogText::inline("Yes, I'll help.");
        let option = DialogueOption::new(1, text, DialogTarget::node(2), DialogueNodeId::new(1));
        check!(option.id.0 == 1);
        check!(option.tags.is_empty());
        check!(option.conditions.is_empty());
        check!(option.outcomes.is_empty());
        check!(let DialogTarget::Node(_) = option.target);
    }

    #[test]
    fn dialogue_graph_default() {
        let graph = DialogueGraph::default();
        check!(graph.nodes.is_empty());
        check!(graph.entry_node.is_none());
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    fn arb_dialogue_node_id() -> impl Strategy<Value = DialogueNodeId> {
        any::<u64>().prop_map(DialogueNodeId::new)
    }

    #[allow(dead_code)]
    fn arb_dialogue_option_id() -> impl Strategy<Value = DialogueOptionId> {
        any::<u64>().prop_map(DialogueOptionId::new)
    }

    #[allow(dead_code)]
    fn arb_dialog_text() -> impl Strategy<Value = DialogText> {
        prop_oneof![
            ".*".prop_map(DialogText::inline),
            ".*".prop_map(|s: String| DialogText::localized(s)),
        ]
    }

    fn arb_dialog_target() -> impl Strategy<Value = DialogTarget> {
        prop_oneof![
            arb_dialogue_node_id().prop_map(DialogTarget::Node),
            Just(DialogTarget::End),
            Just(DialogTarget::Return),
            ".*".prop_map(|s: String| DialogTarget::custom(s)),
        ]
    }

    #[test]
    fn prop_node_ids_unique() {
        proptest!(|(node_ids in proptest::collection::hash_set(any::<u64>(), 1..20))| {
            let mut seen = std::collections::HashSet::new();
            for id in &node_ids {
                prop_assert!(seen.insert(*id));
            }
        });
    }

    #[test]
    fn prop_option_targets_valid_or_terminal() {
        proptest!(|(target in arb_dialog_target())| {
            match &target {
                DialogTarget::Node(_) | DialogTarget::End | DialogTarget::Return | DialogTarget::Custom(_) => {}
            }
        });
    }

    #[test]
    fn prop_dialogue_graph_default_entry_none() {
        let graph = DialogueGraph::new();
        assert!(graph.entry_node.is_none());
    }

    #[test]
    fn prop_dialog_text_roundtrip() {
        proptest!(|(text in ".*")| {
            let dt = DialogText::inline(&text);
            if let DialogText::Inline(s) = dt {
                prop_assert_eq!(s, text);
            } else {
                prop_assert!(false, "expected Inline variant");
            }
        });
    }

    #[test]
    fn prop_dialog_target_node_id_preserved() {
        proptest!(|(id: u64)| {
            let target = DialogTarget::node(id);
            if let DialogTarget::Node(node_id) = target {
                prop_assert_eq!(node_id.0, id);
            } else {
                prop_assert!(false, "expected Node variant");
            }
        });
    }

    #[test]
    fn prop_dialogue_node_construction_preserves_fields() {
        proptest!(|(
            id in any::<u64>(),
            speaker in any::<u64>(),
            text in ".*"
        )| {
            let speaker = CharacterId::new(crate::character::EntityId::new(speaker));
            let node = DialogueNode::new(id, speaker, DialogText::inline(&text));
            prop_assert_eq!(node.id.0, id);
            prop_assert!(node.conditions.is_empty());
            prop_assert!(node.outcomes_on_enter.is_empty());
            prop_assert!(node.options.is_empty());
        });
    }

    #[test]
    fn prop_dialogue_option_construction_preserves_fields() {
        proptest!(|(
            id in any::<u64>(),
            text in ".*",
            target_id in any::<u64>(),
            source_node_id in any::<u64>()
        )| {
            let option = DialogueOption::new(id, DialogText::inline(&text), DialogTarget::node(target_id), DialogueNodeId::new(source_node_id));
            prop_assert_eq!(option.id.0, id);
            prop_assert!(option.tags.is_empty());
            prop_assert!(option.conditions.is_empty());
            prop_assert!(option.outcomes.is_empty());
            if let DialogTarget::Node(n) = option.target {
                prop_assert_eq!(n.0, target_id);
            } else {
                prop_assert!(false, "expected Node target");
            }
            prop_assert_eq!(option.source_node_id.0, source_node_id);
        });
    }
}
