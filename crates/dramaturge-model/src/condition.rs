use crate::alignment::{AlignmentAxisId, AlignmentPoleId};
use crate::character::EntityId;
use crate::fact::{FactId, FactScope, FactValue};
use crate::memory::MemoryQuery;
use crate::relationship::RelationshipAxisId;
use crate::tag::DialogTagKind;

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Always,
    Never,
    All(Vec<Condition>),
    Any(Vec<Condition>),
    Not(Box<Condition>),
    FactEquals {
        scope: FactScope,
        fact_id: FactId,
        value: FactValue,
    },
    FactExists {
        scope: FactScope,
        fact_id: FactId,
    },
    AlignmentAtLeast {
        axis: AlignmentAxisId,
        pole: AlignmentPoleId,
        value: i32,
    },
    RelationshipAtLeast {
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        value: i32,
    },
    HasMemory {
        owner: EntityId,
        query: MemoryQuery,
    },
    HasTagAvailable {
        tag: DialogTagKind,
    },
    Custom {
        key: String,
        args: Vec<FactValue>,
    },
}

impl Condition {
    pub fn always() -> Self {
        Self::Always
    }

    pub fn never() -> Self {
        Self::Never
    }

    pub fn all(conditions: Vec<Condition>) -> Self {
        Self::All(conditions)
    }

    pub fn any(conditions: Vec<Condition>) -> Self {
        Self::Any(conditions)
    }

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
