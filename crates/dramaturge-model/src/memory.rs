//! Character memory — subjective beliefs about entities and events.
//!
//! Memories represent a character's recollection about other entities
//! and facts. Unlike [`Fact`](crate::Fact), which represents objective or
//! scoped knowledge, memories are subjective — they carry a
//! [`Confidence`] level indicating how certain the character is, and
//! [`MemoryTag`] tags describing the emotional significance of the
//! recollection.

use std::sync::Arc;

use crate::character::EntityId;
use crate::fact::FactId;

/// Unique numeric identifier for a memory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryId(pub u64);

impl MemoryId {
    /// Creates a new memory identifier from the given `u64` value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// How certain the character is about this memory.
///
/// The variants are ordered from least to most certain:
/// `Rumor` < `Suspected` < `Believed` < `Known` < `Witnessed`.
/// The default confidence level for new memories is [`Confidence::Believed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Confidence {
    /// Heard from others; lowest certainty.
    Rumor,
    /// Weak evidence suggests this is true.
    Suspected,
    /// Default confidence — the character believes this.
    Believed,
    /// Strong evidence supports this memory.
    Known,
    /// The character personally witnessed this event.
    Witnessed,
}

/// Categorical tags describing the emotional/narrative significance of a memory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryTag {
    /// This entity helped the character.
    HelpedMe,
    /// This entity betrayed the character.
    BetrayedMe,
    /// This entity lied to the character.
    LiedToMe,
    /// This entity saved the character's life.
    SavedLife,
    /// This entity threatened the character.
    ThreatenedMe,
    /// This entity showed mercy.
    ShowedMercy,
    /// The entity killed someone.
    KilledSomeone,
    /// The entity revealed a secret.
    RevealedSecret,
    /// User-defined custom memory tag.
    Custom(Arc<str>),
}

/// Structured query for filtering memories by criteria.
///
/// Multiple filters use **AND** semantics — all specified filters must match.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryQuery {
    /// Match memories that have this exact tag.
    pub tag: Option<MemoryTag>,
    /// Minimum confidence level required (inclusive).
    pub min_confidence: Option<Confidence>,
    /// Minimum emotional weight required.
    pub min_emotional_weight: Option<i32>,
    /// Match memories whose subject is this entity.
    pub subject: Option<EntityId>,
    /// Match memories that reference any of these facts.
    pub facts: Vec<FactId>,
}

impl MemoryQuery {
    /// Creates an empty query that matches all memories.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a query that matches memories with the given tag.
    pub fn by_tag(tag: MemoryTag) -> Self {
        Self {
            tag: Some(tag),
            ..Self::default()
        }
    }

    /// Creates a query that matches memories about the given subject.
    pub fn by_subject(subject: EntityId) -> Self {
        Self {
            subject: Some(subject),
            ..Self::default()
        }
    }

    /// Creates a query that matches memories referencing the given fact.
    pub fn by_fact(fact: FactId) -> Self {
        Self {
            facts: vec![fact],
            ..Self::default()
        }
    }

    /// Checks if the given memory matches all filters in this query.
    ///
    /// Uses **AND** semantics — all specified filters must match.
    pub fn matches(&self, memory: &Memory) -> bool {
        if let Some(ref tag) = self.tag
            && !memory.tags.contains(tag)
        {
            return false;
        }
        if let Some(min_conf) = self.min_confidence
            && memory.confidence < min_conf
        {
            return false;
        }
        if let Some(min_weight) = self.min_emotional_weight
            && memory.emotional_weight < min_weight
        {
            return false;
        }
        if let Some(ref subject) = self.subject
            && memory.subject.as_ref() != Some(subject)
        {
            return false;
        }
        if !self.facts.is_empty() && !self.facts.iter().any(|f| memory.facts.contains(f)) {
            return false;
        }
        true
    }
}

