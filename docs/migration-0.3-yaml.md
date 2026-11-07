# Migrate a YAML dialogue from 0.2 to 0.3

dramaturge 0.3 has one dialogue format. The YAML loader of `dramaturge-serde`,
the `dialogue!` macro and loremaster's dialogues read into the same AST in
`dramaturge-syntax`. The shape is the loremaster dialogue shape, because the
loremaster editor uses and tests it (decision D3). The 0.2 shape
(`version: "2.0"`, `characters`, a list of `nodes`) does not load in 0.3.

This page tells you how to change a 0.2 file. The example
`crates/dramaturge/examples/dialogue.yaml` is a complete 0.3 file.

## Before and after

0.2:

```yaml
version: "2.0"
entry_node: [mara.again, mara.intro]
characters:
  player: { name: "Player" }
  mara: { name: "Mara Vey" }
  orren: { name: "Captain Orren Vale" }
nodes:
  - id: mara.intro
    speaker: mara
    text: "Hello."
    outcomes:
      - { type: set_fact, fact: TalkedToMara, value: true }
    options:
      - id: mara.intro.comfort
        text: "Easy."
        conditions:
          - { type: fact_at_least, fact: trust, value: 3 }
        outcomes:
          - { type: add_relationship, source: player, target: mara, axis: trust, delta: 5 }
          - { type: add_relationship, source: player, target: orren, axis: trust, delta: -1 }
        target: mara.confession
      - id: mara.intro.leave
        navigation_hint: go_to
        go_to_target: "Captain Vale"
        target: orren.intro
  - id: mara.again
    speaker: mara
    text: "You again."
    conditions:
      - { type: fact_exists, fact: TalkedToMara }
    options:
      - { id: mara.again.bye, text: "Bye.", target: end }
```

0.3:

```yaml
with: mara
start: [mara.again, mara.intro]
nodes:
  mara.intro:
    say: "Hello."
    set: { TalkedToMara: true }
    options:
      - id: mara.intro.comfort
        say: "Easy."
        when: trust >= 3
        relation: { trust: 5 }
        then:
          - relation: { from: player, to: orren, axis: trust, delta: -1 }
        go: mara.confession
      - id: mara.intro.leave
        hint: { go_to: "Captain Vale" }
        go: orren.intro
  mara.again:
    when: exists(TalkedToMara)
    say: "You again."
    options:
      - { id: mara.again.bye, say: "Bye.", go: end }
```

## Field by field

| 0.2 | 0.3 |
|---|---|
| `version: "2.0"` | Remove it. |
| `characters: { key: { name } }` | Remove it. Speakers are keys; the lowering interns them. Keep display names in your game. |
| `entry_node` / `entry_nodes` | `start`: one key or a list of keys. |
| `nodes:` as a list with `id:` | `nodes:` as a map from the node key to the node. The order of the file stays. |
| `speaker` on each node | `with: <key>` once. Write `speaker:` only on a node that another character says. |
| node `text` | `say` |
| `lines: [{ speaker, text }]` | `lines: [{ speaker, say }]` |
| `next` | `next` (also `back` now) |
| `conditions:` (list of typed maps) | `when:` one string in the condition language (see below). More conditions: join them with `and`. |
| node `outcomes:` | `set`, `add`, `relation`, `remember`, `then` on the node |
| option `id` | `id` (optional now; the default key is `<node>#<index>`) |
| option `text` | `say` (optional when the option has a `hint`) |
| option `target` | `go`. Without `go`, the option ends the talk. |
| option `outcomes:` | `set`, `add`, `relation`, `remember`, `then` on the option |
| `tags: [{ axis, key, display_prefix, requires_trait }]` | `tags: [{ axis, key, prefix, requires_trait }]` |
| `intentions_explicit`, `intentions_implicit` | `intentions: { explicit: [...], implicit: [...] }` |
| `navigation_hint: continue` (`leave`, `end`, `return`) | `hint: continue` (`leave`, `end`, `return`) |
| `navigation_hint: go_to` + `go_to_target: X` | `hint: { go_to: X }` |
| `navigation_hint: { custom: X }` | `hint: { custom: X }` |
| fragment `when: { type: ... }` | fragment `when:` a string in the condition language |

## Outcomes

| 0.2 | 0.3 |
|---|---|
| `{ type: set_fact, fact: f, value: v }` | `set: { f: v }` |
| `{ type: set_fact, scope: player, fact: f, value: v }` | `set: { player.f: v }` |
| `{ type: add_fact, fact: f, delta: n }` | `add: { f: n }` |
| `{ type: add_relationship, source: player, target: <speaker>, axis: a, delta: n }` | `relation: { a: n }` (player to the speaker of the node) |
| `{ type: add_relationship, source: s, target: t, axis: a, delta: n }` (other pairs) | `then: [{ relation: { from: s, to: t, axis: a, delta: n } }]` |
| (new) | `remember: [key]`: the speaker of the node keeps the memory |
| (new) | `then: [{ custom: { key: k, args: [...] } }]`: `Outcome::Custom` |

## Conditions

| 0.2 | 0.3 `when:` |
|---|---|
| `{ type: always }` | `true` |
| `{ type: fact_exists, fact: f }` | `exists(f)` |
| `{ type: fact_equals, fact: f, value: v }` | `f == v` (`f` alone for `f == true`) |
| `{ type: fact_at_least, fact: f, value: n }` | `f >= n` |
| `{ type: fact_at_most, fact: f, value: n }` | `f <= n` |
| `scope: player` | `player.f` |
| `{ type: has_memory, owner: o, id: m }` | `remembers(o, m)` |
| (new) | `relation(trust) >= 3`, `relation(trust, orren) < 0` |
| (new) | `not`, `and`, `or`, `( )`, `!=`, `>`, `<` |

## Other changes

- The key `player` is the player: `SymbolTable::player()` (`@player`). In 0.2
  `player` was an ordinary character key. Look the player up with
  `symbols.entity(SymbolTable::PLAYER_KEY)`.
- The loader is a function now: `dramaturge_serde::loader::load_yaml(yaml)`
  returns `LoadedDialogue { graph, symbols, warnings }`. `from_yaml` and
  `to_yaml` read and write the AST (`dramaturge_syntax::Dialogue`).
- The loader checks keys and targets. A `go`, `next` or `start` key that names
  no node is an error now (0.2 left it to the validator, or warned for an
  entry node). A node with no options and no `next` is a warning.
- `back` is a target now (`DialogTarget::Return`, the node before). `end`
  and `back` are reserved: rename a node with one of these keys.
- Unknown fields are errors. `notes:` (author notes, a list of strings) is
  allowed on the dialogue, on nodes and on options.
- The text of an option without `say` is `NavigationHint::default_text`:
  `Go to Captain Vale` (0.2: `Go to Captain Vale...`).

## Steps

1. Remove `version` and `characters`. Add `with:` with the main speaker.
2. Rename `entry_node` to `start`.
3. Change `nodes` from a list to a map: `- id: k` becomes `k:`.
4. Remove `speaker:` where it is the `with` character.
5. Rename `text` to `say` and `target` to `go`.
6. Change each `conditions:` list to one `when:` string.
7. Change each `outcomes:` list to `set`, `add`, `relation` and `then`.
8. Change `navigation_hint`, `go_to_target`, `display_prefix` and the
   intention lists.
9. Load the file and fix the errors. Each error names its place, for example
   `nodes.mara.intro.options[0].go: no node `mara.confesion``.
