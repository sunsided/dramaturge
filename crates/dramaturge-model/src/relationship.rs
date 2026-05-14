//! Inter-entity relationship axes and scores.
//!
//! This module defines types for tracking relationships between entities.
//! Each relationship is directional (from a source to a target) and scored
//! along a named axis (e.g., "trust", "romance").

use std::sync::Arc;

use crate::character::EntityId;

/// Identifier for a relationship axis (e.g., `"trust"`, `"romance"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RelationshipAxisId(pub Arc<str>);

impl RelationshipAxisId {
    /// Creates a new relationship axis identifier from the given string-like value.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// Definition of a relationship axis.
///
/// An axis represents a dimension along which relationships are measured,
/// such as trust, romance, or rivalry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RelationshipAxis {
    /// The unique identifier for this axis.
    pub id: RelationshipAxisId,
    /// Human-facing axis name (e.g., "Trust").
    pub display_name: Arc<str>,
}

impl RelationshipAxis {
    /// Creates a new relationship axis with the given ID and display name.
    pub fn new(id: impl Into<Arc<str>>, display_name: impl Into<Arc<str>>) -> Self {
        Self {
            id: RelationshipAxisId::new(id),
            display_name: display_name.into(),
        }
    }
}

/// Numeric score between two entities on a specific relationship axis.
///
/// Relationships are directional: `source` is the entity holding the
/// relationship toward `target`. A positive value indicates a favorable
/// attitude, while a negative value indicates an unfavorable one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RelationshipScore {
    /// Entity initiating or holding the relationship.
    pub source: EntityId,
    /// Entity the relationship is directed toward.
    pub target: EntityId,
    /// Which relationship axis is scored (e.g., `"trust"`).
    pub axis: RelationshipAxisId,
    /// Numeric score; positive = favorable, negative = unfavorable.
    pub value: i32,
}

impl RelationshipScore {
    /// Creates a new relationship score.
    pub fn new(source: EntityId, target: EntityId, axis: RelationshipAxisId, value: i32) -> Self {
        Self {
            source,
            target,
            axis,
            value,
        }
    }

    /// Creates a positive relationship score (absolute value is taken).
    pub fn positive(
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        value: i32,
    ) -> Self {
        Self::new(source, target, axis, value.saturating_abs())
    }

    /// Creates a negative relationship score (absolute value is taken and negated).
    pub fn negative(
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        value: i32,
    ) -> Self {
        Self::new(source, target, axis, -(value.saturating_abs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn relationship_axis_construction() {
        let id = RelationshipAxisId::new("trust");
        check!(&*id.0 == "trust");
    }

    #[test]
    fn relationship_score_construction() {
        let score = RelationshipScore {
            source: EntityId::new(1),
            target: EntityId::new(2),
            axis: RelationshipAxisId::new("trust"),
            value: 50,
        };
        check!(score.source.0 == 1);
        check!(score.target.0 == 2);
        check!(score.value == 50);
    }

    #[test]
    fn relationship_axis_full_construction() {
        let axis = RelationshipAxis::new("trust", "Trust");
        check!(&*axis.id.0 == "trust");
        check!(&*axis.display_name == "Trust");
    }

    #[test]
    fn relationship_score_new() {
        let score = RelationshipScore::new(
            EntityId::new(1),
            EntityId::new(2),
            RelationshipAxisId::new("trust"),
            50,
        );
        check!(score.source.0 == 1);
        check!(score.target.0 == 2);
        check!(&*score.axis.0 == "trust");
        check!(score.value == 50);
    }

    #[test]
    fn relationship_score_positive() {
        let score = RelationshipScore::positive(
            EntityId::new(1),
            EntityId::new(2),
            RelationshipAxisId::new("trust"),
            50,
        );
        check!(score.value == 50);
        let score = RelationshipScore::positive(
            EntityId::new(1),
            EntityId::new(2),
            RelationshipAxisId::new("trust"),
            -50,
        );
        check!(score.value == 50);
    }

    #[test]
    fn relationship_score_negative() {
        let score = RelationshipScore::negative(
            EntityId::new(1),
            EntityId::new(2),
            RelationshipAxisId::new("trust"),
            50,
        );
        check!(score.value == -50);
        let score = RelationshipScore::negative(
            EntityId::new(1),
            EntityId::new(2),
            RelationshipAxisId::new("trust"),
            -50,
        );
        check!(score.value == -50);
    }

    #[test]
    fn relationship_score_positive_i32_min() {
        let score = RelationshipScore::positive(
            EntityId::new(1),
            EntityId::new(2),
            RelationshipAxisId::new("trust"),
            i32::MIN,
        );
        check!(score.value == i32::MAX);
    }

    #[test]
    fn relationship_score_negative_i32_min() {
        let score = RelationshipScore::negative(
            EntityId::new(1),
            EntityId::new(2),
            RelationshipAxisId::new("trust"),
            i32::MIN,
        );
        check!(score.value == i32::MIN + 1);
    }
}
