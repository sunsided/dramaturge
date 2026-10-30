# ADR 0001: Numeric runtime ids, string keys at authoring time

- Status: accepted
- Date: 2026-10-30
- Issue: #55

## Context

The model had two kinds of ids. Entities, dialogue nodes, dialogue options,
memories and events used `u64` newtypes. Facts, character keys and axes used
`Arc<str>`.

Authors and editors need ids that are stable and readable (`npc.mara`,
`dlg.mara.intro`). Numeric ids collide when two people add content on
different branches, make diffs and error messages hard to read, and force
hosts such as lantern to keep their own key-to-number table stable across
edits and saves. The YAML loader showed the split: speakers were keys
(`speaker: mara`) while targets were numbers in strings (`target: "2"`).

## Options

1. String keys everywhere (`Arc<str>` in every id newtype).
2. Numbers at runtime, strings at authoring time, bridged by a symbol table.
3. A generic id type parameter on all model types.

## Decision

Option 2.

- The numeric id newtypes (`EntityId`, `CharacterId`, `DialogueNodeId`,
  `DialogueOptionId`, `MemoryId`) stay. The runtime does not change.
- `dramaturge_model::SymbolTable` holds one `Interner` per id kind. Interning a
  key hands out the next dense id; the table maps ids back to keys.
- The YAML loader reads keys for every reference (node and option ids,
  targets, the entry node, speakers, relationship endpoints) and returns
  `LoadedDialogue { graph, symbols, warnings }`.
- `GraphValidator::validate_with_symbols` prints keys (`'dlg.intro'`) and falls
  back to `#<number>` for ids the table does not know.
- Save games store keys, not numbers (see ADR 0002), so a save stays valid
  when content is added or removed.
- `EventId` stays numeric and is not interned: the runtime allocates event
  ids, authors never write them.

## Consequences

- Runtime state keeps small `Copy` ids, which keeps cloning, hashing and
  comparing states cheap for exploration (#59).
- Rust code that builds models directly is unaffected.
- The YAML format changes from 1.0 to 2.0. Numeric `id`, `target`,
  `entry_node`, relationship `source`/`target` and character `id` values are
  still accepted for one release; each use adds a deprecation message to
  `LoadedDialogue::warnings`. Numeric node and option ids become the key of
  their decimal string (`2` → `"2"`), so legacy targets keep resolving.
- `YamlDialogueGraph::to_model` returns `LoadedDialogue` instead of
  `DialogueGraph` (breaking).
- Ids are assigned in interning order. Characters are interned first, sorted
  by key, so entity ids do not depend on reference order. Ids are not stable
  across content edits; only keys are.
