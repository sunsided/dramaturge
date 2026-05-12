//! Dialog world state — the mutable game state evaluated against conditions.
//!
//! [`DialogWorldState`] is the central snapshot of the narrative world,
//! holding facts, alignments, relationships, memories, and events. It
//! is mutated by [`OutcomeApplier`](crate::OutcomeApplier) implementations
//! and queried by [`ConditionEvaluator`](crate::ConditionEvaluator)
//! implementations during dialogue evaluation.

use dramaturge_model::{AlignmentScore, EntityId, Event, Memory, RelationshipScore, ScopedFact};

/// Complete snapshot of the narrative world state.
///
/// This struct holds all mutable state that the dialogue system interacts
/// with during evaluation. Facts are scoped, alignments and relationships
/// are directional, and memories are owned by entities.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DialogWorldState {
    /// Scoped facts known in the world.
    pub facts: Vec<ScopedFact>,
    /// Alignment scores across various axes.
    pub alignments: Vec<AlignmentScore>,
    /// Directional relationship scores between entities.
    pub relationships: Vec<RelationshipScore>,
    /// Memories owned by entities (owner, memory) tuples.
    pub memories: Vec<(EntityId, Memory)>,
    /// Timeline of recorded narrative events.
    pub events: Vec<Event>,
}

/// Type alias for backward compatibility with earlier versions.
///
/// This is identical to [`DialogWorldState`] and exists to avoid
/// breaking changes in downstream code.
pub type EvaluationContext = DialogWorldState;

impl DialogWorldState {
    /// Creates an empty world state with all collections initialized to empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks up a fact by scope and ID.
    ///
    /// Returns `None` if the fact is not found in the given scope.
    pub fn get_fact(
        &self,
        scope: &dramaturge_model::FactScope,
        fact_id: &dramaturge_model::FactId,
    ) -> Option<&dramaturge_model::FactValue> {
        self.facts
            .iter()
            .find(|sf| &sf.scope == scope && sf.fact.id == *fact_id)
            .map(|sf| &sf.fact.value)
    }

    /// Inserts or replaces a scoped fact.
    ///
    /// If a fact with the same scope and ID already exists, its value is
    /// updated. Otherwise, a new scoped fact is appended.
    pub fn set_fact(&mut self, scope: dramaturge_model::FactScope, fact: dramaturge_model::Fact) {
        if let Some(sf) = self
            .facts
            .iter_mut()
            .find(|sf| sf.scope == scope && sf.fact.id == fact.id)
        {
            sf.fact = fact;
        } else {
            self.facts.push(ScopedFact { scope, fact });
        }
    }

    /// Removes a scoped fact if present.
    pub fn remove_fact(
        &mut self,
        scope: &dramaturge_model::FactScope,
        fact_id: &dramaturge_model::FactId,
    ) {
        self.facts
            .retain(|sf| !(sf.scope == *scope && sf.fact.id == *fact_id));
    }

    /// Convenience check for whether a scoped fact is present.
    pub fn fact_exists(
        &self,
        scope: &dramaturge_model::FactScope,
        fact_id: &dramaturge_model::FactId,
    ) -> bool {
        self.get_fact(scope, fact_id).is_some()
    }

    /// Returns the current alignment score, or `None` if untracked.
    pub fn get_alignment(
        &self,
        axis: &dramaturge_model::AlignmentAxisId,
        pole: &dramaturge_model::AlignmentPoleId,
    ) -> Option<i32> {
        self.alignments
            .iter()
            .find(|a| a.axis == *axis && a.pole == *pole)
            .map(|a| a.value)
    }

    /// Increments an existing alignment score, or initializes it to the
    /// given delta if not yet tracked.
    pub fn add_alignment(
        &mut self,
        axis: dramaturge_model::AlignmentAxisId,
        pole: dramaturge_model::AlignmentPoleId,
        delta: i32,
    ) {
        if let Some(a) = self
            .alignments
            .iter_mut()
            .find(|a| a.axis == axis && a.pole == pole)
        {
            a.value += delta;
        } else {
            self.alignments.push(AlignmentScore {
                axis,
                pole,
                value: delta,
            });
        }
    }

    /// Returns the relationship score, or `None` if untracked.
    pub fn get_relationship(
        &self,
        source: EntityId,
        target: EntityId,
        axis: &dramaturge_model::RelationshipAxisId,
    ) -> Option<i32> {
        self.relationships
            .iter()
            .find(|r| r.source == source && r.target == target && r.axis == *axis)
            .map(|r| r.value)
    }

    /// Increments an existing relationship score, or initializes it to
    /// the given delta if not yet tracked.
    pub fn add_relationship(
        &mut self,
        source: EntityId,
        target: EntityId,
        axis: dramaturge_model::RelationshipAxisId,
        delta: i32,
    ) {
        if let Some(r) = self
            .relationships
            .iter_mut()
            .find(|r| r.source == source && r.target == target && r.axis == axis)
        {
            r.value += delta;
        } else {
            self.relationships.push(RelationshipScore {
                source,
                target,
                axis,
                value: delta,
            });
        }
    }

