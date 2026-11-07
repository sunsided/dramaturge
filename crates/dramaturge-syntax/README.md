# dramaturge-syntax

One dialogue AST for the [dramaturge](https://crates.io/crates/dramaturge) dialogue toolkit.

The YAML loader of `dramaturge-serde`, the `dialogue!` macro of
`dramaturge-macros` and host macros (for example loremaster's `world!`) read a
dialogue into the same `Dialogue`. `lower` turns it into a
`dramaturge_model::DialogueGraph`.

```text
 YAML / JSON (serde)   ─┐
 dialogue! (syn)       ─┼─▶ Dialogue<C, X, E> ─ check ─ lower ─▶ DialogueGraph<H, T>
 world! dialogue (syn) ─┘                                 ▲
                                          Lower hooks of the host
```

## The shape

```yaml
with: mara                 # the character that the player talks to
start: [again, intro]      # entry nodes: the first whose `when` holds starts
nodes:
  intro:
    say: "Hello, traveller."
    set: { met_mara: true }          # outcomes when the talk enters the node
    options:
      - say: "Who are you?"
        go: names                    # a node key, `end` or `back`
        relation: { trust: 1 }       # outcomes when the player takes the option
      - say: "Goodbye."              # without `go`, the option ends the talk
  again:
    when: met_mara
    say: "You again."
    options:
      - { say: "Goodbye.", go: end }
  names:
    say: "They call me Mara."
    lines:                           # more lines, each with its own speaker
      - { speaker: player, say: "Mara. I will remember." }
    next: end                        # go on without a choice
```

| Field | On | Meaning |
|---|---|---|
| `id` | dialogue | Optional. Node and option keys become `<id>/<key>` |
| `with` | dialogue | The character key. Nodes without `speaker` are its lines |
| `start` | dialogue | A key or a list of keys: the entry nodes, in order |
| `nodes` | dialogue | A map from node key to node, in file order |
| `say` | node, option | The line. On an option, also the label of the choice |
| `speaker` | node | The character of the node (default: `with`) |
| `when` | node, option | The condition |
| `lines` | node | More lines: `{ speaker, say }` |
| `next` | node | The target when no option is available |
| `options` | node | The choices of the player |
| `go` | option | A node key, `end` (the default) or `back` |
| `set`, `add` | node, option | Facts: `{ name: value }`, `{ name: delta }`; `player.` prefix for the player scope |
| `relation` | node, option | Add to the relation of the player to the node's speaker |
| `remember` | node, option | Memories that the node's speaker keeps |
| `then` | node, option | Host effects. Plain dramaturge: `relation` and `custom` |
| `id`, `tags`, `intentions`, `hint` | option | Option key, tags, intentions, navigation hint |

The key `player` is the player (`SymbolTable::player`).

## Payloads and hooks

`Dialogue<C, X, E>` is generic over the condition (`C`), text (`X`) and effect
(`E`) payloads of the host. The parsers read them as they are. The host's
`Lower` implementation turns them into model values:

| Hook | Default |
|---|---|
| `condition(&C)` | required |
| `text(&X)` | required |
| `effects(&[E])` | required |
| `set`, `add`, `relation`, `remember` | `SetFact`, `AddFact`, `AddRelationship`, `AddMemory` |
| `entity(key)` | `player` is the player; other keys are interned |

`Plain` is the host for plain dramaturge: `C = String` in the default
condition language (`gold >= 10`, `not met`, `remembers(mara, saw_it)`,
`relation(trust) >= 3`), `X = Text` (a string or a list of fragments) and
`E = Effect`. `lower_plain` uses it.

## The token form

With the `syn` feature, `tokens::parse_dialogue` reads the same dialogue from
Rust tokens. `dramaturge-macros` uses it for `dialogue!`, and a host macro
can use it for `dialogue` blocks, with its own `ParseHost`:

```rust,ignore
dialogue with mara start [again, intro] {
    intro: "Hello, traveller." {
        set met_mara = true;
        "Who are you?" => names { relation trust += 1; }
        "Goodbye." => end;
    }
    again: "You again." {
        when met_mara;
        "Goodbye." => end;
    }
    names {
        mara: "They call me Mara.";
        player: "Mara. I will remember.";
        -> end;
    }
}
```

`Spanned::check` puts each issue of `check` at the span of its token.

## Features

| Feature | What it enables |
|---|---|
| *(none)* | The AST, `check`, the default condition language and `lower` |
| `serde` | `Serialize` and `Deserialize` for the AST (YAML, JSON) |
| `syn` | The token form (`tokens`) |

The YAML loader uses `serde` only. It does not compile `syn`.
