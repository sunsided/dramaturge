//! Facts — discrete pieces of information with rich value types and scoping.
//!
//! Facts are the fundamental unit of knowledge in the dramaturge system.
//! Each fact has a typed [`FactValue`] and can be scoped to different
//! knowledge domains via [`FactScope`], enabling asymmetric knowledge
//! scenarios where different characters believe different things.
//!
//! Note: [`FactValue`] contains `Float(f64)`, which prevents `Eq` and
//! `Hash` derivation. Types transitively containing `FactValue` (i.e.,
//! `Fact`, `ScopedFact`, `Condition`, `Outcome`, `DialogueNode`, etc.)
//! only derive `Debug, Clone, PartialEq`. Float comparison uses epsilon
//! (`f64::EPSILON`) in the evaluation layer.
//!
//! # Multi-Game Examples
//!
//! **Pokémon** — friendship fact scoped to the player:
//!
//! ```rust
//! use dramaturge_model::{Fact, FactScope};
//!
//! // Track Pikachu's friendship score
//! let friendship = Fact::int("friendship:pikachu", 120);
//! // Scoped to the player — other characters don't see it
//! let scope = FactScope::Player;
//! ```
//!
//! **Mass Effect** — global mission completion flag:
//!
//! ```rust
//! use dramaturge_model::{Fact, FactScope};
//!
//! // Objective truth — every character knows the mission is done
//! let mission_done = Fact::bool("mission:eden_prime_complete", true);
//! let scope = FactScope::World;
//! ```
//!
//! **Asymmetric knowledge** — only Mara knows about the hidden shipment:
//!
//! ```rust
//! use dramaturge_model::{Character, EntityId, CharacterId, Fact, FactScope};
//!
//! let mara_id = CharacterId::new(EntityId::new(2));
//! let scope = FactScope::Character(mara_id);
//! let fact = Fact::bool("knows_about_hidden_shipment", true);
//! // The player and other characters do not see this fact
//! ```

use std::sync::Arc;

use crate::character::{CharacterId, EntityId};

/// String-based identifier for a fact.
///
/// Uses `Arc<str>` internally for cheap cloning.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FactId(pub Arc<str>);

impl FactId {
    /// Creates a new fact identifier from the given string-like value.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// The value stored in a fact.
///
/// Supports boolean, integer, floating-point, text, entity references,
/// and nested sets. Note that `Float(f64)` prevents `Eq`/`Hash` derivation
/// on this type and all types that contain it transitively.
/// Float equality is handled with epsilon comparison in the evaluation layer.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FactValue {
    /// Boolean flag.
    Bool(bool),
    /// 64-bit signed integer.
    Int(i64),
    /// 64-bit floating point; compared with epsilon for equality.
    Float(f64),
    /// UTF-8 string.
    Text(String),
    /// Reference to another entity by [`EntityId`].
    Entity(EntityId),
    /// Ordered collection of fact values.
    Set(Vec<FactValue>),
}

/// A single named piece of information with a typed value.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fact {
    /// Unique fact identifier.
    pub id: FactId,
    /// The fact's current value.
    pub value: FactValue,
}

impl Fact {
    /// Creates a boolean fact with the given ID and value.
    pub fn bool(id: impl Into<Arc<str>>, value: bool) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Bool(value),
        }
    }

    /// Creates an integer fact with the given ID and value.
    pub fn int(id: impl Into<Arc<str>>, value: i64) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Int(value),
        }
    }

    /// Creates a floating-point fact with the given ID and value.
    pub fn float(id: impl Into<Arc<str>>, value: f64) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Float(value),
        }
    }

    /// Creates a text fact with the given ID and value.
    pub fn text(id: impl Into<Arc<str>>, value: impl Into<String>) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Text(value.into()),
        }
    }

    /// Creates an entity-reference fact with the given ID and target entity.
    pub fn entity(id: impl Into<Arc<str>>, entity_id: EntityId) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Entity(entity_id),
        }
    }

    /// Creates a set fact with the given ID and collection of values.
    pub fn set(id: impl Into<Arc<str>>, values: Vec<FactValue>) -> Self {
        Self {
            id: FactId::new(id),
            value: FactValue::Set(values),
        }
    }
}

/// Where a fact is known or believed.
///
/// Enables asymmetric knowledge scenarios — for example, the player may
/// know a secret that a character does not, or a faction may have
/// information not shared with the world.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FactScope {
    /// Fact known globally (objective truth).
    World,
    /// Fact known to the player character.
    Player,
    /// Fact known to a specific character.
    Character(CharacterId),
    /// Fact known to a faction (identified by string key).
    Faction(Arc<str>),
    /// Fact known to the current party group.
    Party,
}

/// A fact bound to a specific [`FactScope`].
///
/// Combines a fact with the scope in which it applies, enabling
/// queries like "what does character X know about fact Y?"
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScopedFact {
    /// Where this fact applies.
    pub scope: FactScope,
    /// The fact data.
    pub fact: Fact,
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn fact_id_construction() {
        let id = FactId::new("met_hero");
        check!(&*id.0 == "met_hero");
    }

    #[test]
    fn fact_bool_construction() {
        let f = Fact::bool("met_hero", true);
        check!(&*f.id.0 == "met_hero");
        check!(f.value == FactValue::Bool(true));
    }

    #[test]
    fn fact_int_construction() {
        let f = Fact::int("gold", 100);
        check!(f.value == FactValue::Int(100));
    }

    #[test]
    fn fact_float_construction() {
        let f = Fact::float("health", 85.5);
        check!(f.value == FactValue::Float(85.5));
    }

    #[test]
    fn fact_text_construction() {
        let f = Fact::text("name", "Elara");
        check!(f.value == FactValue::Text("Elara".to_string()));
    }

    #[test]
    fn fact_entity_construction() {
        let eid = EntityId::new(1);
        let f = Fact::entity("companion", eid);
        check!(f.value == FactValue::Entity(eid));
    }

    #[test]
    fn fact_set_construction() {
        let f = Fact::set("items", vec![FactValue::Int(1), FactValue::Int(2)]);
        check!(let FactValue::Set(_) = f.value);
    }

    #[test]
    fn fact_scope_variants() {
        let world = FactScope::World;
        let player = FactScope::Player;
        let party = FactScope::Party;
        let faction = FactScope::Faction(Arc::from("rebels"));
        check!(let FactScope::World = world);
        check!(let FactScope::Player = player);
        check!(let FactScope::Party = party);
        check!(let FactScope::Faction(_) = faction);
    }

    #[test]
    fn scoped_fact_construction() {
        let fact = Fact::bool("met_hero", true);
        let scoped = ScopedFact {
            scope: FactScope::World,
            fact,
        };
        check!(let FactScope::World = scoped.scope);
    }
}
