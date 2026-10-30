//! Condition evaluation and outcome application traits with default implementations.
//!
//! This module defines the [`ConditionEvaluator`] and [`OutcomeApplier`]
//! traits that bridge the domain model ([`Condition`], [`Outcome`]) with
//! narrative state. Both traits are generic over the state type `S`, which
//! defaults to [`DialogWorldState`]. Any [`NarrativeState`] works, including
//! a host type that embeds a `DialogWorldState`.
//!
//! Default implementations ([`DefaultConditionEvaluator`], [`DefaultOutcomeApplier`])
//! handle every built-in variant and hand `Custom` variants to a
//! [`CustomHandler`]. The default evaluator recurses through `All`, `Any`
//! and `Not` itself, so a `Custom` condition nested anywhere in the tree
//! reaches the handler.
//!
//! # Handling Custom Conditions and Outcomes
//!
//! ```rust
//! use dramaturge_runtime::{
//!     ConditionEvaluator, CustomHandler, DefaultConditionEvaluator, DialogWorldState,
//! };
//! use dramaturge_model::{Condition, FactValue};
//!
//! struct Daytime {
//!     hour: u8,
//! }
//!
//! impl<S: ?Sized> CustomHandler<S> for Daytime {
//!     fn condition(&self, key: &str, _args: &[FactValue], _state: &S) -> bool {
//!         key == "is_daytime" && (6..18).contains(&self.hour)
//!     }
//! }
//!
//! let evaluator = DefaultConditionEvaluator::with_custom(Daytime { hour: 9 });
//! let nested = Condition::All(vec![
//!     Condition::Always,
//!     Condition::Custom { key: "is_daytime".into(), args: vec![] },
//! ]);
//! assert!(evaluator.evaluate(&nested, &DialogWorldState::new()));
//! ```
//!
//! For handlers that need host state, implement [`CustomHandler`] for your
//! state type and use it as `S`; see the `host_state` example.

use dramaturge_model::{Condition, EventKind, Fact, FactScope, FactValue, Outcome};

use crate::context::DialogWorldState;
use crate::state::NarrativeState;

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

/// Trait for evaluating conditions against narrative state.
///
/// `S` defaults to [`DialogWorldState`], so `impl ConditionEvaluator for X`
/// evaluates against the built-in state.
pub trait ConditionEvaluator<S: NarrativeState + ?Sized = DialogWorldState> {
    /// Evaluates a condition against the given state.
    ///
    /// Returns `true` if the condition is satisfied, `false` otherwise.
    fn evaluate(&self, condition: &Condition, state: &S) -> bool;
}

/// Trait for applying outcomes to narrative state.
///
/// `S` defaults to [`DialogWorldState`].
pub trait OutcomeApplier<S: NarrativeState + ?Sized = DialogWorldState> {
    /// Applies an outcome to the given state, mutating it.
    fn apply(&mut self, outcome: &Outcome, state: &mut S);
}

/// Host hook for [`Condition::Custom`] and [`Outcome::Custom`].
///
/// Both methods have defaults: unknown custom conditions are `false` and
/// unknown custom outcomes do nothing.
pub trait CustomHandler<S: ?Sized> {
    /// Answers a custom condition.
    fn condition(&self, key: &str, args: &[FactValue], state: &S) -> bool {
        let _ = (key, args, state);
        false
    }

    /// Applies a custom outcome.
    fn outcome(&mut self, key: &str, args: &[FactValue], state: &mut S) {
        let _ = (key, args, state);
    }
}

/// The default [`CustomHandler`]: every custom condition is `false`, every
/// custom outcome is a no-op.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoCustom;

impl<S: ?Sized> CustomHandler<S> for NoCustom {}

/// Reference implementation handling all built-in condition variants.
///
/// This evaluator provides the default semantics for each condition:
/// `Always`/`Never` return constant booleans, `All`/`Any`/`Not` perform
/// logical composition, fact checks query the state, and `Custom` is
/// answered by the [`CustomHandler`] `H` (by default [`NoCustom`], which
/// returns `false`).
#[derive(Debug, Clone, Default)]
pub struct DefaultConditionEvaluator<H = NoCustom> {
    /// Handler for `Condition::Custom`.
    pub custom: H,
}

