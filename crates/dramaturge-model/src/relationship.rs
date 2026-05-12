use std::sync::Arc;

use crate::character::EntityId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelationshipAxisId(pub Arc<str>);

impl RelationshipAxisId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipScore {
    pub source: EntityId,
    pub target: EntityId,
    pub axis: RelationshipAxisId,
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
