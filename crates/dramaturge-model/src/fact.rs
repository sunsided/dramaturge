use std::sync::Arc;

use crate::character::{CharacterId, EntityId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FactId(pub Arc<str>);

impl FactId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FactValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Entity(EntityId),
    Set(Vec<FactValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    pub id: FactId,
    pub value: FactValue,
}

impl Fact {
    pub fn bool(id: impl Into<Arc<str>>, value: bool) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Bool(value),
        }
    }

    pub fn int(id: impl Into<Arc<str>>, value: i64) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Int(value),
        }
    }

    pub fn float(id: impl Into<Arc<str>>, value: f64) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Float(value),
        }
    }

    pub fn text(id: impl Into<Arc<str>>, value: impl Into<String>) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Text(value.into()),
        }
    }

    pub fn entity(id: impl Into<Arc<str>>, entity_id: EntityId) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Entity(entity_id),
        }
    }

    pub fn set(id: impl Into<Arc<str>>, values: Vec<FactValue>) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Set(values),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FactScope {
    World,
    Player,
    Character(CharacterId),
    Faction(Arc<str>),
    Party,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScopedFact {
    pub scope: FactScope,
    pub fact: Fact,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fact_id_construction() {
        let id = FactId::new("met_hero");
        assert_eq!(&*id.0, "met_hero");
    }

    #[test]
    fn fact_bool_construction() {
        let f = Fact::bool("met_hero", true);
        assert_eq!(&*f.id.0, "met_hero");
        assert_eq!(f.value, FactValue::Bool(true));
    }

    #[test]
    fn fact_int_construction() {
        let f = Fact::int("gold", 100);
        assert_eq!(f.value, FactValue::Int(100));
    }

    #[test]
    fn fact_float_construction() {
        let f = Fact::float("health", 85.5);
        assert_eq!(f.value, FactValue::Float(85.5));
    }

    #[test]
    fn fact_text_construction() {
        let f = Fact::text("name", "Elara");
        assert_eq!(f.value, FactValue::Text("Elara".to_string()));
    }

    #[test]
    fn fact_entity_construction() {
        let eid = EntityId::new(1);
        let f = Fact::entity("companion", eid);
        assert_eq!(f.value, FactValue::Entity(eid));
    }

    #[test]
    fn fact_set_construction() {
        let f = Fact::set("items", vec![FactValue::Int(1), FactValue::Int(2)]);
        assert!(matches!(f.value, FactValue::Set(_)));
    }

    #[test]
    fn fact_scope_variants() {
        let world = FactScope::World;
        let player = FactScope::Player;
        let party = FactScope::Party;
        let faction = FactScope::Faction(Arc::from("rebels"));
        assert!(matches!(world, FactScope::World));
        assert!(matches!(player, FactScope::Player));
        assert!(matches!(party, FactScope::Party));
        assert!(matches!(faction, FactScope::Faction(_)));
    }

    #[test]
    fn scoped_fact_construction() {
        let fact = Fact::bool("met_hero", true);
        let scoped = ScopedFact {
            scope: FactScope::World,
            fact,
        };
        assert!(matches!(scoped.scope, FactScope::World));
    }
}
