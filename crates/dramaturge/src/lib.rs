//! `dramaturge` — A Rust toolkit for branching dialogue, character memory, and narrative state.
//!
//! This facade crate re-exports the most important types from the constituent
//! crates. Start here with `use dramaturge::*;` or import individual items.
//!
//! # Design Philosophy
//!
//! dramaturge is deliberately game-agnostic. It has no knowledge of Mass Effect,
//! Pokémon, or any other game's rules. All game-specific logic is expressed
//! through typed IDs, tags, facts, and state changes:
//!
//! - **Facts** store arbitrary named values scoped to the player, a character,
//!   a faction, or the world. Your game decides what the keys mean.
//! - **Alignment axes** are named dimensions with named poles — you define
//!   `"morality"` / `"paragon"` / `"renegade"`, not the library.
//! - **Tags** are axis/key pairs on dialogue options. You define `"skill:medic"`;
//!   the runtime filters options based on world state.
//! - **Conditions** are composable trees over world state. `Condition::Custom`
//!   lets you add game-specific branches the default evaluator defers to you.
//!
//! # Layered Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │  dramaturge (facade)                                 │
//! ├──────────────────────────────────────────────────────┤
//! │  dramaturge-model     pure domain types, no I/O      │
//! │  dramaturge-runtime   evaluation traits + defaults   │
//! │  dramaturge-authoring validation, graph checks       │
//! │  dramaturge-macros    derive macros for ergonomics   │
//! │  dramaturge-serde     optional serde / YAML support  │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! The model crate has no serde or runtime dependency. The runtime crate knows
//! nothing about serialization. You can depend on individual crates or on this
//! facade for the full public surface.
//!
//! # Extending the System
//!
//! - **Custom conditions** — add `Condition::Custom { key, args }` to your graphs
//!   and implement [`ConditionEvaluator`] to handle them. See
//!   [`dramaturge_runtime::evaluation`] for the trait.
//! - **Custom tags** — implement the [`DialogTag`] trait on your own enum or use
//!   [`CustomDialogTag`] directly. See [`dramaturge_model::tag`] for details.
//! - **Custom outcomes** — implement [`OutcomeApplier`] to handle `Outcome::Custom`
//!   and apply game-specific side effects.
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
    MemoryQuery, MemoryTag, Outcome, RelationshipAxisId, RelationshipScore, ScopedFact,
};

pub use dramaturge_runtime::{
    ConditionEvaluator, DefaultConditionEvaluator, DefaultOutcomeApplier, DefaultRuntime,
    DialogWorldState, DialogueRuntime, EvaluationContext, EvaluationResult, OutcomeApplier,
};

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn facade_re_exports_compile() {
        let _: EntityId = EntityId::new(1);
        let _: CharacterId = CharacterId::new(EntityId::new(1));
        let _: Character = Character::new(EntityId::new(1), "npc", "Guard");
        let _: DialogueGraph = DialogueGraph::default();
        let _: DialogWorldState = DialogWorldState::new();
        let _: EvaluationContext = DialogWorldState::new();
        check!(EvaluationResult::Ok == EvaluationResult::Ok);
        check!(let Condition::Always = Condition::Always);
        let _: FactValue = FactValue::Bool(true);
        let _: MemoryTag = MemoryTag::HelpedMe;
        let _: DialogTagKind = DialogTagKind::new("moral", "paragon");
        let _: Intention = Intention::Persuade;
    }
}
