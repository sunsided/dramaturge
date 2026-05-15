# Serde Isolation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove all serde references from `dramaturge-model` so that serde is owned exclusively by `dramaturge-serde`.

**Architecture:** Strip ~40 `cfg_attr` derive lines and 3 field-level serde attributes from 11 model source files, then remove the optional serde dependency from `dramaturge-model/Cargo.toml`, then drop the `features = ["serde"]` activation from `dramaturge-serde`'s model dep. The `Yaml*` DTO types in `dramaturge-serde/src/loader.rs` already own their own derives and are unaffected.

**Tech Stack:** Rust, Cargo workspace

---

## File Map

| File | Change |
|------|--------|
| `crates/dramaturge-model/Cargo.toml` | Remove `[features]` block, remove `serde` dep, remove docs.rs feature |
| `crates/dramaturge-model/src/lib.rs` | Update crate-level doc comment |
| `crates/dramaturge-model/src/alignment.rs` | Remove 5 `cfg_attr` derive lines |
| `crates/dramaturge-model/src/character.rs` | Remove 5 `cfg_attr` derive lines |
| `crates/dramaturge-model/src/condition.rs` | Remove 1 `cfg_attr` derive line |
| `crates/dramaturge-model/src/dialogue.rs` | Remove 8 `cfg_attr` derive lines + 1 field attr |
| `crates/dramaturge-model/src/event.rs` | Remove 4 `cfg_attr` derive lines |
| `crates/dramaturge-model/src/fact.rs` | Remove 5 `cfg_attr` derive lines |
| `crates/dramaturge-model/src/intent.rs` | Remove 2 `cfg_attr` derive lines |
| `crates/dramaturge-model/src/memory.rs` | Remove 5 `cfg_attr` derive lines |
| `crates/dramaturge-model/src/outcome.rs` | Remove 1 `cfg_attr` derive line |
| `crates/dramaturge-model/src/relationship.rs` | Remove 3 `cfg_attr` derive lines |
| `crates/dramaturge-model/src/tag.rs` | Remove 1 `cfg_attr` derive line + 2 field attrs |
| `crates/dramaturge-serde/Cargo.toml` | Remove `features = ["serde"]` from model dep |

---

### Task 1: Baseline verification

**Files:** none modified

- [ ] **Step 1: Confirm tests pass before touching anything**

```bash
cargo test --workspace
```

Expected: all tests pass. If any fail, stop and investigate — don't proceed with a broken baseline.

- [ ] **Step 2: Record serde presence in model's dep tree**

```bash
cargo tree -p dramaturge-model --features serde 2>/dev/null | grep serde
```

Expected: shows `serde v1.x.x` in the tree (optional deps are only shown when the feature is explicitly activated). After the work is done, this command will produce no output because the `serde` feature will no longer exist.

---

### Task 2: Strip serde annotations from `alignment.rs`

**Files:**
- Modify: `crates/dramaturge-model/src/alignment.rs`

- [ ] **Step 1: Remove the 5 `cfg_attr` derive lines**

Open `crates/dramaturge-model/src/alignment.rs`. Find and delete each of these lines (they appear directly above struct/enum definitions, do not touch the `#[derive(...)]` line below them):

```rust
// DELETE these 5 lines:
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

They appear at lines ~41, ~57, ~93, ~108, ~130 (above `AlignmentAxisId`, `AlignmentPole`, `AlignmentAxis`, `AlignmentPoleId`, `AlignmentScore`). Each is a standalone line — remove the whole line.

- [ ] **Step 2: Verify file compiles**

```bash
cargo check -p dramaturge-model
```

Expected: no errors (model still has its serde dep at this point — that's fine, errors come in Task 10).

---

### Task 3: Strip serde annotations from `character.rs`

**Files:**
- Modify: `crates/dramaturge-model/src/character.rs`

- [ ] **Step 1: Remove the 5 `cfg_attr` derive lines**

Open `crates/dramaturge-model/src/character.rs`. Find and delete each line matching:

```rust
// DELETE these 5 lines (one above each of: EntityId, CharacterId, CharacterKey, CharacterTrait, Character):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 2: Verify**

```bash
cargo check -p dramaturge-model
```

Expected: no errors.

---

### Task 4: Strip serde annotations from `condition.rs`

**Files:**
- Modify: `crates/dramaturge-model/src/condition.rs`

- [ ] **Step 1: Remove the 1 `cfg_attr` derive line**

Open `crates/dramaturge-model/src/condition.rs`. Find and delete the single line:

