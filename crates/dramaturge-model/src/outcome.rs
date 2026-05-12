use crate::alignment::{AlignmentAxisId, AlignmentPoleId};
use crate::character::EntityId;
use crate::event::EventKind;
use crate::fact::{Fact, FactId, FactScope, FactValue};
use crate::memory::Memory;
use crate::relationship::RelationshipAxisId;

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    SetFact {
        scope: FactScope,
        fact: Fact,
    },
    RemoveFact {
        scope: FactScope,
        fact_id: FactId,
    },
    AddAlignment {
        axis: AlignmentAxisId,
        pole: AlignmentPoleId,
        delta: i32,
    },
    AddRelationship {
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        delta: i32,
    },
    AddMemory {
        owner: EntityId,
        memory: Memory,
    },
    EmitEvent {
        kind: EventKind,
    },
    StartQuest {
        quest_id: String,
    },
    AdvanceQuest {
        quest_id: String,
        stage: String,
    },
    Custom {
        key: String,
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
