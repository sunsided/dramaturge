# Text selection core: design note

**Date:** 2026-11-07
**Status:** Draft, for review before the code
**Roadmap item:** P3-05 (#78). The code is R9 (#79, milestone 0.3.0-alpha.4).
**Decision:** D2 (decided 2026-11-06): dramaturge owns the selection core
(fragments, modes, tuples, perceiver), generic over a host text leaf.
loremaster owns markup, layers and room assembly.

## 1. Problem

dramaturge and loremaster have two text systems with almost the same modes:
dramaturge `TextFragment` with `resolve_text`, and loremaster `Fragment` with
`text::visible` and `text::pick`. Both show every fragment whose condition
holds. Neither can show **one** line of a group, choose the **most specific**
line, or choose by speaker, topic and perceiver. Ask and show, default
replies, perceiver text and companion comments need this.

Requirements:

| Id | Requirement |
| --- | --- |
| TX-8 | Select a reply by (speaker, topic or thing, perceiver, condition). A default row (`*`) applies when no other row matches. |
| TX-9 | A text fragment accepts a perceiver condition. The perceiver is the active actor. |
| TX-7 | Dialogue text holds the host's own text type (R8, #75). |
| TX-11 | A companion comments by chance, from a reply table (phase 4; it uses this core). |

The D2 side note in the roadmap adds: `one_of`, `pick: first | best`,
`seen_by(actor)`, `learn`, named pools, one spelling for the modes, and a
trace that tells why a line won. A preview must not change state.

Not in scope: markup, inline inserts (`{drawer.open ? … | }`), layers,
`replaces` across layers, room assembly, localization. These stay with the
host.

## 2. Model

**Candidate.** One line that can show. It has an optional key, an optional
condition (`when`), optional tuple keys (speaker, topic), a mode, one or more
variants, and a `learn` list.

**Pool.** An ordered list of candidates and a pick rule. A pool has an id. A
named pool (`texts/keeper_remark.yaml`) is a pool that rules and dialogue
call by id. A variant can be a leaf or a nested pool, so a description
list can hold a `one_of` group (the stove).

**Pick rules.**

| Pick | Result |
| --- | --- |
| `all` | Every eligible candidate, in file order. This is today's fragment list in both crates. |
| `first` | The first eligible candidate in file order. Default for `one_of`. |
| `best` | The eligible candidate with the highest score. A tie goes to file order. |

**Eligible.** A candidate is eligible when all of these are true:

1. Its tuple keys match the query (a missing key is `*` and matches all).
2. Its mode allows it now (`once`: not shown yet; `first_time`: first
   resolution of the pool by this caller).
3. It has at least one variant.
4. Its condition holds. The condition can test the perceiver (`seen_by`).

An exhausted `once` candidate is not eligible. Thus the next candidate wins.

**Score** (for `best`). The score is the number of tests:

- Each atomic test (a fact test, `in(kitchen)`, `seen_by(x)`) counts 1.
- `a and b` counts score(a) + score(b).
- `a or b` counts min(score(a), score(b)).
- `not a` counts score(a).
- No condition counts 0.
- Each tuple key that is set (not `*`) counts 1.

The score is a property of the candidate. It is computed once, at build time.

**Modes.** One spelling in all surfaces: `always`, `first_time`, `once`,
`cycle`, `shuffle`, `sticky`. These are the existing `FragmentMode`
variants. loremaster keeps `first_visit` as an alias of `first_time` in
rooms. The mode chooses the **variant** of the winning candidate:

- `cycle`: variant `shown % len`.
- `shuffle`: a random variant, not the one shown last. With one variant,
  that variant shows each time (as in `resolve_text` today).
- `sticky`: a random variant the first time, then always the same one.

Only the candidates that show get progress. A candidate that lost does not
step its cycle.

**Perceiver.** The query carries the perceiver: the actor who sees or hears
the text. `seen_by(x)` is true when the perceiver is `x`. With one player
the perceiver is the player. After D6 it is the active actor.

**Learn.** A list of items that the perceiver learns when the text shows.
In the slice: `learn: [ash_is_fresh]` sets the fact `ash_is_fresh` to true.
The core does not apply `learn` itself. It returns the items, and the host
applies them in `commit`, with the perceiver. A preview does not commit, so
it learns nothing.

**Trace.** Each selection returns a trace: for each candidate, the verdict,
the score, and the variant step. The editor shows it as in the D2 side note:

```text
keeper_remark  (pick: best)
  -  "Mind your step."                    score 0  lost
  -  in(kitchen)                          score 1  lost
  *  in(kitchen) and ash_is_fresh         score 2  won, sticky variant 1 of 2
```

## 3. API sketch

The core lives in `dramaturge-model` (data) and `dramaturge-runtime`
(`text` module, as `text.rs` plus `text/`). The names follow the D2 side
note. They can change.

```rust
/// The host's types. dramaturge never looks inside `Leaf`, `Cond` or `Learn`.
pub trait TextTypes {
    type Leaf;                          // host text leaf (R8)
    type Cond: Specificity;             // host condition
    type Learn;                         // what a shown text teaches
    type Actor: Clone + Eq;             // speaker and perceiver
    type Topic: Clone + Eq;             // topic or thing
}

pub trait Specificity {
    /// Number of tests, by the rules in section 2.
    fn specificity(&self) -> u32;
}

#[non_exhaustive]
pub enum Pick { All, First, Best }

pub struct Pool<X: TextTypes> {
    pub id: Arc<str>,
    pub pick: Pick,
    pub candidates: Vec<Candidate<X>>,
}

pub struct Candidate<X: TextTypes> {
    pub key: Option<Arc<str>>,          // progress key; the index when None
    pub speaker: Option<X::Actor>,      // None is `*`
    pub topic: Option<X::Topic>,        // None is `*`
    pub when: Option<X::Cond>,
    pub mode: FragmentMode,
    pub variants: Vec<Entry<X>>,
    pub learn: Vec<X::Learn>,
    score: u32,                         // set by the builder
}

pub enum Entry<X: TextTypes> {
    Leaf(X::Leaf),
    Pool(Pool<X>),                      // inline `one_of`
    Named(Arc<str>),                    // a named pool, found in `Pools<X>`
}

pub struct Query<X: TextTypes> {
    pub speaker: Option<X::Actor>,
    pub topic: Option<X::Topic>,
    pub perceiver: Option<X::Actor>,
}

/// The host's condition evaluation. It borrows the host state.
pub trait Judge<X: TextTypes> {
    fn holds(&self, cond: &X::Cond, query: &Query<X>) -> bool;
}

/// Where the progress lives. `TextProgress` implements it; so can a host store.
pub trait ProgressStore {
    /// `None`: the candidate has not shown yet.
    fn fragment_progress(&self, key: &str) -> Option<FragmentProgress>;
    fn set_fragment_progress(&mut self, key: &str, progress: FragmentProgress);
    /// True on the first resolution of `pool` by `caller`. The caller is a
    /// scope that the host chooses (section 4). A host can answer from its
    /// own data (loremaster: the first visit of the room).
    fn first_time(&self, pool: &str, caller: &str) -> bool;
    fn count_resolution(&mut self, pool: &str, caller: &str);
}

/// Chooses, without a change to the progress. Shuffle and sticky draw from
/// `rng`; a preview passes a copy of the host RNG.
pub fn select<'p, X, J, P, R>(
    pool: &'p Pool<X>,
    pools: &'p Pools<X>,
    caller: Arc<str>,                   // moved into `Selection::caller`
    query: &Query<X>,
    judge: &J,
    progress: &P,
    rng: &mut R,
) -> Selection<'p, X>
where
    X: TextTypes, J: Judge<X> + ?Sized, P: ProgressStore + ?Sized, R: TextRng + ?Sized;

pub struct Selection<'p, X: TextTypes> {
    pub pool: Arc<str>,
    pub caller: Arc<str>,               // `commit` counts the resolution for it
    pub shown: Vec<Shown<'p, X>>,       // leaves in order, nested pools flattened
    pub trace: Trace,
}

pub struct Shown<'p, X: TextTypes> {
    pub progress_key: Arc<str>,
    pub variant: u32,
    pub leaf: &'p X::Leaf,
    pub learn: &'p [X::Learn],
}

/// Records what showed (shown counts, last and sticky variants) and the
/// resolution of `selection.pool` by `selection.caller`, then hands each
/// `learn` item to `learner` with the perceiver.
pub fn commit<X, P, L>(selection: &Selection<'_, X>, query: &Query<X>, progress: &mut P, learner: &mut L)
where
    X: TextTypes, P: ProgressStore + ?Sized, L: FnMut(&X::Learn, Option<&X::Actor>);

pub struct Trace { pub pool: Arc<str>, pub pick: Pick, pub rows: Vec<TraceRow> }
pub struct TraceRow {
    pub index: usize,
    pub key: Option<Arc<str>>,
    pub score: u32,
    pub verdict: Verdict,
    pub variant: Option<(u32, u32)>,    // (chosen, count)
    pub nested: Option<Trace>,
}
#[non_exhaustive]
pub enum Verdict { Won, Shown, Lost, TupleMismatch, ConditionFalse, Exhausted, Empty, MissingPool }
```

**Defaults for users of dramaturge alone.** `DialogTypes` implements
`TextTypes` with `Leaf = DialogText`, `Cond = Condition`, `Learn = Fact`,
`Actor = CharacterId` and `Topic = Arc<str>`. `Condition` gets a variant
`SeenBy(CharacterId)` (non-breaking: `Condition` is `#[non_exhaustive]`).
`ConditionDto` in `dramaturge-serde` needs the same variant and the mapping
in both directions. Without it, the conversion reaches its
`unknown_variant("Condition")` arm and panics on a `SeenBy` condition.
`DefaultJudge { evaluator, state }` walks `All`, `Any` and `Not`, answers
`SeenBy` from the query, and gives every other test to the
`ConditionEvaluator`. A `ConditionEvaluator` alone has no perceiver, so it
returns false for `SeenBy`. `resolve_text` keeps its signature and becomes a
`Pick::All` pool over the core, plus its own `replaces` step.

**Assumption on R8 (#75).** R8 is in progress. This note assumes only one
thing: a dialogue node and an option can hold a host value as their text,
either as `DialogText::Host(..)` or as a type parameter. The core never
names `DialogText` in its own types. It takes the leaf as `X::Leaf`. Thus a
dialogue line that says a pool (`say: { text: keeper_remark }`) is a host
leaf that names the pool, or a `DialogText` variant that R9 adds. Both work
with this core.

## 4. State, saves and determinism

- The progress key is `{pool id}/{candidate key or index}`, and nested
  pools append segments. The shape is that of `resolve_text` today, but
  there the first segment is the `owner` that the caller passes. An old key
  carries over only where the pool id is that same owner: an inline text of
  a node or a description keeps its keys; a named pool starts with new
  keys.
- A named pool has one progress for all callers. The sticky line of
  `keeper_remark` is the same in the rule and in the dialogue.
- `first_time` is the exception: it counts per pool and caller. A named
  pool that two rooms show has a `first_time` candidate for each room.
- The caller is the scope of "the first time", and the host chooses it.
  dramaturge passes the `owner` of `resolve_text`: the node or option key.
  loremaster passes the room for room text, and the current room for a
  rule or a dialogue line. Thus a rule and a dialogue node in one room
  share one first time, and `first_time` means "first visit" as today.
- `TextProgress` and its DTO (`dramaturge-serde`) store the progress. No new
  save field is necessary for users of dramaturge alone.
- The resolution counts stay in the `owners` map of `TextProgress`. The
  key is the caller alone when the pool id is the caller (an inline text:
  `resolve_text` passes the owner as both). Otherwise the key is
  `{pool id}@{caller}`. The validator rejects `@` in pool ids, so the keys
  cannot clash. Old counts are keyed by the bare owner, so they load for
  inline texts. Named pools are new and have no old counts.
- `ProgressStore` uses its own method names (`fragment_progress`,
  `set_fragment_progress`). Thus the inherent `TextProgress::fragment` and
  `set_fragment` do not shadow the trait methods. The impl for
  `TextProgress` forwards to them and keeps `None` for "not shown yet".
- A candidate without a key gets its index as key. A reorder then moves its
  progress to another candidate. The validator reports such candidates in
  pools with `once`, `cycle`, `shuffle` or `sticky` (issue kind
  `UnkeyedProgress`, already in R6).
- The core draws randomness only through `TextRng`. The host owns the RNG
  and its state. The same pool, query, state, progress and RNG state give
  the same selection and the same trace.
- `select` does not change state. `commit` changes only the progress and
  calls the learner. Thus the editor can preview and trace with no side
  effects.

## 5. Mapping from loremaster

| loremaster today | With the core | Stays in loremaster |
| --- | --- | --- |
| `Vec<CFragment>` of a room or thing | `Pool { pick: All }`; each fragment is a candidate | Layer sort, `replaces` per layer, runs with `Source`, the joining of text |
| `Vec<Segment>` (compiled markup) | `X::Leaf` | Rendering, inserts, inline choices |
| `loremaster_lang` `Condition` + `Eval::holds` | `X::Cond` + `Judge`; loremaster implements `Specificity` | The condition language |
| `FragmentMode::FirstVisit` | `first_time`; the caller is the room, and `ProgressStore::first_time` answers from the room visit | The alias in YAML |
| `State.progress` (`shown`, `sticky`) | `ProgressStore` on `State` | The save envelope (D5); `sticky` reads as `pick` |
| `State.rng` + `rng::below` | `TextRng` on `State.rng` | The RNG algorithm, so transcripts keep their text |
| New: `one_of`, `pick`, `seen_by`, `learn`, `texts/` | `Entry::Pool`, `Pick`, `Judge`, `commit` learner, `Entry::Named` | YAML and `world!` parsers; `learn` sets a world var |

loremaster calls `select` for each description, calls `commit` only in play,
then assembles the layers from `Selection::shown`. The fields `layer` and
`replaces` stay on the loremaster fragment. The core does not see them.

## 6. Surfaces

The YAML and `world!` forms are those of the D2 side note and of the slice
(`texts/keeper_remark.yaml`, `things/stove.yaml`, `dsl/texts.rs`,
`dsl/stove.rs`). This note adds only the tuple keys of TX-8:

```yaml
# texts/keeper_replies.yaml (ask and show)
id: keeper_replies
pick: best
one_of:
  - { speaker: keeper, topic: stove, text: '"Leave the stove alone."' }
  - { speaker: keeper, text: '"I keep the lock. That is all."' }
  - "Nobody answers."                       # the default row
```

```rust
texts keeper_replies best {
    speaker keeper topic stove => "\"Leave the stove alone.\"";
    speaker keeper => "\"I keep the lock. That is all.\"";
    "Nobody answers.";
}
```

`dramaturge-serde` reads the same keys into `Pool<DialogTypes>` for users of
dramaturge alone: `id`, `pick`, `one_of`, `when`, `text`, `variants`,
`mode`, `learn`, `speaker`, `topic`. `world!` checks ids at compile time and
warns about a tie in a `best` pool.

## 7. Open questions

| # | Question | Recommendation |
| --- | --- | --- |
| Q1 | How does `or` count for the score? | The score of the smaller side (side note proposal). Then a candidate never wins by an `or` branch that did not hold. |
| Q2 | Which pick is the default for named pools? | `first` everywhere (side note proposal). The author can predict the result from the file order. `best` is always written out, as in the slice. |
| Q3 | Who is the perceiver before D6, and what id does `seen_by` take? | The host passes it in `Query::perceiver`. loremaster passes the player until D6, and the slice writes `seen_by(player)`. With D6 it is the active actor (TX-9). |
| Q4 | Where does `learn` write? | The host decides, through the learner. loremaster sets a world var now. dramaturge's default writes `FactScope::World` in 0.3. A perceiver scope (`Character(id)`) waits for D6. |
| Q5 | `shuffle` in loremaster can repeat the last variant; in dramaturge it cannot. Which one? | The dramaturge rule (no immediate repeat). Re-record the transcripts that change, once, in the loremaster PR. The save field `sticky` becomes `pick` with a serde alias. |
| Q6 | Tuple fields: keys on the candidate, or conditions (`when: topic == stove`)? | Keys. They count 1 each, the validator can check them, and a host can index them later. |
| Q7 | An explicit `priority` to break ties in `best`? | Not in 0.3. The author adds a test or changes the order. The validator and `world!` warn about ties. |
| Q8 | Should the core report candidates that can never show? (The golden slice's log of authoring notes has an example: "too hot to touch".) | Not the core. The phase 4 checks (explorer, CK) report a candidate that no explored state shows. The static check reports only ties and unkeyed progress. |

## 8. Test cases

**The slice** (`loremaster`, test `slice_stove_text`, files
`things/stove.yaml` and `agendas/keeper.yaml`):

1. At the start, `examine stove` shows "The stove is cold.". `stove.hot` is
   false, so only candidate 1 (score 0) is eligible. It does not show "too
   hot".
2. After the waits to midnight, the keeper feeds the stove (`stove.hot: true`).
   `examine stove` shows "The ash on the grate is fresh.". Trace: candidate 1
   score 0 lost, candidate 2 score 1 lost, candidate 3 score 2 won.
3. After step 2, `ash_is_fresh` is true (`learn`, applied with the player as
   perceiver).

**Core tests in dramaturge** (with `DialogTypes`):

- `best_picks_highest_score`: the stove pool. Same results as the slice
  steps 1 and 2.
- `best_tie_goes_to_file_order`: two candidates with score 1 that both hold.
- `first_ignores_score`: the stove pool with `Pick::First` shows "The stove
  is cold." in both steps.
- `or_scores_smaller_side`: `a or (b and c)` has score 1.
- `seen_by_uses_query_perceiver`: candidate 3 loses when the perceiver is
  another character.
- `exhausted_once_falls_through`: a `once` candidate shows once, then the
  next candidate wins.
- `sticky_in_named_pool_survives_save`: `keeper_remark` with `ash_is_fresh`.
  Select and commit, save `TextProgress` through the DTO, load, select
  again: the same variant.
- `tuple_default_row`: `keeper_replies` with topic `stove` gives the stove
  line; with topic `lock` the keeper line; with speaker `mara` the default
  row.
- `preview_changes_nothing`: `select` without `commit` leaves the progress
  and the facts as they were. `learn` is not applied.
- `same_seed_same_text`: two runs with the same seed give the same shuffle
  sequence and the same trace.
- `trace_lists_every_candidate`: the trace of step 2 has three rows with the
  verdicts and scores above.
- `resolve_text_unchanged`: the existing `resolve_text` tests pass on the
  new core.
