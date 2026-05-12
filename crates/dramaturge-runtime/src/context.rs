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
