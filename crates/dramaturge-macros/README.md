# dramaturge-macros

Derive macros for the [dramaturge](https://crates.io/crates/dramaturge) dialogue toolkit.

This is an internal proc-macro crate. It is re-exported through the main `dramaturge` facade - you do not need to add it to your dependencies directly.

## Available derives

### `#[derive(DialogTag)]`

Implements the `DialogTag` trait for an enum. Use `#[tag(axis = "...", key = "...")]` on variants to declare the axis and key for each tag.

```rust
use dramaturge_macros::DialogTag;

#[derive(DialogTag)]
enum MyTag {
    #[tag(axis = "skill", key = "medic")]
    Medic,
    #[tag(axis = "skill", key = "engineer")]
    Engineer,
}
```

### `#[derive(FactValue)]`

Generates typed fact helpers for a game-specific fact enum. Defaults to `bool`; use `#[fact(int|float|text|entity)]` to change the type.

```rust
use dramaturge_macros::FactValue;

#[derive(FactValue)]
enum GameFacts {
    MetHero,          // bool (default)
    QuestStarted,     // bool
    #[fact(int)]
    Gold,             // i64
    #[fact(text)]
    PlayerName,       // String
}

// Generated helpers:
// GameFacts::MetHero.is_true(scope)
// GameFacts::Gold.at_least(scope, 100)
// GameFacts::PlayerName.equals_text(scope, "Commander")
```

### `#[derive(ConditionBuilder)]`

Generates fluent condition construction methods on a fact enum. Usually derived alongside `FactValue`.

## Part of the dramaturge workspace

```
dramaturge (facade)
├── dramaturge-model     pure domain types
├── dramaturge-runtime   evaluation traits + default engine
├── dramaturge-authoring authoring-time graph validation
├── dramaturge-macros    ← you are here
└── dramaturge-serde     optional serde + YAML loader
```

See the [main repository](https://github.com/sunsided/dramaturge) for the full overview, quick start, and example.

## License

Licensed under any of EUPL-1.2, MIT, or Apache-2.0 at your option.