    /// Returns all memories owned by the given entity.
    pub fn get_memories(&self, owner: EntityId) -> Vec<&Memory> {
        self.memories
            .iter()
            .filter(|(o, _)| *o == owner)
            .map(|(_, m)| m)
            .collect()
    }

    /// Appends a memory tuple for the given owner entity.
    pub fn add_memory(&mut self, owner: EntityId, memory: Memory) {
        self.memories.push((owner, memory));
    }

    /// Appends an event to the timeline.
    pub fn add_event(&mut self, event: Event) {
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;
    use dramaturge_model::{Fact, FactId, FactScope};

    #[test]
    fn world_state_default() {
        let state = DialogWorldState::default();
        check!(state.facts.is_empty());
        check!(state.alignments.is_empty());
        check!(state.relationships.is_empty());
        check!(state.memories.is_empty());
        check!(state.events.is_empty());
    }

    #[test]
    fn type_alias_works() {
        let _: EvaluationContext = DialogWorldState::new();
    }

    #[test]
    fn set_and_get_fact() {
        let mut state = DialogWorldState::new();
        let fact = Fact::bool("met_hero", true);
        state.set_fact(FactScope::World, fact);
        let value = state.get_fact(&FactScope::World, &FactId::new("met_hero"));
        check!(let Some(dramaturge_model::FactValue::Bool(true)) = value);
    }

    #[test]
    fn remove_fact() {
        let mut state = DialogWorldState::new();
        let fact = Fact::bool("met_hero", true);
        state.set_fact(FactScope::World, fact);
        state.remove_fact(&FactScope::World, &FactId::new("met_hero"));
        check!(
            state
                .get_fact(&FactScope::World, &FactId::new("met_hero"))
                .is_none()
        );
    }

    #[test]
    fn fact_exists() {
        let mut state = DialogWorldState::new();
        let fact = Fact::bool("met_hero", true);
        state.set_fact(FactScope::World, fact);
        check!(state.fact_exists(&FactScope::World, &FactId::new("met_hero")));
        check!(!state.fact_exists(&FactScope::World, &FactId::new("unknown")));
    }

    #[test]
    fn alignment_manipulation() {
        let mut state = DialogWorldState::new();
        use dramaturge_model::{AlignmentAxisId, AlignmentPoleId};
        state.add_alignment(
            AlignmentAxisId::new("moral"),
            AlignmentPoleId::new("paragon"),
            50,
        );
        check!(
            state.get_alignment(
                &AlignmentAxisId::new("moral"),
                &AlignmentPoleId::new("paragon")
            ) == Some(50)
        );
        state.add_alignment(
            AlignmentAxisId::new("moral"),
            AlignmentPoleId::new("paragon"),
            10,
        );
        check!(
            state.get_alignment(
                &AlignmentAxisId::new("moral"),
                &AlignmentPoleId::new("paragon")
            ) == Some(60)
        );
    }

    #[test]
    fn relationship_manipulation() {
        let mut state = DialogWorldState::new();
        use dramaturge_model::RelationshipAxisId;
        let s = EntityId::new(1);
        let t = EntityId::new(2);
        state.add_relationship(s, t, RelationshipAxisId::new("trust"), 30);
        check!(state.get_relationship(s, t, &RelationshipAxisId::new("trust")) == Some(30));
        state.add_relationship(s, t, RelationshipAxisId::new("trust"), -10);
        check!(state.get_relationship(s, t, &RelationshipAxisId::new("trust")) == Some(20));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;
    use dramaturge_model::fact::{Fact, FactScope, FactValue};
    use dramaturge_model::{
        AlignmentAxisId, AlignmentPoleId, Event, EventKind, Memory, MemoryId, RelationshipAxisId,
    };

    fn arb_entity_id() -> impl Strategy<Value = EntityId> {
        any::<u64>().prop_map(EntityId::new)
    }

    fn arb_fact_id() -> impl Strategy<Value = dramaturge_model::FactId> {
        ".*".prop_map(dramaturge_model::FactId::new)
    }

    fn arb_fact_value() -> impl Strategy<Value = FactValue> {
        prop_oneof![
            any::<bool>().prop_map(FactValue::Bool),
            any::<i64>().prop_map(FactValue::Int),
            any::<f64>().prop_map(FactValue::Float),
            ".*".prop_map(FactValue::Text),
            arb_entity_id().prop_map(FactValue::Entity),
        ]
    }

    fn arb_fact_scope() -> impl Strategy<Value = FactScope> {
        use dramaturge_model::character::CharacterId;
        prop_oneof![
            Just(FactScope::World),
            Just(FactScope::Player),
            Just(FactScope::Party),
            ".*".prop_map(|s: String| FactScope::Faction(s.into())),
            arb_entity_id().prop_map(|e| FactScope::Character(CharacterId::new(e))),
        ]
    }

    fn arb_fact() -> impl Strategy<Value = Fact> {
        (arb_fact_id(), arb_fact_value()).prop_map(|(id, value)| Fact { id, value })
    }

    fn arb_scoped_fact() -> impl Strategy<Value = dramaturge_model::ScopedFact> {
        (arb_fact_scope(), arb_fact())
            .prop_map(|(scope, fact)| dramaturge_model::ScopedFact { scope, fact })
    }

    fn arb_alignment_axis_id() -> impl Strategy<Value = AlignmentAxisId> {
        ".*".prop_map(AlignmentAxisId::new)
    }

    fn arb_alignment_pole_id() -> impl Strategy<Value = AlignmentPoleId> {
        ".*".prop_map(AlignmentPoleId::new)
    }

    fn arb_relationship_axis_id() -> impl Strategy<Value = RelationshipAxisId> {
        ".*".prop_map(RelationshipAxisId::new)
    }

    fn arb_memory() -> impl Strategy<Value = Memory> {
        any::<u64>().prop_map(|n| Memory::new(MemoryId::new(n)))
    }

    #[allow(dead_code)]
    fn arb_event() -> impl Strategy<Value = Event> {
        (any::<u64>(), ".*").prop_map(|(id, kind)| {
            let eid = dramaturge_model::EventId::new(id);
            Event::new(eid, EventKind::Custom { kind: kind.into() })
        })
    }

    #[test]
    fn prop_set_get_roundtrip() {
        proptest!(|(scoped in arb_scoped_fact())| {
            let mut ctx = DialogWorldState::new();
            ctx.set_fact(scoped.scope.clone(), scoped.fact.clone());
            let got = ctx.get_fact(&scoped.scope, &scoped.fact.id);
            prop_assert_eq!(got, Some(&scoped.fact.value));
        });
    }

    #[test]
    fn prop_remove_get_none() {
        proptest!(|(scoped in arb_scoped_fact())| {
            let mut ctx = DialogWorldState::new();
            ctx.set_fact(scoped.scope.clone(), scoped.fact.clone());
            ctx.remove_fact(&scoped.scope, &scoped.fact.id);
            prop_assert!(ctx.get_fact(&scoped.scope, &scoped.fact.id).is_none());
        });
    }

    #[test]
    fn prop_set_twice_overwrites() {
        proptest!(|(
            scoped in arb_scoped_fact(),
            new_value in arb_fact_value()
        )| {
            let mut ctx = DialogWorldState::new();
            ctx.set_fact(scoped.scope.clone(), scoped.fact.clone());
            ctx.set_fact(
                scoped.scope.clone(),
                Fact {
                    id: scoped.fact.id.clone(),
                    value: new_value.clone(),
                },
            );
            let got = ctx.get_fact(&scoped.scope, &scoped.fact.id);
            prop_assert_eq!(got, Some(&new_value));
        });
    }

    #[test]
    fn prop_alignment_accumulates() {
        proptest!(|(
            axis in arb_alignment_axis_id(),
            pole in arb_alignment_pole_id(),
            deltas in proptest::collection::vec(any::<i16>(), 1..10)
        )| {
            let mut ctx = DialogWorldState::new();
            let expected: i32 = deltas.iter().map(|d| *d as i32).sum();
            for delta in &deltas {
                ctx.add_alignment(axis.clone(), pole.clone(), *delta as i32);
            }
            let got = ctx.get_alignment(&axis, &pole);
            prop_assert_eq!(got, Some(expected));
        });
    }

    #[test]
    fn prop_relationship_directional() {
        proptest!(|(
            a in arb_entity_id(),
            b in arb_entity_id(),
            axis in arb_relationship_axis_id(),
            delta in any::<i32>()
        )| {
            prop_assume!(a != b);
            let mut ctx = DialogWorldState::new();
            ctx.add_relationship(a, b, axis.clone(), delta);
            let ab = ctx.get_relationship(a, b, &axis);
            let ba = ctx.get_relationship(b, a, &axis);
            prop_assert_eq!(ab, Some(delta));
            prop_assert_eq!(ba, None);
        });
    }

    #[test]
    fn prop_event_ids_monotonic() {
        proptest!(|(kinds in proptest::collection::vec(".*", 1..10))| {
            let mut ctx = DialogWorldState::new();
            let mut last_id: Option<u64> = None;
            for (i, kind_str) in kinds.iter().enumerate() {
                let event_id = dramaturge_model::EventId::new(i as u64 + 1);
                let event = Event::new(event_id, EventKind::Custom { kind: kind_str.as_str().into() });
                ctx.add_event(event.clone());
                prop_assert!(last_id.is_none_or(|prev| event.id.0 > prev));
                last_id = Some(event.id.0);
            }
            prop_assert_eq!(ctx.events.len(), kinds.len());
        });
    }

    #[test]
    fn prop_add_memory_visible() {
        proptest!(|(owner in arb_entity_id(), memory in arb_memory())| {
            let mut ctx = DialogWorldState::new();
            ctx.add_memory(owner, memory.clone());
            let memories = ctx.get_memories(owner);
            prop_assert!(!memories.is_empty());
            prop_assert!(memories.contains(&&memory));
        });
    }
}
