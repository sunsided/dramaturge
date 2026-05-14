//! Dialog world state — the mutable game state evaluated against conditions.
//!
//! [`DialogWorldState`] is the central snapshot of the narrative world,
//! holding facts, alignments, relationships, memories, and events. It
//! is mutated by [`OutcomeApplier`](crate::OutcomeApplier) implementations
//! and queried by [`ConditionEvaluator`](crate::ConditionEvaluator)
//! implementations during dialogue evaluation.

use dramaturge_model::tag::DialogTagKind;
use dramaturge_model::{
    AlignmentScore, EntityId, Event, Memory, MemoryQuery, RelationshipScore, ScopedFact,
};

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
    /// Tags currently available in the world.
    pub available_tags: Vec<DialogTagKind>,
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

    /// Returns the next auto-incremented event ID based on existing events.
    pub fn next_event_id(&self) -> u64 {
        self.events
            .iter()
            .map(|e| e.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    /// Returns all events in chronological order.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Returns events matching the given kind variant name.
    ///
    /// Matches against the variant name (e.g., "DialogOptionChosen", "FactLearned").
    pub fn events_of_kind(&self, kind: &str) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.kind.variant_name() == kind)
            .collect()
    }

    /// Returns events where the actor matches.
    pub fn events_by_actor(&self, actor: EntityId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.actor == Some(actor))
            .collect()
    }

    /// Returns events where the target matches.
    pub fn events_by_target(&self, target: EntityId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.target == Some(target))
            .collect()
    }

    /// Returns the last N events.
    pub fn recent_events(&self, count: usize) -> Vec<&Event> {
        self.events.iter().rev().take(count).rev().collect()
    }

    /// Returns events matching a predicate.
    pub fn find_events<F: Fn(&Event) -> bool>(&self, predicate: F) -> Vec<&Event> {
        self.events.iter().filter(|e| predicate(e)).collect()
    }

    /// Adds a tag to the set of available tags.
    pub fn add_available_tag(&mut self, tag: DialogTagKind) {
        self.available_tags.push(tag);
    }

    /// Checks if a specific tag is available.
    pub fn has_tag_available(&self, tag: &DialogTagKind) -> bool {
        self.available_tags.contains(tag)
    }

    /// Checks if any memory owned by the given entity matches the query.
    ///
    /// The query string is matched against:
    /// - Memory tag variant names (e.g., "HelpedMe") or custom tag strings
    /// - Memory fact IDs (as string)
    /// - Memory subject entity ID (as string)
    /// - Memory about entity IDs (as string)
    pub fn matches_memory_query(&self, owner: EntityId, query: &MemoryQuery) -> bool {
        let query_str = &query.0;
        self.get_memories(owner).iter().any(|memory| {
            memory.tags.iter().any(|tag| {
                let tag_str: &str = match tag {
                    dramaturge_model::memory::MemoryTag::HelpedMe => "HelpedMe",
                    dramaturge_model::memory::MemoryTag::BetrayedMe => "BetrayedMe",
                    dramaturge_model::memory::MemoryTag::LiedToMe => "LiedToMe",
                    dramaturge_model::memory::MemoryTag::SavedLife => "SavedLife",
                    dramaturge_model::memory::MemoryTag::ThreatenedMe => "ThreatenedMe",
                    dramaturge_model::memory::MemoryTag::ShowedMercy => "ShowedMercy",
                    dramaturge_model::memory::MemoryTag::KilledSomeone => "KilledSomeone",
                    dramaturge_model::memory::MemoryTag::RevealedSecret => "RevealedSecret",
                    dramaturge_model::memory::MemoryTag::Custom(tag_str) => tag_str.as_ref(),
                };
                query_str.as_ref() == tag_str
            }) || memory
                .facts
                .iter()
                .any(|fact_id| fact_id.0.as_ref().contains(query_str.as_ref()))
                || memory.subject.is_some_and(|subject| {
                    let subject_str = format!("{}", subject.0);
                    subject_str.contains(query_str.as_ref())
                })
                || memory.about.iter().any(|entity| {
                    let entity_str = format!("{}", entity.0);
                    entity_str.contains(query_str.as_ref())
                })
        })
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

    #[test]
    fn event_query_methods() {
        use dramaturge_model::{Event, EventId, EventKind};

        let mut state = DialogWorldState::new();
        let e1 = Event::new(
            EventId::new(1),
            EventKind::DialogOptionChosen {
                node_id: dramaturge_model::DialogueNodeId::new(1),
                option_id: dramaturge_model::DialogueOptionId::new(1),
                tags: vec![],
                intentions: dramaturge_model::DialogIntentions::new(),
            },
        );
        let mut e2 = Event::new(
            EventId::new(2),
            EventKind::FactLearned {
                scope: FactScope::World,
                fact_id: FactId::new("met_hero"),
            },
        );
        e2.actor = Some(EntityId::new(42));

        let mut e3 = Event::new(
            EventId::new(3),
            EventKind::RelationshipChanged {
                a: EntityId::new(1),
                b: EntityId::new(2),
                axis: dramaturge_model::RelationshipAxisId::new("trust"),
                delta: 10,
            },
        );
        e3.target = Some(EntityId::new(99));

        state.add_event(e1);
        state.add_event(e2);
        state.add_event(e3);

        check!(state.events().len() == 3);

        let chosen = state.events_of_kind("DialogOptionChosen");
        check!(chosen.len() == 1);
        check!(chosen[0].id.0 == 1);

        let learned = state.events_of_kind("FactLearned");
        check!(learned.len() == 1);
        check!(learned[0].id.0 == 2);

        let by_actor = state.events_by_actor(EntityId::new(42));
        check!(by_actor.len() == 1);
        check!(by_actor[0].id.0 == 2);

        let by_target = state.events_by_target(EntityId::new(99));
        check!(by_target.len() == 1);
        check!(by_target[0].id.0 == 3);

        let recent = state.recent_events(2);
        check!(recent.len() == 2);
        check!(recent[0].id.0 == 2);
        check!(recent[1].id.0 == 3);

        let all_recent = state.recent_events(10);
        check!(all_recent.len() == 3);

        let found = state.find_events(|e| matches!(e.kind, EventKind::RelationshipChanged { .. }));
        check!(found.len() == 1);
        check!(found[0].id.0 == 3);
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

    #[test]
    fn prop_recent_events_bounds() {
        proptest!(|(events in proptest::collection::vec(any::<u64>(), 0..20), count in 0usize..30)| {
            let mut ctx = DialogWorldState::new();
            for (i, id) in events.iter().enumerate() {
                let event = Event::new(
                    dramaturge_model::EventId::new(*id),
                    EventKind::Custom { kind: format!("event_{i}").into() },
                );
                ctx.add_event(event);
            }
            let recent = ctx.recent_events(count);
            let expected_len = count.min(events.len());
            prop_assert_eq!(recent.len(), expected_len);
            if !recent.is_empty() {
                prop_assert!(recent.iter().all(|e| ctx.events.iter().any(|ev| ev.id == e.id)));
            }
        });
    }
}
