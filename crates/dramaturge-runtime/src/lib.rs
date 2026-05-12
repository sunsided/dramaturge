//! Runtime evaluation traits and engine interface for the `dramaturge` toolkit.
//!
//! This crate defines the traits that separate the domain model from its
//! evaluation strategy. It provides concrete default implementations for
//! condition evaluation, outcome application, and dialogue session driving.
//!
//! # Modules
//!
//! - [`context`] — [`DialogWorldState`], the mutable game state evaluated against conditions
//! - [`evaluation`] — [`ConditionEvaluator`] and [`OutcomeApplier`] traits with defaults
//! - [`runtime`] — [`DialogueRuntime`] trait and [`DefaultRuntime`] implementation

pub mod context;
pub mod evaluation;
pub mod runtime;

pub use context::{DialogWorldState, EvaluationContext};
pub use evaluation::{
    ConditionEvaluator, DefaultConditionEvaluator, DefaultOutcomeApplier, EvaluationResult,
    OutcomeApplier,
};
pub use runtime::{DefaultRuntime, DialogueRuntime};
