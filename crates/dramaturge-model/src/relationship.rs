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

    #[test]
    fn relationship_axis_construction() {
        let id = RelationshipAxisId::new("trust");
        assert_eq!(&*id.0, "trust");
    }

    #[test]
    fn relationship_score_construction() {
        let score = RelationshipScore {
            source: EntityId::new(1),
            target: EntityId::new(2),
            axis: RelationshipAxisId::new("trust"),
            value: 50,
        };
        assert_eq!(score.source.0, 1);
        assert_eq!(score.target.0, 2);
        assert_eq!(score.value, 50);
    }
}
