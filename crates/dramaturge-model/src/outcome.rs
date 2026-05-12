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
use crate::relationship::RelationshipAxisId;

/// Effect applied to the dialogue world state when triggered.
///
/// Outcomes are attached to [`DialogueOption`](crate::DialogueOption)
/// (applied when the option is chosen) or to [`DialogueNode`](crate::DialogueNode)
/// (applied when the node is entered).
#[derive(Debug, Clone, PartialEq)]
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

    #[test]
    fn outcome_set_fact() {
        let fact = Fact::bool("met_hero", true);
        let o = Outcome::SetFact {
            scope: FactScope::World,
            fact,
        };
        assert!(matches!(o, Outcome::SetFact { .. }));
    }

    #[test]
    fn outcome_remove_fact() {
        let o = Outcome::RemoveFact {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
        };
        assert!(matches!(o, Outcome::RemoveFact { .. }));
    }

    #[test]
    fn outcome_add_alignment() {
        let o = Outcome::AddAlignment {
            axis: AlignmentAxisId::new("moral"),
            pole: AlignmentPoleId::new("paragon"),
            delta: 10,
        };
        assert!(matches!(o, Outcome::AddAlignment { .. }));
    }

    #[test]
    fn outcome_add_relationship() {
        let o = Outcome::AddRelationship {
            source: EntityId::new(1),
            target: EntityId::new(2),
            axis: RelationshipAxisId::new("trust"),
            delta: 5,
        };
        assert!(matches!(o, Outcome::AddRelationship { .. }));
    }

    #[test]
    fn outcome_emit_event() {
        use std::sync::Arc;
        let kind = EventKind::Custom {
            kind: Arc::from("test"),
        };
        let o = Outcome::EmitEvent { kind };
        assert!(matches!(o, Outcome::EmitEvent { .. }));
    }

    #[test]
    fn outcome_start_quest() {
        let o = Outcome::StartQuest {
            quest_id: "main_quest".to_string(),
        };
        assert!(matches!(o, Outcome::StartQuest { .. }));
    }

    #[test]
    fn outcome_advance_quest() {
        let o = Outcome::AdvanceQuest {
            quest_id: "main_quest".to_string(),
            stage: "complete".to_string(),
        };
        assert!(matches!(o, Outcome::AdvanceQuest { .. }));
    }

    #[test]
    fn outcome_custom() {
        let o = Outcome::Custom {
            key: "custom_outcome".to_string(),
            args: vec![],
        };
        assert!(matches!(o, Outcome::Custom { .. }));
    }
}
