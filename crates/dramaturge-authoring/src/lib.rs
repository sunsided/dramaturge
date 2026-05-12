//! Authoring-time validation helpers for `dramaturge` dialogue graphs.
//!
//! Use the [`GraphValidator`] to detect structural issues in a
//! [`DialogueGraph`](dramaturge_model::DialogueGraph) before shipping it
//! to the runtime.

use dramaturge_model::DialogueGraph;

/// The severity level of a [`ValidationIssue`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationSeverity {
    /// Informational note; no action required.
    Info,
    /// A potential problem that should be reviewed.
    Warning,
    /// A structural error that must be resolved before use.
    Error,
}

/// A single issue found during graph validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationIssue {
    /// The severity of this issue.
    pub severity: ValidationSeverity,
    /// A human-readable message describing the issue.
    pub message: String,
}

impl ValidationIssue {
    /// Create a new [`ValidationIssue`].
    pub fn new(severity: ValidationSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
        }
    }
}

/// Validates a [`DialogueGraph`] for structural correctness.
///
/// In the current scaffold this validator always returns an empty issue list.
/// Validation rules will be added in follow-up iterations.
#[derive(Debug, Clone, Default)]
pub struct GraphValidator;

impl GraphValidator {
    /// Create a new [`GraphValidator`].
    pub fn new() -> Self {
        Self
    }

    /// Validate `graph` and return any issues found.
    pub fn validate(&self, _graph: &DialogueGraph) -> Vec<ValidationIssue> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_returns_empty_for_empty_graph() {
        let validator = GraphValidator::new();
        let graph = DialogueGraph::default();
        let issues = validator.validate(&graph);
        assert!(issues.is_empty());
    }

    #[test]
    fn validation_issue_construction() {
        let issue = ValidationIssue::new(ValidationSeverity::Warning, "orphaned node");
        assert_eq!(issue.severity, ValidationSeverity::Warning);
        assert_eq!(issue.message, "orphaned node");
    }
}
