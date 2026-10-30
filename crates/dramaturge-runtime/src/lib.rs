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
//! - [`state`] — [`NarrativeState`], the state abstraction evaluators work against
//! - [`text`] — [`resolve_text`] for state-dependent fragment text
//!
//! # Evaluation Model
//!
//! Condition evaluation and outcome application are separated from the domain
//! model by two traits, both generic over the state type `S` (default:
//! [`DialogWorldState`]):
//!
//! - [`ConditionEvaluator`] takes a [`Condition`](dramaturge_model::Condition)
//!   and a state reference and returns `bool`. The default implementation
//!   ([`DefaultConditionEvaluator`]) handles all built-in variants and hands
//!   [`Condition::Custom`](dramaturge_model::Condition::Custom) to a
//!   [`CustomHandler`] — also when nested inside `All`, `Any` or `Not`.
//!
//! - [`OutcomeApplier`] takes an [`Outcome`](dramaturge_model::Outcome) and
//!   mutable state and applies the state change. The default implementation
//!   ([`DefaultOutcomeApplier`]) handles all built-in variants and hands
//!   [`Outcome::Custom`](dramaturge_model::Outcome::Custom) to a [`CustomHandler`].
//!
//! Any type implementing [`NarrativeState`] can be evaluated. A host that
//! embeds a [`DialogWorldState`] implements [`HasDialogWorldState`] and gets
//! `NarrativeState` for free.
//!
//! [`DefaultRuntime`] composes an evaluator and an applier to drive a
//! dialogue session: it filters available options by evaluating their
//! conditions, and applies outcomes when an option is chosen.
//!
//! # Extension Points
//!
//! Answer custom conditions and outcomes from your own game state:
//!
//! ```rust
//! use dramaturge_runtime::{
//!     CustomHandler, DefaultConditionEvaluator, DefaultOutcomeApplier, DefaultRuntime,
//!     DialogWorldState, DialogueRuntime, HasDialogWorldState,
//! };
//! use dramaturge_model::{
//!     CharacterId, Condition, DialogTarget, DialogText, DialogueNode, DialogueNodeId,
//!     DialogueOption, EntityId, FactValue, Outcome,
//! };
//!
//! struct Game {
//!     current_room: &'static str,
//!     bell_rung: bool,
//!     dialog: DialogWorldState,
//! }
//!
//! impl HasDialogWorldState for Game {
//!     fn dialog(&self) -> &DialogWorldState { &self.dialog }
//!     fn dialog_mut(&mut self) -> &mut DialogWorldState { &mut self.dialog }
//! }
//!
//! #[derive(Clone, Default)]
//! struct Hooks;
//!
//! impl CustomHandler<Game> for Hooks {
//!     fn condition(&self, key: &str, args: &[FactValue], game: &Game) -> bool {
//!         key == "in_room" && matches!(args, [FactValue::Text(r)] if r == game.current_room)
//!     }
//!     fn outcome(&mut self, key: &str, _args: &[FactValue], game: &mut Game) {
//!         if key == "ring_bell" {
//!             game.bell_rung = true;
//!         }
//!     }
//! }
//!
//! let mut runtime = DefaultRuntime::with_parts(
//!     DefaultConditionEvaluator::with_custom(Hooks),
//!     DefaultOutcomeApplier::with_custom(Hooks),
//! );
//!
//! let mut node = DialogueNode::new(1, CharacterId::new(EntityId::new(1)), DialogText::inline("..."));
//! let mut option = DialogueOption::new(1, DialogText::inline("Ring"), DialogTarget::End, node.id);
//! option.conditions.push(Condition::Custom {
//!     key: "in_room".into(),
//!     args: vec![FactValue::Text("tower".into())],
//! });
//! option.outcomes.push(Outcome::Custom { key: "ring_bell".into(), args: vec![] });
//! node.options.push(option);
//!
//! let mut game = Game { current_room: "tower", bell_rung: false, dialog: DialogWorldState::new() };
//! let chosen = runtime.available_options(&node, &game).next().cloned().unwrap();
//! runtime.choose_option(&chosen, &mut game);
//! assert!(game.bell_rung);
//! ```

pub mod context;
pub mod evaluation;
pub mod runtime;
pub mod state;
pub mod text;

pub use context::{DialogWorldState, EvaluationContext};
pub use evaluation::{
    ConditionEvaluator, CustomHandler, DefaultConditionEvaluator, DefaultOutcomeApplier,
    EvaluationResult, NoCustom, OutcomeApplier, partial_eq_values,
};
pub use runtime::{AvailableOptionsIter, DefaultRuntime, DialogueRuntime};
pub use state::{HasDialogWorldState, NarrativeState};
pub use text::{
    FragmentProgress, ResolvedPiece, ResolvedText, SplitMix64, TextProgress, TextRng, resolve_text,
};
