//! Core domain model for the `dramaturge` dialogue toolkit.
//!
//! This crate provides the foundational types that represent characters,
//! dialogue graphs, facts, conditions, outcomes, intents, memories, and events.
//! Types are intentionally lightweight placeholders; full behaviour will be
//! added in subsequent development iterations.

pub mod character;
pub mod condition;
pub mod dialogue;
pub mod event;
pub mod fact;
pub mod intent;
pub mod memory;
pub mod outcome;
pub mod tag;

pub use character::{Alignment, Character, CharacterId};
pub use condition::Condition;
pub use dialogue::{DialogueGraph, DialogueNode, DialogueOption};
pub use event::Event;
pub use fact::{Fact, FactKey, FactValue};
pub use intent::Intent;
pub use memory::Memory;
pub use outcome::Outcome;
pub use tag::OptionTag;
