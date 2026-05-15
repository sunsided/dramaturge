# dramaturge-serde

Optional serialization integration for the [dramaturge](https://crates.io/crates/dramaturge) dialogue toolkit.

`dramaturge-model` intentionally has no serde dependency so game engines that serialize state through other means (binary formats, custom reflection, etc.) pay zero cost. Add this crate only when you need it.

Serialization goes through DTO types defined in this crate — `dramaturge-model` itself has no serde dependency. The `yaml` feature adds the YAML dialogue loader.

## Features

| Feature | What it enables |
|---|---|
| *(default)* | DTO types with `Serialize`/`Deserialize`; `pub use serde` and `pub use model::*` re-exports |
| `yaml` | YAML dialogue graph loader |

## YAML loader

```toml
[dependencies]
dramaturge-serde = { version = "...", features = ["yaml"] }
```

```rust
use dramaturge_serde::loader::YamlDialogueGraph;

let raw = std::fs::read_to_string("dialogue.yaml").unwrap();
let graph = YamlDialogueGraph::from_yaml(&raw)
    .expect("invalid YAML")
    .to_model()
    .expect("invalid graph structure");
```

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
