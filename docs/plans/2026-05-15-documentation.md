# Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Write comprehensive documentation for the dramaturge dialogue system targeting game developers.

**Architecture:** Rustdoc-centric — `//!` doc comments are the primary delivery vehicle (renders on docs.rs and in IDEs). The root `README.md` is rewritten to reflect the current implemented state. Targeted module additions cover Custom variant extension points and multi-game usage patterns.

**Tech Stack:** Rust `//!` doc comments, `cargo doc --all-features --no-deps`, `cargo test --all-features --doc`

---

### Task 1: Rewrite README.md

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Replace README.md content**

Replace the entire file with:

```markdown
# dramaturge

> A Rust toolkit for branching dialogue, character memory, and narrative state.

[![CI](https://github.com/sunsided/dramaturge/actions/workflows/ci.yml/badge.svg)](https://github.com/sunsided/dramaturge/actions/workflows/ci.yml)

---

dramaturge is a game-agnostic Rust library for building branching dialogue systems. It models characters, facts, alignment, relationships, memories, and events as pure data — no game engine knowledge baked in. Your game-specific rules live in typed IDs, tags, facts, and state changes; the library evaluates conditions, applies outcomes, and drives dialogue sessions.

## Quick Start

```toml
[dependencies]
dramaturge = { version = "0.1", path = "crates/dramaturge" }
```

```rust
use dramaturge::{
    Character, EntityId, Fact, FactId, FactScope, FactValue,
    Condition, DialogWorldState, DefaultConditionEvaluator, ConditionEvaluator,
};

// Create a character
let hero = Character::new(EntityId::new(1), "hero", "Elara");
assert_eq!(&*hero.display_name, "Elara");

// Set a fact in the world state
let mut world = DialogWorldState::new();
world.set_fact(FactScope::Player, Fact::bool("met_the_oracle", true));

// Gate a dialogue option on that fact
let condition = Condition::FactEquals {
    scope: FactScope::Player,
    fact_id: FactId::new("met_the_oracle"),
    value: FactValue::Bool(true),
};

let evaluator = DefaultConditionEvaluator;
assert!(evaluator.evaluate(&condition, &world));
```

## Full Example

[`the_broken_seal`](crates/dramaturge/examples/the_broken_seal.rs) demonstrates a complete dialogue scene loaded from YAML: branching conditions based on player skills, relationship outcomes, and a simple text-based TUI. Run it with:

```bash
cargo run --example the_broken_seal --all-features
```

## Workspace Layout

| Crate | Purpose |
|---|---|
| `dramaturge` | Public facade — re-exports the most important items |
| `dramaturge-model` | Core domain types: characters, facts, conditions, outcomes, dialogue graphs |
| `dramaturge-runtime` | Evaluation traits and default runtime engine |
| `dramaturge-macros` | Derive macros: `DialogTag`, `FactValue`, `ConditionBuilder` |
| `dramaturge-serde` | Optional serialization integration (`serde` and `yaml` features) |
| `dramaturge-authoring` | Authoring-time validation: graph structure checks, dead-end detection, orphan facts |

## License

Licensed under any of:

- [EUPL-1.2](LICENSE-EUPL-1.2)
- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
```

- [ ] **Step 2: Verify doc build**

```bash
cargo doc --all-features --no-deps 2>&1 | grep -E "error|warning" | head -20
```

Expected: no errors. Warnings about missing docs on private items are fine.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: rewrite README to reflect implemented state"
```

---

### Task 2: Expand `dramaturge` facade crate doc

**Files:**
- Modify: `crates/dramaturge/src/lib.rs`

- [ ] **Step 1: Replace crate-level `//!` block**

Replace lines 1–17 (the existing `//!` block before `pub use`) with:

