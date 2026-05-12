//! Evaluation result and evaluator/applier traits.

use dramaturge_model::{Condition, Outcome};

use crate::EvaluationContext;

/// The result of evaluating a dialogue step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationResult {
    /// Evaluation succeeded.
    Ok,
    /// Evaluation failed with a reason.
    Err(String),
}

/// Evaluates a [`Condition`] against an [`EvaluationContext`].
///
/// Implement this trait to provide custom condition-checking logic.
pub trait ConditionEvaluator {
    /// Returns `true` if `condition` is satisfied given `context`.
    fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> bool;
}

/// Applies an [`Outcome`] to an [`EvaluationContext`].
///
/// Implement this trait to perform side-effects such as setting facts or
/// triggering events when a dialogue option is chosen.
pub trait OutcomeApplier {
    /// Apply `outcome` to `context`.
    fn apply(&self, outcome: &Outcome, context: &mut EvaluationContext);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_result_variants() {
        assert_eq!(EvaluationResult::Ok, EvaluationResult::Ok);
        assert_ne!(EvaluationResult::Ok, EvaluationResult::Err("oops".into()));
    }
}
