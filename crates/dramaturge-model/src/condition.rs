//! Condition type used to gate dialogue options and outcomes.

/// A condition that can be evaluated against an evaluation context.
///
/// In the current scaffold this is a plain placeholder; evaluation logic
/// will be wired through `dramaturge-runtime` in a follow-up iteration.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// A human-readable description of the condition.
    pub description: String,
}

impl Condition {
    /// Create a new [`Condition`] with the given description.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_construction() {
        let c = Condition::new("player has met the guard");
        assert_eq!(c.description, "player has met the guard");
    }
}
