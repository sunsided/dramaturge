# Changelog

All notable changes to this project are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.0] - 2026-11-08

### Added

- `dramaturge::serialization` re-exports `dramaturge-serde` (feature `serde` or `yaml`), so `dramaturge = { features = ["yaml"] }` alone gives `dramaturge::serialization::loader::load_yaml`. The feature `yaml` now implies `serde`.

### Fixed

- `#[derive(DialogTag)]`, `FactValue` and `ConditionBuilder` work in a crate that depends only on `dramaturge`: they fall back to the new re-export `dramaturge::model` (all of `dramaturge-model`) instead of `::dramaturge_model`.
- The `#[derive(DialogTag)]` example in the `dramaturge-macros` docs compiles and runs as a doctest; it derives `Debug`, `Clone`, `PartialEq`, `Eq` and `Hash`, which the `DialogTag` trait needs.
- **Breaking:** a text pool candidate key or a dialogue fragment key that would share progress with another one is now a load error. A key that is empty, digits only (the implicit key of an unkeyed entry), has a `/` (an inline pool path) or repeats on its progress path (one pool or fragment list, or all inline pools of one candidate) fails `lower`, `lower_pool` (so `load_yaml`, `load_pool_yaml` and `dialogue!`) with the new `IssueKind::BadKey`, and `GraphValidator::validate_pools`/`validate_pool` report the new `ValidationIssueKind::BadKey`. `Candidate::is_valid_key`, `Pool::bad_key` and `DialogText::bad_key` check the rule. The runtime does not check keys.
- A candidate whose pool (named or inline) resolves to no leaf records nothing on `commit`: no progress and no `learn`. Its trace row has the new `Verdict::NoLeaf` and no variant.
- A candidate whose named pools cannot open does not draw from the RNG, and its trace row has no variant.
- `add_relationship` saturates at the `i32` bounds instead of overflowing, as `Outcome::AddFact` does.
- A failed `load_pool_yaml` leaves the caller's `SymbolTable` unchanged.
- `dialogue!`: a negative number in a condition (`when gold >= -3;`) parses. The tokens become `gold >= - 3`, and the condition language now reads `- 3` as `-3`.
- `condition::parse` returns an error for nesting of `(` and `not` deeper than 128 levels instead of overflowing the stack.
- `condition::Expr::lower` returns a `Result`: an ordered compare or a relation with a value that is not a number, or a relation number out of the `i32` range, is an error instead of being coerced to `0` or truncated.
- `dialogue!` points an error that lowering finds (an option with an empty label) at its option instead of at the first token, and names the missing dependency with an error instead of emitting an unresolved `::dramaturge_syntax` path.
- The `NextLoop` check reports a loop that no entry node reaches as a warning, as `Cycle` does, instead of an error.
- `.gitignore` ignores only the workspace `/target/` and `/ci/facade-only/target/`, so a `target` directory in a crate stays in the package.

## [0.3.0-alpha.5] - 2026-11-08

### Added

