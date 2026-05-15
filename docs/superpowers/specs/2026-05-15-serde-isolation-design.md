# Serde Isolation: Move All Serde Concerns to `dramaturge-serde`

**Date:** 2026-05-15
**Branch:** feature/serde
**Status:** Approved

## Problem

`dramaturge-model` carries an optional `serde` feature with ~40 `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` lines and 3 field-level `#[cfg_attr(feature = "serde", serde(default))]` annotations scattered across 11 source files. A dedicated `dramaturge-serde` crate exists for serialization, yet model still knows about serde. This is a leaky abstraction: serde is a serialization concern that should not live in the domain model.

## Goal

- `dramaturge-model` has zero serde references - no dep, no feature, no `cfg_attr`
- `dramaturge-serde` owns all serde: DTOs, format support, and derives
- `dramaturge` (top-level) keeps `serde` feature that pulls in `dramaturge-serde`, with no direct serde code of its own (already the case)

## Non-Goals

- Direct `serde::Serialize`/`Deserialize` on model types (not needed - all serialization goes through the loader)
- Adding new serialization formats
- Changing the public API of `dramaturge-serde`

## Approach

Embrace and formalize the DTO pattern already in use. `dramaturge-serde`'s `Yaml*` types in `loader.rs` are already the canonical serde representation - they own their own derives and convert to model via `to_model()`. The model's serde derives are redundant and unused by any existing code path.

The orphan rule prevents `dramaturge-serde` from implementing `serde::Serialize for model::Foo` directly (owns neither the trait nor the type), so the DTO pattern is the correct long-term design.

## Architecture

```
dramaturge-model
  - pure domain types
  - no serde dep, no serde feature, no serde attributes
  - compiled without any serialization overhead

dramaturge-serde
  - depends on dramaturge-model (no features needed)
  - owns serde dep (hard, non-optional)
  - Yaml* DTO types with their own Serialize/Deserialize derives
  - YamlDialogueGraph::from_yaml() + to_model() conversion path
  - optional: serde_yaml behind "yaml" feature

dramaturge (top-level)
  - serde feature = optional dep on dramaturge-serde (unchanged)
  - no direct serde code (already clean)
```

## Changes Required

### `crates/dramaturge-model/Cargo.toml`

Remove:
- `[package.metadata.docs.rs] features = ["serde"]` block
- `[features] serde = ["dep:serde"]`
- `serde = { workspace = true, optional = true }` from `[dependencies]`

### `crates/dramaturge-model/src/*.rs` (11 files)

Strip all serde annotations:
- `alignment.rs` - 5 derives
- `character.rs` - 5 derives
- `condition.rs` - 1 derive
- `dialogue.rs` - 7 derives + 1 field attr (`serde(default)` on `navigation_hint`)
- `event.rs` - 4 derives
- `fact.rs` - 5 derives
- `intent.rs` - 2 derives
- `memory.rs` - 5 derives
- `outcome.rs` - 1 derive
- `relationship.rs` - 3 derives
- `tag.rs` - 1 derive + 2 field attrs (`serde(default)` on `display_prefix` and `requires_trait`)

Update `lib.rs` doc comment: remove language about serde being opt-in via feature flag.

### `crates/dramaturge-serde/Cargo.toml`

Change:
```toml
# Before
dramaturge-model = { workspace = true, features = ["serde"] }

# After
dramaturge-model = { workspace = true }
```

### Unchanged

- `crates/dramaturge-serde/src/loader.rs` - Yaml* types own their own derives, no change
- `crates/dramaturge-serde/src/lib.rs` - `pub use model::*` re-export stays
- `crates/dramaturge/Cargo.toml` - already clean
- `crates/dramaturge/examples/the_broken_seal.rs` - already uses loader path

## Verification

After changes:

```bash
# Model must compile with no serde in tree
cargo build -p dramaturge-model

# Full build with yaml feature must still work
cargo build -p dramaturge --features yaml

# All tests must pass
cargo test --workspace

# Serde-specific tests must pass
cargo test -p dramaturge-serde --features yaml
```

`cargo tree -p dramaturge-model` must show no `serde` in the dependency tree.

## Trade-offs

- Model types are no longer directly `Serialize + Deserialize` - acceptable because all current serialization already goes through `YamlDialogueGraph`
- If direct model serialization is needed in the future, it must be added back via the serde crate (e.g., newtype wrappers or remote derive) - this is an intentional design constraint that keeps model clean
