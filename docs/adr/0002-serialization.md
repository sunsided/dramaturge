# ADR 0002: Serialization through DTOs in dramaturge-serde

- Status: accepted
- Date: 2026-10-30
- Issue: #57
- Builds on: PR #54 (`docs/superpowers/specs/2026-05-15-serde-isolation-design.md`), ADR 0001

## Context

Hosts need to serialize model types and runtime state: compiled story
bundles, save games, and state fingerprints. Before this change only the YAML
DTOs in `dramaturge-serde` implemented serde, and only in one direction. A
host such as lantern kept a second copy of every fact so it could save it.

PR #54 removed serde from `dramaturge-model` on purpose: serde is not a domain
concern.

## Decision

Keep `dramaturge-model` and `dramaturge-runtime` free of serde. Complete the
DTO pattern in `dramaturge-serde`:

- `dramaturge_serde::dto` has a lossless DTO for every public data type of the
  model and for `DialogWorldState`, with `From<&Model>` and
  `IntoModel::into_model`. Conversions destructure model structs
  exhaustively, so a new model field is a compile error until the DTO follows.
  This gives the same guarantee as serde's `remote` attribute without the
  per-field `with` plumbing that nested remote types require.
- `IntoModel::into_model_with` takes an `IdMap`, so the same conversion code
  either keeps raw ids (`Identity`) or re-resolves them by key (`KeyRemap`).
- `with` modules (`world_state`, `condition`, `outcome`, `dialogue_graph`,
  `symbol_table`) let hosts write `#[serde(with = "dramaturge_serde::world_state")]`.
- `save::SavedWorldState` stores the state together with the `SymbolTable` keys
  of the ids it references. `restore(&mut current)` maps every id through its
  key into the current table, so saves stay valid when content is added,
  removed or reordered (ADR 0001). Restoring is lenient: keys the current
  content no longer defines are interned with fresh ids and listed in a
  `RestoreReport`; a host that wants strict loading rejects the save when the
  report is not empty.
- `DialogWorldState::set_next_event_id` lets a restored state keep its event id
  sequence even when the log was drained.

### Format

- Enums are externally tagged with `snake_case` names.
- Ids are raw numbers in the plain DTOs; `SavedWorldState` adds the keys.
- `FactValue::Float` is a JSON number. Non-finite floats are not
  representable in JSON. With `serde_json`, enable `float_roundtrip` for
  bit-exact floats; round-trip tests use it and compare exactly.
- `WorldStateDto` lists entries in `DialogWorldState`'s deterministic
  iteration order (ADR context: #59), so equal states serialize to equal bytes.

### The `Yaml*` DTOs

They stay a separate format. The YAML format is written by hand and is
deliberately friendlier: keys instead of ids (ADR 0001), defaults for scope,
untagged values, only the condition and outcome variants authors need, and
navigation-hint text defaults. The lossless DTOs are the machine format for
bundles and saves. Sharing one type would make either the hand-written format
noisy or the machine format lossy.

## Alternatives

- An optional `serde` feature on `dramaturge-model` and `dramaturge-runtime`
  with `cfg_attr` derives. Least code, but it reverts #54 and couples the
  domain types to one serialization framework. Rejected.
- serde `remote` derives. Rejected for the reasons above; the exhaustive
  destructuring gives the same drift check.

## Consequences

- New public API in `dramaturge-serde` (`dto`, `save`, `with` modules); no
  breaking change for existing YAML users beyond ADR 0001.
- `dramaturge-serde` now depends on `dramaturge-runtime`.
- The DTOs must be kept in sync with the model; the compiler enforces it.
- `dramaturge-serde` and `dramaturge-runtime` build for
  `wasm32-unknown-unknown`; CI checks this.
