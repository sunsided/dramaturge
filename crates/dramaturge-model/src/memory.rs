use std::sync::Arc;

use crate::character::EntityId;
use crate::fact::FactId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryId(pub u64);

impl MemoryId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Confidence {
    Rumor,
    Suspected,
    Believed,
    Known,
    Witnessed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryTag {
    HelpedMe,
    BetrayedMe,
    LiedToMe,
    SavedLife,
    ThreatenedMe,
    ShowedMercy,
    KilledSomeone,
    RevealedSecret,
    Custom(Arc<str>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryQuery(pub Arc<str>);

impl MemoryQuery {
    pub fn new(query: impl Into<Arc<str>>) -> Self {
        Self(query.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    pub id: MemoryId,
    pub subject: Option<EntityId>,
    pub about: Vec<EntityId>,
    pub facts: Vec<FactId>,
    pub tags: Vec<MemoryTag>,
    pub emotional_weight: i32,
    pub confidence: Confidence,
}

impl Memory {
    pub fn new(id: MemoryId) -> Self {
        Self {
            id,
            subject: None,
            about: Vec::new(),
            facts: Vec::new(),
            tags: Vec::new(),
            emotional_weight: 0,
            confidence: Confidence::Believed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_id_construction() {
        let id = MemoryId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn memory_id_is_copy() {
        let a = MemoryId::new(1);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn confidence_variants() {
        assert!(matches!(Confidence::Rumor, Confidence::Rumor));
        assert!(matches!(Confidence::Witnessed, Confidence::Witnessed));
    }

    #[test]
    fn memory_tag_variants() {
        assert!(matches!(MemoryTag::HelpedMe, MemoryTag::HelpedMe));
        let tag = MemoryTag::Custom(std::sync::Arc::from("test"));
        assert!(matches!(tag, MemoryTag::Custom(..)));
    }

    #[test]
    fn memory_query_construction() {
        let q = MemoryQuery::new("recent interactions");
        assert_eq!(&*q.0, "recent interactions");
    }

    #[test]
    fn memory_construction() {
        let m = Memory::new(MemoryId::new(1));
        assert_eq!(m.id.0, 1);
        assert!(m.subject.is_none());
        assert!(m.about.is_empty());
        assert!(m.facts.is_empty());
        assert!(m.tags.is_empty());
        assert_eq!(m.emotional_weight, 0);
        assert!(matches!(m.confidence, Confidence::Believed));
    }
}
