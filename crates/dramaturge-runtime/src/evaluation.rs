//! Condition evaluation and outcome application traits with default implementations.
//!
//! This module defines the [`ConditionEvaluator`] and [`OutcomeApplier`]
//! traits that bridge the domain model ([`Condition`], [`Outcome`]) with
//! the mutable world state ([`DialogWorldState`]).
//! Default implementations ([`DefaultConditionEvaluator`], [`DefaultOutcomeApplier`])
//! are provided for all condition and outcome variants.

use dramaturge_model::{Condition, Outcome};

use crate::context::DialogWorldState;

/// Legacy result enum kept for backward compatibility.
///
/// Currently not used by the default evaluator, which returns `bool` directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationResult {
    /// Evaluation succeeded.
    Ok,
    /// Evaluation failed with the given error message.
    Err(String),
}

/// Trait for evaluating conditions against world state.
pub trait ConditionEvaluator {
    /// Evaluates a condition against the given world state.
    ///
    /// Returns `true` if the condition is satisfied, `false` otherwise.
    fn evaluate(&self, condition: &Condition, context: &DialogWorldState) -> bool;
}

/// Trait for applying outcomes to world state.
pub trait OutcomeApplier {
    /// Applies an outcome to the given world state, mutating it.
    fn apply(&self, outcome: &Outcome, context: &mut DialogWorldState);
}

/// Reference implementation handling all 12 condition variants.
///
/// This evaluator provides the default semantics for each condition:
/// `Always`/`Never` return constant booleans, `All`/`Any`/`Not` perform
/// logical composition, fact checks query the world state, and `Custom`
/// returns `false` (game-specific implementations should provide their own).
#[derive(Debug, Clone, Default)]
pub struct DefaultConditionEvaluator;

impl ConditionEvaluator for DefaultConditionEvaluator {
    fn evaluate(&self, condition: &Condition, context: &DialogWorldState) -> bool {
        match condition {
            Condition::Always => true,
            Condition::Never => false,
            Condition::All(conds) => conds.iter().all(|c| self.evaluate(c, context)),
            Condition::Any(conds) => conds.iter().any(|c| self.evaluate(c, context)),
            Condition::Not(c) => !self.evaluate(c, context),
            Condition::FactEquals {
                scope,
                fact_id,
                value,
            } => match context.get_fact(scope, fact_id) {
                Some(existing) => partial_eq_values(existing, value),
                None => false,
            },
            Condition::FactExists { scope, fact_id } => context.fact_exists(scope, fact_id),
            Condition::AlignmentAtLeast { axis, pole, value } => context
                .get_alignment(axis, pole)
                .map(|v| v >= *value)
                .unwrap_or(false),
            Condition::RelationshipAtLeast {
                source,
                target,
                axis,
                value,
            } => context
                .get_relationship(*source, *target, axis)
                .map(|v| v >= *value)
                .unwrap_or(false),
            Condition::FactAtLeast {
                scope,
                fact_id,
                value,
            } => context
                .get_fact(scope, fact_id)
                .map(|v| matches!(v, dramaturge_model::FactValue::Int(v) if *v >= *value))
                .unwrap_or(false),
            Condition::FactAtMost {
                scope,
                fact_id,
                value,
            } => context
                .get_fact(scope, fact_id)
                .map(|v| matches!(v, dramaturge_model::FactValue::Int(v) if *v <= *value))
                .unwrap_or(false),
            Condition::HasMemory { owner, query } => context.matches_memory_query(*owner, query),
            Condition::HasTagAvailable { tag } => context.has_tag_available(tag),
            Condition::Custom { .. } => false,
        }
    }
}

