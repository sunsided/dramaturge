//! The state abstraction that evaluators and appliers work against.
//!
//! [`NarrativeState`] lists the operations the default condition evaluator
//! and outcome applier need. [`DialogWorldState`] implements it, and so does
//! any host type that embeds a `DialogWorldState` and implements
//! [`HasDialogWorldState`].
//!
//! A host can use the default evaluator and applier in two ways:
//!
//! 1. **Embed a `DialogWorldState`.** Implement [`HasDialogWorldState`]. The
//!    built-in conditions and outcomes go to the embedded state. A
//!    [`CustomHandler`](crate::CustomHandler) answers `Custom` conditions and
//!    outcomes from the other host fields.
//! 2. **Implement `NarrativeState` directly.** Use this path when the host
//!    already has a store for facts, relations and memories, for example a
//!    save state. The default evaluator and applier then read and write the
//!    host store, and the host keeps one store only. [`NarrativeState::fact`]
//!    returns a [`Cow`], so a host that keeps its own value type can return a
//!    converted value.
//!
//! The first path:
//!
//! ```rust
//! use dramaturge_runtime::{DialogWorldState, HasDialogWorldState, NarrativeState};
//! use dramaturge_model::{Fact, FactId, FactScope};
//!
//! struct GameState {
//!     current_room: String,
//!     dialog: DialogWorldState,
//! }
//!
//! impl HasDialogWorldState for GameState {
//!     fn dialog(&self) -> &DialogWorldState {
//!         &self.dialog
//!     }
//!     fn dialog_mut(&mut self) -> &mut DialogWorldState {
//!         &mut self.dialog
//!     }
//! }
//!
//! let mut game = GameState { current_room: "shore".into(), dialog: DialogWorldState::new() };
//! NarrativeState::set_fact(&mut game, FactScope::World, Fact::bool("lamp_lit", true));
//! assert!(game.fact(&FactScope::World, &FactId::new("lamp_lit")).is_some());
//! ```
//!
//! The second path, with a host store that keeps only flags:
//!
//! ```rust
//! use std::borrow::Cow;
//! use std::collections::BTreeMap;
//! use dramaturge_model::{
//!     AlignmentAxisId, AlignmentPoleId, DialogTagKind, EntityId, EventKind, Fact, FactId,
//!     FactScope, FactValue, Memory, MemoryId, MemoryQuery, RelationshipAxisId,
//! };
//! use dramaturge_runtime::NarrativeState;
//!
//! #[derive(Default)]
//! struct SaveState {
//!     flags: BTreeMap<String, bool>,
//! }
//!
//! impl NarrativeState for SaveState {
//!     fn fact(&self, _: &FactScope, id: &FactId) -> Option<Cow<'_, FactValue>> {
//!         let flag = self.flags.get(id.0.as_ref())?;
//!         Some(Cow::Owned(FactValue::Bool(*flag)))
//!     }
//!     fn set_fact(&mut self, _: FactScope, fact: Fact) {
//!         if let FactValue::Bool(b) = fact.value {
//!             self.flags.insert(fact.id.0.to_string(), b);
//!         }
//!     }
//!     fn remove_fact(&mut self, _: &FactScope, id: &FactId) -> bool {
//!         self.flags.remove(id.0.as_ref()).is_some()
//!     }
//!     fn alignment(&self, _: &AlignmentAxisId, _: &AlignmentPoleId) -> Option<i32> { None }
//!     fn add_alignment(&mut self, _: AlignmentAxisId, _: AlignmentPoleId, _: i32) {}
//!     fn relationship(&self, _: EntityId, _: EntityId, _: &RelationshipAxisId) -> Option<i32> {
//!         None
//!     }
//!     fn add_relationship(&mut self, _: EntityId, _: EntityId, _: RelationshipAxisId, _: i32) {}
//!     fn add_memory(&mut self, _: EntityId, _: Memory) {}
//!     fn remove_memory(&mut self, _: EntityId, _: MemoryId) -> bool { false }
//!     fn matches_memory_query(&self, _: EntityId, _: &MemoryQuery) -> bool { false }
//!     fn has_tag_available(&self, _: &DialogTagKind) -> bool { false }
//!     // This store keeps no event log.
//!     fn emit_event(&mut self, _: EventKind) {}
//! }
//!
//! let mut save = SaveState::default();
//! save.set_fact(FactScope::World, Fact::bool("lamp_lit", true));
//! assert!(save.fact(&FactScope::World, &FactId::new("lamp_lit")).is_some());
//! ```

use crate::context::DialogWorldState;
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, DialogTagKind, EntityId, EventKind, Fact, FactId, FactScope,
    FactValue, Memory, MemoryId, MemoryQuery, RelationshipAxisId,
};
use std::borrow::Cow;

