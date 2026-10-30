//! Dialog world state — the mutable game state evaluated against conditions.
//!
//! [`DialogWorldState`] is the central snapshot of the narrative world,
//! holding facts, alignments, relationships, memories, and events. It
//! is mutated by [`OutcomeApplier`](crate::OutcomeApplier) implementations
//! and queried by [`ConditionEvaluator`](crate::ConditionEvaluator)
//! implementations during dialogue evaluation.
//!
//! # Storage
//!
//! Facts, alignments and relationships live in ordered maps, so lookups
//! are `O(log n)` and iteration order is deterministic: it depends only on
//! the keys, not on insertion order. Memories are grouped by owner and keep
//! their insertion order per owner. The event log is a `Vec` in
//! chronological order.
//!
//! # Fingerprints
//!
//! [`DialogWorldState::fingerprint`] hashes the *logical* state — facts,
//! alignments, relationships, memories and available tags — and ignores the
//! event log, which is history rather than state. Two states with the same
//! fingerprint are (with overwhelming probability) indistinguishable to
//! conditions. Automated explorers use it to detect states they have
//! already visited.

use dramaturge_model::tag::DialogTagKind;
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, EntityId, Event, EventId, EventKind, Fact, FactId, FactScope,
    FactValue, Memory, MemoryId, MemoryQuery, RelationshipAxisId,
};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

/// Complete snapshot of the narrative world state.
///
/// This struct holds all mutable state that the dialogue system interacts
/// with during evaluation. Facts are scoped, alignments and relationships
/// are directional, and memories are owned by entities.
///
/// `PartialEq` compares everything, including the event log. Use
/// [`fingerprint`](Self::fingerprint) or [`same_state`](Self::same_state)
/// to compare only the logical state.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogWorldState {
    facts: BTreeMap<FactScope, BTreeMap<FactId, FactValue>>,
    alignments: BTreeMap<AlignmentAxisId, BTreeMap<AlignmentPoleId, i32>>,
    relationships: BTreeMap<(EntityId, EntityId), BTreeMap<RelationshipAxisId, i32>>,
    memories: BTreeMap<EntityId, Vec<Memory>>,
    available_tags: BTreeSet<DialogTagKind>,
    events: Vec<Event>,
    /// Next event ID to allocate via [`emit_event`](Self::emit_event).
    ///
    /// Zero means the counter has wrapped around and the ID space is exhausted.
    event_id_counter: u64,
}

impl Default for DialogWorldState {
    fn default() -> Self {
        Self {
            facts: BTreeMap::new(),
            alignments: BTreeMap::new(),
            relationships: BTreeMap::new(),
            memories: BTreeMap::new(),
            available_tags: BTreeSet::new(),
            events: Vec::new(),
            event_id_counter: 1,
        }
    }
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
    pub fn get_fact(&self, scope: &FactScope, fact_id: &FactId) -> Option<&FactValue> {
        self.facts.get(scope)?.get(fact_id)
    }

    /// Inserts or replaces a scoped fact.
    pub fn set_fact(&mut self, scope: FactScope, fact: Fact) {
        self.facts
            .entry(scope)
            .or_default()
            .insert(fact.id, fact.value);
    }

    /// Removes a scoped fact if present. Returns `true` if it was present.
    pub fn remove_fact(&mut self, scope: &FactScope, fact_id: &FactId) -> bool {
        let Some(scoped) = self.facts.get_mut(scope) else {
            return false;
        };
        let removed = scoped.remove(fact_id).is_some();
        if scoped.is_empty() {
            self.facts.remove(scope);
        }
        removed
    }

    /// Convenience check for whether a scoped fact is present.
    pub fn fact_exists(&self, scope: &FactScope, fact_id: &FactId) -> bool {
        self.get_fact(scope, fact_id).is_some()
    }

    /// Iterates over all facts as `(scope, id, value)`, ordered by scope, then id.
    pub fn facts(&self) -> impl Iterator<Item = (&FactScope, &FactId, &FactValue)> {
        self.facts
            .iter()
            .flat_map(|(scope, facts)| facts.iter().map(move |(id, value)| (scope, id, value)))
    }

    /// Iterates over the facts of one scope as `(id, value)`, ordered by id.
    pub fn facts_in(&self, scope: &FactScope) -> impl Iterator<Item = (&FactId, &FactValue)> {
        self.facts.get(scope).into_iter().flatten()
    }

    /// Returns the number of facts across all scopes.
    pub fn fact_count(&self) -> usize {
        self.facts.values().map(BTreeMap::len).sum()
    }

