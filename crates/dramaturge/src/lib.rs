//! `dramaturge` — A Rust toolkit for branching dialogue, character memory, and narrative state.
//!
//! This facade crate re-exports the most important types from the constituent
//! crates. Start here with `use dramaturge::*;` or import individual items.
//!
//! # Design Philosophy
//!
//! dramaturge is deliberately game-agnostic. It has no knowledge of Mass Effect,
//! Pokémon, or any other game's rules. All game-specific logic is expressed
//! through typed IDs, tags, facts, and state changes:
//!
//! - **Facts** store arbitrary named values scoped to the player, a character,
//!   a faction, or the world. Your game decides what the keys mean.
//! - **Alignment axes** are named dimensions with named poles — you define
//!   `"morality"` / `"paragon"` / `"renegade"`, not the library.
//! - **Tags** are axis/key pairs on dialogue options. You define `"skill:medic"`;
//!   the runtime filters options based on world state.
//! - **Conditions** are composable trees over world state. `Condition::Custom`
//!   lets you add game-specific branches the default evaluator defers to you.
//!
//! # Layered Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │  dramaturge (facade)                                 │
//! ├──────────────────────────────────────────────────────┤
//! │  dramaturge-model     pure domain types, no I/O      │
//! │  dramaturge-runtime   evaluation traits + defaults   │
//! │  dramaturge-authoring validation, graph checks       │
//! │  dramaturge-syntax    dialogue AST, parsers, lowering│
//! │  dramaturge-macros    dialogue! and derive macros    │
//! │  dramaturge-serde     optional serde / YAML support  │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! The model crate has no serde or runtime dependency. Serialization is handled
//! exclusively by `dramaturge-serde` via DTO types; enabling the `serde` feature
//! on this facade pulls that crate in. The runtime crate knows nothing about
//! serialization. You can depend on individual crates or on this facade for the
//! full public surface.
//!
//! # Extending the System
//!
//! - **Custom conditions** — add `Condition::Custom { key, args }` to your graphs
//!   and answer them with a [`CustomHandler`] passed to
//!   [`DefaultConditionEvaluator::with_custom`]. See
//!   [`dramaturge_runtime::evaluation`] for details.
//! - **Host state** — embed [`DialogWorldState`] in your own game state and
//!   implement [`HasDialogWorldState`]; evaluators, appliers and the runtime
//!   are generic over any [`NarrativeState`].
//! - **Custom tags** — implement the [`DialogTag`] trait on your own enum or use
//!   [`CustomDialogTag`] directly. See [`dramaturge_model::tag`] for details.
//! - **Custom outcomes** — handle `Outcome::Custom` in the same
//!   [`CustomHandler`], passed to [`DefaultOutcomeApplier::with_custom`].
//! - **Host outcomes** — put typed host effects in `Outcome::Host(H)`
//!   (`Outcome<H>`, `DialogueGraph<H>`; `H` defaults to `()`) and apply
//!   them with a [`HostHandler`] on the same handler type.
//! - **Scripted exchanges** — give a node more [`DialogLine`]s with their
//!   own speakers and a `next` target; [`DialogueRuntime::advance`] enters
//!   such nodes one after the other until a node offers a choice. See the
//!   `exchange` example.
//! - **One dialogue format** — write a dialogue as YAML (feature `yaml`,
//!   `dramaturge::serde::loader::load_yaml`) or as Rust tokens with
//!   `dialogue!` (feature `macros`). Both read into the AST of
//!   `dramaturge::syntax` (feature `syntax`) and make the same graph. A host
//!   with its own conditions, text or effects lowers the AST through its
//!   own `syntax::Lower` hooks. See the `dialogue_macro` example.
//! - **Host text** — put text in your own format (for example compiled
//!   markup) in `DialogText::Host(T)` (`DialogText<T>`,
//!   `DialogueGraph<H, T>`; `T` defaults to `()`). `resolve_text` returns
//!   it as it is, and the validator does not check it.
//! - **Text selection** — put lines in a text [`Pool`] with a [`Pick`] rule
//!   (`all`, `first`, `best`), tuple keys (speaker, topic) and a perceiver
//!   test ([`Condition::SeenBy`]). [`select`](fn@select) chooses with no side effect
//!   and returns a [`Selection`] with a [`Trace`]; [`commit`] records the progress and the
//!   `learn` items. The pool types are generic over the host text leaf
//!   ([`TextTypes`]). See the `text_selection` example.
//!
//! # Example
//!
//! ```rust
//! use dramaturge::{Character, CharacterId, DialogueGraph, DialogueNodeId, EntityId};
//!
//! let hero = Character::new(EntityId::new(1), "hero", "Elara");
//! assert_eq!(&*hero.display_name, "Elara");
//!
//! let graph = DialogueGraph::default();
//! assert!(graph.nodes.is_empty());
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

