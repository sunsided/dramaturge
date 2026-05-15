# dramaturge-runtime

Runtime evaluation traits and default engine for the [dramaturge](https://crates.io/crates/dramaturge) dialogue toolkit.

This crate separates the domain model from its evaluation strategy. It defines the traits that let you plug in game-specific logic, and ships default implementations that handle all built-in condition and outcome variants.

## When to depend on this directly

Most projects should depend on `dramaturge` (the facade crate) instead. Depend on `dramaturge-runtime` directly when:

- you are implementing a custom `ConditionEvaluator` (e.g. to handle `Condition::Custom` for game-specific checks like time of day or quest state)
- you are implementing a custom `OutcomeApplier` (e.g. to dispatch to your game engine on `Outcome::Custom`)
- you want to compose the runtime traits without importing the full facade

## Extension points

```rust
use dramaturge_runtime::{ConditionEvaluator, DefaultConditionEvaluator, DialogWorldState};
use dramaturge_model::{Condition, FactValue};

struct MyEvaluator {
    inner: DefaultConditionEvaluator,
}

impl ConditionEvaluator for MyEvaluator {
    fn evaluate(&self, condition: &Condition, ctx: &DialogWorldState) -> bool {
        match condition {
            Condition::Custom { key, .. } if key == "is_daytime" => {
                // delegate to your game engine here
                false
            }
            // Recurse through composites via self so nested Custom conditions
            // reach this evaluator, not the default.
            Condition::All(cs) => cs.iter().all(|c| self.evaluate(c, ctx)),
            Condition::Any(cs) => cs.iter().any(|c| self.evaluate(c, ctx)),
            Condition::Not(c) => !self.evaluate(c, ctx),
            other => self.inner.evaluate(other, ctx),
        }
    }
}
```

## Key types

| Type | Purpose |
|---|---|
| `DialogWorldState` | Mutable game state evaluated against conditions |
| `ConditionEvaluator` | Trait: `evaluate(&Condition, &DialogWorldState) -> bool` |
| `DefaultConditionEvaluator` | Handles all built-in `Condition` variants; `Condition::Custom` returns `false` |
| `OutcomeApplier` | Trait: applies an `Outcome` to mutable world state |
| `DefaultOutcomeApplier` | Handles all built-in `Outcome` variants |
| `DialogueRuntime` | Trait: filters options and drives a dialogue session |
| `DefaultRuntime` | Composes evaluator + applier to drive sessions |

## Part of the dramaturge workspace

```
dramaturge (facade)
├── dramaturge-model     pure domain types
├── dramaturge-runtime   ← you are here
├── dramaturge-authoring authoring-time graph validation
├── dramaturge-macros    derive macros for ergonomics
└── dramaturge-serde     optional serde + YAML loader
```

See the [main repository](https://github.com/sunsided/dramaturge) for the full overview, quick start, and example.

## License

Licensed under any of EUPL-1.2, MIT, or Apache-2.0 at your option.
