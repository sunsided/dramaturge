# dramaturge-serde

Optional serialization integration for the [dramaturge](https://crates.io/crates/dramaturge) dialogue toolkit.

`dramaturge-model` intentionally has no serde dependency so game engines that serialize state through other means (binary formats, custom reflection, etc.) pay zero cost. Add this crate only when you need it.

Serialization goes through DTO types defined in this crate — `dramaturge-model` itself has no serde dependency. The `yaml` feature adds the YAML dialogue loader.

## Features

| Feature | What it enables |
|---|---|
| *(default)* | Lossless DTOs (`dto`) for every model type and `DialogWorldState`, `with` modules, keyed save games (`save`); `pub use serde` and `pub use model::*` re-exports |
| `yaml` | YAML dialogue loader (backed by `serde-saphyr` and `dramaturge-syntax`) |

## Saving state

```rust
use dramaturge_runtime::DialogWorldState;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct GameState {
    room: String,
    #[serde(with = "dramaturge_serde::world_state")]
    dialog: DialogWorldState,
}
```

The `with` modules store raw ids. `save::SavedWorldState` also stores the
`SymbolTable` keys and re-resolves ids on restore, so saves survive content
changes. See `docs/adr/0002-serialization.md`.

## YAML loader

```toml
[dependencies]
dramaturge-serde = { version = "...", features = ["yaml"] }
```

```yaml
with: mara
start: [again, intro]
nodes:
  intro:
    say: "Hello, traveller."
    set: { met_mara: true }
    options:
      - say: "Who are you?"
        go: names
        relation: { trust: 1 }
      - { say: "Goodbye.", go: end }
  again:
    when: met_mara
    say: "You again."
    options:
      - { say: "Goodbye.", go: end }
  names:
    say: "They call me Mara."
    next: end
```

```rust
use dramaturge_serde::loader::load_yaml;

let raw = std::fs::read_to_string("dialogue.yaml").unwrap();
let loaded = load_yaml(&raw).expect("invalid dialogue"); // errors name the place

// `loaded.graph` is the model, `loaded.symbols` maps keys such as
// `intro` to ids, and `loaded.warnings` lists non-fatal issues.
```

The shape is the dialogue shape of `dramaturge-syntax` (one format for YAML,
JSON and the `dialogue!` macro; see its README for every field). It changed in
0.3: see `docs/migration-0.3-yaml.md`. All references are stable keys; numeric
ids are an error (`go: "2"` in quotes is a key). `from_yaml` and `to_yaml`
read and write the AST, generic over the condition, text and effect payloads
of a host.

See [`crates/dramaturge/examples/dialogue.yaml`](https://github.com/sunsided/dramaturge/blob/main/crates/dramaturge/examples/dialogue.yaml) in the repository for the full YAML schema.

## Part of the dramaturge workspace

```
dramaturge (facade)
├── dramaturge-model     pure domain types
├── dramaturge-runtime   evaluation traits + default engine
├── dramaturge-authoring authoring-time graph validation
├── dramaturge-macros    derive macros for ergonomics
└── dramaturge-serde     ← you are here
```

See the [main repository](https://github.com/sunsided/dramaturge) for the full overview, quick start, and example.

## License

Licensed under any of EUPL-1.2, MIT, or Apache-2.0 at your option.
