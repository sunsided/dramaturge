//! Text pools in YAML: the slice cases of the text selection design note
//! (`docs/specs/2026-11-07-text-selection-design.md`, section 8), in the
//! condition language of plain dramaturge.

#![cfg(feature = "yaml")]

use assert2::{assert, check};
use dramaturge_model::{
    CharacterId, DialogText, DialogTypes, Fact, FactId, FactScope, FactValue, Pick, Pool, Pools,
    SymbolTable,
};
use dramaturge_runtime::{
    DefaultConditionEvaluator, DefaultJudge, DialogWorldState, Query, SplitMix64, TextProgress,
    Verdict, commit, select, select_named, world_learner,
};
use dramaturge_serde::loader::{
    LoadError, TextPool, load_pool_yaml, load_yaml, pool_from_yaml, pool_to_yaml,
};

/// `things/stove.yaml` of the slice: the description group.
const STOVE: &str = r#"
id: stove_state
pick: best
one_of:
  - "The stove is cold."
  - when: stove.hot
    text: "The stove is still too hot to touch."
  - when: stove.hot and seen_by(player)
    text: "Someone fed the stove after midnight. The ash on the grate is fresh."
    learn: [ash_is_fresh]
"#;

/// `texts/keeper_remark.yaml` of the slice, with plain text.
const KEEPER_REMARK: &str = r#"
id: keeper_remark
pick: best
one_of:
  - "Mind your step."
  - when: room == "kitchen"
    text: "Do not touch the stove, the lock-keeper says."
  - when: room == "kitchen" and ash_is_fresh
    key: midnight
    mode: sticky
    variants:
      - "I let it go out at midnight, he says. He does not look at you."
      - "Cold stove, cold supper, he says."
"#;

/// `texts/keeper_replies.yaml` of the design note.
const KEEPER_REPLIES: &str = r#"
id: keeper_replies
pick: best
one_of:
  - { speaker: keeper, topic: stove, text: '"Leave the stove alone."' }
  - { speaker: keeper, text: '"I keep the lock. That is all."' }
  - "Nobody answers."
"#;

/// `dialogues/keeper.yaml` of the slice, with plain text.
const KEEPER_DIALOGUE: &str = r#"
with: keeper
start: [stove]
nodes:
  stove:
    say: { text: keeper_remark }
    options:
      - say: "Who fed it?"
        when: ash_is_fresh
        go: who_fed_it
      - say: "Good night."
        go: end
  who_fed_it:
    say: ["He turns the poker in his hands.", "\"Nobody. The stove was cold all night.\""]
    options:
      - say: "Good night."
        go: end
"#;

fn shown(
    pool: &Pool<DialogTypes>,
    state: &DialogWorldState,
    query: &Query<DialogTypes>,
) -> Vec<String> {
    let evaluator = DefaultConditionEvaluator::new();
    let judge = DefaultJudge::new(&evaluator, state);
    let pools = Pools::new();
    let progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    select(pool, &pools, "kitchen", query, &judge, &progress, &mut rng)
        .leaves()
        .filter_map(DialogText::as_inline)
        .map(str::to_string)
        .collect()
}

#[test]
fn slice_stove_text() {
    let mut symbols = SymbolTable::new();
    let stove = load_pool_yaml(STOVE, &mut symbols).unwrap();
    let player = CharacterId::new(symbols.player());
    let query = Query::new().perceiver(player);
    let evaluator = DefaultConditionEvaluator::new();
    let pools = Pools::new();
    let mut state = DialogWorldState::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(5);

    // 1. At the start, `stove.hot` is false: only the cold stove shows.
    check!(shown(&stove, &state, &query) == ["The stove is cold."]);

    // 2. The keeper fed the stove: the fresh ash wins with score 2.
    state.set_fact(FactScope::World, Fact::bool("stove.hot", true));
    let judge = DefaultJudge::new(&evaluator, &state);
    let selection = select(
        &stove, &pools, "kitchen", &query, &judge, &progress, &mut rng,
    );
    let texts: Vec<_> = selection
        .leaves()
        .filter_map(DialogText::as_inline)
        .collect();
    check!(texts == ["Someone fed the stove after midnight. The ash on the grate is fresh."]);
    let rows: Vec<_> = selection
        .trace
        .rows
        .iter()
        .map(|r| (r.score, r.verdict))
        .collect();
    check!(rows == [(0, Verdict::Lost), (1, Verdict::Lost), (2, Verdict::Won)]);

    // 3. `learn`, applied with the player as perceiver.
    commit(
        &selection,
        &query,
        &mut progress,
        &mut world_learner(&mut state),
    );
    check!(
        state.get_fact(&FactScope::World, &FactId::new("ash_is_fresh"))
            == Some(&FactValue::Bool(true))
    );
}