```rust
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
//! │  dramaturge-macros    derive macros for ergonomics   │
//! │  dramaturge-serde     optional serde / YAML support  │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! The model crate has no serde or runtime dependency. The runtime crate knows
//! nothing about serialization. You can depend on individual crates or on this
//! facade for the full public surface.
//!
//! # Extending the System
//!
//! - **Custom conditions** — add `Condition::Custom { key, args }` to your graphs
//!   and implement [`ConditionEvaluator`] to handle them. See
//!   [`dramaturge_runtime::evaluation`] for the trait.
//! - **Custom tags** — implement the [`DialogTag`] trait on your own enum or use
//!   [`CustomDialogTag`] directly. See [`dramaturge_model::tag`] for details.
//! - **Custom outcomes** — implement [`OutcomeApplier`] to handle `Outcome::Custom`
//!   and apply game-specific side effects.
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
```

- [ ] **Step 2: Run doctest to verify example compiles**

```bash
cargo test --all-features --doc -p dramaturge 2>&1 | tail -10
```

Expected: `test result: ok. 1 passed`

- [ ] **Step 3: Verify doc build**

```bash
cargo doc --all-features --no-deps -p dramaturge 2>&1 | grep "^error" | head -10
```

Expected: no output (no errors).

- [ ] **Step 4: Commit**

```bash
git add crates/dramaturge/src/lib.rs
git commit -m "docs: expand dramaturge facade crate doc with design philosophy and architecture"
```

---

### Task 3: Expand `dramaturge-model` crate-level doc

**Files:**
- Modify: `crates/dramaturge-model/src/lib.rs`

- [ ] **Step 1: Replace crate-level `//!` block**

The current block ends before `pub mod alignment;`. Replace the entire `//!` section (lines 1 through the last `//!` line before `pub mod`) with:

```rust
//! Core domain model for the `dramaturge` dialogue toolkit.
//!
//! This crate provides game-agnostic foundational types for branching dialogue
//! systems. It is intentionally free of serde and runtime dependencies.
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
//! modify a relationship, add a memory, emit an event, or a custom extension.
//! Applied by [`OutcomeApplier`](dramaturge_runtime::OutcomeApplier) at runtime.
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
//! ## Tags
//!
//! [`DialogTag`] is a trait for axis/key annotation on dialogue options.
//! [`CustomDialogTag`] is the built-in implementation. Implement [`DialogTag`]
//! on your own enum for a type-safe tag system with derive macro support.
```

- [ ] **Step 2: Verify doc build**

```bash
cargo doc --all-features --no-deps -p dramaturge-model 2>&1 | grep "^error" | head -10
```

Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add crates/dramaturge-model/src/lib.rs
git commit -m "docs: expand dramaturge-model crate doc with concept grouping narrative"
```

---

### Task 4: Expand `dramaturge-runtime` crate-level doc

**Files:**
- Modify: `crates/dramaturge-runtime/src/lib.rs`

- [ ] **Step 1: Replace crate-level `//!` block**

Replace lines 1–13 (the existing `//!` block) with:

```rust
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
//!   variants. [`Condition::Custom`] always returns `false` in the default;
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
//!             Condition::Custom { key, args } if key == "is_daytime" => {
//!                 // delegate to game engine
//!                 false // placeholder
//!             }
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
```

- [ ] **Step 2: Check `Outcome::Custom` variant exists**

```bash
grep -n "Custom" crates/dramaturge-model/src/outcome.rs | head -10
```

Expected: at least one line showing `Custom` variant. If not present, adjust the `OutcomeApplier` example to use an existing variant instead.

- [ ] **Step 3: Run doctests**

```bash
cargo test --all-features --doc -p dramaturge-runtime 2>&1 | tail -10
```

Expected: `test result: ok.` (all doctests pass).

- [ ] **Step 4: Verify doc build**

```bash
cargo doc --all-features --no-deps -p dramaturge-runtime 2>&1 | grep "^error" | head -10
```

Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/dramaturge-runtime/src/lib.rs
git commit -m "docs: expand dramaturge-runtime crate doc with evaluation model and extension points"
```

---

### Task 5: Expand `dramaturge-authoring`, `dramaturge-macros`, and `dramaturge-serde` crate docs

**Files:**
- Modify: `crates/dramaturge-authoring/src/lib.rs`
- Modify: `crates/dramaturge-macros/src/lib.rs`
- Modify: `crates/dramaturge-serde/src/lib.rs`

- [ ] **Step 1: Expand `dramaturge-authoring` crate `//!` block**

