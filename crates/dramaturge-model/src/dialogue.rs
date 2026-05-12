//! Dialogue graph types.

use crate::{Condition, OptionTag, Outcome};

/// A graph of dialogue nodes representing a branching conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueGraph {
    /// The nodes that make up this dialogue graph.
    pub nodes: Vec<DialogueNode>,
}

impl DialogueGraph {
    /// Create an empty [`DialogueGraph`].
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl Default for DialogueGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// A single node in a [`DialogueGraph`].
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueNode {
    /// Unique identifier for this node within the graph.
    pub id: String,
    /// The text spoken or displayed at this node.
    pub text: String,
    /// Available options branching from this node.
    pub options: Vec<DialogueOption>,
}

impl DialogueNode {
    /// Create a new [`DialogueNode`] with the given id and text.
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            options: Vec::new(),
        }
    }
}

/// A branching option available from a [`DialogueNode`].
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueOption {
    /// The display text of this option.
    pub text: String,
    /// Tags associated with this option.
    pub tags: Vec<OptionTag>,
    /// An optional condition that must be satisfied for this option to appear.
    pub condition: Option<Condition>,
    /// Outcomes that occur when this option is chosen.
    pub outcomes: Vec<Outcome>,
    /// The id of the node this option leads to, if any.
    pub target_node_id: Option<String>,
}

impl DialogueOption {
    /// Create a new unconditional [`DialogueOption`] with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tags: Vec::new(),
            condition: None,
            outcomes: Vec::new(),
            target_node_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialogue_graph_default() {
        let graph = DialogueGraph::default();
        assert!(graph.nodes.is_empty());
    }

    #[test]
    fn dialogue_node_construction() {
        let node = DialogueNode::new("start", "Hello, traveller.");
        assert_eq!(node.id, "start");
        assert!(node.options.is_empty());
    }
}