/// A character's recollection about entities and facts.
///
/// Memories are subjective and carry metadata about confidence,
/// emotional weight, and categorical tags. They are distinct from
/// facts, which represent scoped knowledge in the world state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    /// Unique identifier for this memory.
    pub id: MemoryId,
    /// Who the memory is primarily about (the main subject).
    pub subject: Option<EntityId>,
    /// Related entities referenced by this memory.
    pub about: Vec<EntityId>,
    /// Fact identifiers referenced by this memory.
    pub facts: Vec<FactId>,
    /// Emotional/narrative significance tags.
    pub tags: Vec<MemoryTag>,
    /// Numeric weight representing the emotional intensity of this memory.
    pub emotional_weight: i32,
    /// How certain the character is about this memory.
    pub confidence: Confidence,
}

impl Memory {
    /// Creates a new memory with the given ID and default values.
    ///
    /// Defaults: no subject, empty collections, zero emotional weight,
    /// and `Confidence::Believed`.
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

    /// Sets the subject of this memory and returns self.
    pub fn with_subject(mut self, subject: EntityId) -> Self {
        self.subject = Some(subject);
        self
    }

    /// Sets the about entities of this memory and returns self.
    pub fn with_about(mut self, about: Vec<EntityId>) -> Self {
        self.about = about;
        self
    }