The existing block ends at line 7 (`use std::collections...`). Append after the existing intro:

```rust
//!
//! # When to Use
//!
//! Run the validator at authoring time — in CI, in an editor plugin, or as
//! a build step — not at game runtime. Validation catches structural problems
//! early, before they manifest as silent failures during play.
//!
//! # Severity Levels in Practice
//!
//! - [`ValidationSeverity::Error`] — graph is broken; the dialogue cannot
//!   run correctly. Ship-blocker. Examples: option targets a non-existent
//!   node, entry node missing.
//! - [`ValidationSeverity::Warning`] — likely a mistake but not fatal.
//!   Examples: unreachable node, always-false condition, orphaned fact reference.
//! - [`ValidationSeverity::Info`] — informational. Dead-end nodes at the
//!   narrative leaves are common and expected.
//!
//! # Example
//!
//! ```rust
//! use dramaturge_authoring::GraphValidator;
//! use dramaturge_model::DialogueGraph;
//!
//! let graph = DialogueGraph::default();
//! let issues = GraphValidator::validate(&graph);
//! // An empty graph has no issues.
//! assert!(issues.is_empty());
//! ```
```

- [ ] **Step 2: Run authoring doctest**

```bash
cargo test --all-features --doc -p dramaturge-authoring 2>&1 | tail -10
```

Expected: `test result: ok.`

- [ ] **Step 3: Expand `dramaturge-macros` crate `//!` block**

The existing block covers all four macros but lacks short inline examples for `FactValue` and `ConditionBuilder`. Find the end of the existing `//! # \`#[derive(DialogTag)]\`` section and add after it:

```rust
//!
//! # `#[derive(FactValue)]`
//!
//! Generates typed fact helpers and condition builders for a user-defined
//! fact enum. Each variant becomes a `Fact` constructor and a `Condition`
//! builder method.
//!
//! ```ignore
//! use dramaturge_macros::FactValue;
//!
//! #[derive(FactValue)]
//! enum MyFact {
//!     QuestComplete,
//!     GoldAmount(i64),
//! }
//! ```
//!
//! # `#[derive(ConditionBuilder)]`
//!
//! Generates fluent condition construction methods on a fact enum, enabling
//! `MyFact::QuestComplete.is_true()` style condition building.
//!
//! ```ignore
//! use dramaturge_macros::ConditionBuilder;
//!
//! #[derive(ConditionBuilder)]
//! enum MyFact {
//!     QuestComplete,
//! }
//! ```
//!
//! # `#[derive(Dramaturge)]`
//!
//! Placeholder derive macro. Currently a no-op; reserved for future use.
```

- [ ] **Step 4: Expand `dramaturge-serde` crate `//!` block**

Replace the existing `//!` block (lines 1–12) with:

```rust
//! Optional serialization integration for the `dramaturge` toolkit.
//!
//! # Features
//!
//! - `serde` — activates [`serde`](https://docs.rs/serde) support across the
//!   model types. Enable with `dramaturge-serde = { features = ["serde"] }`.
//! - `yaml` — enables the YAML dialogue loader (implies `serde`). Requires the
//!   `yaml` feature: `dramaturge-serde = { features = ["yaml"] }`.
//!
//! # YAML Loader
//!
//! With the `yaml` feature, load a [`DialogueGraph`] from a YAML file:
//!
//! ```ignore
//! use dramaturge_serde::loader::YamlDialogueGraph;
//!
//! let raw = std::fs::read_to_string("dialogue.yaml").unwrap();
//! let graph = YamlDialogueGraph::from_str(&raw)
//!     .expect("invalid YAML")
//!     .into_graph()
//!     .expect("invalid graph structure");
//! ```
//!
//! See `crates/dramaturge/examples/dialogue.yaml` for the full YAML format.
//!
//! # Why a Separate Crate?
//!
//! `dramaturge-model` intentionally has no serde dependency so that game
//! engines that serialize state through other means (binary formats, custom
//! reflection, etc.) pay zero cost. Add this crate only when you need it.
```

- [ ] **Step 5: Verify doc builds for all three crates**

```bash
cargo doc --all-features --no-deps -p dramaturge-authoring -p dramaturge-macros -p dramaturge-serde 2>&1 | grep "^error" | head -10
```

Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add crates/dramaturge-authoring/src/lib.rs crates/dramaturge-macros/src/lib.rs crates/dramaturge-serde/src/lib.rs
git commit -m "docs: expand authoring, macros, and serde crate docs"
```

