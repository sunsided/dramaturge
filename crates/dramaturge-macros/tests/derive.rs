//! The derive macros on the model types.
//!
//! These tests were in `dramaturge-model`. They are here so that
//! `dramaturge-model` has no dev-dependency on this crate (this crate
//! depends on `dramaturge-syntax`, which depends on the model).

use assert2::check;
use dramaturge_macros::{ConditionBuilder, FactValue};
use dramaturge_model::{Condition, EntityId, FactScope};

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
    check!(let Condition::FactEquals { value: dramaturge_model::FactValue::Bool(true), .. } = cond);

    let cond = TestFacts::MetHero.is_false(FactScope::World);
    check!(let Condition::FactEquals { value: dramaturge_model::FactValue::Bool(false), .. } = cond);
}

#[test]
fn fact_value_int_conditions() {
    let cond = TestFacts::Gold.equals_int(FactScope::World, 100);
    check!(let Condition::FactEquals { value: dramaturge_model::FactValue::Int(100), .. } = cond);

    let cond = TestFacts::Reputation.at_least(FactScope::Player, 50);
    check!(let Condition::FactAtLeast { value: 50, .. } = cond);

    let cond = TestFacts::Gold.at_most(FactScope::World, 1000);
    check!(let Condition::FactAtMost { value: 1000, .. } = cond);
}

#[test]
fn fact_value_text_conditions() {
    let cond = TestFacts::PlayerName.equals_text(FactScope::World, "Commander");
    check!(let Condition::FactEquals { value: dramaturge_model::FactValue::Text(_), .. } = cond);
}

#[test]
fn fact_value_float_conditions() {
    let cond = TestFacts::Health.equals_float(FactScope::Player, 85.5);
    check!(let Condition::FactEquals { value: dramaturge_model::FactValue::Float(85.5), .. } = cond);
}

#[test]
fn fact_value_entity_conditions() {
    let entity = EntityId::new(42);
    let cond = TestFacts::Companion.equals_entity(FactScope::Player, entity);
    check!(let Condition::FactEquals { value: dramaturge_model::FactValue::Entity(e), .. } = cond && e.0 == 42);
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
    use dramaturge_model::MemoryQuery;
    let owner = EntityId::new(1);
    let cond = BuilderFacts::has_memory(owner, MemoryQuery::new());
    check!(let Condition::HasMemory { .. } = cond);
}

#[test]
fn condition_builder_has_tag_available() {
    use dramaturge_model::DialogTagKind;
    let tag = DialogTagKind::new("moral", "paragon");
    let cond = BuilderFacts::has_tag_available(tag);
    check!(let Condition::HasTagAvailable { .. } = cond);
}
