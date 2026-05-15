//! Runtime evaluation traits and engine interface for the `dramaturge` toolkit.
//!
//! This crate defines the traits that separate the domain model from its
//! evaluation strategy, and provides concrete default implementations for
//! condition evaluation, outcome application, and dialogue session driving.
//!
//! # Modules
//!
//! - [`context`] — [`DialogWorldState`], the mutable game state evaluated against conditions
//! - [`evaluation`] — [`ConditionEvaluator`] and [`OutcomeApplier`] traits with defaults
//! - [`runtime`] — [`DialogueRuntime`] trait and [`DefaultRuntime`] implementation
//!
//! # Evaluation Model
//!
//! Condition evaluation and outcome application are separated from the domain
//! model by two traits:
//!
//! - [`ConditionEvaluator`] takes a [`Condition`](dramaturge_model::Condition)
//!   and a [`DialogWorldState`] reference and returns `bool`. The default
//!   implementation ([`DefaultConditionEvaluator`]) handles all built-in
//!   variants. [`Condition::Custom`](dramaturge_model::Condition::Custom) always returns `false` in the default;
//!   override it to add game-specific logic.
//!
//! - [`OutcomeApplier`] takes an [`Outcome`](dramaturge_model::Outcome) and a
//!   mutable [`DialogWorldState`] and applies the state change. The default
//!   implementation ([`DefaultOutcomeApplier`]) handles all built-in variants.
//!
//! [`DefaultRuntime`] composes both traits to drive a dialogue session:
//! it filters available options by evaluating their conditions, and applies
//! outcomes when an option is chosen.
//!
//! # Extension Points
//!
//! Implement [`ConditionEvaluator`] to handle `Condition::Custom` variants:
//!
//! ```rust
//! use dramaturge_runtime::{ConditionEvaluator, DefaultConditionEvaluator, DialogWorldState};
//! use dramaturge_model::{Condition, FactValue};
//!
//! struct MyEvaluator {
//!     inner: DefaultConditionEvaluator,
//! }
//!
//! impl ConditionEvaluator for MyEvaluator {
//!     fn evaluate(&self, condition: &Condition, ctx: &DialogWorldState) -> bool {
//!         match condition {
//!             Condition::Custom { key, .. } if key == "is_daytime" => {
//!                 // delegate to game engine
//!                 false // placeholder
//!             }
//!             // Recurse through composites using self so nested Custom
//!             // conditions reach this evaluator, not the default.
//!             Condition::All(cs) => cs.iter().all(|c| self.evaluate(c, ctx)),
//!             Condition::Any(cs) => cs.iter().any(|c| self.evaluate(c, ctx)),
//!             Condition::Not(c) => !self.evaluate(c, ctx),
//!             other => self.inner.evaluate(other, ctx),
//!         }
//!     }
//! }
//! ```
//!
//! Implement [`OutcomeApplier`] to handle custom outcome side effects:
//!
//! ```rust
//! use dramaturge_runtime::{OutcomeApplier, DefaultOutcomeApplier, DialogWorldState};
//! use dramaturge_model::Outcome;
//!
//! struct MyApplier {
//!     inner: DefaultOutcomeApplier,
//! }
//!
//! impl OutcomeApplier for MyApplier {
//!     fn apply(&mut self, outcome: &Outcome, state: &mut DialogWorldState) {
//!         match outcome {
//!             Outcome::Custom { key, args } if key == "unlock_achievement" => {
//!                 // call into game engine
//!             }
//!             other => self.inner.apply(other, state),
//!         }
//!     }
//! }
//! ```

pub mod context;
pub mod evaluation;
pub mod runtime;

pub use context::{DialogWorldState, EvaluationContext};
pub use evaluation::{
    ConditionEvaluator, DefaultConditionEvaluator, DefaultOutcomeApplier, EvaluationResult,
    OutcomeApplier,
};
pub use runtime::{AvailableOptionsIter, DefaultRuntime, DialogueRuntime};
