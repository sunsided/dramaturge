//! Intent type for annotating the communicative purpose of a dialogue option.

/// The communicative intent behind a dialogue option or node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// An informational statement.
    Inform,
    /// A question.
    Question,
    /// A persuasive or coercive statement.
    Persuade,
    /// A threat.
    Threaten,
    /// A custom intent label.
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_variants() {
        assert_eq!(Intent::Inform, Intent::Inform);
        assert_ne!(Intent::Question, Intent::Threaten);
    }
}
