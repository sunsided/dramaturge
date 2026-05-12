//! The `DialogueRuntime` trait.

use dramaturge_model::DialogueGraph;

use crate::{EvaluationContext, EvaluationResult};

/// The main runtime interface for driving a dialogue session.
///
/// Concrete implementations will handle node traversal, condition evaluation,
/// and outcome application. This trait is a placeholder; the full API surface
/// will be defined in follow-up iterations.
pub trait DialogueRuntime {
    /// Begin executing `graph` starting from its first node.
    ///
    /// Returns [`EvaluationResult::Ok`] on success or
    /// [`EvaluationResult::Err`] with a reason on failure.
    fn start(&mut self, graph: &DialogueGraph, context: &mut EvaluationContext)
    -> EvaluationResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopRuntime;

    impl DialogueRuntime for NoopRuntime {
        fn start(
            &mut self,
            _graph: &DialogueGraph,
            _context: &mut EvaluationContext,
        ) -> EvaluationResult {
            EvaluationResult::Ok
        }
    }

    #[test]
    fn noop_runtime() {
        let mut rt = NoopRuntime;
        let graph = DialogueGraph::default();
        let mut ctx = EvaluationContext::new();
        assert_eq!(rt.start(&graph, &mut ctx), EvaluationResult::Ok);
    }
}
