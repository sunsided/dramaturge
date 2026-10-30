//! Core domain model for the `dramaturge` dialogue toolkit.
//!
//! This crate provides game-agnostic foundational types for branching dialogue
//! systems. It has no serde or runtime dependency. Serialization is handled
//! exclusively by the `dramaturge-serde` crate via its DTO types.
//!
//! # Concepts
//!
//! ## Characters and Entities
//!
//! [`EntityId`] is the universal identifier — a `u64` wrapper used everywhere.
//! [`Character`] wraps it with a string key, display name, and optional traits.
//! [`CharacterTrait`] is a generic `namespace::key` pair; your game decides
//! what traits exist.
//!
//! ## Facts and Scoping
//!
//! [`Fact`] is the unit of knowledge: a named [`FactValue`] (bool, int, float,
//! text, entity reference, or set). [`FactScope`] controls who knows it:
//! [`FactScope::World`] (objective), [`FactScope::Player`], [`FactScope::Character`],
//! [`FactScope::Faction`], or [`FactScope::Party`]. Asymmetric knowledge is
//! first-class — the player can know something a character does not.
//!
//! Note: [`FactValue::Float`] contains `f64`, which prevents `Eq`/`Hash`
//! derivation. All types that transitively contain `FactValue` derive only
//! `Debug, Clone, PartialEq`. Float equality uses epsilon comparison in the
//! evaluation layer.
//!
//! ## Alignment
//!
//! [`AlignmentAxis`] defines a named dimension (e.g., `"morality"`) with named
//! [`AlignmentPole`]s (e.g., `"paragon"`, `"renegade"`). [`AlignmentScore`]
//! tracks a character's current position. Your game defines the axes and poles.
//!
//! ## Relationships
//!
//! [`RelationshipScore`] tracks directional, named relationships between entities
//! (e.g., player → NPC trust). Axes are arbitrary string keys.
//!
//! ## Dialogue Graphs
//!
//! [`DialogueGraph`] is a pure data structure: a collection of [`DialogueNode`]s,
//! each holding text, a speaker, [`DialogueOption`]s, and optional enter/exit
//! [`Outcome`]s. No evaluation logic lives here.
//!
//! ## Conditions
//!
//! [`Condition`] is a composable boolean expression tree evaluated against world
//! state. Supports logical operators, fact checks, alignment/relationship
//! thresholds, memory queries, tag availability, and `Condition::Custom` for
//! game-specific extensions.
//!
//! ## Outcomes
//!
//! [`Outcome`] is a declarative state change: set a fact, adjust alignment,
//! modify a relationship, add a memory, emit an event, or a custom extension.
//! Applied by [`OutcomeApplier`](https://docs.rs/dramaturge-runtime/latest/dramaturge_runtime/trait.OutcomeApplier.html) at runtime.
//!
//! ## Memories
//!
//! [`Memory`] is a character's subjective belief about another entity. Unlike
//! facts (objective/scoped knowledge), memories carry a [`Confidence`] level
//! and optional [`MemoryTag`]s (e.g., `HelpedMe`, `Betrayed`). Characters can
//! have multiple memories about the same entity.
//!
//! ## Events
//!
//! [`Event`] records a moment in the narrative timeline — a dialogue choice, a
//! fact learned, a relationship change. Events are queryable by kind, actor,
//! and target, enabling reactive narrative logic.
//!
//! ## Intentions
//!
//! [`Intention`] describes the communicative purpose of a dialogue option
//! (e.g., `Persuade`, `Threaten`, `Comfort`). Complements [`DialogTag`] by
//! capturing *why* an option exists, not just *what category* it belongs to.
//!
//! ## Symbols
//!
//! Authors refer to content by stable string keys (`npc.mara`,
//! `dlg.mara.intro`); the runtime uses dense numeric ids. A [`SymbolTable`]
//! interns each key once and maps ids back to keys for diagnostics and save
//! games. Loaders return the table together with the model.
//!
//! ## Tags
//!
//! [`DialogTag`] is a trait for axis/key annotation on dialogue options.
//! [`CustomDialogTag`] is the built-in implementation. Implement [`DialogTag`]
//! on your own enum for a type-safe tag system with derive macro support.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod alignment;
pub mod character;
pub mod condition;
pub mod dialogue;
pub mod event;
pub mod fact;
pub mod intent;
pub mod memory;
pub mod outcome;
pub mod relationship;
pub mod symbol;
pub mod tag;

