//! `dramaturge` — A Rust toolkit for branching dialogue, character memory, and narrative state.
//!
//! This facade crate re-exports the most important types from the constituent
//! crates so that users can start with a single `use dramaturge::*;` or by
//! importing individual items.
//!
//! # Example
//!
//! ```rust
//! use dramaturge::{Character, CharacterId, DialogueGraph, DialogueNodeId, EntityId};
//!
//! let hero = Character::new(EntityId::new(1), "hero", "Elara");
//! assert_eq!(&*hero.display_name, "Elara");
//!
//! let graph = DialogueGraph::default();
//! assert!(graph.nodes.is_empty());
//! ```

pub use dramaturge_model::{
    AlignmentAxis, AlignmentAxisId, AlignmentPole, AlignmentPoleId, AlignmentScore, Character,
    CharacterId, CharacterKey, CharacterTrait, Condition, Confidence, CustomDialogTag,
    DialogIntentions, DialogTag, DialogTagKind, DialogTarget, DialogText, DialogueGraph,
    DialogueNode, DialogueNodeId, DialogueOption, DialogueOptionId, EntityId, Event, EventContext,
    EventId, EventKind, Fact, FactId, FactScope, FactValue, Intention, Memory, MemoryId,
    MemoryQuery, MemoryTag, MoralTag, Outcome, RelationshipAxisId, RelationshipScore, ScopedFact,
    SkillTag, SpeechActTag,
};

pub use dramaturge_runtime::{
    ConditionEvaluator, DefaultConditionEvaluator, DefaultOutcomeApplier, DefaultRuntime,
    DialogWorldState, DialogueRuntime, EvaluationContext, EvaluationResult, OutcomeApplier,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_re_exports_compile() {
        let _: EntityId = EntityId::new(1);
        let _: CharacterId = CharacterId::new(EntityId::new(1));
        let _: Character = Character::new(EntityId::new(1), "npc", "Guard");
        let _: DialogueGraph = DialogueGraph::default();
        let _: DialogWorldState = DialogWorldState::new();
        let _: EvaluationContext = DialogWorldState::new();
        assert_eq!(EvaluationResult::Ok, EvaluationResult::Ok);
        assert!(matches!(Condition::Always, Condition::Always));
        let _: FactValue = FactValue::Bool(true);
        let _: MemoryTag = MemoryTag::HelpedMe;
        let _: DialogTagKind = DialogTagKind::Moral(MoralTag::Paragon);
        let _: Intention = Intention::Persuade;
    }
}
