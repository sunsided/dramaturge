use dramaturge_model::{Condition, Outcome};

use crate::context::DialogWorldState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationResult {
    Ok,
    Err(String),
}

pub trait ConditionEvaluator {
    fn evaluate(&self, condition: &Condition, context: &DialogWorldState) -> bool;
}

pub trait OutcomeApplier {
    fn apply(&self, outcome: &Outcome, context: &mut DialogWorldState);
}

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
            Condition::HasMemory { owner, query: _ } => !context.get_memories(*owner).is_empty(),
            Condition::HasTagAvailable { .. } => true,
            Condition::Custom { .. } => false,
        }
    }
}

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

#[derive(Debug, Clone, Default)]
pub struct DefaultOutcomeApplier;

impl OutcomeApplier for DefaultOutcomeApplier {
    fn apply(&self, outcome: &Outcome, context: &mut DialogWorldState) {
        match outcome {
            Outcome::SetFact { scope, fact } => {
                context.set_fact(scope.clone(), fact.clone());
            }
            Outcome::RemoveFact { scope, fact_id } => {
                context.remove_fact(scope, fact_id);
            }
            Outcome::AddAlignment { axis, pole, delta } => {
                context.add_alignment(axis.clone(), pole.clone(), *delta);
            }
            Outcome::AddRelationship {
                source,
                target,
                axis,
                delta,
            } => {
                context.add_relationship(*source, *target, axis.clone(), *delta);
            }
            Outcome::AddMemory { owner, memory } => {
                context.add_memory(*owner, memory.clone());
            }
            Outcome::EmitEvent { kind } => {
                let event_id = dramaturge_model::EventId::new(
                    context.events.iter().map(|e| e.id.0).max().unwrap_or(0) + 1,
                );
                let event = dramaturge_model::Event::new(event_id, kind.clone());
                context.add_event(event);
            }
            Outcome::StartQuest { quest_id } => {
                context.set_fact(
                    dramaturge_model::FactScope::World,
                    dramaturge_model::Fact::bool(format!("quest_started_{quest_id}"), true),
                );
            }
            Outcome::AdvanceQuest { quest_id, stage } => {
                context.set_fact(
                    dramaturge_model::FactScope::World,
                    dramaturge_model::Fact::text(format!("quest_stage_{quest_id}"), stage.as_str()),
                );
            }
            Outcome::Custom { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dramaturge_model::{Fact, FactId, FactScope, FactValue};

    #[test]
    fn evaluation_result_variants() {
        assert_eq!(EvaluationResult::Ok, EvaluationResult::Ok);
        assert_ne!(EvaluationResult::Ok, EvaluationResult::Err("oops".into()));
    }

    #[test]
    fn default_evaluator_always() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        assert!(evaluator.evaluate(&Condition::Always, &ctx));
    }

    #[test]
    fn default_evaluator_never() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        assert!(!evaluator.evaluate(&Condition::Never, &ctx));
    }

    #[test]
    fn default_evaluator_all() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        let cond = Condition::All(vec![Condition::Always, Condition::Always]);
        assert!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::All(vec![Condition::Always, Condition::Never]);
        assert!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_any() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        let cond = Condition::Any(vec![Condition::Never, Condition::Always]);
        assert!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::Any(vec![Condition::Never, Condition::Never]);
        assert!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_not() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        assert!(!evaluator.evaluate(&Condition::Not(Box::new(Condition::Always)), &ctx));
        assert!(evaluator.evaluate(&Condition::Not(Box::new(Condition::Never)), &ctx));
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
        assert!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::FactEquals {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
            value: FactValue::Bool(false),
        };
        assert!(!evaluator.evaluate(&cond, &ctx));
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
        assert!(evaluator.evaluate(&cond, &ctx));
        let cond = Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new("unknown"),
        };
        assert!(!evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_has_tag_available() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        use dramaturge_model::{DialogTagKind, MoralTag};
        let cond = Condition::HasTagAvailable {
            tag: DialogTagKind::Moral(MoralTag::Paragon),
        };
        assert!(evaluator.evaluate(&cond, &ctx));
    }

    #[test]
    fn default_evaluator_custom() {
        let evaluator = DefaultConditionEvaluator;
        let ctx = DialogWorldState::new();
        let cond = Condition::Custom {
            key: "custom".to_string(),
            args: vec![],
        };
        assert!(!evaluator.evaluate(&cond, &ctx));
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
        assert!(ctx.fact_exists(&FactScope::World, &FactId::new("met_hero")));
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
        assert!(!ctx.fact_exists(&FactScope::World, &FactId::new("met_hero")));
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
        assert!(ctx.facts.is_empty());
    }
}
