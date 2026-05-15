# Documentation Design — dramaturge

**Date:** 2026-05-15
**Issue:** #36 — Write documentation
**Approach:** Rustdoc-centric (A)
**Audience:** Game developers integrating dramaturge into their engines

---

## Goals

- Rewrite `README.md` to reflect the current, implemented state of the library
- Expand crate-level `//!` docs to explain design philosophy and entry points
- Add targeted module-level `//!` additions for extension and multi-game examples
- Surface Custom variant extension points and multi-game patterns throughout

---

## README.md

Rewrite top-to-bottom. Structure:

1. **Tagline + CI badge** — keep existing
2. **One-paragraph description** — what dramaturge is, what problem it solves, why game-agnostic (no Pokemon/Mass Effect knowledge baked in)
3. **Quick-start snippet** — minimal working code: create a character, set a fact, build a condition, check it against world state
4. **Architecture table** — update existing table; remove "placeholder" notes from `dramaturge-macros`; reflect actual implemented state
5. **Full example callout** — link to `the_broken_seal` example with one sentence on what it demonstrates (YAML-loaded dialogue, branching conditions, relationship outcomes)
6. **License** — keep existing

Remove the "Bootstrap / Prototype — APIs are unstable" warning; it no longer reflects reality.

---

## Crate-level `//!` docs

### `dramaturge` (facade)

Primary home for design philosophy. Sections to add after existing intro:

- **Design philosophy** — game-agnostic by construction: no hardcoded game rules, all game-specific logic expressed through typed IDs, tags, facts, and state changes
- **Layered architecture** — model (pure domain types) → runtime (evaluation traits + default impls) → authoring (validation) → macros (ergonomics) → serde (serialization)
- **How to extend** — brief overview directing readers to Custom variants and evaluator traits; links to relevant modules

### `dramaturge-model`

Expand existing type inventory intro with a concept grouping narrative:
- Characters and entities are identity, not behavior
- Facts are the unit of knowledge; scope controls who knows what
- Conditions are composable boolean trees over world state
- Outcomes are declarative state changes
- Dialogue graphs are pure data — no evaluation logic

### `dramaturge-runtime`

Add sections:
- **Evaluation model** — how `DefaultConditionEvaluator` walks the `Condition` tree, how `DefaultOutcomeApplier` mutates `DialogWorldState`
- **Extension points** — implement `ConditionEvaluator` for custom condition logic; implement `OutcomeApplier` for custom state changes; implement `DialogueRuntime` for a fully custom engine

### `dramaturge-authoring`

Expand with:
- When to run validation (build time, CI, editor tooling — not at game runtime)
- Severity levels in practice: Error = ship-blocker, Warning = worth reviewing, Info = informational

### `dramaturge-macros`

One section per derive macro:
- `#[derive(DialogTag)]` — annotate enum variants with `#[tag(axis, key)]`
- `#[derive(FactValue)]` — generate typed fact helpers and condition builders for a fact enum
- `#[derive(ConditionBuilder)]` — fluent condition construction on a fact enum
- `#[derive(Dramaturge)]` — placeholder, no-op

### `dramaturge-serde`

- Feature flags: `serde` and `yaml` (implies `serde`)
- YAML loader: how to load a `DialogueGraph` from a YAML file via `YamlDialogueGraph`
- Direct to `dialogue.yaml` example for format reference

---

## Module-level `//!` additions

Targeted additions only; existing intros are sufficient.

### `condition.rs`

Add section: **Custom conditions**
- When built-in variants don't cover a game mechanic, use `Condition::Custom(Box<dyn CustomCondition>)`
- Show trait definition and a short example (e.g., "IsQuestActive" for an RPG)

### `tag.rs`

Add section: **Custom dialog tags**
- `CustomDialogTag` lets games define their own tag namespaces
- Show implementing the `DialogTag` trait for a game-specific enum
- Example: Mass Effect `SpectreTone` vs Pokemon `RivalityTag`

### `evaluation.rs`

Add section: **Implementing custom evaluators**
- `ConditionEvaluator` trait: implement `evaluate` to handle `Condition::Custom` variants
- `OutcomeApplier` trait: implement `apply` to handle custom outcome types or side effects
- Pattern: wrap `DefaultConditionEvaluator` and delegate non-custom arms

### `alignment.rs`

Add multi-game examples in doc:
- Mass Effect: axis `"morality"`, poles `"paragon"` / `"renegade"`
- Generic RPG: axis `"reputation"`, poles `"hero"` / `"villain"`

### `fact.rs`

Add multi-game fact scope examples:
- Pokemon: `FactScope::Player`, key `"friendship:pikachu"`, value `FactValue::Int(120)`
- Mass Effect: `FactScope::Global`, key `"mission:eden_prime_complete"`, value `FactValue::Bool(true)`

---

## Out of scope

- mdBook or rendered documentation site
- Contributor/architecture docs (audience is game developers, not contributors)
- Doctest coverage for every public type (only where examples add real clarity)