/// Compares two [`FactValue`]s for equality, using epsilon comparison for floats.
///
/// This exists because `FactValue::Float(f64)` prevents `Eq` derivation
/// on the type. Float comparison uses `(*x - *y).abs() < f64::EPSILON`
/// to handle floating-point precision issues in game state evaluation.
fn partial_eq_values(a: &dramaturge_model::FactValue, b: &dramaturge_model::FactValue) -> bool {
    match (a, b) {
        (dramaturge_model::FactValue::Bool(x), dramaturge_model::FactValue::Bool(y)) => x == y,
        (dramaturge_model::FactValue::Int(x), dramaturge_model::FactValue::Int(y)) => x == y,
        (dramaturge_model::FactValue::Float(x), dramaturge_model::FactValue::Float(y)) => {
            (*x - *y).abs() < f64::EPSILON
        }
        (dramaturge_model::FactValue::Text(x), dramaturge_model::FactValue::Text(y)) => x == y,
        (dramaturge_model::FactValue::Entity(x), dramaturge_model::FactValue::Entity(y)) => x == y,
        (dramaturge_model::FactValue::Set(xs), dramaturge_model::FactValue::Set(ys)) => {
            xs.len() == ys.len()
                && xs
                    .iter()
                    .zip(ys.iter())
                    .all(|(x, y)| partial_eq_values(x, y))
        }
        _ => false,
    }
}

/// Reference implementation handling all 9 outcome variants.
///
/// This applier provides the default semantics for each outcome:
/// fact operations mutate the world state directly, alignment and
/// relationship adjustments delegate to the world state methods,
/// quest management uses fact-based placeholders, and `Custom` is a no-op.
#[derive(Debug, Clone, Default)]
pub struct DefaultOutcomeApplier;

impl OutcomeApplier for DefaultOutcomeApplier {
    fn apply(&self, outcome: &Outcome, context: &mut DialogWorldState) {
        match outcome {
            Outcome::SetFact { scope, fact } => {
                context.set_fact(scope.clone(), fact.clone());
                let event_id = dramaturge_model::EventId::new(context.next_event_id());
                let event = dramaturge_model::Event::new(
                    event_id,
                    dramaturge_model::EventKind::FactLearned {
                        scope: scope.clone(),
                        fact_id: fact.id.clone(),
                    },
                );
                context.add_event(event);
            }
            Outcome::RemoveFact { scope, fact_id } => {
                context.remove_fact(scope, fact_id);
            }
            Outcome::AddAlignment { axis, pole, delta } => {
                context.add_alignment(axis.clone(), pole.clone(), *delta);
                let event_id = dramaturge_model::EventId::new(context.next_event_id());
                let event = dramaturge_model::Event::new(
                    event_id,
                    dramaturge_model::EventKind::AlignmentChanged {
                        axis: axis.clone(),
                        pole: pole.clone(),
                        delta: *delta,
                    },
                );
                context.add_event(event);
            }
            Outcome::AddRelationship {
                source,
                target,
                axis,
                delta,
            } => {
                context.add_relationship(*source, *target, axis.clone(), *delta);
                let event_id = dramaturge_model::EventId::new(context.next_event_id());
                let event = dramaturge_model::Event::new(
                    event_id,
                    dramaturge_model::EventKind::RelationshipChanged {
                        a: *source,
                        b: *target,
                        axis: axis.clone(),
                        delta: *delta,
                    },
                );
                context.add_event(event);
            }
            Outcome::AddMemory { owner, memory } => {
                context.add_memory(*owner, memory.clone());
            }
            Outcome::EmitEvent { kind } => {
                let event_id = dramaturge_model::EventId::new(context.next_event_id());
                let event = dramaturge_model::Event::new(event_id, kind.clone());
                context.add_event(event);
            }
            Outcome::StartQuest { quest_id } => {
                context.set_fact(
                    dramaturge_model::FactScope::World,
                    dramaturge_model::Fact::bool(format!("quest_started_{quest_id}"), true),
                );
                let event_id = dramaturge_model::EventId::new(context.next_event_id());
                let event = dramaturge_model::Event::new(
                    event_id,
                    dramaturge_model::EventKind::Custom {
                        kind: format!("quest_started_{quest_id}").into(),
                    },
                );
                context.add_event(event);
            }
            Outcome::AdvanceQuest { quest_id, stage } => {
                context.set_fact(
                    dramaturge_model::FactScope::World,
                    dramaturge_model::Fact::text(format!("quest_stage_{quest_id}"), stage.as_str()),
                );
                let event_id = dramaturge_model::EventId::new(context.next_event_id());
                let event = dramaturge_model::Event::new(
                    event_id,
                    dramaturge_model::EventKind::Custom {
                        kind: format!("quest_advanced_{quest_id}_{stage}").into(),
                    },
                );
                context.add_event(event);
            }
            Outcome::Custom { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;
    use dramaturge_model::character::EntityId;
    use dramaturge_model::{Fact, FactId, FactScope, FactValue, RelationshipAxisId};

    #[test]
    fn evaluation_result_variants() {
        check!(EvaluationResult::Ok == EvaluationResult::Ok);
        check!(EvaluationResult::Ok != EvaluationResult::Err("oops".into()));
    }

    #[test]
    fn default_evaluator_always() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        check!(evaluator.evaluate(&Condition::Always, &ctx));
    }

    #[test]
    fn default_evaluator_never() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        check!(!evaluator.evaluate(&Condition::Never, &ctx));
    }

    #[test]
    fn default_evaluator_all() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        let cond = Condition::All(vec![Condition::Always, Condition::Always]);
        check!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::All(vec![Condition::Always, Condition::Never]);
        check!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_any() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        let cond = Condition::Any(vec![Condition::Never, Condition::Always]);
        check!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::Any(vec![Condition::Never, Condition::Never]);
        check!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_not() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        check!(!evaluator.evaluate(&Condition::Not(Box::new(Condition::Always)), &ctx));
        check!(evaluator.evaluate(&Condition::Not(Box::new(Condition::Never)), &ctx));
    }