```rust
// DELETE (above the Condition enum):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 2: Verify**

```bash
cargo check -p dramaturge-model
```

Expected: no errors.

---

### Task 5: Strip serde annotations from `dialogue.rs`

**Files:**
- Modify: `crates/dramaturge-model/src/dialogue.rs`

- [ ] **Step 1: Remove 8 `cfg_attr` derive lines**

Open `crates/dramaturge-model/src/dialogue.rs`. Find and delete each line matching:

```rust
// DELETE (one above each of: DialogueNodeId, DialogueOptionId, DialogText,
//         DialogueGraph, DialogueNode, DialogTarget, DialogueOption, NavigationHint):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 2: Remove the 1 `cfg_attr` field attribute**

In the `DialogueOption` struct, find the field `navigation_hint` and delete the line above it:

```rust
// BEFORE:
    #[cfg_attr(feature = "serde", serde(default))]
    pub navigation_hint: Option<NavigationHint>,

// AFTER:
    pub navigation_hint: Option<NavigationHint>,
```

- [ ] **Step 3: Verify**

```bash
cargo check -p dramaturge-model
```

Expected: no errors.

---

### Task 6: Strip serde annotations from `event.rs`, `fact.rs`, `intent.rs`

**Files:**
- Modify: `crates/dramaturge-model/src/event.rs`
- Modify: `crates/dramaturge-model/src/fact.rs`
- Modify: `crates/dramaturge-model/src/intent.rs`

- [ ] **Step 1: Remove 4 `cfg_attr` lines from `event.rs`**

Open `crates/dramaturge-model/src/event.rs`. Find and delete 4 lines matching:

```rust
// DELETE (one above each of: EventId, EventKind, EventContext, Event):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 2: Remove 5 `cfg_attr` lines from `fact.rs`**

Open `crates/dramaturge-model/src/fact.rs`. Find and delete 5 lines matching:

```rust
// DELETE (one above each of: FactId, FactValue, FactScope, Fact, ScopedFact):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 3: Remove 2 `cfg_attr` lines from `intent.rs`**

Open `crates/dramaturge-model/src/intent.rs`. Find and delete 2 lines matching:

```rust
// DELETE (one above each of: Intention, DialogIntentions):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 4: Verify all three files**

```bash
cargo check -p dramaturge-model
```

Expected: no errors.

---

### Task 7: Strip serde annotations from `memory.rs`, `outcome.rs`, `relationship.rs`

**Files:**
- Modify: `crates/dramaturge-model/src/memory.rs`
- Modify: `crates/dramaturge-model/src/outcome.rs`
- Modify: `crates/dramaturge-model/src/relationship.rs`

- [ ] **Step 1: Remove 5 `cfg_attr` lines from `memory.rs`**

Open `crates/dramaturge-model/src/memory.rs`. Find and delete 5 lines matching:

```rust
// DELETE (one above each of: MemoryId, MemoryTag, Confidence, Memory, MemoryQuery):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 2: Remove 1 `cfg_attr` line from `outcome.rs`**

Open `crates/dramaturge-model/src/outcome.rs`. Find and delete the single line:

```rust
// DELETE (above the Outcome enum):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 3: Remove 3 `cfg_attr` lines from `relationship.rs`**

Open `crates/dramaturge-model/src/relationship.rs`. Find and delete 3 lines matching:

```rust
// DELETE (one above each of: RelationshipAxisId, RelationshipAxis, RelationshipScore):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 4: Verify**

```bash
cargo check -p dramaturge-model
```

Expected: no errors.

---

### Task 8: Strip serde annotations from `tag.rs`

**Files:**
- Modify: `crates/dramaturge-model/src/tag.rs`

- [ ] **Step 1: Remove the 1 `cfg_attr` derive line**

Open `crates/dramaturge-model/src/tag.rs`. Find and delete the line above `CustomDialogTag`:

```rust
// DELETE (above CustomDialogTag struct):
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
```

- [ ] **Step 2: Remove the 2 `cfg_attr` field attribute lines**

In the `CustomDialogTag` struct, find `display_prefix` and `requires_trait` and delete the `cfg_attr` line above each:

```rust
// BEFORE:
    #[cfg_attr(feature = "serde", serde(default))]
    pub display_prefix: Option<Arc<str>>,
    /// Whether this tag represents a required player trait/skill.
    /// When true, the option is only available if the player has this trait.
    #[cfg_attr(feature = "serde", serde(default))]
    pub requires_trait: bool,

// AFTER:
    pub display_prefix: Option<Arc<str>>,
    /// Whether this tag represents a required player trait/skill.
    /// When true, the option is only available if the player has this trait.
    pub requires_trait: bool,
```

- [ ] **Step 3: Verify no serde annotations remain in model**

```bash
grep -rn "cfg_attr.*serde\|serde::" crates/dramaturge-model/src/
```

Expected: zero output. If any lines appear, remove them.

- [ ] **Step 4: Verify model still compiles**

```bash
cargo check -p dramaturge-model
```

Expected: no errors.

---

