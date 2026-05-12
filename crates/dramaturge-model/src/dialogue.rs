use std::sync::Arc;

use crate::character::CharacterId;
use crate::condition::Condition;
use crate::intent::DialogIntentions;
use crate::outcome::Outcome;
use crate::tag::DialogTagKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DialogueNodeId(pub u64);

impl DialogueNodeId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DialogueOptionId(pub u64);

impl DialogueOptionId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DialogText {
    Inline(String),
    LocalizedKey(Arc<str>),
}

impl DialogText {
    pub fn inline(text: impl Into<String>) -> Self {
        Self::Inline(text.into())
    }

    pub fn localized(key: impl Into<Arc<str>>) -> Self {
        Self::LocalizedKey(key.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DialogTarget {
    Node(DialogueNodeId),
    End,
    Return,
    Custom(Arc<str>),
}

impl DialogTarget {
    pub fn node(id: u64) -> Self {
        Self::Node(DialogueNodeId::new(id))
    }

    pub fn end() -> Self {
        Self::End
    }

    pub fn r#return() -> Self {
        Self::Return
    }

    pub fn custom(target: impl Into<Arc<str>>) -> Self {
        Self::Custom(target.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DialogueNode {
    pub id: DialogueNodeId,
    pub speaker: CharacterId,
    pub text: DialogText,
    pub conditions: Vec<Condition>,
    pub outcomes_on_enter: Vec<Outcome>,
    pub options: Vec<DialogueOption>,
}

impl DialogueNode {
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

#[derive(Debug, Clone, PartialEq)]
pub struct DialogueOption {
    pub id: DialogueOptionId,
    pub text: DialogText,
    pub tags: Vec<DialogTagKind>,
    pub intentions: DialogIntentions,
    pub conditions: Vec<Condition>,
    pub outcomes: Vec<Outcome>,
    pub target: DialogTarget,
}

impl DialogueOption {
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

#[derive(Debug, Clone, PartialEq)]
pub struct DialogueGraph {
    pub nodes: Vec<DialogueNode>,
    pub entry_node: Option<DialogueNodeId>,
}

impl DialogueGraph {
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
