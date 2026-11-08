# dramaturge

> A Rust toolkit for branching dialogue, character memory, and narrative state.

[![CI](https://github.com/sunsided/dramaturge/actions/workflows/ci.yml/badge.svg)](https://github.com/sunsided/dramaturge/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/dramaturge.svg)](https://crates.io/crates/dramaturge)
[![docs.rs](https://docs.rs/dramaturge/badge.svg)](https://docs.rs/dramaturge)

---

dramaturge is a game-agnostic Rust library for branching dialogue. It models characters, facts, alignment, relationships, memories, and events as plain data, with no game engine baked in. Your game's rules live in typed IDs, tags, facts, and state changes; the library evaluates conditions, applies outcomes, selects text, and drives dialogue sessions.

## Install

```toml
[dependencies]
dramaturge = { version = "0.3", features = ["macros"] }

# Only to load dialogues from YAML files:
dramaturge-serde = { version = "0.3", features = ["yaml"] }

# Only to check dialogues at authoring time (CI, editor, build step):
dramaturge-authoring = "0.3"
```

No feature is on by default. The features of `dramaturge`:

| Feature | What it enables |
|---|---|
| `syntax` | `dramaturge::syntax`: the dialogue AST, its serde shape (YAML, JSON, ...) and its lowering to a graph |
| `macros` | `dialogue!` and the derives `DialogTag`, `FactValue`, `ConditionBuilder`. Implies `syntax` |
| `serde` | Compiles `dramaturge-serde` (DTOs for graphs and save states) |
| `yaml` | Compiles `dramaturge-serde` with its `yaml` feature |

`dramaturge` does not re-export `dramaturge-serde`, so code that calls the YAML loader depends on `dramaturge-serde` directly, as above.

## One dialogue language

A dialogue has one AST (`dramaturge::syntax::Dialogue`), and every authoring form reads into it. YAML, JSON, the `dialogue!` macro and the Rust builder are the same language with different spelling, and they lower to the same `DialogueGraph`. The four examples below make equal graphs.

The dialogue: Mara greets the player once (`intro`) and briefly after that (`again`). The player can be kind, pay, or threaten her; each choice changes state.

### `dialogue!` (feature `macros`)

The macro checks the dialogue at compile time: unknown targets, duplicate keys and conditions it can not read are compile errors at the offending token.

```rust
use dramaturge::dialogue;

let loaded = dialogue! {
    with mara start [again, intro] {
        intro: "If you came about the shipment, I told the Wardens everything." {
            set met_mara = true;
            "You treated someone with that medicine." => soft { relation trust += 10; }
            "Ten gold says you remember more." when gold >= 10 => soft { add gold -= 10; }
            "Tell me where the crates went." => forced {
                relation trust -= 10;
                remember was_threatened;
            }
            "Never mind." => end;
        }
        again: "You again." {
            when met_mara;
            "Goodbye." => end;
        }
        soft {
            mara: "The refugee camp beyond the east wall.";
            player: "I will speak carefully.";
            -> end;
        }
        forced: "The east wall. Take what you want." {
            -> end;
        }
    }
};
// loaded.graph: the DialogueGraph; loaded.symbols: the keys behind its ids.
```

### YAML (`dramaturge-serde`, feature `yaml`)

```yaml
with: mara                  # the character the player talks to
start: [again, intro]       # entry nodes: the first whose `when` holds starts
nodes:
  intro:
    say: "If you came about the shipment, I told the Wardens everything."
    set: { met_mara: true }  # on enter
    options:
      - say: "You treated someone with that medicine."
        relation: { trust: 10 }   # player -> the node's speaker
        go: soft
      - say: "Ten gold says you remember more."
        when: gold >= 10
        add: { gold: -10 }
        go: soft
      - say: "Tell me where the crates went."
        relation: { trust: -10 }
        remember: [was_threatened]  # a memory of the node's speaker
        go: forced
      - say: "Never mind."    # without `go`, the option ends the talk
  again:
    when: met_mara
    say: "You again."
    options:
      - say: "Goodbye."
  soft:                       # a scripted exchange: more lines, then on
    say: "The refugee camp beyond the east wall."
    lines:
      - { speaker: player, say: "I will speak carefully." }
    next: end
  forced:
    say: "The east wall. Take what you want."
    next: end
```

```rust
let loaded = dramaturge_serde::loader::load_yaml(yaml)?; // graph, symbols, warnings
```

### JSON (feature `syntax`, any serde format)

```json
{
  "with": "mara",
  "start": ["again", "intro"],
  "nodes": {
    "intro": {
      "say": "If you came about the shipment, I told the Wardens everything.",
      "set": { "met_mara": true },
      "options": [
        { "say": "You treated someone with that medicine.", "relation": { "trust": 10 }, "go": "soft" },
        { "say": "Ten gold says you remember more.", "when": "gold >= 10", "add": { "gold": -10 }, "go": "soft" },
        { "say": "Tell me where the crates went.", "relation": { "trust": -10 },
          "remember": ["was_threatened"], "go": "forced" },
        { "say": "Never mind." }
      ]
    },
    "again": { "when": "met_mara", "say": "You again.", "options": [{ "say": "Goodbye." }] },
    "soft": {
      "say": "The refugee camp beyond the east wall.",
      "lines": [{ "speaker": "player", "say": "I will speak carefully." }],
      "next": "end"
    },
    "forced": { "say": "The east wall. Take what you want.", "next": "end" }
  }
}
```

```rust
let dialogue: dramaturge::syntax::Dialogue = serde_json::from_str(json)?;
let loaded = dramaturge::syntax::lower_plain(&dialogue)?;
```

### Rust

<details>
<summary>The same dialogue with the AST builder (feature <code>syntax</code>)</summary>

```rust
use dramaturge::syntax::{Choice, Dialogue, Node, Outcomes, Target, lower_plain};

let dialogue: Dialogue = Dialogue::new("mara")
    .with_start("again")
    .with_start("intro")
    .with_node(
        Node::new("intro", "If you came about the shipment, I told the Wardens everything.".into())
            .with_outcomes(Outcomes::default().with_set("met_mara", true.into()))
            .with_option(
                Choice::new("You treated someone with that medicine.".into(), Target::parse("soft"))
                    .with_outcomes(Outcomes::default().with_relation("trust", 10)),
            )
            .with_option(
                Choice::new("Ten gold says you remember more.".into(), Target::parse("soft"))
                    .with_when("gold >= 10".into())
                    .with_outcomes(Outcomes::default().with_add("gold", -10)),
            )
            .with_option(
                Choice::new("Tell me where the crates went.".into(), Target::parse("forced"))
                    .with_outcomes(
                        Outcomes::default()
                            .with_relation("trust", -10)
                            .with_remember("was_threatened"),
                    ),
            )
            .with_option(Choice::new("Never mind.".into(), Target::End)),
    )
    .with_node(
        Node::new("again", "You again.".into())
            .with_when("met_mara".into())
            .with_option(Choice::new("Goodbye.".into(), Target::End)),
    )
    .with_node(
        Node::new("soft", "The refugee camp beyond the east wall.".into())
            .with_line("player", "I will speak carefully.".into())
            .with_next(Target::End),
    )
    .with_node(
        Node::new("forced", "The east wall. Take what you want.".into())
            .with_next(Target::End),
    );
let loaded = lower_plain(&dialogue)?;
```

</details>

You can also skip the AST and build a `DialogueGraph` from model types (`DialogueNode`, `DialogueOption`, `Outcome`, `Condition`) with numeric ids. The [`exchange`](crates/dramaturge/examples/exchange.rs) example does that.

### The parts

- **Targets**: a node key, `end` (the default for options) or `back` (the node before).
- **Conditions** (`when`): `met_mara`, `exists(f)`, `gold >= 10`, `player.gold` (scopes `player.`, `party.`, `world.`), `remembers(mara, saw_it)`, `relation(trust) >= 3`, `seen_by(player)`, joined with `and`, `or`, `not` and parentheses. In YAML, quote `"true"` and `"false"`.
- **Outcomes** on a node (on enter) or an option (when chosen): `set`, `add`, `relation`, `remember`, and `then` for other effects (`relation` between any two characters, `custom`).
- **Options** also take `id`, `tags`, `intentions` and a navigation `hint` (`continue`, `leave`, `end`, `return`, `{ go_to: X }`).
- **Exchanges**: `lines` give a node more lines, each with its own speaker; `next` goes on without a choice. The runtime enters such nodes one after the other until a node offers a choice.

The full field reference is in the [`dramaturge-syntax` README](crates/dramaturge-syntax/README.md).

## Text selection

New in 0.3: a line can vary. In YAML, `say` takes a list of fragments with their own `when` and `mode` (`always`, `first_time`, `once`, `cycle`, `shuffle`, `sticky`), or `{ text: <pool id> }` to say a named text pool. A pool is an ordered list of candidates with a pick rule: `first` (the default), `all`, or `best` (the candidate with the most tests wins; a tie goes to file order). Candidates can have tuple keys (`speaker`, `topic`), a perceiver test (`seen_by`), and `learn` facts.

```yaml
id: keeper_replies
pick: best
one_of:
  - { speaker: keeper, topic: stove, text: "Leave the stove alone." }
  - { speaker: keeper, text: "I keep the lock. That is all." }
  - "Nobody answers."
  - when: stove.hot and seen_by(player)
    text: "The ash on the grate is fresh."
    learn: [ash_is_fresh]
```

`select` and `select_named` choose without side effects and return a `Selection` with a `Trace` that says why each candidate won or lost. `commit` then records the progress and the `learn` items:

```rust
use dramaturge::*;

let mut symbols = SymbolTable::new();
let pools = Pools::new().with(dramaturge_serde::loader::load_pool_yaml(yaml, &mut symbols)?);
let keeper = CharacterId::new(symbols.entity("keeper").unwrap());

let evaluator = DefaultConditionEvaluator::new();
let (mut state, mut progress, mut rng) = (DialogWorldState::new(), TextProgress::new(), SplitMix64::new(7));

let query = Query::new().speaker(keeper).topic("stove".into());
let judge = DefaultJudge::new(&evaluator, &state);
let selection = select_named("keeper_replies", &pools, "kitchen", &query, &judge, &progress, &mut rng);
println!("{}", selection.trace);
commit(&selection, &query, &mut progress, &mut world_learner(&mut state));
```

`GraphValidator::validate_pools` reports ties in `best` pools, unkeyed progress modes and unknown pool names. See the [`text_selection`](crates/dramaturge/examples/text_selection.rs) example.

## Running a dialogue

`DefaultRuntime` picks the entry node, enters nodes (applying their outcomes) until one offers a choice, and applies the chosen option:

```rust
use dramaturge::*;

let (graph, mut symbols) = (loaded.graph, loaded.symbols);
let player = symbols.player();
let mut runtime = DefaultRuntime::new();
let mut state = DialogWorldState::new();
let mut cursor = DialogCursor::new(runtime.select_entry(&graph, &state).expect("an entry node"));

loop {
    let advance = runtime.advance(&graph, &mut cursor, &mut state);
    for id in &advance.entered {
        for (speaker, text) in graph.node(*id).unwrap().all_lines() {
            let who = if speaker.0 == player { "you" } else { symbols.entity_key(speaker.0).unwrap_or("?") };
            println!("{who}: {:?}", text.as_inline());
        }
    }
    if advance.stop != AdvanceStop::Choice {
        break;
    }
    let node = graph.node(cursor.current).unwrap();
    let option = runtime.available_options(node, &state).next().cloned().unwrap(); // the player's pick
    let target = runtime.choose_option(&option, &mut state);
    if cursor.follow(&target).is_none() {
        break;
    }
}
```

To keep dialogue state inside your own game state, implement `HasDialogWorldState`; to answer game-specific conditions and outcomes (`Condition::Custom`, `Outcome::Custom`), pass a `CustomHandler` to `DefaultConditionEvaluator::with_custom` and `DefaultOutcomeApplier::with_custom`. See the [`host_state`](crates/dramaturge/examples/host_state.rs) example.

### Validation

Check graphs at authoring time with [`dramaturge-authoring`](crates/dramaturge-authoring/README.md): missing targets, unreachable nodes, dead ends, cycles, always-false conditions, orphan facts and more.

```rust
let issues = dramaturge_authoring::GraphValidator::new()
    .validate_with_symbols(&loaded.graph, &loaded.symbols);
assert!(issues.is_empty(), "{issues:?}");
```

## Derive macros (feature `macros`)

- `FactValue` on a fact enum: typed condition helpers per variant (`is_true`, `at_least`, `equals_text`, `exists`, ...). `#[fact(int)]`, `float`, `text`, `entity` set the type; the default is `bool`.
- `ConditionBuilder` on the same enum: `all`, `any`, `not`, `always`, `never`, `custom`, and more.
- `DialogTag` on a tag enum: implements the `DialogTag` trait from `#[tag(axis = "...", key = "...")]`. The trait needs `Debug, Clone, PartialEq, Eq, Hash`.
- `Dramaturge`: a placeholder that generates nothing. It is not re-exported by `dramaturge`.

```rust
use dramaturge::{ConditionBuilder, DialogTag, FactScope, FactValue};

#[derive(FactValue, ConditionBuilder)]
enum Facts {
    MetMara,
    #[fact(int)]
    Gold,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, DialogTag)]
enum Tone {
    #[tag(axis = "moral", key = "paragon")]
    Kind,
    #[tag(axis = "moral", key = "renegade")]
    Harsh,
}

let can_pay = Facts::all(vec![
    Facts::MetMara.is_true(FactScope::World),
    Facts::Gold.at_least(FactScope::World, 10),
]);
```

More in the [`dramaturge-macros` README](crates/dramaturge-macros/README.md).

## Examples

| Example | Shows | Run |
|---|---|---|
| [`the_broken_seal`](crates/dramaturge/examples/the_broken_seal.rs) | A full scene loaded from [YAML](crates/dramaturge/examples/dialogue.yaml): skill tags, fragments, relationships, validation, a text UI | `cargo run -p dramaturge --example the_broken_seal --features yaml` |
| [`dialogue_macro`](crates/dramaturge/examples/dialogue_macro.rs) | A dialogue written with `dialogue!`, played twice | `cargo run -p dramaturge --example dialogue_macro --features macros` |
| [`exchange`](crates/dramaturge/examples/exchange.rs) | A graph built from model types; lines that continue without a choice; `back` | `cargo run -p dramaturge --example exchange` |
| [`host_state`](crates/dramaturge/examples/host_state.rs) | Custom conditions against your own game state | `cargo run -p dramaturge --example host_state` |
| [`text_selection`](crates/dramaturge/examples/text_selection.rs) | Text pools, `best`, tuple keys, `seen_by`, `learn`, the trace | `cargo run -p dramaturge --example text_selection` |

## Upgrading from 0.2

0.3 has one dialogue shape, and the 0.2 YAML shape no longer loads. [docs/migration-0.3-yaml.md](docs/migration-0.3-yaml.md) maps each 0.2 field to its 0.3 form. The [CHANGELOG](CHANGELOG.md) lists the API changes.

## Workspace Layout

| Crate | Purpose |
|---|---|
| [`dramaturge`](https://docs.rs/dramaturge) | Public facade: re-exports the most important items |
| [`dramaturge-model`](crates/dramaturge-model/README.md) | Core domain types: characters, facts, conditions, outcomes, dialogue graphs, text pools |
| [`dramaturge-runtime`](crates/dramaturge-runtime/README.md) | Evaluation traits, the default runtime, text selection |
| [`dramaturge-syntax`](crates/dramaturge-syntax/README.md) | The dialogue AST, its checks, the token and serde forms, and the lowering to graphs |
| [`dramaturge-macros`](crates/dramaturge-macros/README.md) | `dialogue!` and the derives `DialogTag`, `FactValue`, `ConditionBuilder` |
| [`dramaturge-serde`](crates/dramaturge-serde/README.md) | Serialization DTOs and the YAML loader (`yaml` feature) |
| [`dramaturge-authoring`](crates/dramaturge-authoring/README.md) | Authoring-time validation: graph checks, dead ends, orphan facts, pool checks |

## License

Licensed under any of:

- [EUPL-1.2](LICENSE-EUPL-1.2)
- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