/// Operations on narrative state used by the default evaluator and applier.
///
/// Implemented for every [`HasDialogWorldState`] type, including
/// [`DialogWorldState`] itself. Implement it directly when the host has its
/// own store for facts, relations and memories. See the [module
/// documentation](self) for both paths.
pub trait NarrativeState {
    /// Looks up a fact by scope and id.
    ///
    /// A store that holds `FactValue`s returns [`Cow::Borrowed`]. A store
    /// with its own value type returns [`Cow::Owned`].
    fn fact(&self, scope: &FactScope, id: &FactId) -> Option<Cow<'_, FactValue>>;
    /// Inserts or replaces a scoped fact.
    fn set_fact(&mut self, scope: FactScope, fact: Fact);
    /// Removes a scoped fact. Returns `true` if it was present.
    fn remove_fact(&mut self, scope: &FactScope, id: &FactId) -> bool;
    /// Returns an alignment score, or `None` if untracked.
    fn alignment(&self, axis: &AlignmentAxisId, pole: &AlignmentPoleId) -> Option<i32>;
    /// Adds `delta` to an alignment score, starting at zero.
    fn add_alignment(&mut self, axis: AlignmentAxisId, pole: AlignmentPoleId, delta: i32);
    /// Returns a directional relationship score, or `None` if untracked.
    fn relationship(
        &self,
        source: EntityId,
        target: EntityId,
        axis: &RelationshipAxisId,
    ) -> Option<i32>;
    /// Adds `delta` to a directional relationship score, starting at zero.
    fn add_relationship(
        &mut self,
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        delta: i32,
    );
    /// Adds a memory to an entity.
    fn add_memory(&mut self, owner: EntityId, memory: Memory);
    /// Removes a memory from an entity. Returns `true` if it was present.
    fn remove_memory(&mut self, owner: EntityId, id: MemoryId) -> bool;
    /// Returns `true` if any memory of `owner` matches the query.
    fn matches_memory_query(&self, owner: EntityId, query: &MemoryQuery) -> bool;
    /// Returns `true` if the tag is available.
    fn has_tag_available(&self, tag: &DialogTagKind) -> bool;
    /// Records a narrative event with a freshly allocated id.
    fn emit_event(&mut self, kind: EventKind);
}

/// Access to an embedded [`DialogWorldState`].
///
/// Implementing this trait gives a host type a forwarding
/// [`NarrativeState`] implementation for free.
pub trait HasDialogWorldState {
    /// Returns the embedded dialog state.
    fn dialog(&self) -> &DialogWorldState;
    /// Returns the embedded dialog state mutably.
    fn dialog_mut(&mut self) -> &mut DialogWorldState;
}

impl HasDialogWorldState for DialogWorldState {
    fn dialog(&self) -> &DialogWorldState {
        self
    }

    fn dialog_mut(&mut self) -> &mut DialogWorldState {
        self
    }
}

impl<T: HasDialogWorldState + ?Sized> NarrativeState for T {
    fn fact(&self, scope: &FactScope, id: &FactId) -> Option<Cow<'_, FactValue>> {
        self.dialog().get_fact(scope, id).map(Cow::Borrowed)
    }

    fn set_fact(&mut self, scope: FactScope, fact: Fact) {
        DialogWorldState::set_fact(self.dialog_mut(), scope, fact);
    }

    fn remove_fact(&mut self, scope: &FactScope, id: &FactId) -> bool {
        DialogWorldState::remove_fact(self.dialog_mut(), scope, id)
    }

    fn alignment(&self, axis: &AlignmentAxisId, pole: &AlignmentPoleId) -> Option<i32> {
        self.dialog().get_alignment(axis, pole)
    }

    fn add_alignment(&mut self, axis: AlignmentAxisId, pole: AlignmentPoleId, delta: i32) {
        DialogWorldState::add_alignment(self.dialog_mut(), axis, pole, delta);
    }

    fn relationship(
        &self,
        source: EntityId,
        target: EntityId,
        axis: &RelationshipAxisId,
    ) -> Option<i32> {
        self.dialog().get_relationship(source, target, axis)
    }

    fn add_relationship(
        &mut self,
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        delta: i32,
    ) {
        DialogWorldState::add_relationship(self.dialog_mut(), source, target, axis, delta);
    }

    fn add_memory(&mut self, owner: EntityId, memory: Memory) {
        DialogWorldState::add_memory(self.dialog_mut(), owner, memory);
    }

    fn remove_memory(&mut self, owner: EntityId, id: MemoryId) -> bool {
        DialogWorldState::remove_memory(self.dialog_mut(), owner, id)
    }

    fn matches_memory_query(&self, owner: EntityId, query: &MemoryQuery) -> bool {
        DialogWorldState::matches_memory_query(self.dialog(), owner, query)
    }

    fn has_tag_available(&self, tag: &DialogTagKind) -> bool {
        DialogWorldState::has_tag_available(self.dialog(), tag)
    }

    fn emit_event(&mut self, kind: EventKind) {
        DialogWorldState::emit_event(self.dialog_mut(), kind);
    }
}
