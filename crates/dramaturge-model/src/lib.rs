//! Core domain model for the `dramaturge` dialogue toolkit.
//!
//! This crate provides game-agnostic, foundational types for building
//! branching dialogue systems. It is intentionally free of serde or other
//! serialization dependencies — the `dramaturge_serde` crate serves as
//! the designated integration point for optional serialization.
//!
//! The types in this crate represent:
//!
//! - **Characters and entities** — [`EntityId`], [`Character`], [`CharacterTrait`]
//! - **Facts and scoping** — [`Fact`], [`FactValue`], [`FactScope`], [`ScopedFact`]
//!   (note: `FactValue::Float(f64)` prevents `Eq`/`Hash` derivation on
//!   transitively dependent types)
//! - **Alignment** — [`AlignmentAxis`], [`AlignmentPole`], [`AlignmentScore`]
//! - **Relationships** — [`RelationshipScore`], [`RelationshipAxisId`]
//! - **Dialogue graphs** — [`DialogueGraph`], [`DialogueNode`], [`DialogueOption`]
//! - **Conditions** — [`Condition`] (composable boolean expression trees)
//! - **Outcomes** — [`Outcome`] (state changes triggered by dialogue choices)
//! - **Memories** — [`Memory`], [`Confidence`], [`MemoryTag`]
//! - **Events** — [`Event`], [`EventKind`]
//! - **Intentions** — [`Intention`], [`DialogIntentions`]
//! - **Tags** — [`DialogTag`], [`DialogTagKind`], [`MoralTag`], [`SkillTag`], [`SpeechActTag`]
//!
//! Circular module imports between `dialogue`, `outcome`, `event`, and
//! `condition` are intentional and compile cleanly within a single crate.

pub mod alignment;
pub mod character;
pub mod condition;
pub mod dialogue;
pub mod event;
pub mod fact;
pub mod intent;
pub mod memory;
pub mod outcome;
pub mod relationship;
pub mod tag;

// Re-exports: alignment
pub use alignment::{
    AlignmentAxis, AlignmentAxisId, AlignmentPole, AlignmentPoleId, AlignmentScore,
};
// Re-exports: characters and entities
pub use character::{Character, CharacterId, CharacterKey, CharacterTrait, EntityId};
// Re-exports: conditions
pub use condition::Condition;
// Re-exports: dialogue graph
pub use dialogue::{
    DialogTarget, DialogText, DialogueGraph, DialogueNode, DialogueNodeId, DialogueOption,
    DialogueOptionId,
};
// Re-exports: events
pub use event::{Event, EventContext, EventId, EventKind};
// Re-exports: facts and scoping
pub use fact::{Fact, FactId, FactScope, FactValue, ScopedFact};
// Re-exports: intentions
pub use intent::{DialogIntentions, Intention};
// Re-exports: memories
pub use memory::{Confidence, Memory, MemoryId, MemoryQuery, MemoryTag};
// Re-exports: outcomes
pub use outcome::Outcome;
// Re-exports: relationships
pub use relationship::{RelationshipAxisId, RelationshipScore};
// Re-exports: tags
pub use tag::{CustomDialogTag, DialogTag, DialogTagKind, MoralTag, SkillTag, SpeechActTag};