---

### Task 6: Add Custom extension docs to `condition.rs` and `tag.rs`

**Files:**
- Modify: `crates/dramaturge-model/src/condition.rs`
- Modify: `crates/dramaturge-model/src/tag.rs`

- [ ] **Step 1: Add Custom condition section to `condition.rs`**

Append after the last `//!` line of the existing intro (before `use crate::alignment`):

```rust
//!
//! # Custom Conditions
//!
//! When built-in variants don't cover a game mechanic, use
//! `Condition::Custom { key, args }`. The default evaluator always returns
//! `false` for custom conditions — you supply the logic by implementing
//! [`ConditionEvaluator`](https://docs.rs/dramaturge-runtime/latest/dramaturge_runtime/trait.ConditionEvaluator.html).
//!
//! ```rust
//! use dramaturge_model::{Condition, FactValue};
//!
//! // "Is it currently daytime?" — handled by your game engine
//! let cond = Condition::Custom {
//!     key: "is_daytime".to_string(),
//!     args: vec![],
//! };
//!
//! // "Does the player have at least 50 gold?" — with an argument
//! let cond_with_arg = Condition::Custom {
//!     key: "has_gold".to_string(),
//!     args: vec![FactValue::Int(50)],
//! };
//! ```
//!
//! In your evaluator, match on `key` and delegate non-custom arms to
//! [`DefaultConditionEvaluator`](https://docs.rs/dramaturge-runtime/latest/dramaturge_runtime/struct.DefaultConditionEvaluator.html):
//!
//! ```rust
//! // See dramaturge_runtime::evaluation for the full extension example.
//! ```
```

- [ ] **Step 2: Add CustomDialogTag extension section to `tag.rs`**

Append after the last `//!` line of the existing intro (before `use std::sync::Arc`):

```rust
//!
//! # Custom Tag Types
//!
//! Use [`CustomDialogTag`] for ad-hoc axis/key pairs:
//!
//! ```rust
//! use dramaturge_model::CustomDialogTag;
//!
//! // Mass Effect style
//! let paragon = CustomDialogTag::new("moral", "paragon");
//! let renegade = CustomDialogTag::new("moral", "renegade");
//!
//! // Skill-gated options
//! let medic = CustomDialogTag::skill("medic");       // axis = "skill"
//! let hacker = CustomDialogTag::skill("hacking");
//!
//! // With a UI prefix shown to the player
//! let charmed = CustomDialogTag::with_prefix("social", "charm", "[Charm]");
//! ```
//!
//! Or implement the [`DialogTag`] trait on your own enum for compile-time
//! safety and derive macro support:
//!
//! ```rust
//! use std::fmt::Debug;
//! use dramaturge_model::DialogTag;
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//! enum SpectreTone {
//!     Paragon,
//!     Renegade,
//! }
//!
//! impl DialogTag for SpectreTone {
//!     fn axis(&self) -> &str { "moral" }
//!     fn key(&self) -> &str {
//!         match self {
//!             SpectreTone::Paragon => "paragon",
//!             SpectreTone::Renegade => "renegade",
//!         }
//!     }
//! }
//! ```
//!
//! Use `#[derive(DialogTag)]` from `dramaturge-macros` to generate the
//! `DialogTag` impl automatically from `#[tag(axis = "...", key = "...")]`
//! attributes on variants.
```

- [ ] **Step 3: Check `DialogTag` trait signature**

```bash
grep -A5 "pub trait DialogTag" crates/dramaturge-model/src/tag.rs
```

Verify `axis()` and `key()` match the actual method names. Adjust the `SpectreTone` example if they differ.

- [ ] **Step 4: Run doctests**

```bash
cargo test --all-features --doc -p dramaturge-model 2>&1 | tail -15
```

Expected: all doctests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/dramaturge-model/src/condition.rs crates/dramaturge-model/src/tag.rs
git commit -m "docs: add Custom variant extension docs to condition and tag modules"
```