// Re-exports: alignment
pub use alignment::{
    AlignmentAxis, AlignmentAxisId, AlignmentPole, AlignmentPoleId, AlignmentScore,
};
// Re-exports: characters and entities
pub use character::{Character, CharacterId, CharacterKey, CharacterTrait, EntityId};
// Re-exports: conditions
pub use condition::Condition;
// Re-exports: dialogue graph
pub use dialogue::{
    DialogTarget, DialogText, DialogueGraph, DialogueNode, DialogueNodeId, DialogueOption,
    DialogueOptionId, NavigationHint,
};
// Re-exports: events
pub use event::{Event, EventContext, EventId, EventKind};
// Re-exports: facts and scoping
pub use fact::{Fact, FactId, FactScope, FactValue, ScopedFact};
// Re-exports: intentions
pub use intent::{DialogIntentions, Intention};
// Re-exports: memories
pub use memory::{Confidence, Memory, MemoryId, MemoryQuery, MemoryTag};
// Re-exports: outcomes
pub use outcome::Outcome;
// Re-exports: relationships
pub use relationship::{RelationshipAxis, RelationshipAxisId, RelationshipScore};
// Re-exports: symbol tables
pub use symbol::{Interner, SymbolId, SymbolTable};
// Re-exports: tags
pub use tag::{CustomDialogTag, DialogTag, DialogTagKind};

#[cfg(test)]
mod macro_tests {
    use crate::{Condition, EntityId, FactScope};
    use assert2::check;
    use dramaturge_macros::{ConditionBuilder, FactValue};

    #[derive(FactValue)]
    #[allow(dead_code)]
    enum TestFacts {
        MetHero,
        QuestStarted,
        #[fact(int)]
        Gold,
        #[fact(int)]
        Reputation,
        #[fact(text)]
        PlayerName,
        #[fact(float)]
        Health,
        #[fact(entity)]
        Companion,
    }

    #[derive(FactValue, ConditionBuilder)]
    #[allow(dead_code)]
    enum BuilderFacts {
        MetHero,
        #[fact(int)]
        Gold,
    }

    #[test]
    fn fact_value_key_generation() {
        check!(TestFacts::MetHero.fact_key() == "met_hero");
        check!(TestFacts::QuestStarted.fact_key() == "quest_started");
        check!(TestFacts::Gold.fact_key() == "gold");
        check!(TestFacts::PlayerName.fact_key() == "player_name");
        check!(TestFacts::Health.fact_key() == "health");
        check!(TestFacts::Companion.fact_key() == "companion");
    }

    #[test]
    fn fact_value_type_identification() {
        check!(TestFacts::MetHero.fact_value_type() == "bool");
        check!(TestFacts::Gold.fact_value_type() == "int");
        check!(TestFacts::PlayerName.fact_value_type() == "text");
        check!(TestFacts::Health.fact_value_type() == "float");
        check!(TestFacts::Companion.fact_value_type() == "entity");
    }

    #[test]
    fn fact_value_bool_conditions() {
        let cond = TestFacts::MetHero.is_true(FactScope::World);
        check!(let Condition::FactEquals { value: crate::FactValue::Bool(true), .. } = cond);

        let cond = TestFacts::MetHero.is_false(FactScope::World);
        check!(let Condition::FactEquals { value: crate::FactValue::Bool(false), .. } = cond);
    }

