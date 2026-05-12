//! Composable conditions for gating dialogue options and outcomes.
//!
//! Conditions are boolean expression trees evaluated against a
//! [`DialogWorldState`](https://docs.rs/dramaturge-runtime/latest/dramaturge_runtime/struct.DialogWorldState.html).
//! They support logical operators ([`Condition::All`], [`Condition::Any`], [`Condition::Not`]),
//! fact checks ([`Condition::FactEquals`], [`Condition::FactExists`]), alignment and relationship
//! thresholds, memory queries, tag availability checks, and custom
//! extension points.
//!
//! Use the associated constructors ([`Condition::always()`],
//! [`Condition::never()`], [`Condition::all()`], [`Condition::any()`],
//! [`Condition::negate()`]) for convenience.

use crate::alignment::{AlignmentAxisId, AlignmentPoleId};
use crate::character::EntityId;
use crate::fact::{FactId, FactScope, FactValue};
use crate::memory::MemoryQuery;
use crate::relationship::RelationshipAxisId;
use crate::tag::DialogTagKind;

/// Boolean expression tree evaluated against dialogue world state.
///
/// Conditions determine whether a dialogue option is available or
/// whether certain outcomes should apply. They form a composable tree
/// that supports logical operators, fact queries, and extension points.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// Always evaluates to `true`.
    Always,
    /// Always evaluates to `false`.
    Never,
    /// Logical AND — all sub-conditions must be `true`.
    All(Vec<Condition>),
    /// Logical OR — at least one sub-condition must be `true`.
    Any(Vec<Condition>),
    /// Logical NOT — inverts the inner condition's result.
    Not(Box<Condition>),
    /// Checks that a scoped fact equals the given value.
    /// Uses epsilon comparison for float values.
    FactEquals {
        /// The scope in which to look for the fact.
        scope: FactScope,
        /// The fact identifier to check.
        fact_id: FactId,
        /// The expected value to compare against.
        value: FactValue,
    },
    /// Checks that a scoped fact exists (regardless of its value).
    FactExists {
        /// The scope in which to look for the fact.
        scope: FactScope,
        /// The fact identifier to check.
        fact_id: FactId,
    },
    /// Checks that an alignment score is at least the given value.
    AlignmentAtLeast {
        /// The alignment axis to check.
        axis: AlignmentAxisId,
        /// The pole on the axis to check.
        pole: AlignmentPoleId,
        /// The minimum score required (returns `true` if score >= value).
        value: i32,
    },
    /// Checks that a relationship score is at least the given value.
    RelationshipAtLeast {
        /// The source entity in the relationship.
        source: EntityId,
        /// The target entity in the relationship.
        target: EntityId,
        /// The relationship axis to check.
        axis: RelationshipAxisId,
        /// The minimum score required (returns `true` if score >= value).
        value: i32,
    },
    /// Checks that an entity has at least one memory matching the query.
    HasMemory {
        /// The entity whose memories to check.
        owner: EntityId,
        /// The query to match against the entity's memories.
        query: MemoryQuery,
    },
    /// Checks that a dialog tag is available for use.
    /// Currently always returns `true` in the default evaluator.
    HasTagAvailable {
        /// The tag to check availability for.
        tag: DialogTagKind,
    },
    /// User-defined custom condition with arbitrary key and arguments.
    /// Always returns `false` in the default evaluator.
    Custom {
        /// Custom condition identifier.
        key: String,
        /// Arbitrary fact value arguments for the condition.
        args: Vec<FactValue>,
    },
}

impl Condition {
    /// Returns the always-true condition.
    pub fn always() -> Self {
        Self::Always
    }

    /// Returns the always-false condition.
    pub fn never() -> Self {
        Self::Never
    }

    /// Creates a conjunction (AND) of the given conditions.
    pub fn all(conditions: Vec<Condition>) -> Self {
        Self::All(conditions)
    }

    /// Creates a disjunction (OR) of the given conditions.
    pub fn any(conditions: Vec<Condition>) -> Self {
        Self::Any(conditions)
    }

    /// Wraps the given condition in logical NOT.
    ///
    /// Note: This method was renamed from `not()` to `negate()` to avoid
    /// clippy's `should_implement_trait` lint.
    pub fn negate(condition: Condition) -> Self {
        Self::Not(Box::new(condition))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_always() {
        assert!(matches!(Condition::always(), Condition::Always));
    }

    #[test]
    fn condition_never() {
        assert!(matches!(Condition::never(), Condition::Never));
    }

    #[test]
    fn condition_all() {
        let c = Condition::all(vec![Condition::Always, Condition::Never]);
        assert!(matches!(c, Condition::All(_)));
    }

    #[test]
    fn condition_any() {
        let c = Condition::any(vec![Condition::Always, Condition::Never]);
        assert!(matches!(c, Condition::Any(_)));
    }

    #[test]
    fn condition_not() {
        let c = Condition::negate(Condition::Always);
        assert!(matches!(c, Condition::Not(_)));
    }

    #[test]
    fn condition_fact_equals() {
        let c = Condition::FactEquals {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
            value: FactValue::Bool(true),
        };
        assert!(matches!(c, Condition::FactEquals { .. }));
    }

    #[test]
    fn condition_fact_exists() {
        let c = Condition::FactExists {
            scope: FactScope::Player,
            fact_id: FactId::new("has_sword"),
        };
        assert!(matches!(c, Condition::FactExists { .. }));
    }

    #[test]
    fn condition_alignment_at_least() {
        let c = Condition::AlignmentAtLeast {
            axis: AlignmentAxisId::new("moral"),
            pole: AlignmentPoleId::new("paragon"),
            value: 50,
        };
        assert!(matches!(c, Condition::AlignmentAtLeast { .. }));
    }

    #[test]
    fn condition_relationship_at_least() {
        let c = Condition::RelationshipAtLeast {
            source: EntityId::new(1),
            target: EntityId::new(2),
            axis: RelationshipAxisId::new("trust"),
            value: 30,
        };
        assert!(matches!(c, Condition::RelationshipAtLeast { .. }));
    }

    #[test]
    fn condition_custom() {
        let c = Condition::Custom {
            key: "has_achievement".to_string(),
            args: vec![],
        };
        assert!(matches!(c, Condition::Custom { .. }));
    }
}