    #[test]
    fn default_evaluator_fact_equals() {
        let evaluator = DefaultConditionEvaluator;
        let mut ctx = DialogWorldState::new();
        ctx.set_fact(FactScope::World, Fact::bool("met_hero", true));
        let cond = Condition::FactEquals {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
            value: FactValue::Bool(true),
        };
        check!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::FactEquals {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
            value: FactValue::Bool(false),
        };
        check!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_fact_exists() {
        let evaluator = DefaultConditionEvaluator;
        let mut ctx = DialogWorldState::new();
        ctx.set_fact(FactScope::World, Fact::bool("met_hero", true));
        let cond = Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
        };
        check!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new("unknown"),
        };
        check!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_has_tag_available() {
        let evaluator = DefaultConditionEvaluator;
        use dramaturge_model::DialogTagKind;
        let mut ctx = DialogWorldState::new();
        let cond = Condition::HasTagAvailable {
            tag: DialogTagKind::new("moral", "paragon"),
        };
        check!(!evaluator.evaluate(&cond, &ctx));
        ctx.add_available_tag(DialogTagKind::new("moral", "paragon"));
        check!(evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_has_memory_with_query() {
        let evaluator = DefaultConditionEvaluator;
        use dramaturge_model::{Memory, MemoryId, MemoryTag};
        let owner = dramaturge_model::character::EntityId::new(1);
        let mut ctx = DialogWorldState::new();

        let cond = Condition::HasMemory {
            owner,
            query: dramaturge_model::MemoryQuery::new("HelpedMe"),
        };
        check!(!evaluator.evaluate(&cond, &ctx));

        let mut memory = Memory::new(MemoryId::new(1));
        memory.tags.push(MemoryTag::HelpedMe);
        ctx.add_memory(owner, memory);

        check!(evaluator.evaluate(&cond, &ctx));

        let cond2 = Condition::HasMemory {
            owner,
            query: dramaturge_model::MemoryQuery::new("BetrayedMe"),
        };
        check!(!evaluator.evaluate(&cond2, &ctx));
    }

    #[test]
    fn default_evaluator_custom() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        let cond = Condition::Custom {
            key: "custom".to_string(),
            args: vec![],
        };
        check!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_applier_set_fact() {
        let applier = DefaultOutcomeApplier;
        let mut ctx = DialogWorldState::new();
        let fact = Fact::bool("met_hero", true);
        let outcome = Outcome::SetFact {
            scope: FactScope::World,
            fact,
        };
        applier.apply(&outcome, &mut ctx);
        check!(ctx.fact_exists(&FactScope::World, &FactId::new("met_hero")));
    }

    #[test]
    fn default_applier_remove_fact() {
        let applier = DefaultOutcomeApplier;
        let mut ctx = DialogWorldState::new();
        ctx.set_fact(FactScope::World, Fact::bool("met_hero", true));
        let outcome = Outcome::RemoveFact {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
        };
        applier.apply(&outcome, &mut ctx);
        check!(!ctx.fact_exists(&FactScope::World, &FactId::new("met_hero")));
    }

    #[test]
    fn default_applier_custom_noop() {
        let applier = DefaultOutcomeApplier;
        let mut ctx = DialogWorldState::new();
        let outcome = Outcome::Custom {
            key: "custom".to_string(),
            args: vec![],
        };
        applier.apply(&outcome, &mut ctx);
        check!(ctx.facts.is_empty());
    }

    #[test]
    fn default_evaluator_relationship_at_least() {
        let evaluator = DefaultConditionEvaluator;
        let mut ctx = DialogWorldState::new();
        let source = EntityId::new(1);
        let target = EntityId::new(2);

        let cond = Condition::RelationshipAtLeast {
            source,
            target,
            axis: RelationshipAxisId::new("trust"),
            value: 30,
        };
        check!(!evaluator.evaluate(&cond, &ctx));

        ctx.add_relationship(source, target, RelationshipAxisId::new("trust"), 30);
        check!(evaluator.evaluate(&cond, &ctx));

        ctx.add_relationship(source, target, RelationshipAxisId::new("trust"), 20);
        check!(evaluator.evaluate(&cond, &ctx));

        ctx.add_relationship(source, target, RelationshipAxisId::new("trust"), -60);
        check!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_applier_add_relationship() {
        let applier = DefaultOutcomeApplier;
        let mut ctx = DialogWorldState::new();
        let source = EntityId::new(1);
        let target = EntityId::new(2);

        let outcome = Outcome::AddRelationship {
            source,
            target,
            axis: RelationshipAxisId::new("trust"),
            delta: 15,
        };
        applier.apply(&outcome, &mut ctx);
        check!(ctx.get_relationship(source, target, &RelationshipAxisId::new("trust")) == Some(15));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;
    use proptest::strategy::Strategy;

    use super::*;
    use dramaturge_model::character::{CharacterId, EntityId};
    use dramaturge_model::condition::Condition;
    use dramaturge_model::fact::{FactScope, FactValue};
    use proptest::collection::vec as prop_vec;

    const MAX_SET_DEPTH: usize = 2;

    fn arb_fact_value(depth: usize) -> BoxedStrategy<FactValue> {
        let leaf: BoxedStrategy<FactValue> = prop_oneof![
            any::<bool>().prop_map(FactValue::Bool),
            any::<i64>().prop_map(FactValue::Int),
            any::<f64>().prop_map(FactValue::Float),
            ".*".prop_map(FactValue::Text),
            any::<u64>().prop_map(|n| FactValue::Entity(EntityId::new(n))),
        ]
        .boxed();
        if depth == 0 {
            leaf
        } else {
            let inner_depth = depth - 1;
            prop_oneof![
                leaf,
                prop_vec(arb_fact_value(inner_depth), 0..4).prop_map(FactValue::Set),
            ]
            .boxed()
        }
    }

    fn arb_fact_value_shrink() -> BoxedStrategy<FactValue> {
        arb_fact_value(MAX_SET_DEPTH)
    }

    fn arb_fact_scope() -> impl Strategy<Value = FactScope> {
        prop_oneof![
            Just(FactScope::World),
            Just(FactScope::Player),
            Just(FactScope::Party),
            ".*".prop_map(|s: String| FactScope::Faction(s.into())),
            any::<u64>().prop_map(|n| FactScope::Character(CharacterId::new(EntityId::new(n)))),
        ]
    }

    fn arb_condition(depth: usize) -> BoxedStrategy<Condition> {
        let leaf: BoxedStrategy<Condition> = prop_oneof![
            Just(Condition::Always),
            Just(Condition::Never),
            arb_fact_scope()
                .prop_flat_map(|scope| {
                    (".*", arb_fact_value(0)).prop_map(move |(id, value)| Condition::FactEquals {
                        scope: scope.clone(),
                        fact_id: dramaturge_model::FactId::new(id),
                        value,
                    })
                })
                .boxed(),
            arb_fact_scope()
                .prop_map(|scope| {
                    (".*".prop_map(move |id| Condition::FactExists {
                        scope: scope.clone(),
                        fact_id: dramaturge_model::FactId::new(id),
                    }))
                    .boxed()
                })
                .prop_flat_map(|s| s),
            arb_fact_scope()
                .prop_flat_map(|scope| {
                    (".*", any::<i64>()).prop_map(move |(id, value)| Condition::FactAtLeast {
                        scope: scope.clone(),
                        fact_id: dramaturge_model::FactId::new(id),
                        value,
                    })
                })
                .boxed(),
            arb_fact_scope()
                .prop_flat_map(|scope| {
                    (".*", any::<i64>()).prop_map(move |(id, value)| Condition::FactAtMost {
                        scope: scope.clone(),
                        fact_id: dramaturge_model::FactId::new(id),
                        value,
                    })
                })
                .boxed(),
            (".*", any::<i32>())
                .prop_map(|(axis, value)| {
                    Condition::AlignmentAtLeast {
                        axis: dramaturge_model::AlignmentAxisId::new(axis.clone()),
                        pole: dramaturge_model::AlignmentPoleId::new(axis),
                        value,
                    }
                })
                .boxed(),
            (any::<u64>(), any::<u64>(), ".*", any::<i32>())
                .prop_map(|(s, t, axis, value)| {
                    Condition::RelationshipAtLeast {
                        source: EntityId::new(s),
                        target: EntityId::new(t),
                        axis: dramaturge_model::RelationshipAxisId::new(axis.as_str()),
                        value,
                    }
                })
                .boxed(),
            any::<u64>()
                .prop_map(|owner| {
                    Condition::HasMemory {
                        owner: EntityId::new(owner),
                        query: dramaturge_model::MemoryQuery::new("test"),
                    }
                })
                .boxed(),
            (".*", prop_vec(arb_fact_value(0), 0..2))
                .prop_map(|(key, args)| Condition::Custom { key, args })
                .boxed(),
        ]
        .boxed();

        if depth == 0 {
            leaf
        } else {
            let inner_depth = depth - 1;
            prop_oneof![
                leaf,
                prop_vec(arb_condition(inner_depth), 0..3).prop_map(Condition::All),
                prop_vec(arb_condition(inner_depth), 0..3).prop_map(Condition::Any),
                arb_condition(inner_depth).prop_map(|c| Condition::Not(Box::new(c))),
            ]
            .boxed()
        }
    }

    fn arb_condition_shrink() -> BoxedStrategy<Condition> {
        arb_condition(2)
    }

    #[test]
    fn prop_partial_eq_reflexive() {
        proptest!(|(val in arb_fact_value_shrink())| {
            prop_assert!(partial_eq_values(&val, &val));
        });
    }

    #[test]
    fn prop_partial_eq_symmetric() {
        proptest!(|(a in arb_fact_value_shrink(), b in arb_fact_value_shrink())| {
            prop_assert_eq!(partial_eq_values(&a, &b), partial_eq_values(&b, &a));
        });
    }

    #[test]
    fn prop_type_mismatch_false() {
        let cases: Vec<(FactValue, FactValue)> = vec![
            (FactValue::Bool(true), FactValue::Int(1)),
            (FactValue::Int(1), FactValue::Float(1.0)),
            (FactValue::Float(1.0), FactValue::Text("1".into())),
            (
                FactValue::Text("1".into()),
                FactValue::Entity(EntityId::new(1)),
            ),
            (FactValue::Entity(EntityId::new(1)), FactValue::Set(vec![])),
            (FactValue::Set(vec![]), FactValue::Bool(false)),
        ];
        for (a, b) in cases {
            assert!(!partial_eq_values(&a, &b));
        }
    }

    #[test]
    fn prop_float_epsilon_boundary() {
        proptest!(|(diff: f64)| {
            let base = 0.0_f64;
            let diff_magnitude = diff.abs();
            prop_assume!(diff_magnitude < 1e-10);
            prop_assume!(diff_magnitude > 0.0);
            let same = FactValue::Float(base);
            let off = FactValue::Float(base + diff);
            let is_equal = partial_eq_values(&same, &off);
            if diff_magnitude < f64::EPSILON {
                prop_assert!(is_equal);
            } else {
                prop_assert!(!is_equal);
            }
        });
    }

    #[test]
    fn prop_set_order_sensitive() {
        proptest!(|
            (v1 in arb_fact_value_shrink(),
             v2 in arb_fact_value_shrink())
        | {
            let set1 = FactValue::Set(vec![v1.clone(), v2.clone()]);
            let set2 = FactValue::Set(vec![v2.clone(), v1.clone()]);
            let eq1 = partial_eq_values(&set1, &set2);
            let eq2 = partial_eq_values(&set2, &set1);
            prop_assert_eq!(eq1, eq2);
        });
    }

    #[test]
    fn prop_de_morgan_not_all() {
        proptest!(|(a in arb_condition_shrink(), b in arb_condition_shrink())| {
            let ctx = DialogWorldState::new();
            let not_all = Condition::Not(Box::new(Condition::All(vec![a.clone(), b.clone()])));
            let any_not = Condition::Any(vec![Condition::Not(Box::new(a)), Condition::Not(Box::new(b))]);
            let evaluator = DefaultConditionEvaluator;
            prop_assert_eq!(evaluator.evaluate(&not_all, &ctx), evaluator.evaluate(&any_not, &ctx));
        });
    }

    #[test]
    fn prop_de_morgan_not_any() {
        proptest!(|(a in arb_condition_shrink(), b in arb_condition_shrink())| {
            let ctx = DialogWorldState::new();
            let not_any = Condition::Not(Box::new(Condition::Any(vec![a.clone(), b.clone()])));
            let all_not = Condition::All(vec![Condition::Not(Box::new(a)), Condition::Not(Box::new(b))]);
            let evaluator = DefaultConditionEvaluator;
            prop_assert_eq!(evaluator.evaluate(&not_any, &ctx), evaluator.evaluate(&all_not, &ctx));
        });
    }

    #[test]
    fn prop_all_single_element() {
        proptest!(|(c in arb_condition_shrink())| {
            let ctx = DialogWorldState::new();
            let wrapped = Condition::All(vec![c.clone()]);
            let evaluator = DefaultConditionEvaluator;
            prop_assert_eq!(evaluator.evaluate(&wrapped, &ctx), evaluator.evaluate(&c, &ctx));
        });
    }

    #[test]
    fn prop_any_single_element() {
        proptest!(|(c in arb_condition_shrink())| {
            let ctx = DialogWorldState::new();
            let wrapped = Condition::Any(vec![c.clone()]);
            let evaluator = DefaultConditionEvaluator;
            prop_assert_eq!(evaluator.evaluate(&wrapped, &ctx), evaluator.evaluate(&c, &ctx));
        });
    }

    #[test]
    fn prop_all_empty_true() {
        let ctx = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator;
        assert!(evaluator.evaluate(&Condition::All(vec![]), &ctx));
    }

    #[test]
    fn prop_any_empty_false() {
        let ctx = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator;
        assert!(!evaluator.evaluate(&Condition::Any(vec![]), &ctx));
    }

    #[test]
    fn prop_double_negation() {
        proptest!(|(c in arb_condition_shrink())| {
            let ctx = DialogWorldState::new();
            let double = Condition::Not(Box::new(Condition::Not(Box::new(c.clone()))));
            let evaluator = DefaultConditionEvaluator;
            prop_assert_eq!(evaluator.evaluate(&double, &ctx), evaluator.evaluate(&c, &ctx));
        });
    }

    #[test]
    fn prop_not_always_is_never() {
        let ctx = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator;
        let not_always = Condition::Not(Box::new(Condition::Always));
        assert_eq!(
            evaluator.evaluate(&not_always, &ctx),
            evaluator.evaluate(&Condition::Never, &ctx)
        );
    }

    #[test]
    fn prop_not_never_is_always() {
        let ctx = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator;
        let not_never = Condition::Not(Box::new(Condition::Never));
        assert_eq!(
            evaluator.evaluate(&not_never, &ctx),
            evaluator.evaluate(&Condition::Always, &ctx)
        );
    }
}