- Text selection core (decision D2, design note
  `docs/specs/2026-11-07-text-selection-design.md`). `dramaturge-model`
  module `pool`: `Pool` (an ordered list of `Candidate`s and a `Pick` rule:
  `All`, `First` (the default) or `Best`), `Candidate` (key, tuple keys
  `speaker` and `topic`, `when`, `mode`, variants, `learn`), `Entry` (a
  leaf, an inline pool, or a named pool by id), `Pools` (named pools by id),
  and `TextTypes`, the host types (text leaf, condition, learn item, actor,
  topic). `DialogTypes<T>` are the types of plain dramaturge. `Specificity`
  gives the score of `Best`: each test counts 1, `and` adds, `or` takes the
  smaller side, and each tuple key counts 1. A tie goes to file order (#79).
- `dramaturge-runtime` module `text::select`: `select` (an inline pool) and
  `select_named` (a named pool) choose what shows for a `Query` (speaker,
  topic, perceiver) with no change to the state. They return a `Selection`
  (`steps`, `shown` leaves, `counts`) with a `Trace` (score, `Verdict` and
  variant for each candidate; `Display` prints it). `commit` records the
  progress and hands each `learn` item to a learner; `world_learner` sets
  the facts in the world scope. `Judge` is the host's condition evaluation;
  `DefaultJudge` walks `All`, `Any` and `Not`, answers `SeenBy` from the
  perceiver and asks a `ConditionEvaluator` for the rest. `ProgressStore`
  is where the progress lives (`TextProgress` implements it). `Count` and
  `CountKey` say where a resolution counts: inline pools per owner (the
  pool id), named pools per pool and caller. Both are `#[non_exhaustive]`
  (#79).
- `Condition::SeenBy(CharacterId)`: true when the perceiver of a text is
  the character. A `ConditionEvaluator` alone has no perceiver, so
  `DefaultConditionEvaluator` returns `false` for it (#79).
- `DialogText::Pool(id)` and `ResolvedText::Pool(id)`: a line that says a
  named text pool. `resolve_text` returns the id as a piece; the host
  selects the pool with `select_named`. `DialogText::as_inline` (#79).
- `TextProgress::named_resolutions`, `set_named_resolutions` and `named`:
  resolution counts of named pools, per pool and caller, in a map apart
  from the owner counts (#79).
- `dramaturge-serde`: `PoolDto`, `CandidateDto`, `EntryDto`, `PickDto`,
  `NamedResolutionsDto`; `loader::load_pool_yaml`, `pool_from_yaml` and
  `pool_to_yaml` for text pools in YAML (#79).
- `dramaturge-syntax`: the text pool AST (`TextPool`, `Candidate`,
  `Variant`, `Pick`) with its YAML shape (`id`, `pick`, `one_of`, and on a
  candidate `key`, `when`, `speaker`, `topic`, `mode`, `text`, `variants`,
  `learn`, or an inline pool), `lower_pool` and `lower_pool_plain`.
  `Text::Pool`: `say: { text: <pool id> }` in a dialogue. `seen_by(x)` in
  the default condition language. `Place::Pool` and `IssueKind::NoPoolId`
  (#79).
- `dramaturge-authoring`: `GraphValidator::validate_pools` and
  `validate_pool` report ties in `best` pools (`PickTie`), `once`, `cycle`,
  `shuffle` and `sticky` candidates without a key (`UnkeyedProgress`), and
  names of pools that do not exist (`UnknownPool`).
  `ValidationIssue::path` gives the place in the pool (#79).
- `dramaturge` facade: re-exports of the text selection types and the
  example `text_selection` (#79).

### Changed

- `resolve_text` runs on the text selection core: a fragment list is a
  `Pick::All` pool with the id `owner`. Its signature, progress keys,
  counts, random draws and results do not change (#79).
- **Breaking:** `ConditionDto` has the variant `SeenBy` and `DialogTextDto`
  has the variant `Pool`. These enums are exhaustive, so a `match` on them
  needs the new arms (#79).
- **Breaking:** `TextProgressDto` has the field `named` (the counts of
  named pools). Code that builds the struct with a literal must set it.
  The field has a serde default: a save without it loads, its `owners`
  counts keep their meaning, and named pools start with no counts (#79).

## [0.3.0-alpha.4] - 2026-11-07

### Added

- New crate `dramaturge-syntax`: one dialogue AST (`Dialogue`, `Node`,
  `Line`, `Choice`, `Target`, `Outcomes`) for YAML, JSON and Rust tokens.
  It holds the start nodes (entry nodes), `lines` and `next`, and the
  outcomes of nodes and options (`set`, `add`, `relation`, `remember`,
  `then`). It is generic over the host's condition, text and effect
  payloads, and over the host's author notes (`notes` on the dialogue,
  nodes and options; the lowering ignores them):
  `Dialogue<C = String, X = Text, E = Effect, M = String>`. `check` reports
  keys and targets as `Issue`s with a `Place` (`nodes.meet.options[1].go`).
  `lower(dialogue, symbols, host)` makes a `DialogueGraph<H, T>` through the
  `Lower` hooks of the host (`condition`, `text`, `effects`, and `set`,
  `add`, `relation`, `remember`, `entity` with defaults). `Plain` and
  `lower_plain` are the hooks of plain dramaturge, with a default condition
  language (`gold >= 10`, `not met`, `remembers(mara, saw_it)`,
  `relation(trust) >= 3`). The feature `serde` adds the YAML and JSON
  shape; the feature `syn` adds the token form (`tokens::parse_dialogue`,
  `ParseHost`, `Spanned`) for `dialogue!` and for `dialogue` blocks in a
  host macro (#76).
- `dialogue!` in `dramaturge-macros`: a dialogue in Rust tokens, checked at
  compile time, that returns `dramaturge_syntax::Loaded` (graph, symbols).
  An unknown target, a duplicate node key, a node with no options and no
  `-> target;`, and a condition that does not parse are compile errors at
  the span of the token (#76).
- `dramaturge` facade: the feature `syntax` re-exports `dramaturge-syntax`
  as `dramaturge::syntax`; the feature `macros` re-exports `dialogue!` and
  the derive macros. Example `dialogue_macro` (#76).
- `dramaturge_serde::loader`: `load_yaml`, `from_yaml` (generic over the
  payloads), `to_yaml` and `LoadError` (#76).

### Changed

- **Breaking:** the YAML dialogue shape of `dramaturge-serde` is the
  dialogue shape of loremaster (decision D3): `with`, `start`, `nodes` as a
  map from key to node, `say`, `when` (a condition string), `options` with
  `say`, `when` and `go` (`end` by default, or `back`), and the outcomes
  `set`, `add`, `relation`, `remember` and `then` on nodes and options.
  The 0.2 shape (`version`, `characters`, a list of `nodes`, `text`,
  `target`, typed `conditions` and `outcomes`) does not load.
  `YamlDialogueGraph` and the other `Yaml*` types are removed: use
  `load_yaml(yaml)`, which returns `LoadedDialogue { graph, symbols,
  warnings }` (now `dramaturge_syntax::Loaded`, with `Issue` warnings).
  The key `player` is the player (`SymbolTable::player`). A `go`, `next` or
  `start` key that names no node is a load error now. The `dramaturge-serde`
  feature `yaml` pulls in `dramaturge-syntax` (without `syn`). `end` and
  `back` (`DialogTarget::Return`) are reserved targets, so no node can have
  these keys. See `docs/migration-0.3-yaml.md` (#76).

- `dramaturge-model` has no dev-dependency on
  `dramaturge-macros` now (`dramaturge-macros` depends on
  `dramaturge-syntax`, which depends on the model). The derive tests moved
  to `dramaturge-macros/tests/derive.rs`. Nothing changes for users; this
  note is for the release order: publish the model before the macros (#76).


## [0.3.0-alpha.3] - 2026-11-07

### Added

- Design note for the text selection core, before R9:
  `docs/specs/2026-11-07-text-selection-design.md` (#78).
- `Outcome::Host(H)`: a typed host outcome. `Outcome::as_host` and
  `Outcome::map_host`; `map_host` also on `DialogueGraph`, `DialogueNode`
  and `DialogueOption`, to give a graph a host outcome type (#72).
- `HostHandler<H, S>` (`host_outcome(&mut self, &H, &mut S)`):
  `DefaultOutcomeApplier` hands `Outcome::Host` to the `HostHandler`
  implementation of its handler. Every type implements
  `HostHandler<(), S>` as a no-op (#72).
- `DialogueGraph::typed`, `DialogueNode::typed` and `DialogueOption::typed`
  create graph types for a host outcome type `H` (#72).
- `OutcomeDto::Host(H)` (JSON `{"host": ...}`); the graph DTOs carry `H`.
  The conversion from the model needs `H: Clone`, and the DTOs serialize
  when `H` does. The `with` modules `dramaturge_serde::outcome` and
  `dramaturge_serde::dialogue_graph` accept any such `H`. The YAML format
  has no host outcomes (#72).
- `dramaturge_runtime::select_entry(graph, evaluator, state)` and
  `DefaultRuntime::select_entry(graph, state)` return the first entry node
  whose `conditions` hold, or `None` (#71).
- `DialogueGraph::entry_node()` (the first entry node),
  `with_entry_node`, `node(id)` and `entries()` (#71).
- YAML: `entry_node` takes one key or a non-empty list of keys
  (`entry_node: [mara.again, mara.intro]`; also `entry_nodes:`), and a
  node has optional `conditions`. `to_model` warns about an entry key that
  names no node and about a repeated entry key (the first one counts).
  Serializing `YamlDialogueGraph` writes one entry key as a scalar (#71).
- `DialogueRuntime::enter_node(node, state) -> EnterResult` checks the
  node's `conditions`, applies its `outcomes_on_enter` (once per call; it is
  not idempotent) and reports if the node has an available option (checked
  after the outcomes).
  `EnterResult` is `Blocked` or `Entered { has_options }`. The default
  implementation uses the new trait methods `conditions_hold` and
  `apply_outcomes`, which `DefaultRuntime` implements (#73).
- `DialogCursor { current, back }` in `dramaturge-runtime`: `follow(target)`
  pushes the current node on `Node`, pops on `Return` and returns `None`
  when the talk ends (`End`, `Return` with an empty stack) or for a
  `Custom` target. `DialogCursorDto` and the `with` module
  `dramaturge_serde::dialog_cursor` serialize it with raw ids;
  `dramaturge_serde::save::SavedDialogCursor` stores the node keys and
  `restore` re-resolves `current` and the return stack by key (#73).
- `DialogText::Host(T)`: host text, a text leaf in a format that only the
  host knows (for example compiled markup). `DialogText::host`,
  `DialogText::as_host` and `DialogText::map_text`; `map_text` also on
  `TextFragment`, `DialogueNode`, `DialogueOption` and `DialogueGraph`, to
  give a graph a host text type. Host text can be a fragment variant, so
  fragment selection works for it (#75).
- `resolve_text` returns host text as it is, as `ResolvedText::Host(T)`
  (`ResolvedText::as_host`). Each call clones the selected host text; use
  a type that is cheap to clone (such as `Arc<Markup>`) for large text. The validator does not check host text: the
  `EmptyText` check counts it as text, and `UnkeyedProgress` checks only the
  fragments around it (#75).
- `DialogTextDto::Host(T)` (JSON `{"host": ...}`); the text, fragment and
  graph DTOs carry `T` and store the host text as it is. The new `with`
  module `dramaturge_serde::dialog_text` serializes a `DialogText<T>`, and
  `dramaturge_serde::dialogue_graph` accepts any `H` and `T` that are
  `Clone` and serializable. The YAML format has no host text (#75).
- Auto-continue: `DialogueNode::next: Option<DialogTarget>` is where the
  talk goes on without a choice, when the node has no available option.
  `DialogueNode::lines: Vec<DialogLine<T>>` holds more lines after the
  node's own text, each with its own speaker (`DialogLine { speaker, text
  }`, `DialogLine::new`). `DialogueNode::with_line`, `with_next` and
  `all_lines` (the node's own line, then `lines`). Use them for scripted
  exchanges, cutscenes and background talk (#74).
- `DialogueRuntime::advance(graph, cursor, state) -> Advance` enters the
  node at the cursor with `enter_node` (so the outcomes on enter apply per
  node) and follows `next` through `DialogCursor::follow` until a node has
  an available option. `Advance { entered, stop }` lists the entered nodes;
  `AdvanceStop` is `Choice`, `End`, `Custom(key)`, `Blocked`, `Missing` or
  `Limit`. `advance_limited` takes a node limit; `advance` uses
  `ADVANCE_LIMIT` (1024). Both are default trait methods (#74).
- Validator: `next` targets count for reachability, cycles, `MissingTarget`
  and `SelfLoop`; a node with `next` is not a dead end. The new kind
  `ValidationIssueKind::NextLoop` (Error) reports a loop of `next` targets
  through nodes without options and without conditions: a loop that never
  stops, whatever the state. It does not follow `Return` targets, and it
  does not report loops that only state can stop or keep going;
  `advance` stops those at its limit. `UnkeyedProgress` also checks the lines;
  `EmptyText` does not, since a line that resolves to no text is how a line
  depends on the state (#74).
- Serde: `DialogueNodeDto` has `lines` (`DialogLineDto`) and `next`, both
  left out when empty; older DTOs still load. YAML: a node has `lines`
  (`- { speaker: player, text: "..." }`, text as a string or fragments) and
  `next` (a node key or `end`). YAML has no `Return` target, as for option
  targets: a keyword would take a name that a node can have (#74).
- Example `exchange`: a scripted exchange that runs to a choice (#74).

### Changed

- **Breaking:** `Outcome`, `DialogueGraph`, `DialogueNode` and
  `DialogueOption` have a type parameter `H` for host outcomes, with the
  default `()`, so `Outcome` and `DialogueGraph` still mean the types
  without host outcomes. `OutcomeApplier<S, H = ()>`,
  `DialogueRuntime<S, H = ()>`, `AvailableOptionsIter<'a, E, S, H = ()>`,
  `OutcomeDto<H = ()>`, `DialogueGraphDto<H = ()>`, `DialogueNodeDto<H = ()>`
  and `DialogueOptionDto<H = ()>` follow. `GraphValidator::validate` and
  `validate_with_symbols` accept a graph with any `H`. What can break:
  an outcome literal whose type the compiler can not infer from the
  context (`let o = Outcome::EmitEvent { kind };` with no later use that
  fixes the type) needs `let o: Outcome = ...`; `DialogueGraph::new`,
  `impl Default for DialogueGraph` (only `DialogueGraph<()>`),
  `DialogueNode::new` and `DialogueOption::new` stay for `H = ()` only
  (use `typed` for another `H`); a `DefaultOutcomeApplier` handler must
  implement `HostHandler<H, S>` for the `H` it applies. Converting a typed
  outcome or graph to its DTO needs `H: Clone` (#72).

- **Breaking:** `DialogueGraph::entry_node: Option<DialogueNodeId>` is now
  `entry_nodes: Vec<DialogueNodeId>`, in priority order. Replace
  `graph.entry_node` with `graph.entry_node()` (first entry node) or
  `select_entry` (first entry node whose conditions hold). The validator
  walks from every entry node: `Unreachable` and `DeadEnd` use all entry
  nodes, and `EntryNodeNotFound` is reported once per missing entry node.
  `DialogueGraphDto::entry_node: Option<u64>` is now `entry_nodes:
  Vec<u64>`; a DTO with the old `entry_node` field still loads.
  `YamlDialogueGraph::entry_node: YamlKey` is now `entry_nodes:
  Vec<YamlKey>`, and `YamlNode` has the new field `conditions` (#71).

- **Breaking:** `DialogueRuntime` has two new required methods,
  `conditions_hold(conditions, state)` and `apply_outcomes(outcomes,
  state)`. Only hosts that implement `DialogueRuntime` themselves must add
  them; `DefaultRuntime` implements them (#73).

- **Breaking:** `DialogText`, `TextFragment`, `DialogueNode`,
  `DialogueOption` and `DialogueGraph` have a type parameter `T` for host
  text, with the default `()`: `DialogueGraph<H = (), T = ()>`. `DialogText`
  and `DialogueGraph` without parameters still mean the types without host
  data. `ResolvedText<T = ()>`, `ResolvedPiece<T = ()>`,
  `AvailableOptionsIter<'a, E, S, H = (), T = ()>`, `DialogTextDto<T = ()>`,
  `TextFragmentDto<T = ()>`, `DialogueOptionDto<H = (), T = ()>`,
  `DialogueNodeDto<H = (), T = ()>` and `DialogueGraphDto<H = (), T = ()>`
  follow. `resolve_text`, `select_entry` and the validator accept any `T`.
  `DialogueRuntime` stays `DialogueRuntime<S, H>`: its methods
  `available_options`, `choose_option` and `enter_node` are generic over
  `T`, and `OptionsIter<'a, T>` takes `T`. What can break: hosts that
  implement `DialogueRuntime` themselves change these signatures;
  `ResolvedText::as_str` returns `Option<&str>`: `None` for host text (use
  `as_host`), so a string-only renderer can not drop host text in silence;
  `DialogText::inline`, `localized` and `fragments` make `DialogText<()>`
  only (for another `T` use the variants or `DialogText::host`); a
  `DialogText` literal whose type nothing fixes needs an annotation.
  Converting a graph with host text to its DTO needs `T: Clone` (#75).

  Why a type parameter, and not `DialogText::Host(Arc<str>)` or one
  bundle type for `H` and `T`: a string leaf would make hosts parse their
  text again or keep side tables; a typed leaf keeps the host text in the
  graph and gives it back typed. A second parameter next to `H` keeps the
  two choices apart (a host can have host text without host outcomes) and
  keeps the R3 API as it is, and both default to `()`.

- **Breaking:** `DialogueNode`, `DialogueNodeDto` and `YamlNode` have the
  new public fields `lines` and `next`. Code that builds these structs with
  a struct literal adds `lines: vec![]` and `next: None` (or uses
  `DialogueNode::new`/`typed`), and code that destructures them without
  `..` names the new fields (#74).

## [0.3.0-alpha.2] - 2026-11-07

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
- `DialogWorldState::with_event_log(false)`, `set_event_log` and
  `event_log_enabled`. With the log off, `emit_event` and `add_event` do
  nothing (`emit_event` returns `None`), so a cloned state does not carry a
  growing log. `WorldStateDto::event_log` stores the switch (absent means
  on) (#69).
- `SymbolTable::PLAYER_KEY` (`"@player"`) and `SymbolTable::player()`, which
  interns the key on first use and returns the player's entity id (#69).

### Changed

- The crates of the workspace depend on each other with an exact version
  (`=0.3.0-…`), so Cargo cannot resolve a mix of versions, for example a
  newer `dramaturge-model` with an older `dramaturge-serde`. They are released
  together, and since many types are `#[non_exhaustive]`, the compiler no
  longer finds such a mix. A test keeps the pins (#84).
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

[Unreleased]: https://github.com/sunsided/dramaturge/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/sunsided/dramaturge/compare/v0.3.0-alpha.5...v0.3.0
[0.3.0-alpha.5]: https://github.com/sunsided/dramaturge/compare/v0.3.0-alpha.4...v0.3.0-alpha.5
[0.3.0-alpha.4]: https://github.com/sunsided/dramaturge/compare/v0.3.0-alpha.3...v0.3.0-alpha.4
[0.3.0-alpha.3]: https://github.com/sunsided/dramaturge/compare/v0.3.0-alpha.2...v0.3.0-alpha.3
[0.3.0-alpha.2]: https://github.com/sunsided/dramaturge/compare/v0.3.0-alpha.1...v0.3.0-alpha.2
[0.3.0-alpha.1]: https://github.com/sunsided/dramaturge/compare/v0.2.0...v0.3.0-alpha.1
[0.2.0]: https://github.com/sunsided/dramaturge/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/sunsided/dramaturge/releases/tag/v0.1.0