pub use dramaturge_model::{
    AlignmentAxis, AlignmentAxisId, AlignmentPole, AlignmentPoleId, AlignmentScore, Candidate,
    Character, CharacterId, CharacterKey, CharacterTrait, Condition, Confidence, CustomDialogTag,
    DialogIntentions, DialogLine, DialogTag, DialogTagKind, DialogTarget, DialogText, DialogTypes,
    DialogueGraph, DialogueNode, DialogueNodeId, DialogueOption, DialogueOptionId, EntityId, Entry,
    Event, EventContext, EventId, EventKind, Fact, FactId, FactScope, FactValue, FragmentMode,
    Intention, Memory, MemoryId, MemoryQuery, MemoryTag, Outcome, Pick, Pool, Pools,
    RelationshipAxisId, RelationshipScore, ScopedFact, Specificity, SymbolTable, TextFragment,
    TextTypes,
};
pub use dramaturge_runtime::{
    ADVANCE_LIMIT, Advance, AdvanceStop, ConditionEvaluator, Count, CountKey, CustomHandler,
    DefaultConditionEvaluator, DefaultJudge, DefaultOutcomeApplier, DefaultRuntime, DialogCursor,
    DialogWorldState, DialogueRuntime, EnterResult, EvaluationContext, EvaluationResult,
    FragmentProgress, HasDialogWorldState, HostHandler, Judge, NarrativeState, NoCustom,
    OutcomeApplier, ProgressStore, Query, ResolvedPiece, ResolvedText, Selection, Shown,
    SplitMix64, Step, TextProgress, TextRng, Trace, TraceRow, Verdict, commit, resolve_text,
    select, select_entry, select_named, world_learner,
};

/// The dialogue AST, its serde shape and its lowering to a
/// [`DialogueGraph`] (the `dramaturge-syntax` crate).
#[cfg(feature = "syntax")]
#[cfg_attr(docsrs, doc(cfg(feature = "syntax")))]
pub use dramaturge_syntax as syntax;

/// The YAML loader and the serde shapes (the `dramaturge-serde` crate).
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub use dramaturge_serde as serde;

/// The macros: [`dialogue!`] builds a dialogue graph from Rust tokens.
#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub use dramaturge_macros::{ConditionBuilder, DialogTag, FactValue, dialogue};

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn facade_re_exports_compile() {
        let _: EntityId = EntityId::new(1);
        let _: CharacterId = CharacterId::new(EntityId::new(1));
        let _: Character = Character::new(EntityId::new(1), "npc", "Guard");
        let _: DialogueGraph = DialogueGraph::default();
        let _: DialogWorldState = DialogWorldState::new();
        let _: EvaluationContext = DialogWorldState::new();
        check!(EvaluationResult::Ok == EvaluationResult::Ok);
        check!(let Condition::Always = Condition::Always);
        let _: FactValue = FactValue::Bool(true);
        let _: MemoryTag = MemoryTag::HelpedMe;
        let _: DialogTagKind = DialogTagKind::new("moral", "paragon");
        let _: Intention = Intention::Persuade;
    }
}