### Task 9: Remove serde dependency from `dramaturge-model/Cargo.toml`

**Files:**
- Modify: `crates/dramaturge-model/Cargo.toml`

- [ ] **Step 1: Remove the serde dep, feature, and docs.rs metadata**

Open `crates/dramaturge-model/Cargo.toml`. Apply this diff:

```toml
# REMOVE only this line from [package.metadata.docs.rs] (keep rustdoc-args):
features = ["serde"]

# REMOVE this entire block:
[features]
serde = ["dep:serde"]

# REMOVE this line from [dependencies]:
serde = { workspace = true, optional = true }
```

After editing, the file should look like:

```toml
[package]
name = "dramaturge-model"
description = "Core domain model for the dramaturge dialogue toolkit."
readme = "README.md"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
keywords.workspace = true
categories.workspace = true

[package.metadata.docs.rs]
rustdoc-args = ["--cfg", "docsrs"]

[dev-dependencies]
dramaturge-macros = { workspace = true }
assert2 = { workspace = true }
proptest = "1"
```

- [ ] **Step 2: Verify model compiles without serde**

```bash
cargo build -p dramaturge-model
```

Expected: success with no errors.

- [ ] **Step 3: Confirm serde is absent from the dep tree**

```bash
cargo tree -p dramaturge-model 2>/dev/null | grep serde
```

Expected: zero output — serde must not appear anywhere in model's tree.

---

### Task 10: Update `dramaturge-serde`'s model dependency

**Files:**
- Modify: `crates/dramaturge-serde/Cargo.toml`

- [ ] **Step 1: Drop `features = ["serde"]` from the model dep**

Open `crates/dramaturge-serde/Cargo.toml`. Change:

```toml
# BEFORE:
dramaturge-model = { workspace = true, features = ["serde"] }

# AFTER:
dramaturge-model = { workspace = true }
```

- [ ] **Step 2: Verify serde crate still builds**

```bash
cargo build -p dramaturge-serde --features yaml
```

Expected: success.

---

### Task 11: Update `lib.rs` doc comment

**Files:**
- Modify: `crates/dramaturge-model/src/lib.rs`

- [ ] **Step 1: Remove the serde feature reference from the crate doc**

Open `crates/dramaturge-model/src/lib.rs`. Find lines 3-5:

```rust
//! This crate provides game-agnostic foundational types for branching dialogue
//! systems. It has no hard serde or runtime dependency — `serde` support is
//! gated behind an optional feature flag.
```

Replace with:

```rust
//! This crate provides game-agnostic foundational types for branching dialogue
//! systems. It has no serde or runtime dependency. Serialization is handled
//! exclusively by the `dramaturge-serde` crate via its DTO types.
```

- [ ] **Step 2: Verify**

```bash
cargo doc -p dramaturge-model --no-deps 2>&1 | head -5
```

Expected: no warnings about the changed comment.

---

### Task 12: Full verification and commit

**Files:** none modified

- [ ] **Step 1: Run full workspace test suite**

```bash
cargo test --workspace
```

Expected: all tests pass.

- [ ] **Step 2: Run serde crate tests with yaml feature**

```bash
cargo test -p dramaturge-serde --features yaml
```

Expected: all tests pass.

- [ ] **Step 3: Build the example**

```bash
cargo build --example the_broken_seal --features yaml -p dramaturge
```

Expected: success.

- [ ] **Step 4: Final serde-in-model check**

```bash
grep -rn "serde" crates/dramaturge-model/
```

Expected: only appears in `README.md` (documentation prose) or `src/lib.rs` doc comment referring to `dramaturge-serde` by name. Zero occurrences inside `src/**/*.rs` that reference the `serde` crate or feature.

- [ ] **Step 5: Commit**

```bash
git add crates/dramaturge-model/Cargo.toml \
        crates/dramaturge-model/src/lib.rs \
        crates/dramaturge-model/src/alignment.rs \
        crates/dramaturge-model/src/character.rs \
        crates/dramaturge-model/src/condition.rs \
        crates/dramaturge-model/src/dialogue.rs \
        crates/dramaturge-model/src/event.rs \
        crates/dramaturge-model/src/fact.rs \
        crates/dramaturge-model/src/intent.rs \
        crates/dramaturge-model/src/memory.rs \
        crates/dramaturge-model/src/outcome.rs \
        crates/dramaturge-model/src/relationship.rs \
        crates/dramaturge-model/src/tag.rs \
        crates/dramaturge-serde/Cargo.toml
git commit -m "refactor: isolate all serde concerns to dramaturge-serde

Remove optional serde dep and all cfg_attr derive annotations from
dramaturge-model. The Yaml* DTO types in dramaturge-serde already own
their own derives and serve as the canonical serde representation.
Model is now a pure domain crate with zero serialization awareness."
```
