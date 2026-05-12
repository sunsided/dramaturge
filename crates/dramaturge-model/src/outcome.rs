//! Outcome type representing the effect of a dialogue choice.

/// The effect that occurs when a dialogue option is chosen or a node is reached.
///
/// This is a placeholder for the full outcome model which will be implemented
/// in a follow-up iteration.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// A human-readable description of this outcome.
    pub description: String,
}

impl Outcome {
    /// Create a new [`Outcome`] with the given description.
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
    fn outcome_construction() {
        let o = Outcome::new("guard becomes hostile");
        assert_eq!(o.description, "guard becomes hostile");
    }
}
