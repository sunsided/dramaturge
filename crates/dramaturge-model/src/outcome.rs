//! Outcomes — state changes triggered by dialogue choices.
//!
//! Outcomes represent effects applied to the dialogue world state when
//! a dialogue option is chosen or a node is entered. They cover fact
//! manipulation, alignment and relationship adjustments, memory creation,
//! event emission, quest management, and custom extension points.

use crate::alignment::{AlignmentAxisId, AlignmentPoleId};
use crate::character::EntityId;
use crate::event::EventKind;
use crate::fact::{Fact, FactId, FactScope, FactValue};
use crate::memory::Memory;
use crate::memory::MemoryId;
use crate::relationship::RelationshipAxisId;

/// Effect applied to the dialogue world state when triggered.
///
/// Outcomes are attached to [`DialogueOption`](crate::DialogueOption)
/// (applied when the option is chosen) or to [`DialogueNode`](crate::DialogueNode)
/// (applied when the node is entered).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Outcome {
    /// Sets or updates a scoped fact in the world state.
    SetFact {
        /// The scope in which to apply the fact.
        scope: FactScope,
        /// The fact data to set.
        fact: Fact,
    },
    /// Removes a scoped fact from the world state.
    RemoveFact {
        /// The scope from which to remove the fact.
        scope: FactScope,
        /// The identifier of the fact to remove.
        fact_id: FactId,
    },
    /// Adjusts an alignment score on a specific axis/pole.
    AddAlignment {
        /// The alignment axis to adjust.
        axis: AlignmentAxisId,
        /// The pole on the axis to adjust.
        pole: AlignmentPoleId,
        /// The amount to add (positive or negative).
        delta: i32,
    },
    /// Adjusts a relationship score between two entities.
    AddRelationship {
        /// The source entity in the relationship.
        source: EntityId,
        /// The target entity in the relationship.
        target: EntityId,
        /// The relationship axis to adjust.
        axis: RelationshipAxisId,
        /// The amount to add (positive or negative).
        delta: i32,
    },
    /// Adds a memory to an entity's memory bank.
    AddMemory {
        /// The entity that owns the memory.
        owner: EntityId,
        /// The memory to add.
        memory: Memory,
    },
    /// Removes a memory from an entity's memory bank.
    RemoveMemory {
        /// The entity that owns the memory.
        owner: EntityId,
        /// The identifier of the memory to remove.
        memory_id: MemoryId,
    },
    /// Emits a narrative event into the world state's event log.
    EmitEvent {
        /// The event kind to emit.
        kind: EventKind,
    },
    /// Marks a quest as started.
    /// Implemented via a fact-based placeholder (`quest_started_{quest_id}`).
    StartQuest {
        /// The identifier of the quest to start.
        quest_id: String,
    },
    /// Advances a quest to a new stage.
    /// Implemented via a fact-based placeholder (`quest_stage_{quest_id}`).
    AdvanceQuest {
        /// The identifier of the quest to advance.
        quest_id: String,
        /// The new stage identifier.
        stage: String,
    },
    /// User-defined custom outcome with arbitrary key and arguments.
    /// Currently a no-op in the default outcome applier.
    Custom {
        /// Custom outcome identifier.
        key: String,
        /// Arbitrary fact value arguments for the outcome.
        args: Vec<FactValue>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryTag;
    use assert2::check;

    #[test]
    fn outcome_set_fact() {
        let fact = Fact::bool("met_hero", true);
        let o = Outcome::SetFact {
            scope: FactScope::World,
            fact,
        };
        check!(let Outcome::SetFact { .. } = o);
    }

    #[test]
    fn outcome_remove_fact() {
        let o = Outcome::RemoveFact {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
        };
        check!(let Outcome::RemoveFact { .. } = o);
    }

    #[test]
    fn outcome_add_alignment() {
        let o = Outcome::AddAlignment {
            axis: AlignmentAxisId::new("moral"),
            pole: AlignmentPoleId::new("paragon"),
            delta: 10,
        };
        check!(let Outcome::AddAlignment { .. } = o);
    }

    #[test]
    fn outcome_add_memory() {
        let memory = Memory::new(MemoryId::new(1)).with_tag(MemoryTag::HelpedMe);
        let o = Outcome::AddMemory {
            owner: EntityId::new(1),
            memory,
        };
        check!(let Outcome::AddMemory { owner, memory: m } = o && owner.0 == 1 && m.id.0 == 1);
    }

    #[test]
    fn outcome_remove_memory() {
        let o = Outcome::RemoveMemory {
            owner: EntityId::new(1),
            memory_id: MemoryId::new(42),
        };
        check!(let Outcome::RemoveMemory { owner, memory_id } = o && owner.0 == 1 && memory_id.0 == 42);
    }

    #[test]
    fn outcome_add_relationship() {
        let o = Outcome::AddRelationship {
            source: EntityId::new(1),
            target: EntityId::new(2),
            axis: RelationshipAxisId::new("trust"),
            delta: 5,
        };
        check!(let Outcome::AddRelationship { .. } = o);
    }

    #[test]
    fn outcome_emit_event() {
        use std::sync::Arc;
        let kind = EventKind::Custom {
            kind: Arc::from("test"),
        };
        let o = Outcome::EmitEvent { kind };
        check!(let Outcome::EmitEvent { .. } = o);
    }

    #[test]
    fn outcome_start_quest() {
        let o = Outcome::StartQuest {
            quest_id: "main_quest".to_string(),
        };
        check!(let Outcome::StartQuest { .. } = o);
    }

    #[test]
    fn outcome_advance_quest() {
        let o = Outcome::AdvanceQuest {
            quest_id: "main_quest".to_string(),
            stage: "complete".to_string(),
        };
        check!(let Outcome::AdvanceQuest { .. } = o);
    }

    #[test]
    fn outcome_custom() {
        let o = Outcome::Custom {
            key: "custom_outcome".to_string(),
            args: vec![],
        };
        check!(let Outcome::Custom { .. } = o);
    }
}
