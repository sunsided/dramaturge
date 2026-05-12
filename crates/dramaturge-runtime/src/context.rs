//! Evaluation context passed to runtime trait implementations.

use dramaturge_model::Fact;

/// The context in which conditions and outcomes are evaluated.
///
/// Holds the collection of facts currently known to the runtime. Further
/// state (e.g. active character memories, scene information) will be added
/// in follow-up iterations.
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    /// Facts available during evaluation.
    pub facts: Vec<Fact>,
}

impl EvaluationContext {
    /// Create a new empty [`EvaluationContext`].
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_construction() {
        let ctx = EvaluationContext::new();
        assert!(ctx.facts.is_empty());
    }
}