    /// Returns the current alignment score, or `None` if untracked.
    pub fn get_alignment(&self, axis: &AlignmentAxisId, pole: &AlignmentPoleId) -> Option<i32> {
        self.alignments.get(axis)?.get(pole).copied()
    }

    /// Increments an existing alignment score, or initializes it to the
    /// given delta if not yet tracked.
    pub fn add_alignment(&mut self, axis: AlignmentAxisId, pole: AlignmentPoleId, delta: i32) {
        *self
            .alignments
            .entry(axis)
            .or_default()
            .entry(pole)
            .or_insert(0) += delta;
    }

    /// Iterates over all alignment scores as `(axis, pole, value)`.
    pub fn alignments(&self) -> impl Iterator<Item = (&AlignmentAxisId, &AlignmentPoleId, i32)> {
        self.alignments
            .iter()
            .flat_map(|(axis, poles)| poles.iter().map(move |(pole, v)| (axis, pole, *v)))
    }

    /// Returns the relationship score, or `None` if untracked.
    pub fn get_relationship(
        &self,
        source: EntityId,
        target: EntityId,
        axis: &RelationshipAxisId,
    ) -> Option<i32> {
        self.relationships
            .get(&(source, target))?
            .get(axis)
            .copied()
    }

    /// Increments an existing relationship score, or initializes it to
    /// the given delta if not yet tracked.
    pub fn add_relationship(
        &mut self,
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        delta: i32,
    ) {
        *self
            .relationships
            .entry((source, target))
            .or_default()
            .entry(axis)
            .or_insert(0) += delta;
    }

    /// Iterates over all relationship scores as `(source, target, axis, value)`.
    pub fn relationships(
        &self,
    ) -> impl Iterator<Item = (EntityId, EntityId, &RelationshipAxisId, i32)> {
        self.relationships
            .iter()
            .flat_map(|((source, target), axes)| {
                axes.iter()
                    .map(move |(axis, v)| (*source, *target, axis, *v))
            })
    }

    /// Returns all memories owned by the given entity, in insertion order.
    pub fn get_memories(&self, owner: EntityId) -> &[Memory] {
        self.memories.get(&owner).map_or(&[], Vec::as_slice)
    }

    /// Appends a memory for the given owner entity.
    pub fn add_memory(&mut self, owner: EntityId, memory: Memory) {
        self.memories.entry(owner).or_default().push(memory);
    }

    /// Removes a memory by ID from the given owner entity.
    /// Returns `true` if a memory was found and removed.
    pub fn remove_memory(&mut self, owner: EntityId, memory_id: MemoryId) -> bool {
        let Some(owned) = self.memories.get_mut(&owner) else {
            return false;
        };
        let before = owned.len();
        owned.retain(|m| m.id != memory_id);
        let removed = owned.len() < before;
        if owned.is_empty() {
            self.memories.remove(&owner);
        }
        removed
    }

    /// Iterates over all memories as `(owner, memory)`, ordered by owner.
    pub fn memories(&self) -> impl Iterator<Item = (EntityId, &Memory)> {
        self.memories
            .iter()
            .flat_map(|(owner, ms)| ms.iter().map(move |m| (*owner, m)))
    }

    /// Appends an event to the timeline.
    ///
    /// If the event's ID is at or above the current allocation counter, the
    /// counter is advanced past it so that subsequent [`emit_event`](Self::emit_event)
    /// calls cannot produce a duplicate ID.
    pub fn add_event(&mut self, event: Event) {
        if event.id.0 >= self.event_id_counter {
            self.event_id_counter = event.id.0.wrapping_add(1);
        }
        self.events.push(event);
    }

    /// Returns the next event ID that [`emit_event`](Self::emit_event) would allocate.
    ///
    /// Returns `None` when the event ID space is exhausted (after 2^64 - 1 events).
    pub fn next_event_id(&self) -> Option<u64> {
        (self.event_id_counter != 0).then_some(self.event_id_counter)
    }

    /// Sets the next event ID that [`emit_event`](Self::emit_event) allocates.
    ///
    /// `None` marks the ID space as exhausted. Used when restoring a saved
    /// state whose log was drained. Never set it at or below an ID that is
    /// still in the log, or emitted IDs will repeat.
    pub fn set_next_event_id(&mut self, next: Option<u64>) {
        self.event_id_counter = next.unwrap_or(0);
    }

    /// Allocates a new event ID and appends an event with the given kind.
    ///
    /// Returns `None` when the event ID space is exhausted.
    pub fn emit_event(&mut self, kind: EventKind) -> Option<&Event> {
        let id = self.next_event_id()?;
        let event = Event::new(EventId::new(id), kind);
        self.events.push(event);
        self.event_id_counter = self.event_id_counter.wrapping_add(1);
        self.events.last()
    }