---

### Task 7: Add custom evaluator extension guide to `evaluation.rs`

**Files:**
- Modify: `crates/dramaturge-runtime/src/evaluation.rs`

- [ ] **Step 1: Check `Outcome::Custom` variant**

```bash
grep -n "Custom" crates/dramaturge-model/src/outcome.rs
```

Note the exact field names of the `Custom` variant. Use them in Step 2.

- [ ] **Step 2: Append to `evaluation.rs` `//!` block**

Add after the last `//!` line (before `use dramaturge_model`):

```rust
//!
//! # Implementing a Custom Evaluator
//!
//! Wrap [`DefaultConditionEvaluator`] and handle only your custom keys:
//!
//! ```rust
//! use dramaturge_runtime::{ConditionEvaluator, DefaultConditionEvaluator, DialogWorldState};
//! use dramaturge_model::{Condition, FactValue};
//!
//! struct GameEvaluator {
//!     inner: DefaultConditionEvaluator,
//! }
//!
//! impl ConditionEvaluator for GameEvaluator {
//!     fn evaluate(&self, condition: &Condition, ctx: &DialogWorldState) -> bool {
//!         match condition {
//!             Condition::Custom { key, args } if key == "is_daytime" => {
//!                 // Your game engine call goes here.
//!                 false
//!             }
//!             Condition::Custom { key, args } if key == "has_gold" => {
//!                 let min = args.first()
//!                     .and_then(|v| if let FactValue::Int(n) = v { Some(*n) } else { None })
//!                     .unwrap_or(0);
//!                 // Query your game's gold counter here.
//!                 false
//!             }
//!             other => self.inner.evaluate(other, ctx),
//!         }
//!     }
//! }
//! ```
//!
//! # Implementing a Custom Outcome Applier
//!
//! Wrap [`DefaultOutcomeApplier`] and handle only your custom keys:
//!
//! ```rust
//! use dramaturge_runtime::{OutcomeApplier, DefaultOutcomeApplier, DialogWorldState};
//! use dramaturge_model::Outcome;
//!
//! struct GameApplier {
//!     inner: DefaultOutcomeApplier,
//! }
//!
//! impl OutcomeApplier for GameApplier {
//!     fn apply(&mut self, outcome: &Outcome, state: &mut DialogWorldState) {
//!         match outcome {
//!             Outcome::Custom { key, args } if key == "unlock_achievement" => {
//!                 // Fire your achievement system here.
//!             }
//!             other => self.inner.apply(other, state),
//!         }
//!     }
//! }
//! ```
```

- [ ] **Step 3: Verify `Outcome::Custom` field names match**

If Step 1 showed different field names (e.g., `id` instead of `key`), update the `GameApplier` example to use the correct names.

- [ ] **Step 4: Run doctests**

```bash
cargo test --all-features --doc -p dramaturge-runtime 2>&1 | tail -15
```

Expected: all doctests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/dramaturge-runtime/src/evaluation.rs
git commit -m "docs: add custom evaluator and outcome applier extension guide"
```

---

### Task 8: Add multi-game examples to `alignment.rs` and `fact.rs`

**Files:**
- Modify: `crates/dramaturge-model/src/alignment.rs`
- Modify: `crates/dramaturge-model/src/fact.rs`

