# dramaturge

> A Rust toolkit for branching dialogue, character memory, and narrative state.

[![CI](https://github.com/sunsided/dramaturge/actions/workflows/ci.yml/badge.svg)](https://github.com/sunsided/dramaturge/actions/workflows/ci.yml)

---

dramaturge is a game-agnostic Rust library for building branching dialogue systems. It models characters, facts, alignment, relationships, memories, and events as pure data — no game engine knowledge baked in. Your game-specific rules live in typed IDs, tags, facts, and state changes; the library evaluates conditions, applies outcomes, and drives dialogue sessions.

## Quick Start

```toml
[dependencies]
# workspace member (not yet published to crates.io)
dramaturge = { path = "crates/dramaturge" }
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
cargo run -p dramaturge --example the_broken_seal
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
