# dramaturge-authoring

Authoring-time dialogue graph validation for the [dramaturge](https://crates.io/crates/dramaturge) toolkit.

Run this at authoring time - in CI, an editor plugin, or a build step - not at game runtime. It catches structural problems before they become silent failures during play.

## Usage

```rust
use dramaturge_authoring::GraphValidator;
use dramaturge_model::DialogueGraph;

let graph = /* load your graph */;
let validator = GraphValidator::new();
let issues = validator.validate(&graph);

for issue in &issues {
    println!("{:?} — {:?}", issue.severity, issue.kind);
}
```

## Validation checks

| Check | Severity | Description |
|---|---|---|
| Missing targets | Error | Options referencing non-existent node IDs |
| Unreachable nodes | Warning | Nodes not reachable from the entry node via BFS |
| Dead-end nodes | Info | Reachable nodes with no options or enter outcomes |
| Self-loops | Warning | Options whose target is their own parent node |
| Cycles | Warning | Nodes participating in cycles |
| Always-false conditions | Warning | Options with conditions that constant-fold to `false` |
| Orphaned fact references | Warning | Facts referenced in conditions but never set by any outcome |
| Mismatched source node IDs | Error | Options whose `source_node_id` does not match their parent node |

## Severity levels

- `Error` - graph is broken and cannot run correctly. Ship-blocker.
- `Warning` - likely a mistake but not fatal. Review before shipping.
- `Info` - informational. Dead-end nodes at narrative leaves are common and expected.

## When to depend on this

This crate is a dev/tooling dependency in most projects. Add it to `[dev-dependencies]` for use in tests and build scripts, or pull it into a standalone editor tool. The main `dramaturge` crate does not depend on it at runtime.

## Part of the dramaturge workspace

```
dramaturge (facade)
├── dramaturge-model     pure domain types
├── dramaturge-runtime   evaluation traits + default engine
├── dramaturge-authoring ← you are here
├── dramaturge-macros    derive macros for ergonomics
└── dramaturge-serde     optional serde + YAML loader
```

See the [main repository](https://github.com/sunsided/dramaturge) for the full overview, quick start, and example.

## License

Licensed under any of EUPL-1.2, MIT, or Apache-2.0 at your option.
