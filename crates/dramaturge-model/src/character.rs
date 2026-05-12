use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

impl EntityId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharacterId(pub EntityId);

impl CharacterId {
    pub fn new(id: EntityId) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharacterKey(pub Arc<str>);

impl CharacterKey {
    pub fn new(key: impl Into<Arc<str>>) -> Self {
        Self(key.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    pub id: CharacterId,
    pub key: CharacterKey,
    pub display_name: Arc<str>,
    pub traits: Vec<CharacterTrait>,
}

impl Character {
    pub fn new(id: EntityId, key: impl Into<Arc<str>>, display_name: impl Into<Arc<str>>) -> Self {
        Self {
            id: CharacterId::new(id),
            key: CharacterKey::new(key),
            display_name: display_name.into(),
            traits: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharacterTrait {
    pub namespace: Arc<str>,
    pub key: Arc<str>,
}

impl CharacterTrait {
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

    #[test]
    fn entity_id_is_copy() {
        let a = EntityId::new(1);
        let b = a;
        assert_eq!(a, b);
    }
}
