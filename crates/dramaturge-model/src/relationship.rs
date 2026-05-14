//! Inter-entity relationship axes and scores.
//!
//! This module defines types for tracking relationships between entities.
//! Each relationship is directional (from a source to a target) and scored
//! along a named axis (e.g., "trust", "romance").

use std::sync::Arc;

use crate::character::EntityId;

/// Identifier for a relationship axis (e.g., `"trust"`, `"romance"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
}
