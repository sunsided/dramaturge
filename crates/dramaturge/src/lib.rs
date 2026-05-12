//! `dramaturge` — A Rust toolkit for branching dialogue, character memory, and narrative state.
//!
//! This facade crate re-exports the most important types from the constituent
//! crates so that users can start with a single `use dramaturge::*;` or by
//! importing individual items.
//!
//! # Example
//!
//! ```rust
//! use dramaturge::{Character, CharacterId, DialogueGraph};
//!
//! let hero = Character::new("hero", "Elara");
//! assert_eq!(hero.name, "Elara");
//!
//! let graph = DialogueGraph::default();
//! assert!(graph.nodes.is_empty());
//! ```

// Re-export the complete public model surface.
pub use dramaturge_model::{
    Alignment, Character, CharacterId, Condition, DialogueGraph, DialogueNode, DialogueOption,
    Event, Fact, FactKey, FactValue, Intent, Memory, OptionTag, Outcome,
};

// Re-export the runtime interface.
pub use dramaturge_runtime::{
    ConditionEvaluator, DialogueRuntime, EvaluationContext, EvaluationResult, OutcomeApplier,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_re_exports_compile() {
        let _: CharacterId = CharacterId::new("npc");
        let _: Character = Character::new("npc", "Guard");
        let _: DialogueGraph = DialogueGraph::default();
        let _: EvaluationContext = EvaluationContext::new();
        assert_eq!(EvaluationResult::Ok, EvaluationResult::Ok);
    }
}
