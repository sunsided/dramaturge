# dramaturge

> A Rust toolkit for branching dialogue, character memory, and narrative state.

[![CI](https://github.com/sunsided/dramaturge/actions/workflows/ci.yml/badge.svg)](https://github.com/sunsided/dramaturge/actions/workflows/ci.yml)

---

**Status: Bootstrap / Prototype — APIs are unstable and incomplete.**

This workspace is in its earliest scaffolding stage. The crate layout, public API shapes,
and module structure are established; full implementation will follow in subsequent milestones.

---

## Intended Architecture

The workspace is split into focused crates:

| Crate | Purpose |
|---|---|
| `dramaturge` | Public facade — re-exports the most important items |
| `dramaturge-model` | Core domain types (characters, dialogue graphs, facts, …) |
| `dramaturge-runtime` | Evaluation traits and runtime engine interface |
| `dramaturge-macros` | Procedural macros (placeholder) |
| `dramaturge-serde` | Optional serialization integration |
| `dramaturge-authoring` | Authoring-time validation helpers |

## Crate Layout

```
crates/
├── dramaturge/           # Facade crate
├── dramaturge-model/     # Core domain model
├── dramaturge-runtime/   # Runtime/evaluation traits
├── dramaturge-macros/    # Proc-macro crate (placeholder)
├── dramaturge-serde/     # Serialization integration (optional serde feature)
└── dramaturge-authoring/ # Authoring/validation helpers
```

## Example Sketch

The following sketch shows the intended import ergonomics once the library matures.
**Nothing below is implemented yet** — it is a design target only.

```rust
use dramaturge::{
    Character, CharacterId,
    DialogueGraph, DialogueNode, DialogueOption,
    OptionTag, Alignment,
    Fact, Condition, Outcome,
    Intent, Memory, Event,
    DialogueRuntime, EvaluationContext, EvaluationResult,
};

// Construct a character
let hero = Character {
    id: CharacterId("hero".into()),
    name: "Elara".into(),
};

// Build a dialogue graph
let graph = DialogueGraph { nodes: vec![] };
```

## License

Licensed under any of:

- [EUPL-1.2](LICENSE-EUPL-1.2)
- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
