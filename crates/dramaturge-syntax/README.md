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
| `say` | node, option | The line. On an option, also the label of the choice. `{ text: <pool id> }` says a named text pool |
| `speaker` | node | The character of the node (default: `with`) |
| `when` | node, option | The condition |
| `lines` | node | More lines: `{ speaker, say }` |
| `next` | node | The target when no option is available |
| `options` | node | The choices of the player |
| `go` | option | A node key, `end` (the default) or `back` |
| `set`, `add` | node, option | Facts: `{ name: value }`, `{ name: delta }`; prefixes `player.`, `party.`, `world.` for the scope (as in conditions) |
| `relation` | node, option | Add to the relation of the player to the node's speaker |
| `remember` | node, option | Memories that the node's speaker keeps |
| `then` | node, option | Host effects. Plain dramaturge: `relation` and `custom` |
| `id`, `tags`, `intentions`, `hint` | option | Option key, tags, intentions, navigation hint |
| `notes` | dialogue, node, option | Author notes (a list of `M`, default strings). Not in the graph |

The key `player` is the player (`SymbolTable::player`).

## Payloads and hooks

`Dialogue<C, X, E, M>` is generic over the condition (`C`), text (`X`),
effect (`E`) and note (`M`) payloads of the host. The parsers read them as
they are. The host's `Lower` implementation turns the first three into model
values; the lowering ignores notes, so a host keeps its own note type (for
example loremaster's `Note`) without a hook:

| Hook | Default |
|---|---|
| `condition(&C)` | required |
| `text(&X)` | required |
| `effects(&[E])` | required |
| `set`, `add`, `relation`, `remember` | `SetFact`, `AddFact`, `AddRelationship`, `AddMemory` |
| `entity(key)` | `player` is the player; other keys are interned |

`Plain` is the host for plain dramaturge: `C = String` in the default
condition language (`gold >= 10`, `not met`, `remembers(mara, saw_it)`,
`relation(trust) >= 3`, `seen_by(player)`), `X = Text` (a string, a list of
fragments, or `{ text: <pool id> }`) and `E = Effect`. `lower_plain` uses it.

## Text pools

A `TextPool` is an ordered list of candidates and a pick rule. It lowers to
`dramaturge_model::Pool`, for the text selection core of
`dramaturge-runtime` (`select`, `commit`).

```yaml
id: keeper_replies
pick: best                 # all, first (the default) or best
one_of:
  - { speaker: keeper, topic: stove, text: '"Leave the stove alone."' }
  - { speaker: keeper, text: '"I keep the lock. That is all."' }
  - "Nobody answers."      # a bare text: the default row
  - when: stove.hot and seen_by(player)
    text: "The ash on the grate is fresh."
    learn: [ash_is_fresh]
  - id: weather            # an inline pool in place of a text
    one_of: ["Rain.", "Wind."]
```

| Field | Meaning |
|---|---|
| `id` | The pool id. A named pool needs it. On a candidate: the id of its inline pool |
| `pick` | `all`, `first` (the default) or `best` (the most tests; a tie goes to file order) |
| `one_of` | The candidates, in file order. On a candidate: an inline pool |
| `key` | A stable progress key (without it, the index) |
| `when` | The condition. `seen_by(x)` is true when the perceiver is `x` |
| `speaker`, `topic` | Tuple keys. A candidate without them matches every query |
| `mode` | `always`, `first_time`, `once`, `cycle`, `shuffle`, `sticky` |
| `text`, `variants` | One text, or the variants of `cycle`, `shuffle` and `sticky` |
| `learn` | Facts that the perceiver learns when the candidate shows (plain: `true` in the world scope) |

`lower_pool(pool, symbols, host)` lowers a pool through the same `Lower`
hooks as a dialogue (`condition`, `text`, `entity`). `lower_pool_plain` uses
`Plain`. In a pool, `text: { text: <pool id> }` shows a named pool.

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
