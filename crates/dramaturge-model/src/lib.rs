//! Core domain model for the `dramaturge` dialogue toolkit.
//!
//! This crate provides game-agnostic, foundational types for building
//! branching dialogue systems. It is intentionally free of serde or other
//! serialization dependencies — the `dramaturge_serde` crate serves as
//! the designated integration point for optional serialization.
//!
//! The types in this crate represent:
//!
//! - **Characters and entities** — [`EntityId`], [`Character`], [`CharacterTrait`]
//! - **Facts and scoping** — [`Fact`], [`FactValue`], [`FactScope`], [`ScopedFact`]
//!   (note: `FactValue::Float(f64)` prevents `Eq`/`Hash` derivation on
//!   transitively dependent types)
//! - **Alignment** — [`AlignmentAxis`], [`AlignmentPole`], [`AlignmentScore`]
//! - **Relationships** — [`RelationshipScore`], [`RelationshipAxisId`]
//! - **Dialogue graphs** — [`DialogueGraph`], [`DialogueNode`], [`DialogueOption`]
//! - **Conditions** — [`Condition`] (composable boolean expression trees)
//! - **Outcomes** — [`Outcome`] (state changes triggered by dialogue choices)
//! - **Memories** — [`Memory`], [`Confidence`], [`MemoryTag`]
//! - **Events** — [`Event`], [`EventKind`]
//! - **Intentions** — [`Intention`], [`DialogIntentions`]
//! - **Tags** — [`DialogTag`], [`DialogTagKind`], [`CustomDialogTag`]
//!
//! Circular module imports between `dialogue`, `outcome`, `event`, and
//! `condition` are intentional and compile cleanly within a single crate.

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
    DialogueOptionId,
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
pub use relationship::{RelationshipAxisId, RelationshipScore};
// Re-exports: tags
pub use tag::{CustomDialogTag, DialogTag, DialogTagKind};

#[cfg(test)]
mod macro_tests {
    use assert2::check;
    use dramaturge_macros::{ConditionBuilder, FactValue};

    use crate::{Condition, EntityId, FactScope};

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
        let cond = BuilderFacts::has_memory(owner, MemoryQuery::new("test"));
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
