//! Core domain model for the `dramaturge` dialogue toolkit.
//!
//! This crate provides the foundational types that represent characters,
//! dialogue graphs, facts, conditions, outcomes, intents, memories, and events.
//! Types are intentionally lightweight placeholders; full behaviour will be
//! added in subsequent development iterations.

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

pub use alignment::{
    AlignmentAxis, AlignmentAxisId, AlignmentPole, AlignmentPoleId, AlignmentScore,
};
pub use character::{Character, CharacterId, CharacterKey, CharacterTrait, EntityId};
pub use condition::Condition;
pub use dialogue::{
    DialogTarget, DialogText, DialogueGraph, DialogueNode, DialogueNodeId, DialogueOption,
    DialogueOptionId,
};
pub use event::{Event, EventContext, EventId, EventKind};
pub use fact::{Fact, FactId, FactScope, FactValue, ScopedFact};
pub use intent::{DialogIntentions, Intention};
pub use memory::{Confidence, Memory, MemoryId, MemoryQuery, MemoryTag};
pub use outcome::Outcome;
pub use relationship::{RelationshipAxisId, RelationshipScore};
pub use tag::{CustomDialogTag, DialogTag, DialogTagKind, MoralTag, SkillTag, SpeechActTag};
