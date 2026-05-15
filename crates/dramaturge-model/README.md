# dramaturge-model

Core domain types for the [dramaturge](https://crates.io/crates/dramaturge) dialogue toolkit.

This crate is the foundation layer: pure data, no I/O, no serde dependency by default. It defines everything the rest of the workspace reasons about - characters, facts, conditions, outcomes, alignment axes, relationships, memories, and dialogue graphs.

## When to depend on this directly

Most projects should depend on `dramaturge` (the facade crate) instead. Depend on `dramaturge-model` directly when:

- you are implementing a custom [`ConditionEvaluator`] or [`OutcomeApplier`] without pulling in the full runtime
- you are writing a tool or editor that works with dialogue graphs as data (e.g. a validator or serializer)
- you want to keep a crate dependency-light and only need the domain types

## Key types

| Type | Purpose |
|---|---|
| `EntityId` | Universal `u64` identifier used across all entities |
| `Character` | Wraps `EntityId` with a string key, display name, and optional traits |
| `Fact` / `FactValue` | Named value (bool, int, float, text, entity ref, set) with scope |
| `FactScope` | Controls knowledge ownership: `World`, `Player`, `Character`, `Faction`, `Party` |
| `Condition` | Composable boolean expression tree evaluated against world state |
| `Outcome` | Declarative state change applied at runtime |
| `DialogueGraph` | Pure data structure holding nodes and options - no evaluation logic |
| `AlignmentAxis` / `AlignmentScore` | Named moral/political dimensions with named poles |
| `RelationshipScore` | Directional, named relationship strength between entities |

## Optional serde support

`serde` support is gated behind a feature flag so game engines that serialize through other means (binary formats, custom reflection) pay zero cost:

```toml
dramaturge-model = { version = "...", features = ["serde"] }
```

For YAML loading, use [`dramaturge-serde`](https://crates.io/crates/dramaturge-serde) instead.

## Part of the dramaturge workspace

```
dramaturge (facade)
├── dramaturge-model     ← you are here
├── dramaturge-runtime   evaluation traits + default engine
├── dramaturge-authoring authoring-time graph validation
├── dramaturge-macros    derive macros for ergonomics
└── dramaturge-serde     optional serde + YAML loader
```

See the [main repository](https://github.com/sunsided/dramaturge) for the full overview, quick start, and example.

## License

Licensed under any of EUPL-1.2, MIT, or Apache-2.0 at your option.