    /// Adds a tag to this memory and returns self.
    pub fn with_tag(mut self, tag: MemoryTag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Adds multiple tags to this memory and returns self.
    pub fn with_tags(mut self, tags: Vec<MemoryTag>) -> Self {
        self.tags.extend(tags);
        self
    }

    /// Sets the confidence level of this memory and returns self.
    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    /// Sets the emotional weight of this memory and returns self.
    pub fn with_emotional_weight(mut self, weight: i32) -> Self {
        self.emotional_weight = weight;
        self
    }

    /// Sets the facts referenced by this memory and returns self.
    pub fn with_facts(mut self, facts: Vec<FactId>) -> Self {
        self.facts = facts;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn memory_id_construction() {
        let id = MemoryId::new(42);
        check!(id.0 == 42);
    }

    #[test]
    fn memory_id_is_copy() {
        let a = MemoryId::new(1);
        let b = a;
        check!(a == b);
    }

    #[test]
    fn confidence_variants() {
        check!(let Confidence::Rumor = Confidence::Rumor);
        check!(let Confidence::Witnessed = Confidence::Witnessed);
    }

    #[test]
    fn memory_tag_variants() {
        check!(let MemoryTag::HelpedMe = MemoryTag::HelpedMe);
        let tag = MemoryTag::Custom(std::sync::Arc::from("test"));
        check!(let MemoryTag::Custom(..) = tag);
    }

    #[test]
    fn memory_query_construction() {
        let q = MemoryQuery::new();
        check!(q.tag.is_none());
        check!(q.min_confidence.is_none());
        check!(q.subject.is_none());
        check!(q.facts.is_empty());
    }

    #[test]
    fn memory_construction() {
        let m = Memory::new(MemoryId::new(1));
        check!(m.id.0 == 1);
        check!(m.subject.is_none());
        check!(m.about.is_empty());
        check!(m.facts.is_empty());
        check!(m.tags.is_empty());
        check!(m.emotional_weight == 0);
        check!(let Confidence::Believed = m.confidence);
    }

    #[test]
    fn confidence_ordering() {
        assert!(Confidence::Rumor < Confidence::Suspected);
        assert!(Confidence::Suspected < Confidence::Believed);
        assert!(Confidence::Believed < Confidence::Known);
        assert!(Confidence::Known < Confidence::Witnessed);
    }

    #[test]
    fn memory_with_subject_builder() {
        let m = Memory::new(MemoryId::new(1)).with_subject(EntityId::new(42));
        check!(m.subject == Some(EntityId::new(42)));
    }

    #[test]
    fn memory_with_tag_builder() {
        let m = Memory::new(MemoryId::new(1)).with_tag(MemoryTag::HelpedMe);
        check!(m.tags == vec![MemoryTag::HelpedMe]);
    }

    #[test]
    fn memory_with_tags_builder() {
        let m = Memory::new(MemoryId::new(1))
            .with_tags(vec![MemoryTag::HelpedMe, MemoryTag::SavedLife]);
        check!(m.tags.len() == 2);
        check!(m.tags.contains(&MemoryTag::HelpedMe));
        check!(m.tags.contains(&MemoryTag::SavedLife));
    }

    #[test]
    fn memory_with_confidence_builder() {
        let m = Memory::new(MemoryId::new(1)).with_confidence(Confidence::Witnessed);
        check!(let Confidence::Witnessed = m.confidence);
    }

    #[test]
    fn memory_with_emotional_weight_builder() {
        let m = Memory::new(MemoryId::new(1)).with_emotional_weight(100);
        check!(m.emotional_weight == 100);
    }

    #[test]
    fn memory_with_facts_builder() {
        let facts = vec![FactId::new("fact1"), FactId::new("fact2")];
        let m = Memory::new(MemoryId::new(1)).with_facts(facts.clone());
        check!(m.facts == facts);
    }

    #[test]
    fn memory_with_about_builder() {
        let about = vec![EntityId::new(1), EntityId::new(2)];
        let m = Memory::new(MemoryId::new(1)).with_about(about.clone());
        check!(m.about == about);
    }

    #[test]
    fn memory_builder_chaining() {
        let facts = vec![FactId::new("witnessed_event")];
        let m = Memory::new(MemoryId::new(1))
            .with_subject(EntityId::new(42))
            .with_tag(MemoryTag::HelpedMe)
            .with_confidence(Confidence::Witnessed)
            .with_emotional_weight(50)
            .with_facts(facts.clone())
            .with_about(vec![EntityId::new(99)]);

        check!(m.id.0 == 1);
        check!(m.subject == Some(EntityId::new(42)));
        check!(m.tags == vec![MemoryTag::HelpedMe]);
        check!(let Confidence::Witnessed = m.confidence);
        check!(m.emotional_weight == 50);
        check!(m.facts == facts);
        check!(m.about == vec![EntityId::new(99)]);
    }

    #[test]
    fn memory_equality_with_populated_fields() {
        let a = Memory::new(MemoryId::new(1))
            .with_subject(EntityId::new(42))
            .with_tag(MemoryTag::HelpedMe)
            .with_confidence(Confidence::Known)
            .with_emotional_weight(75)
            .with_facts(vec![FactId::new("fact1")])
            .with_about(vec![EntityId::new(99)]);

        let b = Memory::new(MemoryId::new(1))
            .with_subject(EntityId::new(42))
            .with_tag(MemoryTag::HelpedMe)
            .with_confidence(Confidence::Known)
            .with_emotional_weight(75)
            .with_facts(vec![FactId::new("fact1")])
            .with_about(vec![EntityId::new(99)]);

        check!(a == b);
    }

    #[test]
    fn memory_query_by_tag() {
        let q = MemoryQuery::by_tag(MemoryTag::HelpedMe);
        check!(q.tag == Some(MemoryTag::HelpedMe));
        check!(q.subject.is_none());
        check!(q.facts.is_empty());
    }

    #[test]
    fn memory_query_by_subject() {
        let q = MemoryQuery::by_subject(EntityId::new(42));
        check!(q.subject == Some(EntityId::new(42)));
        check!(q.tag.is_none());
    }

    #[test]
    fn memory_query_by_fact() {
        let fact_id = FactId::new("test_fact");
        let q = MemoryQuery::by_fact(fact_id.clone());
        check!(q.facts == vec![fact_id]);
        check!(q.tag.is_none());
    }

    #[test]
    fn memory_query_matches_by_tag() {
        let q = MemoryQuery::by_tag(MemoryTag::HelpedMe);
        let m1 = Memory::new(MemoryId::new(1)).with_tag(MemoryTag::HelpedMe);
        let m2 = Memory::new(MemoryId::new(2)).with_tag(MemoryTag::BetrayedMe);
        check!(q.matches(&m1));
        check!(!q.matches(&m2));
    }

    #[test]
    fn memory_query_matches_by_min_confidence() {
        let q = MemoryQuery {
            min_confidence: Some(Confidence::Known),
            ..MemoryQuery::new()
        };
        let m1 = Memory::new(MemoryId::new(1)).with_confidence(Confidence::Known);
        let m2 = Memory::new(MemoryId::new(2)).with_confidence(Confidence::Witnessed);
        let m3 = Memory::new(MemoryId::new(3)).with_confidence(Confidence::Believed);
        check!(q.matches(&m1));
        check!(q.matches(&m2));
        check!(!q.matches(&m3));
    }

    #[test]
    fn memory_query_matches_by_min_emotional_weight() {
        let q = MemoryQuery {
            min_emotional_weight: Some(50),
            ..MemoryQuery::new()
        };
        let m1 = Memory::new(MemoryId::new(1)).with_emotional_weight(50);
        let m2 = Memory::new(MemoryId::new(2)).with_emotional_weight(100);
        let m3 = Memory::new(MemoryId::new(3)).with_emotional_weight(25);
        check!(q.matches(&m1));
        check!(q.matches(&m2));
        check!(!q.matches(&m3));
    }

    #[test]
    fn memory_query_matches_by_subject() {
        let q = MemoryQuery::by_subject(EntityId::new(42));
        let m1 = Memory::new(MemoryId::new(1)).with_subject(EntityId::new(42));
        let m2 = Memory::new(MemoryId::new(2)).with_subject(EntityId::new(99));
        let m3 = Memory::new(MemoryId::new(3));
        check!(q.matches(&m1));
        check!(!q.matches(&m2));
        check!(!q.matches(&m3));
    }

    #[test]
    fn memory_query_matches_by_facts() {
        let q = MemoryQuery::by_fact(FactId::new("fact_a"));
        let m1 = Memory::new(MemoryId::new(1)).with_facts(vec![FactId::new("fact_a")]);
        let m2 = Memory::new(MemoryId::new(2)).with_facts(vec![FactId::new("fact_b")]);
        let m3 = Memory::new(MemoryId::new(3))
            .with_facts(vec![FactId::new("fact_a"), FactId::new("fact_b")]);
        check!(q.matches(&m1));
        check!(!q.matches(&m2));
        check!(q.matches(&m3));
    }

    #[test]
    fn memory_query_no_filters_matches_all() {
        let q = MemoryQuery::new();
        let m1 = Memory::new(MemoryId::new(1));
        let m2 = Memory::new(MemoryId::new(2))
            .with_tag(MemoryTag::HelpedMe)
            .with_confidence(Confidence::Witnessed)
            .with_emotional_weight(100)
            .with_subject(EntityId::new(42))
            .with_facts(vec![FactId::new("fact1")]);
        check!(q.matches(&m1));
        check!(q.matches(&m2));
    }

    #[test]
    fn memory_query_multiple_filters_and_semantics() {
        let q = MemoryQuery {
            tag: Some(MemoryTag::HelpedMe),
            min_confidence: Some(Confidence::Known),
            ..MemoryQuery::new()
        };
        let m1 = Memory::new(MemoryId::new(1))
            .with_tag(MemoryTag::HelpedMe)
            .with_confidence(Confidence::Known);
        let m2 = Memory::new(MemoryId::new(2))
            .with_tag(MemoryTag::HelpedMe)
            .with_confidence(Confidence::Believed);
        let m3 = Memory::new(MemoryId::new(3))
            .with_tag(MemoryTag::BetrayedMe)
            .with_confidence(Confidence::Known);
        check!(q.matches(&m1));
        check!(!q.matches(&m2));
        check!(!q.matches(&m3));
    }
}
