# Changelog

All notable changes to this project are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `Fact::new(id, value)`, and `From<&str>`, `From<String>` and
  `From<Arc<str>>` for `FactId` (#66).
- `MemoryQuery` builder methods: `with_tag`, `with_min_confidence`,
  `with_min_emotional_weight`, `with_subject` and `with_fact` (#66).
- `Outcome::AddFact { scope, fact_id, delta }` adds to an integer fact. The
  default applier starts a missing fact at 0, leaves a non-integer fact
  unchanged, saturates at the `i64` limits and emits `FactLearned`. YAML:
  `{ type: add_fact, fact: gold, delta: -5 }` (optional `scope`). The
  validator counts `AddFact` as a writer of the fact, and
  `OutcomeDto::AddFact` serializes it (#70).
- Relationship defaults per axis on `DialogWorldState`:
  `set_relationship_default`, `remove_relationship_default`,
  `relationship_default` and `relationship_defaults`. `get_relationship`
  returns the default for an untracked pair, so `RelationshipAtMost` and
  `RelationshipAtLeast` hold at the start; `add_relationship` starts an
  untracked score at the default. Without a default nothing changes.
  Defaults count in `fingerprint` and `same_state`, and `WorldStateDto`
  stores them (`relationship_defaults`, `RelationshipDefaultDto`) (#63).
- `DialogWorldState::set_relationship` sets a score directly (#63).
- `MemoryQuery::id`, `MemoryQuery::by_id` and `MemoryQuery::with_id` match a
  memory by its `MemoryId`. `MemoryQueryDto` has the optional `id`, mapped
  through the `IdMap` like other memory ids. YAML condition
  `{ type: has_memory, owner: mara, id: mara.saw_the_seal }` interns the
  memory key into `symbols.memories` (#64).
- `ValidationIssueKind` (`MissingTarget`, `NoEntryNode`, `EntryNodeNotFound`,
  `Unreachable`, `DeadEnd`, `SelfLoop`, `Cycle`, `AlwaysFalse`,
  `OrphanFact`, `SourceNodeMismatch`, `EmptyText`, `UnkeyedProgress`) and
  `ValidationIssue::kind`, so hosts filter issues by kind instead of by
  message text (#65).
- `ValidationOptions` (`skip`, `report`, `allow_cycles`, `allow_dead_ends`,
  `skip_orphan_facts`) and `GraphValidator::with_options` switch checks off
  (#65).

### Changed

- **Breaking:** public types that will grow are now `#[non_exhaustive]`, so
  new variants and fields in later 0.3.x releases are not breaking (#66):
  - Enums (a `match` outside dramaturge needs a `_` arm): `Outcome`,
    `Condition`, `DialogText`, `DialogTarget`, `NavigationHint`,
    `FragmentMode`, `EventKind`, `MemoryTag`, `FactScope`, `Intention`
    (`dramaturge-model`); `ValidationSeverity` (`dramaturge-authoring`);
    `ResolvedText` (`dramaturge-runtime`); `SymbolKind`
    (`dramaturge-serde`).
  - Structs (no struct literals and no `..` update syntax outside
    dramaturge; use the constructors): `Fact` (`Fact::new`, `Fact::int`,
    ...), `MemoryQuery` (`MemoryQuery::new`, `by_*`, `with_*`),
    `ValidationIssue` (`ValidationIssue::new`, `with_node`, `with_option`),
    `RestoreReport`, `WorldStateDto` (`From<&DialogWorldState>`,
    `Default`). The fields stay public for reading and writing.
  - Not changed: `FactValue`, `Confidence`, the id types, and the graph
    types `DialogueGraph`, `DialogueNode`, `DialogueOption` and
    `TextFragment`.
- **Breaking:** the YAML loader no longer accepts numeric ids (deprecated
  in 0.2, ADR 0001). A YAML number as node, option, target, entry node or
  relationship endpoint id, and a character `id:`, are load errors with a
  message that says to use a key. A number in quotes (`id: "2"`) is a key.
  `YamlKey` is now a string newtype (`YamlKey(String)`, `as_key()` returns
  `&str`), and `YamlCharacter::id` is removed (#67).
- **Breaking:** `ValidationIssue::new` takes the issue kind as its first
  argument: `ValidationIssue::new(kind, severity, message)`. `GraphValidator`
  is no longer a unit struct; build it with `GraphValidator::new()`,
  `default()` or `with_options` (#65).
- The default evaluator treats an unknown `Condition` as `false`, the
  default applier ignores an unknown `Outcome`, and `resolve_text` treats an
  unknown `FragmentMode` as `Always` (#66).

## [0.3.0-alpha.1] - 2026-11-07

### Changed

- **Breaking:** `NarrativeState::fact` returns `Option<Cow<'_, FactValue>>`
  instead of `Option<&FactValue>`. A host can now implement `NarrativeState`
  on its own store and return converted values (`Cow::Owned`). Types that
  implement `HasDialogWorldState` return `Cow::Borrowed` and do not copy.
  Callers that need a reference use `as_ref()` or `&*value`.
- The `NarrativeState` documentation shows two paths: embed a
  `DialogWorldState`, or implement `NarrativeState` directly on a host store.

## [0.2.0] - 2026-10-30

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
  in 0.2 with a deprecation warning; support is removed in 0.3 (#55).
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

## [0.1.0] - 2026-05-15

Initial release.

[Unreleased]: https://github.com/sunsided/dramaturge/compare/v0.3.0-alpha.1...HEAD
[0.3.0-alpha.1]: https://github.com/sunsided/dramaturge/compare/v0.2.0...v0.3.0-alpha.1
[0.2.0]: https://github.com/sunsided/dramaturge/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/sunsided/dramaturge/releases/tag/v0.1.0