    /// Returns all events in chronological order.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Removes all events and returns them, keeping the id counter.
    ///
    /// Event ids stay unique after a drain: the next emitted event continues
    /// the sequence.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
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
    pub fn find_events<F: FnMut(&Event) -> bool>(&self, mut predicate: F) -> Vec<&Event> {
        self.events.iter().filter(|e| predicate(e)).collect()
    }

    /// Adds a tag to the set of available tags.
    pub fn add_available_tag(&mut self, tag: DialogTagKind) {
        self.available_tags.insert(tag);
    }

    /// Removes a tag from the set of available tags. Returns `true` if it was present.
    pub fn remove_available_tag(&mut self, tag: &DialogTagKind) -> bool {
        self.available_tags.remove(tag)
    }

    /// Checks if a specific tag is available.
    pub fn has_tag_available(&self, tag: &DialogTagKind) -> bool {
        self.available_tags.contains(tag)
    }

    /// Iterates over the available tags in order.
    pub fn available_tags(&self) -> impl Iterator<Item = &DialogTagKind> {
        self.available_tags.iter()
    }

    /// Checks if any memory owned by the given entity matches the query.
    pub fn matches_memory_query(&self, owner: EntityId, query: &MemoryQuery) -> bool {
        self.get_memories(owner).iter().any(|m| query.matches(m))
    }

    /// Hashes the logical state into a 64-bit fingerprint.
    ///
    /// Covers facts, alignments, relationships, memories and available tags.
    /// The event log and the event id counter are excluded: they record
    /// history, not state.
    ///
    /// Floats are hashed by bit pattern after mapping `-0.0` to `0.0` and
    /// every NaN to one canonical NaN. Untracked and zero-valued scores are
    /// distinct (a score of `0` is still tracked).
    ///
    /// The value is deterministic for a given build: it does not depend on
    /// insertion order or on the process. It is not guaranteed to be stable
    /// across Rust or dramaturge versions, so do not persist it.
    pub fn fingerprint(&self) -> u64 {
        // `DefaultHasher::new()` uses fixed keys, unlike `RandomState`, so the
        // result does not change between processes.
        let mut hasher = std::hash::DefaultHasher::new();
        self.hash_logical(&mut hasher);
        hasher.finish()
    }

    /// Feeds the logical state (everything except the event log) into `hasher`.
    ///
    /// Use this to combine the dialog state into a host's own fingerprint.
    pub fn hash_logical<H: Hasher>(&self, hasher: &mut H) {
        self.facts.len().hash(hasher);
        for (scope, facts) in &self.facts {
            scope.hash(hasher);
            facts.len().hash(hasher);
            for (id, value) in facts {
                id.hash(hasher);
                hash_fact_value(value, hasher);
            }
        }
        self.alignments.hash(hasher);
        self.relationships.hash(hasher);
        self.memories.hash(hasher);
        self.available_tags.hash(hasher);
    }

    /// Returns `true` if both states have the same logical state.
    ///
    /// Like [`fingerprint`](Self::fingerprint), this ignores the event log,
    /// and it treats floats the same way: `-0.0` equals `0.0`, and all NaNs
    /// equal each other. So `same_state` implies equal fingerprints, and a
    /// state is always the same as itself.
    pub fn same_state(&self, other: &Self) -> bool {
        self.facts.len() == other.facts.len()
            && self
                .facts
                .iter()
                .zip(&other.facts)
                .all(|((sa, fa), (sb, fb))| {
                    sa == sb
                        && fa.len() == fb.len()
                        && fa
                            .iter()
                            .zip(fb)
                            .all(|((ia, va), (ib, vb))| ia == ib && same_fact_value(va, vb))
                })
            && self.alignments == other.alignments
            && self.relationships == other.relationships
            && self.memories == other.memories
            && self.available_tags == other.available_tags
    }
}

/// Hashes a [`FactValue`] consistently with its `PartialEq`.
fn hash_fact_value<H: Hasher>(value: &FactValue, hasher: &mut H) {
    std::mem::discriminant(value).hash(hasher);
    match value {
        FactValue::Bool(b) => b.hash(hasher),
        FactValue::Int(i) => i.hash(hasher),
        FactValue::Float(f) => canonical_float_bits(*f).hash(hasher),
        FactValue::Text(t) => t.hash(hasher),
        FactValue::Entity(e) => e.hash(hasher),
        FactValue::Set(values) => {
            values.len().hash(hasher);
            for v in values {
                hash_fact_value(v, hasher);
            }
        }
    }
}