    #[test]
    fn fact_value_int_conditions() {
        let cond = TestFacts::Gold.equals_int(FactScope::World, 100);
        check!(let Condition::FactEquals { value: crate::FactValue::Int(100), .. } = cond);

        let cond = TestFacts::Reputation.at_least(FactScope::Player, 50);
        check!(let Condition::FactAtLeast { value: 50, .. } = cond);

        let cond = TestFacts::Gold.at_most(FactScope::World, 1000);
        check!(let Condition::FactAtMost { value: 1000, .. } = cond);
    }

    #[test]
    fn fact_value_text_conditions() {
        let cond = TestFacts::PlayerName.equals_text(FactScope::World, "Commander");
        check!(let Condition::FactEquals { value: crate::FactValue::Text(_), .. } = cond);
    }

    #[test]
    fn fact_value_float_conditions() {
        let cond = TestFacts::Health.equals_float(FactScope::Player, 85.5);
        check!(let Condition::FactEquals { value: crate::FactValue::Float(85.5), .. } = cond);
    }

    #[test]
    fn fact_value_entity_conditions() {
        let entity = EntityId::new(42);
        let cond = TestFacts::Companion.equals_entity(FactScope::Player, entity);
        check!(let Condition::FactEquals { value: crate::FactValue::Entity(e), .. } = cond && e.0 == 42);
    }

    #[test]
    fn fact_value_exists_and_missing() {
        let cond = TestFacts::MetHero.exists(FactScope::World);
        check!(let Condition::FactExists { .. } = cond);

        let cond = TestFacts::MetHero.missing(FactScope::World);
        check!(let Condition::Not(_) = cond);
    }

    #[test]
    fn condition_builder_all() {
        let cond = BuilderFacts::all(vec![
            BuilderFacts::MetHero.is_true(FactScope::World),
            BuilderFacts::Gold.at_least(FactScope::World, 100),
        ]);
        check!(let Condition::All(_) = cond);
    }

    #[test]
    fn condition_builder_any() {
        let cond = BuilderFacts::any(vec![BuilderFacts::MetHero.is_true(FactScope::World)]);
        check!(let Condition::Any(_) = cond);
    }

    #[test]
    fn condition_builder_not() {
        let cond = BuilderFacts::not(BuilderFacts::MetHero.is_false(FactScope::World));
        check!(let Condition::Not(_) = cond);
    }

    #[test]
    fn condition_builder_always_and_never() {
        check!(let Condition::Always = BuilderFacts::always());
        check!(let Condition::Never = BuilderFacts::never());
    }

    #[test]
    fn condition_builder_custom() {
        let cond = BuilderFacts::custom("check_achievement", vec![]);
        check!(let Condition::Custom { key, .. } = cond && key == "check_achievement");
    }

    #[test]
    fn condition_builder_alignment_at_least() {
        let cond = BuilderFacts::alignment_at_least("morality", "paragon", 50);
        check!(let Condition::AlignmentAtLeast { value: 50, .. } = cond);
    }

    #[test]
    fn condition_builder_relationship_at_least() {
        let source = EntityId::new(1);
        let target = EntityId::new(2);
        let cond = BuilderFacts::relationship_at_least(source, target, "trust", 30);
        check!(let Condition::RelationshipAtLeast { value: 30, .. } = cond);
    }

    #[test]
    fn condition_builder_has_memory() {
        use crate::MemoryQuery;
        let owner = EntityId::new(1);
        let cond = BuilderFacts::has_memory(owner, MemoryQuery::new());
        check!(let Condition::HasMemory { .. } = cond);
    }

    #[test]
    fn condition_builder_has_tag_available() {
        use crate::DialogTagKind;
        let tag = DialogTagKind::new("moral", "paragon");
        let cond = BuilderFacts::has_tag_available(tag);
        check!(let Condition::HasTagAvailable { .. } = cond);
    }
}
