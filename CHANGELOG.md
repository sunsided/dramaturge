# Changelog

All notable changes to this project are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased] — 0.2.0

### Added

- `SymbolTable` and `Interner` in `dramaturge-model`: stable author keys mapped
  to dense runtime ids, one namespace per id kind (#55).
- `GraphValidator::validate_with_symbols` names nodes and options by key (#55).
- `NarrativeState` trait in `dramaturge-runtime` with the operations the default
  evaluator and applier use; `HasDialogWorldState` gives any host type that
  embeds a `DialogWorldState` a forwarding implementation (#56).
- `CustomHandler` hook (default `NoCustom`) for `Condition::Custom` and
  `Outcome::Custom`. The default evaluator keeps recursing through
  `All`/`Any`/`Not`, so nested custom conditions reach the handler (#56).
- `DefaultRuntime::with_parts`, `DefaultConditionEvaluator::with_custom`,
  `DefaultOutcomeApplier::with_custom`, and a `host_state` example (#56).
- `DialogWorldState::fingerprint`, `hash_logical` and `same_state` compare the
  logical state and ignore the event log (#59).
- `DialogWorldState` iterators and helpers: `facts()`, `facts_in()`,
  `fact_count()`, `alignments()`, `relationships()`, `memories()`,
  `available_tags()`, `remove_available_tag()`, `drain_events()` (#59).
- `Ord` on all id and key types (`EntityId`, `FactScope`, `FactId`, axis and
  pole ids, `CustomDialogTag` by axis/key); `Hash` on `Memory` (#59).
- A criterion benchmark for world-state lookups (`cargo bench -p dramaturge-runtime`).
- `dramaturge_serde::dto`: lossless serde DTOs for every public model type and
  `DialogWorldState`, with `IntoModel` conversions and an `IdMap` hook (#57).
- `with` modules `dramaturge_serde::{world_state, condition, outcome,
  dialogue_graph, symbol_table}` for host structs (#57).
- `dramaturge_serde::save::SavedWorldState`: saves store keys and re-resolve ids
  against the current `SymbolTable` on restore. `restore` returns a
  `RestoreReport` listing keys the current content no longer defines (#57).
- `DialogWorldState::set_next_event_id` (#57).
- CI builds `dramaturge-serde` and `dramaturge-runtime` for `wasm32-unknown-unknown`.
- `DialogText::Fragments` with `TextFragment` and `FragmentMode` (`Always`,
  `FirstTime`, `Once`, `Cycle`, `Shuffle`, `Sticky`) for state-dependent
  lines without copying nodes (#58).
- `resolve_text` in `dramaturge-runtime`: evaluates fragments, tracks
  `TextProgress`, and returns `ResolvedPiece`s with their source fragment.
  Deterministic for a seed via the `TextRng` trait and `SplitMix64` (#58).
- YAML `text:` accepts fragment lists; YAML conditions gain `fact_at_least` and
  `fact_at_most` (#58).
- Validator warning for fragment texts that can resolve to empty text (#58).
- Validator warning for `Once`/`Cycle`/`Sticky` fragments without a key, whose
  progress resets when a fragment is inserted before them (#58).
- `TextProgressDto` and the `text_progress` `with` module (#58).
- `partial_eq_values` is public (epsilon float comparison used by the evaluator).
- CI lints GitHub Actions workflows with actionlint and zizmor.

### Changed

- **Breaking:** the YAML format is now 2.0. All references are keys
  (`target: mara.confession.soft`, `source: player`). Numeric ids still load
  for one release with a deprecation warning (#55).
- **Breaking:** `YamlDialogueGraph::to_model` returns `LoadedDialogue
  { graph, symbols, warnings }` (#55).
- **Breaking:** `ConditionEvaluator<S>`, `OutcomeApplier<S>` and
  `DialogueRuntime<S>` are generic over the state. `S` defaults to
  `DialogWorldState`, so most `impl` blocks and annotations compile unchanged (#56).
- **Breaking:** `DefaultConditionEvaluator` and `DefaultOutcomeApplier` are no
  longer unit structs; construct them with `::new()` / `::default()`.
  `DefaultRuntime<E, A>` and `AvailableOptionsIter<'a, E, S>` gained type
  parameters with defaults (#56).
- **Breaking:** `DialogWorldState::remove_fact` returns `bool` (#56).
- Custom evaluators that wrapped the default and re-implemented the
  `All`/`Any`/`Not` recursion can become a `CustomHandler` instead (#56).
- **Breaking:** `DialogWorldState` stores facts, alignments and relationships
  in ordered maps; lookups are `O(log n)` instead of linear scans and
  iteration order is deterministic. Its fields are private; use the
  iterator methods. `get_memories` returns `&[Memory]` (#59).
- **Breaking:** `DialogText` is no longer `Eq`/`Hash` (fragments carry
  conditions), and exhaustive matches need a `Fragments` arm (#58).
- **Breaking:** `YamlNode::text` is a `YamlText`, `YamlOption::text` an
  `Option<YamlText>` (#58).
- **Breaking:** the `yaml` feature uses `serde-saphyr` instead of the
  deprecated `serde_yaml` 0.9. `YamlDialogueGraph::from_yaml` returns
  `YamlError` (with `line()`/`column()`), which hides the backend (#60).
- `dramaturge-serde` depends on `dramaturge-runtime` (#57).
- `available_tags` is a set: adding the same tag twice stores it once (#59).
- Validator messages print ids as `'key'` or `#<number>` instead of `ID <number>`.