impl DefaultConditionEvaluator {
    /// Creates an evaluator that answers every custom condition with `false`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<H> DefaultConditionEvaluator<H> {
    /// Creates an evaluator that hands custom conditions to `custom`.
    pub fn with_custom(custom: H) -> Self {
        Self { custom }
    }
}

impl<S, H> ConditionEvaluator<S> for DefaultConditionEvaluator<H>
where
    S: NarrativeState + ?Sized,
    H: CustomHandler<S>,
{
    fn evaluate(&self, condition: &Condition, state: &S) -> bool {
        match condition {
            Condition::Always => true,
            Condition::Never => false,
            Condition::All(conds) => conds.iter().all(|c| self.evaluate(c, state)),
            Condition::Any(conds) => conds.iter().any(|c| self.evaluate(c, state)),
            Condition::Not(c) => !self.evaluate(c, state),
            Condition::FactEquals {
                scope,
                fact_id,
                value,
            } => match state.fact(scope, fact_id) {
                Some(existing) => partial_eq_values(existing, value),
                None => false,
            },
            Condition::FactExists { scope, fact_id } => state.fact(scope, fact_id).is_some(),
            Condition::AlignmentAtLeast { axis, pole, value } => state
                .alignment(axis, pole)
                .map(|v| v >= *value)
                .unwrap_or(false),
            Condition::RelationshipAtLeast {
                source,
                target,
                axis,
                value,
            } => state
                .relationship(*source, *target, axis)
                .map(|v| v >= *value)
                .unwrap_or(false),
            Condition::AlignmentAtMost { axis, pole, value } => state
                .alignment(axis, pole)
                .map(|v| v <= *value)
                .unwrap_or(false),
            Condition::RelationshipAtMost {
                source,
                target,
                axis,
                value,
            } => state
                .relationship(*source, *target, axis)
                .map(|v| v <= *value)
                .unwrap_or(false),
            Condition::FactAtLeast {
                scope,
                fact_id,
                value,
            } => state
                .fact(scope, fact_id)
                .map(|v| matches!(v, FactValue::Int(v) if *v >= *value))
                .unwrap_or(false),
            Condition::FactAtMost {
                scope,
                fact_id,
                value,
            } => state
                .fact(scope, fact_id)
                .map(|v| matches!(v, FactValue::Int(v) if *v <= *value))
                .unwrap_or(false),
            Condition::HasMemory { owner, query } => state.matches_memory_query(*owner, query),
            Condition::HasTagAvailable { tag } => state.has_tag_available(tag),
            Condition::Custom { key, args } => self.custom.condition(key, args, state),
        }
    }
}

/// Compares two [`FactValue`]s for equality, using epsilon comparison for floats.
///
/// This exists because `FactValue::Float(f64)` prevents `Eq` derivation
/// on the type. Float comparison uses `(*x - *y).abs() < f64::EPSILON`
/// to handle floating-point precision issues in game state evaluation.
pub fn partial_eq_values(a: &FactValue, b: &FactValue) -> bool {
    match (a, b) {
        (FactValue::Bool(x), FactValue::Bool(y)) => x == y,
        (FactValue::Int(x), FactValue::Int(y)) => x == y,
        (FactValue::Float(x), FactValue::Float(y)) => (*x - *y).abs() < f64::EPSILON,
        (FactValue::Text(x), FactValue::Text(y)) => x == y,
        (FactValue::Entity(x), FactValue::Entity(y)) => x == y,
        (FactValue::Set(xs), FactValue::Set(ys)) => {
            xs.len() == ys.len()
                && xs
                    .iter()
                    .zip(ys.iter())
                    .all(|(x, y)| partial_eq_values(x, y))
        }
        _ => false,
    }
}

/// Reference implementation handling all built-in outcome variants.
///
/// This applier provides the default semantics for each outcome:
/// fact operations mutate the state directly, alignment and
/// relationship adjustments delegate to the state methods,
/// quest management uses fact-based placeholders, and `Custom` goes to
/// the [`CustomHandler`] `H` (by default [`NoCustom`], a no-op).
#[derive(Debug, Clone, Default)]
pub struct DefaultOutcomeApplier<H = NoCustom> {
    /// Handler for `Outcome::Custom`.
    pub custom: H,
}