/// Compares [`FactValue`]s consistently with [`hash_fact_value`].
fn same_fact_value(a: &FactValue, b: &FactValue) -> bool {
    match (a, b) {
        (FactValue::Float(x), FactValue::Float(y)) => {
            canonical_float_bits(*x) == canonical_float_bits(*y)
        }
        (FactValue::Set(xs), FactValue::Set(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| same_fact_value(x, y))
        }
        _ => a == b,
    }
}

fn canonical_float_bits(f: f64) -> u64 {
    if f == 0.0 {
        0.0_f64.to_bits()
    } else if f.is_nan() {
        f64::NAN.to_bits()
    } else {
        f.to_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;
    use dramaturge_model::{Fact, FactId, FactScope};

    #[test]
    fn add_event_advances_counter_to_prevent_duplicate_ids() {
        use dramaturge_model::{Event, EventId, EventKind};
        let mut state = DialogWorldState::new();
        state.add_event(Event::new(
            EventId::new(100),
            EventKind::Custom { kind: "x".into() },
        ));
        check!(state.next_event_id() == Some(101));
        let emitted = state
            .emit_event(EventKind::Custom { kind: "y".into() })
            .unwrap();
        check!(emitted.id.0 == 101);
    }

    fn sample_state(reverse: bool) -> DialogWorldState {
        use dramaturge_model::{AlignmentAxisId, AlignmentPoleId, RelationshipAxisId};
        let mut facts = vec![
            (FactScope::World, Fact::bool("a", true)),
            (FactScope::Player, Fact::int("b", 3)),
            (FactScope::World, Fact::float("c", 1.5)),
        ];
        if reverse {
            facts.reverse();
        }
        let mut state = DialogWorldState::new();
        for (scope, fact) in facts {
            state.set_fact(scope, fact);
        }
        state.add_alignment(AlignmentAxisId::new("moral"), AlignmentPoleId::new("p"), 2);
        state.add_relationship(
            EntityId::new(1),
            EntityId::new(2),
            RelationshipAxisId::new("trust"),
            5,
        );
        state
    }

    #[test]
    fn fingerprint_ignores_insertion_order() {
        let a = sample_state(false);
        let b = sample_state(true);
        check!(a.fingerprint() == b.fingerprint());
        check!(a.same_state(&b));
        let order_a: Vec<_> = a.facts().map(|(_, id, _)| id.clone()).collect();
        let order_b: Vec<_> = b.facts().map(|(_, id, _)| id.clone()).collect();
        check!(order_a == order_b);
    }

    #[test]
    fn fingerprint_ignores_events() {
        let a = sample_state(false);
        let mut b = sample_state(false);
        b.emit_event(EventKind::Custom { kind: "x".into() });
        check!(a.fingerprint() == b.fingerprint());
        check!(a.same_state(&b));
        check!(a != b);
    }

    #[test]
    fn fingerprint_changes_with_state() {
        let a = sample_state(false);
        let mut b = sample_state(false);
        b.set_fact(FactScope::World, Fact::bool("a", false));
        check!(a.fingerprint() != b.fingerprint());
        check!(!a.same_state(&b));
    }

    #[test]
    fn fingerprint_treats_negative_zero_as_zero() {
        let mut a = DialogWorldState::new();
        let mut b = DialogWorldState::new();
        a.set_fact(FactScope::World, Fact::float("f", 0.0));
        b.set_fact(FactScope::World, Fact::float("f", -0.0));
        check!(a.fingerprint() == b.fingerprint());
    }

    #[test]
    fn same_state_agrees_with_fingerprint_on_nan() {
        let mut a = DialogWorldState::new();
        let mut b = DialogWorldState::new();
        a.set_fact(FactScope::World, Fact::float("f", f64::NAN));
        b.set_fact(FactScope::World, Fact::float("f", -f64::NAN));
        check!(a.same_state(&a));
        check!(a.same_state(&b));
        check!(a.fingerprint() == b.fingerprint());
        b.set_fact(FactScope::World, Fact::float("f", 1.0));
        check!(!a.same_state(&b));
    }

    #[test]
    fn drain_events_keeps_counter() {
        let mut state = DialogWorldState::new();
        state.emit_event(EventKind::Custom { kind: "x".into() });
        check!(state.drain_events().len() == 1);
        check!(state.events().is_empty());
        let next = state
            .emit_event(EventKind::Custom { kind: "y".into() })
            .unwrap();
        check!(next.id.0 == 2);
    }

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

        let found = state.find_events(|e| matches!(&e.kind, EventKind::RelationshipChanged { .. }));
        check!(found.len() == 1);
        check!(found[0].id.0 == 3);
    }

    #[test]
    fn remove_memory_success() {
        use dramaturge_model::{Memory, MemoryId};
        let owner = EntityId::new(1);
        let mut state = DialogWorldState::new();
        let memory = Memory::new(MemoryId::new(1));
        state.add_memory(owner, memory);
        check!(state.get_memories(owner).len() == 1);
        let removed = state.remove_memory(owner, MemoryId::new(1));
        check!(removed);
        check!(state.get_memories(owner).is_empty());
    }

    #[test]
    fn remove_memory_nonexistent() {
        use dramaturge_model::{Memory, MemoryId};
        let owner = EntityId::new(1);
        let mut state = DialogWorldState::new();
        let memory = Memory::new(MemoryId::new(1));
        state.add_memory(owner, memory);
        let removed = state.remove_memory(owner, MemoryId::new(999));
        check!(!removed);
        check!(state.get_memories(owner).len() == 1);
    }

    #[test]
    fn remove_memory_only_affects_matching_owner() {
        use dramaturge_model::{Memory, MemoryId};
        let owner_a = EntityId::new(1);
        let owner_b = EntityId::new(2);
        let mut state = DialogWorldState::new();
        state.add_memory(owner_a, Memory::new(MemoryId::new(1)));
        state.add_memory(owner_b, Memory::new(MemoryId::new(1)));
        let removed = state.remove_memory(owner_a, MemoryId::new(1));
        check!(removed);
        check!(state.get_memories(owner_a).is_empty());
        check!(state.get_memories(owner_b).len() == 1);
    }

    #[test]
    fn matches_memory_query_with_structured_query() {
        use dramaturge_model::{Confidence, Memory, MemoryId, MemoryQuery, MemoryTag};
        let owner = EntityId::new(1);
        let mut state = DialogWorldState::new();
        let memory = Memory::new(MemoryId::new(1))
            .with_tag(MemoryTag::HelpedMe)
            .with_confidence(Confidence::Known)
            .with_emotional_weight(50)
            .with_subject(EntityId::new(42));
        state.add_memory(owner, memory);

        let q_tag = MemoryQuery::by_tag(MemoryTag::HelpedMe);
        check!(state.matches_memory_query(owner, &q_tag));

        let q_wrong_tag = MemoryQuery::by_tag(MemoryTag::BetrayedMe);
        check!(!state.matches_memory_query(owner, &q_wrong_tag));

        let q_confidence = MemoryQuery {
            min_confidence: Some(Confidence::Known),
            ..MemoryQuery::new()
        };
        check!(state.matches_memory_query(owner, &q_confidence));

        let q_high_confidence = MemoryQuery {
            min_confidence: Some(Confidence::Witnessed),
            ..MemoryQuery::new()
        };
        check!(!state.matches_memory_query(owner, &q_high_confidence));

        let q_weight = MemoryQuery {
            min_emotional_weight: Some(50),
            ..MemoryQuery::new()
        };
        check!(state.matches_memory_query(owner, &q_weight));

        let q_high_weight = MemoryQuery {
            min_emotional_weight: Some(100),
            ..MemoryQuery::new()
        };
        check!(!state.matches_memory_query(owner, &q_high_weight));

        let q_subject = MemoryQuery::by_subject(EntityId::new(42));
        check!(state.matches_memory_query(owner, &q_subject));

        let q_wrong_subject = MemoryQuery::by_subject(EntityId::new(99));
        check!(!state.matches_memory_query(owner, &q_wrong_subject));

        let q_all = MemoryQuery::new();
        check!(state.matches_memory_query(owner, &q_all));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use dramaturge_model::fact::{Fact, FactScope, FactValue};
    use dramaturge_model::{
        AlignmentAxisId, AlignmentPoleId, Event, EventKind, Memory, MemoryId, RelationshipAxisId,
    };
    use proptest::prelude::*;

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
            prop_assert!(memories.contains(&memory));
        });
    }

    #[test]
    fn prop_emit_event_counter_sequential() {
        proptest!(|(n in 1usize..50)| {
            let mut ctx = DialogWorldState::new();
            for expected_id in 1..=(n as u64) {
                let before = ctx.next_event_id();
                prop_assert_eq!(before, Some(expected_id));
                let event = ctx
                    .emit_event(EventKind::Custom { kind: "x".into() })
                    .unwrap();
                prop_assert_eq!(event.id.0, expected_id);
            }
            prop_assert_eq!(ctx.next_event_id(), Some(n as u64 + 1));
            prop_assert_eq!(ctx.events.len(), n);
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
