//! Entity and character identity types.
//!
//! This module defines the types used to identify and describe characters
//! and other entities in the narrative. [`EntityId`] serves as a universal
//! identifier, while [`Character`] wraps it with additional identity and
//! trait information.

use std::sync::Arc;

/// Universal identifier for any game entity.
///
/// This is a newtype over `u64` that provides type safety, preventing
/// accidental mixing of entity IDs with other numeric identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

impl EntityId {
    /// Creates a new entity identifier from the given `u64` value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Wraps an [`EntityId`] to distinguish characters from generic entities
/// at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharacterId(pub EntityId);

impl CharacterId {
    /// Creates a new character identifier wrapping the given [`EntityId`].
    pub fn new(id: EntityId) -> Self {
        Self(id)
    }
}

/// Human-readable string key for character lookup.
///
/// Uses `Arc<str>` internally for cheap cloning, making it suitable
/// for use in authoring tools and scripting environments where the same
/// key may be referenced many times.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharacterKey(pub Arc<str>);

impl CharacterKey {
    /// Creates a new character key from the given string-like value.
    pub fn new(key: impl Into<Arc<str>>) -> Self {
        Self(key.into())
    }
}

/// A named, typed participant in the narrative.
///
/// Characters are the agents that speak, remember, form relationships,
/// and hold beliefs about facts in the world. Each character has a unique
/// numeric identity, a stable string key for authoring, a display name
/// for UI, and a set of arbitrary traits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    /// Unique numeric identifier for this character.
    pub id: CharacterId,
    /// Stable string key for authoring and scripting references.
    pub key: CharacterKey,
    /// Human-facing name shown in dialogue and UI.
    pub display_name: Arc<str>,
    /// Arbitrary key-value traits (e.g., `species:pokemon`, `role:doctor`).
    pub traits: Vec<CharacterTrait>,
}

impl Character {
    /// Creates a new character with the given identity, key, and display name.
    ///
    /// The traits vector starts empty.
    pub fn new(id: EntityId, key: impl Into<Arc<str>>, display_name: impl Into<Arc<str>>) -> Self {
        Self {
            id: CharacterId::new(id),
            key: CharacterKey::new(key),
            display_name: display_name.into(),
            traits: Vec::new(),
        }
    }
}

/// A single trait attached to a [`Character`].
///
/// Uses a namespace/key pair for cross-game reusability. For example,
/// `namespace: "species"` with `key: "pokemon"` or `namespace: "role"`
/// with `key: "doctor"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharacterTrait {
    /// Trait category (e.g., `"species"`, `"role"`).
    pub namespace: Arc<str>,
    /// Specific trait value within the namespace (e.g., `"pokemon"`, `"doctor"`).
    pub key: Arc<str>,
}

impl CharacterTrait {
    /// Creates a new trait with the given namespace and key.
    pub fn new(namespace: impl Into<Arc<str>>, key: impl Into<Arc<str>>) -> Self {
        Self {
            namespace: namespace.into(),
            key: key.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_construction() {
        let id = EntityId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn character_id_construction() {
        let id = CharacterId::new(EntityId::new(1));
        assert_eq!(id.0.0, 1);
    }

    #[test]
    fn character_key_construction() {
        let key = CharacterKey::new("hero");
        assert_eq!(&*key.0, "hero");
    }

    #[test]
    fn character_construction() {
        let c = Character::new(EntityId::new(1), "hero", "Elara");
        assert_eq!(c.id.0.0, 1);
        assert_eq!(&*c.key.0, "hero");
        assert_eq!(&*c.display_name, "Elara");
        assert!(c.traits.is_empty());
    }

    #[test]
    fn character_trait_construction() {
        let t = CharacterTrait::new("combat", "melee");
        assert_eq!(&*t.namespace, "combat");
        assert_eq!(&*t.key, "melee");
    }

    /// Verify that `EntityId` implements `Copy` semantics, allowing
    /// use after assignment without consuming the original.
    #[test]
    fn entity_id_is_copy() {
        let a = EntityId::new(1);
        let b = a;
        assert_eq!(a, b);
    }
}
