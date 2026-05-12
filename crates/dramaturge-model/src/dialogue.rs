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
pub struct DialogueNodeId(pub u64);

impl DialogueNodeId {
    /// Creates a new dialogue node identifier from the given `u64` value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique numeric identifier for a dialogue option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DialogueOptionId(pub u64);

impl DialogueOptionId {
    /// Creates a new dialogue option identifier from the given `u64` value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Text content that can be inline or localized.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
}

impl DialogueOption {
    /// Creates an option with the given ID, text, and target.
    ///
    /// Tags, intentions, conditions, and outcomes all start empty.
    pub fn new(id: u64, text: DialogText, target: DialogTarget) -> Self {
        Self {
            id: DialogueOptionId::new(id),
            text,
            tags: Vec::new(),
            intentions: DialogIntentions::new(),
            conditions: Vec::new(),
            outcomes: Vec::new(),
            target,
        }
    }
}

/// Root container for a complete dialogue tree.
///
/// Holds all nodes and optionally designates an entry point for
/// traversal.
#[derive(Debug, Clone, PartialEq)]
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

    #[test]
    fn dialogue_node_id_construction() {
        let id = DialogueNodeId::new(1);
        assert_eq!(id.0, 1);
    }

    #[test]
    fn dialogue_node_id_is_copy() {
        let a = DialogueNodeId::new(1);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn dialogue_option_id_construction() {
        let id = DialogueOptionId::new(2);
        assert_eq!(id.0, 2);
    }

    #[test]
    fn dialog_text_variants() {
        let inline = DialogText::inline("Hello");
        let localized = DialogText::localized("greeting_key");
        assert!(matches!(inline, DialogText::Inline(_)));
        assert!(matches!(localized, DialogText::LocalizedKey(_)));
    }

    #[test]
    fn dialog_target_variants() {
        let node = DialogTarget::node(1);
        let end = DialogTarget::end();
        let ret = DialogTarget::r#return();
        let custom = DialogTarget::custom("custom_target");
        assert!(matches!(node, DialogTarget::Node(_)));
        assert!(matches!(end, DialogTarget::End));
        assert!(matches!(ret, DialogTarget::Return));
        assert!(matches!(custom, DialogTarget::Custom(_)));
    }

    #[test]
    fn dialogue_node_construction() {
        let speaker = CharacterId::new(crate::character::EntityId::new(1));
        let text = DialogText::inline("Hello, traveller.");
        let node = DialogueNode::new(1, speaker, text);
        assert_eq!(node.id.0, 1);
        assert!(node.conditions.is_empty());
        assert!(node.outcomes_on_enter.is_empty());
        assert!(node.options.is_empty());
    }

    #[test]
    fn dialogue_option_construction() {
        let text = DialogText::inline("Yes, I'll help.");
        let option = DialogueOption::new(1, text, DialogTarget::node(2));
        assert_eq!(option.id.0, 1);
        assert!(option.tags.is_empty());
        assert!(option.conditions.is_empty());
        assert!(option.outcomes.is_empty());
        assert!(matches!(option.target, DialogTarget::Node(_)));
    }

    #[test]
    fn dialogue_graph_default() {
        let graph = DialogueGraph::default();
        assert!(graph.nodes.is_empty());
        assert!(graph.entry_node.is_none());
    }
}
