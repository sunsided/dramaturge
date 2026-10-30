//! The state abstraction that evaluators and appliers work against.
//!
//! [`NarrativeState`] lists the operations the default condition evaluator
//! and outcome applier need. [`DialogWorldState`] implements it, and so does
//! any host type that embeds a `DialogWorldState` and implements
//! [`HasDialogWorldState`].
//!
//! This lets a host keep all of its game state (rooms, clocks, inventories)
//! in one value and still use the default evaluator: a
//! [`CustomHandler`](crate::CustomHandler) answers `Custom` conditions and
//! outcomes from the host fields, and every built-in variant goes to the
//! embedded `DialogWorldState`.
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

use crate::context::DialogWorldState;
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, DialogTagKind, EntityId, EventKind, Fact, FactId, FactScope,
    FactValue, Memory, MemoryId, MemoryQuery, RelationshipAxisId,
};

/// Operations on narrative state used by the default evaluator and applier.
///
/// Implemented for every [`HasDialogWorldState`] type, including
/// [`DialogWorldState`] itself. Implement it directly only if your state
/// does not embed a `DialogWorldState`.
pub trait NarrativeState {
    /// Looks up a fact by scope and id.
    fn fact(&self, scope: &FactScope, id: &FactId) -> Option<&FactValue>;
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
    fn fact(&self, scope: &FactScope, id: &FactId) -> Option<&FactValue> {
        self.dialog().get_fact(scope, id)
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
