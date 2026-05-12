//! Character-related domain types.

/// A unique identifier for a [`Character`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharacterId(pub String);

impl CharacterId {
    /// Create a new [`CharacterId`] from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// A character participating in the narrative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    /// Unique identifier for this character.
    pub id: CharacterId,
    /// Display name of the character.
    pub name: String,
}

impl Character {
    /// Create a new [`Character`] with the given id and name.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: CharacterId::new(id),
            name: name.into(),
        }
    }
}

/// The moral or relational alignment of a character or faction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alignment {
    /// No specific alignment.
    Neutral,
    /// Friendly alignment.
    Friendly,
    /// Hostile alignment.
    Hostile,
    /// A custom alignment label.
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_construction() {
        let c = Character::new("hero", "Elara");
        assert_eq!(c.id, CharacterId::new("hero"));
        assert_eq!(c.name, "Elara");
    }

    #[test]
    fn alignment_debug() {
        let a = Alignment::Custom("rebel".into());
        assert_eq!(format!("{a:?}"), r#"Custom("rebel")"#);
    }
}
