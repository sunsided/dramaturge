//! Fact-related domain types.

/// Identifies a [`Fact`] within a fact store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FactKey(pub String);

impl FactKey {
    /// Create a new [`FactKey`] from any string-like value.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

/// The value associated with a [`Fact`].
#[derive(Debug, Clone, PartialEq)]
pub enum FactValue {
    /// A boolean value.
    Bool(bool),
    /// An integer value.
    Int(i64),
    /// A string value.
    Text(String),
}

/// A discrete piece of information stored in the game world or character memory.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    /// The key identifying this fact.
    pub key: FactKey,
    /// The value of this fact.
    pub value: FactValue,
}

impl Fact {
    /// Create a new boolean [`Fact`].
    pub fn bool(key: impl Into<String>, value: bool) -> Self {
        Self {
            key: FactKey::new(key),
            value: FactValue::Bool(value),
        }
    }

    /// Create a new integer [`Fact`].
    pub fn int(key: impl Into<String>, value: i64) -> Self {
        Self {
            key: FactKey::new(key),
            value: FactValue::Int(value),
        }
    }

    /// Create a new text [`Fact`].
    pub fn text(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: FactKey::new(key),
            value: FactValue::Text(value.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fact_construction() {
        let f = Fact::bool("met_hero", true);
        assert_eq!(f.key, FactKey::new("met_hero"));
        assert_eq!(f.value, FactValue::Bool(true));
    }
}