impl DefaultOutcomeApplier {
    /// Creates an applier that ignores custom outcomes.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<H> DefaultOutcomeApplier<H> {
    /// Creates an applier that hands custom outcomes to `custom`.
    pub fn with_custom(custom: H) -> Self {
        Self { custom }
    }
}

impl<S, H> OutcomeApplier<S> for DefaultOutcomeApplier<H>
where
    S: NarrativeState + ?Sized,
    H: CustomHandler<S>,
{
    fn apply(&mut self, outcome: &Outcome, state: &mut S) {
        match outcome {
            Outcome::SetFact { scope, fact } => {
                state.set_fact(scope.clone(), fact.clone());
                state.emit_event(EventKind::FactLearned {
                    scope: scope.clone(),
                    fact_id: fact.id.clone(),
                });
            }
            Outcome::RemoveFact { scope, fact_id } => {
                state.remove_fact(scope, fact_id);
            }
            Outcome::AddAlignment { axis, pole, delta } => {
                state.add_alignment(axis.clone(), pole.clone(), *delta);
                state.emit_event(EventKind::AlignmentChanged {
                    axis: axis.clone(),
                    pole: pole.clone(),
                    delta: *delta,
                });
            }
            Outcome::AddRelationship {
                source,
                target,
                axis,
                delta,
            } => {
                state.add_relationship(*source, *target, axis.clone(), *delta);
                state.emit_event(EventKind::RelationshipChanged {
                    a: *source,
                    b: *target,
                    axis: axis.clone(),
                    delta: *delta,
                });
            }
            Outcome::AddMemory { owner, memory } => {
                state.add_memory(*owner, memory.clone());
            }
            Outcome::RemoveMemory { owner, memory_id } => {
                state.remove_memory(*owner, *memory_id);
            }
            Outcome::EmitEvent { kind } => {
                state.emit_event(kind.clone());
            }
            Outcome::StartQuest { quest_id } => {
                state.set_fact(
                    FactScope::World,
                    Fact::bool(format!("quest_started_{quest_id}"), true),
                );
                state.emit_event(EventKind::Custom {
                    kind: format!("quest_started_{quest_id}").into(),
                });
            }
            Outcome::AdvanceQuest { quest_id, stage } => {
                state.set_fact(
                    FactScope::World,
                    Fact::text(format!("quest_stage_{quest_id}"), stage.as_str()),
                );
                state.emit_event(EventKind::Custom {
                    kind: format!("quest_advanced_{quest_id}_{stage}").into(),
                });
            }
            Outcome::Custom { key, args } => self.custom.outcome(key, args, state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;
    use dramaturge_model::alignment::{AlignmentAxisId, AlignmentPoleId};
    use dramaturge_model::character::EntityId;
    use dramaturge_model::{Fact, FactId, FactScope, FactValue, RelationshipAxisId};

    #[test]
    fn evaluation_result_variants() {
        check!(EvaluationResult::Ok == EvaluationResult::Ok);
        check!(EvaluationResult::Ok != EvaluationResult::Err("oops".into()));
    }

    #[test]
    fn default_evaluator_always() {
        let evaluator = DefaultConditionEvaluator::new();
        let ctx = DialogWorldState::new();
        check!(evaluator.evaluate(&Condition::Always, &ctx));
    }

    #[test]
    fn default_evaluator_never() {
        let evaluator = DefaultConditionEvaluator::new();
        let ctx = DialogWorldState::new();
        check!(!evaluator.evaluate(&Condition::Never, &ctx));
    }

    #[test]
    fn default_evaluator_all() {
        let evaluator = DefaultConditionEvaluator::new();
        let ctx = DialogWorldState::new();
        let cond = Condition::All(vec![Condition::Always, Condition::Always]);
        check!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::All(vec![Condition::Always, Condition::Never]);
        check!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_any() {
        let evaluator = DefaultConditionEvaluator::new();
        let ctx = DialogWorldState::new();
        let cond = Condition::Any(vec![Condition::Never, Condition::Always]);
        check!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::Any(vec![Condition::Never, Condition::Never]);
        check!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_not() {
        let evaluator = DefaultConditionEvaluator::new();
        let ctx = DialogWorldState::new();
        check!(!evaluator.evaluate(&Condition::Not(Box::new(Condition::Always)), &ctx));
        check!(evaluator.evaluate(&Condition::Not(Box::new(Condition::Never)), &ctx));
    }

    #[test]
    fn default_evaluator_fact_equals() {
        let evaluator = DefaultConditionEvaluator::new();
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
        let evaluator = DefaultConditionEvaluator::new();
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
        let evaluator = DefaultConditionEvaluator::new();
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
        let evaluator = DefaultConditionEvaluator::new();
        use dramaturge_model::{Memory, MemoryId, MemoryTag};
        let owner = dramaturge_model::character::EntityId::new(1);
        let mut ctx = DialogWorldState::new();

        let cond = Condition::HasMemory {
            owner,
            query: dramaturge_model::MemoryQuery::by_tag(MemoryTag::HelpedMe),
        };
        check!(!evaluator.evaluate(&cond, &ctx));

        let mut memory = Memory::new(MemoryId::new(1));
        memory.tags.push(MemoryTag::HelpedMe);
        ctx.add_memory(owner, memory);

        check!(evaluator.evaluate(&cond, &ctx));

        let cond2 = Condition::HasMemory {
            owner,
            query: dramaturge_model::MemoryQuery::by_tag(MemoryTag::BetrayedMe),
        };
        check!(!evaluator.evaluate(&cond2, &ctx));
    }

    #[test]
    fn default_evaluator_custom() {
        let evaluator = DefaultConditionEvaluator::new();
        let ctx = DialogWorldState::new();
        let cond = Condition::Custom {
            key: "custom".to_string(),
            args: vec![],
        };
        check!(!evaluator.evaluate(&cond, &ctx));
    }

    struct InRoom;

    struct Host {
        room: &'static str,
        dialog: DialogWorldState,
        log: Vec<String>,
    }

    impl crate::HasDialogWorldState for Host {
        fn dialog(&self) -> &DialogWorldState {
            &self.dialog
        }
        fn dialog_mut(&mut self) -> &mut DialogWorldState {
            &mut self.dialog
        }
    }

    impl CustomHandler<Host> for InRoom {
        fn condition(&self, key: &str, args: &[FactValue], state: &Host) -> bool {
            key == "in_room" && matches!(args, [FactValue::Text(r)] if r == state.room)
        }
        fn outcome(&mut self, key: &str, _args: &[FactValue], state: &mut Host) {
            state.log.push(key.to_string());
        }
    }

    fn in_room(room: &str) -> Condition {
        Condition::Custom {
            key: "in_room".into(),
            args: vec![FactValue::Text(room.into())],
        }
    }

    #[test]
    fn custom_handler_reached_inside_composites() {
        let mut host = Host {
            room: "shore",
            dialog: DialogWorldState::new(),
            log: vec![],
        };
        host.dialog
            .set_fact(FactScope::World, Fact::bool("lamp_lit", true));
        let evaluator = DefaultConditionEvaluator::with_custom(InRoom);
        let lamp = Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new("lamp_lit"),
        };
        check!(evaluator.evaluate(&Condition::All(vec![lamp.clone(), in_room("shore")]), &host));
        check!(!evaluator.evaluate(&Condition::All(vec![lamp, in_room("cottage")]), &host));
        check!(evaluator.evaluate(&Condition::Not(Box::new(in_room("cottage"))), &host));
        check!(evaluator.evaluate(
            &Condition::Any(vec![Condition::Never, in_room("shore")]),
            &host
        ));
    }

    #[test]
    fn custom_handler_applies_outcomes_on_host_state() {
        let mut host = Host {
            room: "shore",
            dialog: DialogWorldState::new(),
            log: vec![],
        };
        let mut applier = DefaultOutcomeApplier::with_custom(InRoom);
        applier.apply(
            &Outcome::Custom {
                key: "ring_bell".into(),
                args: vec![],
            },
            &mut host,
        );
        applier.apply(
            &Outcome::SetFact {
                scope: FactScope::World,
                fact: Fact::bool("bell_rung", true),
            },
            &mut host,
        );
        check!(host.log == vec!["ring_bell".to_string()]);
        check!(
            host.dialog
                .fact_exists(&FactScope::World, &FactId::new("bell_rung"))
        );
    }

    #[test]
    fn default_applier_set_fact() {
        let mut applier = DefaultOutcomeApplier::new();
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
        let mut applier = DefaultOutcomeApplier::new();
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
        let mut applier = DefaultOutcomeApplier::new();
        let mut ctx = DialogWorldState::new();
        let outcome = Outcome::Custom {
            key: "custom".to_string(),
            args: vec![],
        };
        applier.apply(&outcome, &mut ctx);
        check!(ctx.facts.is_empty());
    }

    #[test]
    fn default_applier_remove_memory() {
        use dramaturge_model::{Memory, MemoryId, MemoryTag};
        let mut applier = DefaultOutcomeApplier::new();
        let mut ctx = DialogWorldState::new();
        let owner = EntityId::new(1);
        let memory_id = MemoryId::new(42);
        let memory = Memory::new(memory_id).with_tag(MemoryTag::HelpedMe);
        ctx.add_memory(owner, memory);
        check!(ctx.get_memories(owner).len() == 1);

        let outcome = Outcome::RemoveMemory { owner, memory_id };
        applier.apply(&outcome, &mut ctx);
        check!(ctx.get_memories(owner).is_empty());
    }

    #[test]
    fn default_evaluator_relationship_at_least() {
        let evaluator = DefaultConditionEvaluator::new();
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
    fn default_evaluator_relationship_at_most() {
        let evaluator = DefaultConditionEvaluator::new();

        // No relationship set returns false (via unwrap_or)
        {
            let ctx = DialogWorldState::new();
            let cond = Condition::RelationshipAtMost {
                source: EntityId::new(1),
                target: EntityId::new(2),
                axis: RelationshipAxisId::new("hostility"),
                value: -20,
            };
            check!(!evaluator.evaluate(&cond, &ctx));
        }

        // Exactly at the boundary returns true
        {
            let mut ctx = DialogWorldState::new();
            let cond = Condition::RelationshipAtMost {
                source: EntityId::new(1),
                target: EntityId::new(2),
                axis: RelationshipAxisId::new("hostility"),
                value: -20,
            };
            ctx.add_relationship(
                EntityId::new(1),
                EntityId::new(2),
                RelationshipAxisId::new("hostility"),
                -20,
            );
            check!(evaluator.evaluate(&cond, &ctx));
        }

        // Below the boundary returns true
        {
            let mut ctx = DialogWorldState::new();
            let cond = Condition::RelationshipAtMost {
                source: EntityId::new(1),
                target: EntityId::new(2),
                axis: RelationshipAxisId::new("hostility"),
                value: -20,
            };
            ctx.add_relationship(
                EntityId::new(1),
                EntityId::new(2),
                RelationshipAxisId::new("hostility"),
                -50,
            );
            check!(evaluator.evaluate(&cond, &ctx));
        }

        // Above the boundary returns false
        {
            let mut ctx = DialogWorldState::new();
            let cond = Condition::RelationshipAtMost {
                source: EntityId::new(1),
                target: EntityId::new(2),
                axis: RelationshipAxisId::new("hostility"),
                value: -20,
            };
            ctx.add_relationship(
                EntityId::new(1),
                EntityId::new(2),
                RelationshipAxisId::new("hostility"),
                10,
            );
            check!(!evaluator.evaluate(&cond, &ctx));
        }
    }

    #[test]
    fn default_evaluator_alignment_at_most() {
        let evaluator = DefaultConditionEvaluator::new();

        // No alignment set returns false (via unwrap_or)
        {
            let ctx = DialogWorldState::new();
            let cond = Condition::AlignmentAtMost {
                axis: AlignmentAxisId::new("chaos"),
                pole: AlignmentPoleId::new("order"),
                value: 30,
            };
            check!(!evaluator.evaluate(&cond, &ctx));
        }

        // Exactly at the boundary returns true
        {
            let mut ctx = DialogWorldState::new();
            let cond = Condition::AlignmentAtMost {
                axis: AlignmentAxisId::new("chaos"),
                pole: AlignmentPoleId::new("order"),
                value: 30,
            };
            ctx.add_alignment(
                AlignmentAxisId::new("chaos"),
                AlignmentPoleId::new("order"),
                30,
            );
            check!(evaluator.evaluate(&cond, &ctx));
        }

        // Below the boundary returns true
        {
            let mut ctx = DialogWorldState::new();
            let cond = Condition::AlignmentAtMost {
                axis: AlignmentAxisId::new("chaos"),
                pole: AlignmentPoleId::new("order"),
                value: 30,
            };
            ctx.add_alignment(
                AlignmentAxisId::new("chaos"),
                AlignmentPoleId::new("order"),
                10,
            );
            check!(evaluator.evaluate(&cond, &ctx));
        }

        // Above the boundary returns false
        {
            let mut ctx = DialogWorldState::new();
            let cond = Condition::AlignmentAtMost {
                axis: AlignmentAxisId::new("chaos"),
                pole: AlignmentPoleId::new("order"),
                value: 30,
            };
            ctx.add_alignment(
                AlignmentAxisId::new("chaos"),
                AlignmentPoleId::new("order"),
                50,
            );
            check!(!evaluator.evaluate(&cond, &ctx));
        }
    }

    #[test]
    fn default_applier_add_relationship() {
        let mut applier = DefaultOutcomeApplier::new();
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
            (".*", any::<i32>())
                .prop_map(|(axis, value)| {
                    Condition::AlignmentAtMost {
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
            (any::<u64>(), any::<u64>(), ".*", any::<i32>())
                .prop_map(|(s, t, axis, value)| {
                    Condition::RelationshipAtMost {
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
                        query: dramaturge_model::MemoryQuery::new(),
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
            let evaluator = DefaultConditionEvaluator::new();
            prop_assert_eq!(evaluator.evaluate(&not_all, &ctx), evaluator.evaluate(&any_not, &ctx));
        });
    }

    #[test]
    fn prop_de_morgan_not_any() {
        proptest!(|(a in arb_condition_shrink(), b in arb_condition_shrink())| {
            let ctx = DialogWorldState::new();
            let not_any = Condition::Not(Box::new(Condition::Any(vec![a.clone(), b.clone()])));
            let all_not = Condition::All(vec![Condition::Not(Box::new(a)), Condition::Not(Box::new(b))]);
            let evaluator = DefaultConditionEvaluator::new();
            prop_assert_eq!(evaluator.evaluate(&not_any, &ctx), evaluator.evaluate(&all_not, &ctx));
        });
    }

    #[test]
    fn prop_all_single_element() {
        proptest!(|(c in arb_condition_shrink())| {
            let ctx = DialogWorldState::new();
            let wrapped = Condition::All(vec![c.clone()]);
            let evaluator = DefaultConditionEvaluator::new();
            prop_assert_eq!(evaluator.evaluate(&wrapped, &ctx), evaluator.evaluate(&c, &ctx));
        });
    }

    #[test]
    fn prop_any_single_element() {
        proptest!(|(c in arb_condition_shrink())| {
            let ctx = DialogWorldState::new();
            let wrapped = Condition::Any(vec![c.clone()]);
            let evaluator = DefaultConditionEvaluator::new();
            prop_assert_eq!(evaluator.evaluate(&wrapped, &ctx), evaluator.evaluate(&c, &ctx));
        });
    }

    #[test]
    fn prop_all_empty_true() {
        let ctx = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator::new();
        assert!(evaluator.evaluate(&Condition::All(vec![]), &ctx));
    }

    #[test]
    fn prop_any_empty_false() {
        let ctx = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator::new();
        assert!(!evaluator.evaluate(&Condition::Any(vec![]), &ctx));
    }

    #[test]
    fn prop_double_negation() {
        proptest!(|(c in arb_condition_shrink())| {
            let ctx = DialogWorldState::new();
            let double = Condition::Not(Box::new(Condition::Not(Box::new(c.clone()))));
            let evaluator = DefaultConditionEvaluator::new();
            prop_assert_eq!(evaluator.evaluate(&double, &ctx), evaluator.evaluate(&c, &ctx));
        });
    }

    #[test]
    fn prop_not_always_is_never() {
        let ctx = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator::new();
        let not_always = Condition::Not(Box::new(Condition::Always));
        assert_eq!(
            evaluator.evaluate(&not_always, &ctx),
            evaluator.evaluate(&Condition::Never, &ctx)
        );
    }

    #[test]
    fn prop_not_never_is_always() {
        let ctx = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator::new();
        let not_never = Condition::Not(Box::new(Condition::Never));
        assert_eq!(
            evaluator.evaluate(&not_never, &ctx),
            evaluator.evaluate(&Condition::Always, &ctx)
        );
    }
}
