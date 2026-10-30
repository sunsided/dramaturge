# dramaturge-runtime

Runtime evaluation traits and default engine for the [dramaturge](https://crates.io/crates/dramaturge) dialogue toolkit.

This crate separates the domain model from its evaluation strategy. It defines the traits that let you plug in game-specific logic, and ships default implementations that handle all built-in condition and outcome variants.

## When to depend on this directly

Most projects should depend on `dramaturge` (the facade crate) instead. Depend on `dramaturge-runtime` directly when:

- you are writing a `CustomHandler` that answers `Condition::Custom` and `Outcome::Custom` from your own game state (time of day, current room, inventory)
- you are implementing a custom `ConditionEvaluator` or `OutcomeApplier`
- you want to compose the runtime traits without importing the full facade

## Extension points

Evaluators, appliers and the runtime are generic over the state type. Embed
`DialogWorldState` in your own state, implement `HasDialogWorldState`, and
answer `Custom` conditions and outcomes with a `CustomHandler`. The default
evaluator recurses through `All`/`Any`/`Not`, so nested `Custom` nodes reach
the handler.

```rust
use dramaturge_runtime::{
    ConditionEvaluator, CustomHandler, DefaultConditionEvaluator, DialogWorldState,
    HasDialogWorldState,
};
use dramaturge_model::{Condition, FactValue};

struct GameState {
    current_room: String,
    dialog: DialogWorldState,
}

impl HasDialogWorldState for GameState {
    fn dialog(&self) -> &DialogWorldState { &self.dialog }
    fn dialog_mut(&mut self) -> &mut DialogWorldState { &mut self.dialog }
}

struct Rooms;

impl CustomHandler<GameState> for Rooms {
    fn condition(&self, key: &str, args: &[FactValue], state: &GameState) -> bool {
        key == "in_room" && matches!(args, [FactValue::Text(r)] if *r == state.current_room)
    }
}

let state = GameState { current_room: "shore".into(), dialog: DialogWorldState::new() };
let evaluator = DefaultConditionEvaluator::with_custom(Rooms);
let cond = Condition::All(vec![
    Condition::Always,
    Condition::Custom { key: "in_room".into(), args: vec![FactValue::Text("shore".into())] },
]);
assert!(evaluator.evaluate(&cond, &state));
```

## Key types

| Type | Purpose |
|---|---|
| `DialogWorldState` | Mutable game state evaluated against conditions |
| `NarrativeState` | Trait: the state operations evaluators and appliers use |
| `HasDialogWorldState` | Trait: embed `DialogWorldState` in a host type and get `NarrativeState` for free |
| `ConditionEvaluator<S>` | Trait: `evaluate(&Condition, &S) -> bool` (`S` defaults to `DialogWorldState`) |
| `DefaultConditionEvaluator<H>` | Handles all built-in `Condition` variants; `Condition::Custom` goes to `H` |
| `OutcomeApplier<S>` | Trait: applies an `Outcome` to mutable state |
| `DefaultOutcomeApplier<H>` | Handles all built-in `Outcome` variants; `Outcome::Custom` goes to `H` |
| `CustomHandler<S>` | Trait: host hook for `Custom` conditions and outcomes |
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
