//! Runtime evaluation traits and engine interface for the `dramaturge` toolkit.
//!
//! This crate defines the traits that separate the domain model from its
//! evaluation strategy. Concrete implementations will be provided in
//! follow-up iterations.

pub mod context;
pub mod evaluation;
pub mod runtime;

pub use context::EvaluationContext;
pub use evaluation::{ConditionEvaluator, EvaluationResult, OutcomeApplier};
pub use runtime::DialogueRuntime;