#[test]
fn slice_keeper_dialogue() {
    let loaded = load_yaml(KEEPER_DIALOGUE).unwrap();
    let node = &loaded.graph.nodes[0];
    check!(node.text == DialogText::pool("keeper_remark"));

    let mut symbols = loaded.symbols;
    let pools = Pools::new().with(load_pool_yaml(KEEPER_REMARK, &mut symbols).unwrap());
    let DialogText::Pool(id) = &node.text else {
        unreachable!()
    };

    let mut state = DialogWorldState::new();
    state.set_fact(
        FactScope::World,
        Fact::new("room", FactValue::Text("kitchen".into())),
    );
    state.set_fact(FactScope::World, Fact::bool("ash_is_fresh", true));
    let evaluator = DefaultConditionEvaluator::new();
    let query = Query::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(9);
    let mut lines = Vec::new();
    for _ in 0..4 {
        let judge = DefaultJudge::new(&evaluator, &state);
        let s = select_named(id, &pools, "kitchen", &query, &judge, &progress, &mut rng);
        lines.extend(
            s.leaves()
                .filter_map(DialogText::as_inline)
                .map(str::to_string),
        );
        commit(&s, &query, &mut progress, &mut |_, _| {});
    }
    // The sticky line keeps its variant.
    check!(lines.len() == 4);
    check!(lines.iter().all(|l| *l == lines[0]));
    check!(lines[0].contains("he says"));
    check!(progress.fragment("keeper_remark/midnight").map(|p| p.shown) == Some(4));
}

#[test]
fn keeper_replies_by_speaker_and_topic() {
    let mut symbols = SymbolTable::new();
    let pool = load_pool_yaml(KEEPER_REPLIES, &mut symbols).unwrap();
    let keeper = CharacterId::new(symbols.entity("keeper").unwrap());
    let mara = CharacterId::new(symbols.intern_entity("mara"));
    let state = DialogWorldState::new();
    let ask = |who, topic: &str| Query::new().speaker(who).topic(topic.into());
    check!(shown(&pool, &state, &ask(keeper, "stove")) == ["\"Leave the stove alone.\""]);
    check!(shown(&pool, &state, &ask(keeper, "lock")) == ["\"I keep the lock. That is all.\""]);
    check!(shown(&pool, &state, &ask(mara, "stove")) == ["Nobody answers."]);
}

#[test]
fn inline_pool_in_a_description() {
    // The stove group as one entry of a description list (`pick: all`).
    let yaml = r#"
id: stove.description
pick: all
one_of:
  - "An iron stove."
  - id: stove_state
    pick: best
    one_of:
      - "It is cold."
      - when: stove.hot
        text: "It is hot."
"#;
    let mut symbols = SymbolTable::new();
    let pool = load_pool_yaml(yaml, &mut symbols).unwrap();
    let mut state = DialogWorldState::new();
    check!(shown(&pool, &state, &Query::new()) == ["An iron stove.", "It is cold."]);
    state.set_fact(FactScope::World, Fact::bool("stove.hot", true));
    check!(shown(&pool, &state, &Query::new()) == ["An iron stove.", "It is hot."]);
    let dramaturge_model::Entry::Pool(inner) = &pool.candidates[1].variants[0] else {
        panic!("an inline pool");
    };
    check!(&*inner.id == "stove_state");
    check!(inner.pick == Pick::Best);
}

#[test]
fn pool_round_trips_through_yaml() {
    for yaml in [STOVE, KEEPER_REMARK, KEEPER_REPLIES] {
        let pool: TextPool = pool_from_yaml(yaml).unwrap();
        let again: TextPool = pool_from_yaml(&pool_to_yaml(&pool).unwrap()).unwrap();
        check!(again == pool, "{yaml}");
    }
}

#[test]
fn pool_shape_errors() {
    let cases = [
        ("one_of:\n  - { txt: a }\nid: p\n", "did not match"),
        ("id: p\none_of:\n  - id: x\n    text: a\n", "need `one_of`"),
        ("id: p\npick: worst\none_of: []\n", "worst"),
    ];
    for (yaml, needle) in cases {
        let err = load_pool_yaml(yaml, &mut SymbolTable::new()).unwrap_err();
        assert!(let LoadError::Yaml(e) = &err, "{yaml}");
        check!(e.message().contains(needle), "{e}");
    }
    // A named pool needs an id.
    let err = load_pool_yaml("one_of: [a]\n", &mut SymbolTable::new()).unwrap_err();
    assert!(let LoadError::Lower(dramaturge_serde::loader::LowerError::Check(issues)) = &err);
    check!(issues[0].kind == dramaturge_syntax::IssueKind::NoPoolId);
}

#[test]
fn pool_reference_in_a_pool_round_trips() {
    // `text: { text: <pool id> }` names a pool; written back, it must not
    // read as the plain text `keeper_remark`.
    let yaml = "id: talk\none_of:\n  - text: { text: keeper_remark }\n";
    let pool: TextPool = pool_from_yaml(yaml).unwrap();
    let again: TextPool = pool_from_yaml(&pool_to_yaml(&pool).unwrap()).unwrap();
    check!(again == pool);
    let lowered = dramaturge_syntax::lower_pool_plain(&again, &mut SymbolTable::new()).unwrap();
    check!(lowered.candidates[0].variants[0] == dramaturge_model::Entry::named("keeper_remark"));
}
