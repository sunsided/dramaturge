//! Core domain model for the `dramaturge` dialogue toolkit.
//!
//! This crate provides game-agnostic foundational types for branching dialogue
//! systems. It has no serde or runtime dependency. Serialization is handled
//! exclusively by the `dramaturge-serde` crate via its DTO types.
//!
//! # Concepts
//!
//! ## Characters and Entities
//!
//! [`EntityId`] is the universal identifier — a `u64` wrapper used everywhere.
//! [`Character`] wraps it with a string key, display name, and optional traits.
//! [`CharacterTrait`] is a generic `namespace::key` pair; your game decides
//! what traits exist.
//!
//! ## Facts and Scoping
//!
//! [`Fact`] is the unit of knowledge: a named [`FactValue`] (bool, int, float,
//! text, entity reference, or set). [`FactScope`] controls who knows it:
//! [`FactScope::World`] (objective), [`FactScope::Player`], [`FactScope::Character`],
//! [`FactScope::Faction`], or [`FactScope::Party`]. Asymmetric knowledge is
//! first-class — the player can know something a character does not.
//!
//! Note: [`FactValue::Float`] contains `f64`, which prevents `Eq`/`Hash`
//! derivation. All types that transitively contain `FactValue` derive only
//! `Debug, Clone, PartialEq`. Float equality uses epsilon comparison in the
//! evaluation layer.
//!
//! ## Alignment
//!
//! [`AlignmentAxis`] defines a named dimension (e.g., `"morality"`) with named
//! [`AlignmentPole`]s (e.g., `"paragon"`, `"renegade"`). [`AlignmentScore`]
//! tracks a character's current position. Your game defines the axes and poles.
//!
//! ## Relationships
//!
//! [`RelationshipScore`] tracks directional, named relationships between entities
//! (e.g., player → NPC trust). Axes are arbitrary string keys.
//!
//! ## Dialogue Graphs
//!
//! [`DialogueGraph`] is a pure data structure: a collection of [`DialogueNode`]s,
//! each holding text, a speaker, [`DialogueOption`]s, and optional enter/exit
//! [`Outcome`]s. No evaluation logic lives here.
//!
//! [`DialogText`] is inline text, a localization key, a list of
//! [`TextFragment`]s, or host text ([`DialogText::Host`]): text in a format
//! that only the host knows. The type parameter `T` on [`DialogText`] and
//! the graph types sets the host text type; the default is `()`.
//!
//! ## Text pools
//!
//! A [`Pool`] is an ordered list of [`Candidate`]s and a [`Pick`] rule
//! (`all`, `first` or `best`). It is the data of the text selection core:
//! `one_of` groups, named pools that rules and dialogue call by id, and
//! replies by speaker and topic. The pool types are generic over the host
//! types in [`TextTypes`]; [`DialogTypes`] are those of plain dramaturge.
//! The selection is in `dramaturge_runtime::text`.
//!
//! ## Conditions
//!
//! [`Condition`] is a composable boolean expression tree evaluated against world
//! state. Supports logical operators, fact checks, alignment/relationship
//! thresholds, memory queries, tag availability, and `Condition::Custom` for
//! game-specific extensions.
//!
//! ## Outcomes
//!
//! [`Outcome`] is a declarative state change: set a fact, adjust alignment,
//! modify a relationship, add a memory, emit an event, a custom extension,
//! or a typed host outcome ([`Outcome::Host`], with the type parameter `H`
//! on [`Outcome`] and the graph types; the default is `()`).
//! Applied by [`OutcomeApplier`](https://docs.rs/dramaturge-runtime/latest/dramaturge_runtime/trait.OutcomeApplier.html) at runtime.
//!
//! ## Memories
//!
//! [`Memory`] is a character's subjective belief about another entity. Unlike
//! facts (objective/scoped knowledge), memories carry a [`Confidence`] level
//! and optional [`MemoryTag`]s (e.g., `HelpedMe`, `Betrayed`). Characters can
//! have multiple memories about the same entity.
//!
//! ## Events
//!
//! [`Event`] records a moment in the narrative timeline — a dialogue choice, a
//! fact learned, a relationship change. Events are queryable by kind, actor,
//! and target, enabling reactive narrative logic.
//!
//! ## Intentions
//!
//! [`Intention`] describes the communicative purpose of a dialogue option
//! (e.g., `Persuade`, `Threaten`, `Comfort`). Complements [`DialogTag`] by
//! capturing *why* an option exists, not just *what category* it belongs to.
//!
//! ## Symbols
//!
//! Authors refer to content by stable string keys (`npc.mara`,
//! `dlg.mara.intro`); the runtime uses dense numeric ids. A [`SymbolTable`]
//! interns each key once and maps ids back to keys for diagnostics and save
//! games. Loaders return the table together with the model.
//!
//! ## Tags
//!
//! [`DialogTag`] is a trait for axis/key annotation on dialogue options.
//! [`CustomDialogTag`] is the built-in implementation. Implement [`DialogTag`]
//! on your own enum for a type-safe tag system with derive macro support.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod alignment;
pub mod character;
pub mod condition;
pub mod dialogue;
pub mod event;
pub mod fact;
pub mod intent;
pub mod memory;
pub mod outcome;
pub mod pool;
pub mod relationship;
pub mod symbol;
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
    DialogLine, DialogTarget, DialogText, DialogueGraph, DialogueNode, DialogueNodeId,
    DialogueOption, DialogueOptionId, FragmentMode, NavigationHint, TextFragment,
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
// Re-exports: text pools
pub use pool::{Candidate, DialogTypes, Entry, Pick, Pool, Pools, Specificity, TextTypes};
// Re-exports: relationships
pub use relationship::{RelationshipAxis, RelationshipAxisId, RelationshipScore};
// Re-exports: symbol tables
pub use symbol::{Interner, SymbolId, SymbolTable};
// Re-exports: tags
pub use tag::{CustomDialogTag, DialogTag, DialogTagKind};
