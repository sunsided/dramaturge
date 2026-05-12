//! Memory types for storing per-character factual recollections.

use crate::{CharacterId, Fact};

/// A collection of facts associated with a specific character.
#[derive(Debug, Clone, PartialEq)]
pub struct Memory {
    /// The character who owns this memory.
    pub owner: CharacterId,
    /// Facts stored in this memory.
    pub facts: Vec<Fact>,
}

impl Memory {
    /// Create a new empty [`Memory`] for the given character.
    pub fn new(owner: CharacterId) -> Self {
        Self {
            owner,
            facts: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_construction() {
        let m = Memory::new(CharacterId::new("guard"));
        assert!(m.facts.is_empty());
        assert_eq!(m.owner, CharacterId::new("guard"));
    }
}
