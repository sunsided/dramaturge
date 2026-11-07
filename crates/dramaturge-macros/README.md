# dramaturge-macros

Macros for the [dramaturge](https://crates.io/crates/dramaturge) dialogue toolkit.

This is a proc-macro crate. The `dramaturge` facade re-exports its macros with the `macros` feature, so you do not need to add it to your dependencies directly.

## `dialogue!`

Writes a dialogue in Rust tokens and builds its graph. The grammar is the token
form of `dramaturge-syntax`, so the text is the same as in a `dialogue` block of
a host macro (loremaster's `world!`). The macro returns
`dramaturge_syntax::Loaded` (`graph`, `symbols`).

```rust
use dramaturge_macros::dialogue;

let loaded = dialogue! {
    with mara start [again, intro] {
        intro: "Hello, traveller." {
            set met_mara = true;
            "Who are you?" => names { relation trust += 1; }
            "Goodbye." => end;
        }
        again: "You again." {
            when met_mara and relation(trust) >= 1;
            "Goodbye." => end;
        }
        names {
            mara: "They call me Mara.";
            player: "Mara. I will remember.";
            -> end;
        }
    }
};
```

These are compile errors at the span of the token: a syntax error, a target
that names no node, a duplicate node key, a node with no options and no
`-> target;`, and a condition that does not parse. The calling crate needs
`dramaturge-syntax`, or `dramaturge` with the `macros` feature.

Why here and not behind a feature: a proc macro must be in a `proc-macro`
crate, and this crate is that crate already. The grammar is in
`dramaturge-syntax` (a normal library), so host macros can use it too.

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