- [ ] **Step 1: Append multi-game examples to `alignment.rs` `//!` block**

Add after the last `//!` line (before `use std::sync::Arc`):

```rust
//!
//! # Multi-Game Examples
//!
//! **Mass Effect** — moral alignment with Paragon and Renegade poles:
//!
//! ```rust
//! use dramaturge_model::{AlignmentAxis, AlignmentPole, AlignmentAxisId, AlignmentPoleId, Condition};
//!
//! let mut morality = AlignmentAxis::new("morality", "Moral Alignment");
//! morality.add_pole(AlignmentPole::new("paragon", "Paragon"));
//! morality.add_pole(AlignmentPole::new("renegade", "Renegade"));
//!
//! // Check if the player has enough Paragon score to unlock the option
//! let condition = Condition::AlignmentAtLeast {
//!     axis: AlignmentAxisId::new("morality"),
//!     pole: AlignmentPoleId::new("paragon"),
//!     value: 60,
//! };
//! ```
//!
//! **Generic RPG** — faction reputation axis:
//!
//! ```rust
//! use dramaturge_model::{AlignmentAxis, AlignmentPole};
//!
//! let mut rep = AlignmentAxis::new("reputation", "Guild Reputation");
//! rep.add_pole(AlignmentPole::new("honored", "Honored"));
//! rep.add_pole(AlignmentPole::new("reviled", "Reviled"));
//! ```
```

- [ ] **Step 2: Check `AlignmentAxisId` / `AlignmentPoleId` `From<&str>` impl**

```bash
grep -n "impl From\|impl Into\|from_str\|AlignmentAxisId\|AlignmentPoleId" crates/dramaturge-model/src/alignment.rs | head -10
```

If `"morality".into()` does not work directly for `AlignmentAxisId`, replace with `AlignmentAxisId::new("morality")` and `AlignmentPoleId::new("paragon")` in the `Condition::AlignmentAtLeast` example.

- [ ] **Step 3: Append multi-game examples to `fact.rs` `//!` block**

Add after the last `//!` line (before `use std::sync::Arc`):

```rust
//!
//! # Multi-Game Examples
//!
//! **Pokémon** — friendship fact scoped to the player:
//!
//! ```rust
//! use dramaturge_model::{Fact, FactScope};
//!
//! // Track Pikachu's friendship score
//! let friendship = Fact::int("friendship:pikachu", 120);
//! // Scoped to the player — other characters don't see it
//! let scope = FactScope::Player;
//! ```
//!
//! **Mass Effect** — global mission completion flag:
//!
//! ```rust
//! use dramaturge_model::{Fact, FactScope};
//!
//! // Objective truth — every character knows the mission is done
//! let mission_done = Fact::bool("mission:eden_prime_complete", true);
//! let scope = FactScope::World;
//! ```
//!
//! **Asymmetric knowledge** — only Mara knows about the hidden shipment:
//!
//! ```rust
//! use dramaturge_model::{Character, EntityId, CharacterId, Fact, FactScope};
//!
//! let mara_id = CharacterId::new(EntityId::new(2));
//! let scope = FactScope::Character(mara_id);
//! let fact = Fact::bool("knows_about_hidden_shipment", true);
//! // The player and other characters do not see this fact
//! ```
```

- [ ] **Step 4: Run doctests**

```bash
cargo test --all-features --doc -p dramaturge-model 2>&1 | tail -15
```

Expected: all doctests pass. If any fail, fix the code in the failing example (most likely an API mismatch — check the constructor signatures in the relevant file).

- [ ] **Step 5: Verify full doc build**

```bash
cargo doc --all-features --no-deps 2>&1 | grep "^error" | head -10
```

Expected: no output.

- [ ] **Step 6: Run full test suite**

```bash
task test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/dramaturge-model/src/alignment.rs crates/dramaturge-model/src/fact.rs
git commit -m "docs: add multi-game examples to alignment and fact modules"
```
