//! Tag types for annotating dialogue options.

/// A tag that annotates a [`DialogueOption`](crate::DialogueOption).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OptionTag(pub String);

impl OptionTag {
    /// Create a new [`OptionTag`] from any string-like value.
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_tag_equality() {
        assert_eq!(OptionTag::new("aggressive"), OptionTag::new("aggressive"));
        assert_ne!(OptionTag::new("aggressive"), OptionTag::new("friendly"));
    }
}
