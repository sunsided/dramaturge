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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Placeholder for querying memories by criteria.
///
/// Currently wraps an `Arc<str>` query string. Future iterations may
/// expand this to support structured query expressions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryQuery(pub Arc<str>);

impl MemoryQuery {
    /// Creates a new memory query from the given string-like value.
    pub fn new(query: impl Into<Arc<str>>) -> Self {
        Self(query.into())
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
        let q = MemoryQuery::new("recent interactions");
        check!(&*q.0 == "recent interactions");
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
}
