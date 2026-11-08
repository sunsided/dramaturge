//! The core tests of the text selection design note
//! (`docs/specs/2026-11-07-text-selection-design.md`, section 8), with
//! `DialogTypes`.

use assert2::{assert, check};
use dramaturge_model::{
    Candidate, CharacterId, Condition, DialogText, DialogTypes, EntityId, Fact, FactId, FactScope,
    FactValue, FragmentMode, Pick, Pool, Pools,
};
use dramaturge_runtime::{
    Count, CountKey, DefaultConditionEvaluator, DefaultJudge, DialogWorldState, ProgressStore,
    Query, Selection, SplitMix64, TextProgress, Verdict, commit, select, select_named,
    world_learner,
};

type X = DialogTypes;

fn player() -> CharacterId {
    CharacterId::new(EntityId::new(0))
}

fn keeper() -> CharacterId {
    CharacterId::new(EntityId::new(1))
}

fn mara() -> CharacterId {
    CharacterId::new(EntityId::new(2))
}

fn is(name: &str) -> Condition {
    Condition::FactEquals {
        scope: FactScope::World,
        fact_id: FactId::new(name),
        value: FactValue::Bool(true),
    }
}

fn in_room(room: &str) -> Condition {
    Condition::FactEquals {
        scope: FactScope::World,
        fact_id: FactId::new("room"),
        value: FactValue::Text(room.into()),
    }
}

fn text(s: &str) -> DialogText {
    DialogText::inline(s)
}

/// `things/stove.yaml` of the slice.
fn stove(pick: Pick) -> Pool<X> {
    Pool::new("stove_state", pick)
        .with_candidate(Candidate::leaf(text("The stove is cold.")))
        .with_candidate(
            Candidate::leaf(text("The stove is still too hot to touch.")).when(is("stove.hot")),
        )
        .with_candidate(
            Candidate::leaf(text(
                "Someone fed the stove after midnight. The ash on the grate is fresh.",
            ))
            .when(Condition::All(vec![
                is("stove.hot"),
                Condition::SeenBy(player()),
            ]))
            .learn(Fact::bool("ash_is_fresh", true)),
        )
}

/// `texts/keeper_remark.yaml` of the slice.
fn keeper_remark() -> Pool<X> {
    Pool::new("keeper_remark", Pick::Best)
        .with_candidate(Candidate::leaf(text("Mind your step.")))
        .with_candidate(Candidate::leaf(text("Do not touch the stove.")).when(in_room("kitchen")))
        .with_candidate(
            Candidate::new(vec![
                dramaturge_model::Entry::Leaf(text("I let it go out at midnight.")),
                dramaturge_model::Entry::Leaf(text("Cold stove, cold supper.")),
            ])
            .mode(FragmentMode::Sticky)
            .when(Condition::All(vec![in_room("kitchen"), is("ash_is_fresh")])),
        )
}

/// `texts/keeper_replies.yaml` of the design note (ask and show).
fn keeper_replies() -> Pool<X> {
    Pool::new("keeper_replies", Pick::Best)
        .with_candidate(
            Candidate::leaf(text("\"Leave the stove alone.\""))
                .speaker(keeper())
                .topic("stove".into()),
        )
        .with_candidate(
            Candidate::leaf(text("\"I keep the lock. That is all.\"")).speaker(keeper()),
        )
        .with_candidate(Candidate::leaf(text("Nobody answers.")))
}

struct World {
    state: DialogWorldState,
    evaluator: DefaultConditionEvaluator,
    progress: TextProgress,
    rng: SplitMix64,
    pools: Pools<X>,
}

impl World {
    fn new() -> Self {
        Self {
            state: DialogWorldState::new(),
            evaluator: DefaultConditionEvaluator::new(),
            progress: TextProgress::new(),
            rng: SplitMix64::new(7),
            pools: Pools::new(),
        }
    }

    fn set(&mut self, name: &str, value: FactValue) {
        self.state
            .set_fact(FactScope::World, Fact::new(name, value));
    }

    /// Selects without a commit (a preview).
    fn select(&mut self, pool: &Pool<X>, query: &Query<X>) -> Seen {
        let judge = DefaultJudge::new(&self.evaluator, &self.state);
        let s = select(
            pool,
            &self.pools,
            "kitchen",
            query,
            &judge,
            &self.progress,
            &mut self.rng,
        );
        Seen {
            texts: shown(&s),
            learned: s.steps.iter().map(|st| st.learn().len()).sum(),
            trace: s.trace,
        }
    }
}

/// What a preview showed.
struct Seen {
    texts: Vec<String>,
    learned: usize,
    trace: dramaturge_runtime::Trace,
}

fn shown(selection: &Selection<'_, X>) -> Vec<String> {
    selection
        .leaves()
        .map(|t| t.as_inline().expect("inline text").to_string())
        .collect()
}

/// Selects and commits `pool` with its own state, as play does.
fn play(
    pool: &Pool<X>,
    pools: &Pools<X>,
    caller: &str,
    query: &Query<X>,
    state: &mut DialogWorldState,
    progress: &mut TextProgress,
    rng: &mut SplitMix64,
) -> Vec<String> {
    let evaluator = DefaultConditionEvaluator::new();
    let judge = DefaultJudge::new(&evaluator, &*state);
    let selection = select(pool, pools, caller, query, &judge, &*progress, rng);
    let out = shown(&selection);
    commit(&selection, query, progress, &mut world_learner(state));
    out
}

#[test]
fn best_picks_highest_score() {
    let pool = stove(Pick::Best);
    let pools = Pools::new();
    let query = Query::new().perceiver(player());
    let mut state = DialogWorldState::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(1);

    // Slice step 1: only the cold stove (score 0) is eligible.
    let out = play(
        &pool,
        &pools,
        "kitchen",
        &query,
        &mut state,
        &mut progress,
        &mut rng,
    );
    check!(out == ["The stove is cold."]);

    // Slice step 2: the keeper fed the stove.
    state.set_fact(FactScope::World, Fact::bool("stove.hot", true));
    let out = play(
        &pool,
        &pools,
        "kitchen",
        &query,
        &mut state,
        &mut progress,
        &mut rng,
    );
    check!(out == ["Someone fed the stove after midnight. The ash on the grate is fresh."]);

    // Slice step 3: `learn`, applied with the player as perceiver.
    check!(
        state.get_fact(&FactScope::World, &FactId::new("ash_is_fresh"))
            == Some(&FactValue::Bool(true))
    );
}

#[test]
fn best_tie_goes_to_file_order() {
    let pool: Pool<X> = Pool::new("tie", Pick::Best)
        .with_candidate(Candidate::leaf(text("none")))
        .with_candidate(Candidate::leaf(text("first")).when(is("a")))
        .with_candidate(Candidate::leaf(text("second")).when(is("b")));
    let mut w = World::new();
    w.set("a", FactValue::Bool(true));
    w.set("b", FactValue::Bool(true));
    let selection = w.select(&pool, &Query::new());
    check!(selection.texts == ["first"]);
    let verdicts: Vec<_> = selection.trace.rows.iter().map(|r| r.verdict).collect();
    check!(verdicts == [Verdict::Lost, Verdict::Won, Verdict::Lost]);
}

#[test]
fn first_ignores_score() {
    let pool = stove(Pick::First);
    let query = Query::new().perceiver(player());
    let mut w = World::new();
    check!((w.select(&pool, &query).texts) == ["The stove is cold."]);
    w.set("stove.hot", FactValue::Bool(true));
    check!((w.select(&pool, &query).texts) == ["The stove is cold."]);
}

#[test]
fn or_scores_smaller_side() {
    let c: Candidate<X> = Candidate::leaf(text("x")).when(Condition::Any(vec![
        is("a"),
        Condition::All(vec![is("b"), is("c")]),
    ]));
    check!(c.score() == 1);
}

#[test]
fn seen_by_uses_query_perceiver() {
    let pool = stove(Pick::Best);
    let mut w = World::new();
    w.set("stove.hot", FactValue::Bool(true));

    let selection = w.select(&pool, &Query::new().perceiver(mara()));
    check!(selection.texts == ["The stove is still too hot to touch."]);
    check!(selection.trace.rows[2].verdict == Verdict::ConditionFalse);

    // Without a perceiver, `SeenBy` is false too.
    let selection = w.select(&pool, &Query::new());
    check!(selection.trace.rows[2].verdict == Verdict::ConditionFalse);

    // A `ConditionEvaluator` alone has no perceiver.
    use dramaturge_runtime::ConditionEvaluator;
    check!(!w.evaluator.evaluate(&Condition::SeenBy(player()), &w.state));
}

#[test]
fn exhausted_once_falls_through() {
    let pool: Pool<X> = Pool::new("greeting", Pick::First)
        .with_candidate(
            Candidate::leaf(text("Welcome, stranger."))
                .mode(FragmentMode::Once)
                .keyed("welcome"),
        )
        .with_candidate(Candidate::leaf(text("You again.")));
    let pools = Pools::new();
    let query = Query::new();
    let mut state = DialogWorldState::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    let mut run = || {
        play(
            &pool,
            &pools,
            "inn",
            &query,
            &mut state,
            &mut progress,
            &mut rng,
        )
    };
    check!(run() == ["Welcome, stranger."]);
    check!(run() == ["You again."]);
    check!(run() == ["You again."]);

    let mut w = World::new();
    w.progress = progress;
    let selection = w.select(&pool, &query);
    check!(selection.trace.rows[0].verdict == Verdict::Exhausted);
    check!(selection.trace.rows[1].verdict == Verdict::Won);
}

#[test]
fn tuple_default_row() {
    let pool = keeper_replies();
    let mut w = World::new();
    let ask = |speaker: CharacterId, topic: &str| Query::new().speaker(speaker).topic(topic.into());
    check!((w.select(&pool, &ask(keeper(), "stove")).texts) == ["\"Leave the stove alone.\""]);
    check!(
        (w.select(&pool, &ask(keeper(), "lock")).texts) == ["\"I keep the lock. That is all.\""]
    );
    check!((w.select(&pool, &ask(mara(), "stove")).texts) == ["Nobody answers."]);

    let selection = w.select(&pool, &ask(mara(), "stove"));
    let verdicts: Vec<_> = selection.trace.rows.iter().map(|r| r.verdict).collect();
    check!(verdicts == [Verdict::TupleMismatch, Verdict::TupleMismatch, Verdict::Won]);
    let scores: Vec<_> = selection.trace.rows.iter().map(|r| r.score).collect();
    check!(scores == [2, 1, 0]);
}

#[test]
fn preview_changes_nothing() {
    let pool = stove(Pick::Best);
    let mut w = World::new();
    w.set("stove.hot", FactValue::Bool(true));
    let before_state = w.state.clone();
    let rng = w.rng;
    let selection = w.select(&pool, &Query::new().perceiver(player()));
    check!(!selection.texts.is_empty());
    check!(selection.learned == 1);
    // No commit: the progress and the facts are as they were.
    check!(w.progress == TextProgress::new());
    check!(w.state == before_state);
    check!(
        w.state
            .get_fact(&FactScope::World, &FactId::new("ash_is_fresh"))
            .is_none()
    );
    // A preview with a copy of the host RNG leaves the host RNG as it was.
    let mut copy = rng;
    let judge = DefaultJudge::new(&w.evaluator, &w.state);
    let _ = select(
        &pool,
        &w.pools,
        "kitchen",
        &Query::new(),
        &judge,
        &w.progress,
        &mut copy,
    );
    check!(w.rng == rng);
}

#[test]
fn same_seed_same_text() {
    let pool: Pool<X> = Pool::new("weather", Pick::All).with_candidate(
        Candidate::new(
            ["Rain.", "Wind.", "Fog.", "Sun."]
                .map(|s| dramaturge_model::Entry::Leaf(text(s)))
                .into(),
        )
        .mode(FragmentMode::Shuffle)
        .keyed("sky"),
    );
    let run = |seed: u64| {
        let pools = Pools::new();
        let query = Query::new();
        let mut state = DialogWorldState::new();
        let mut progress = TextProgress::new();
        let mut rng = SplitMix64::new(seed);
        let mut traces = Vec::new();
        let mut texts = Vec::new();
        for _ in 0..20 {
            let evaluator = DefaultConditionEvaluator::new();
            let judge = DefaultJudge::new(&evaluator, &state);
            let selection = select(&pool, &pools, "x", &query, &judge, &progress, &mut rng);
            traces.push(selection.trace.clone());
            texts.extend(shown(&selection));
            commit(
                &selection,
                &query,
                &mut progress,
                &mut world_learner(&mut state),
            );
        }
        (texts, traces)
    };
    let (a, ta) = run(42);
    let (b, tb) = run(42);
    check!(a == b);
    check!(ta == tb);
    check!(a.windows(2).all(|w| w[0] != w[1]));
}

#[test]
fn trace_lists_every_candidate() {
    let pool = stove(Pick::Best);
    let mut w = World::new();
    w.set("stove.hot", FactValue::Bool(true));
    let selection = w.select(&pool, &Query::new().perceiver(player()));
    let trace = &selection.trace;
    check!(&*trace.pool == "stove_state");
    check!(trace.pick == Pick::Best);
    let rows: Vec<_> = trace.rows.iter().map(|r| (r.score, r.verdict)).collect();
    check!(rows == [(0, Verdict::Lost), (1, Verdict::Lost), (2, Verdict::Won)]);
    check!(trace.rows[2].variant == Some((0, 1)));
    check!(trace.winner().map(|r| r.index) == Some(2));
    check!(
        trace.to_string()
            == "stove_state  (pick: best)\n  -  #0  score 0  lost\n  -  #1  score 1  lost\n  *  #2  score 2  won\n"
    );
}

#[test]
fn resolve_text_unchanged() {
    use dramaturge_model::TextFragment;
    use dramaturge_runtime::resolve_text;
    // The keys and counts of `resolve_text` are those of 0.3.0-alpha.4.
    let text = DialogText::fragments(vec![
        TextFragment::with_mode(FragmentMode::FirstTime, vec![text("first")]),
        TextFragment::always(DialogText::fragments(vec![
            TextFragment::with_mode(FragmentMode::Cycle, vec![text("x"), text("y")]).keyed("inner"),
        ]))
        .keyed("outer"),
        TextFragment::always(text("replaced")).replacing(),
    ]);
    let state = DialogWorldState::new();
    let evaluator = DefaultConditionEvaluator::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    let first = resolve_text(&text, "n", &state, &evaluator, &mut progress, &mut rng);
    let texts: Vec<_> = first.iter().filter_map(|p| p.text.as_str()).collect();
    check!(texts == ["replaced"]);
    let keys: Vec<_> = progress
        .fragments()
        .map(|(k, p)| (k.to_string(), p.shown))
        .collect();
    check!(
        keys == [
            ("n/0".to_string(), 1),
            ("n/2".to_string(), 1),
            ("n/outer".to_string(), 1),
            ("n/outer/inner".to_string(), 1),
        ]
    );
    check!(progress.resolutions("n") == 1);
    check!(progress.named().count() == 0);
}

// ----------------------------------------------------- the review points

#[test]
fn named_pool_has_one_progress_for_all_callers() {
    let pools = Pools::new().with(keeper_remark());
    let query = Query::new();
    let mut state = DialogWorldState::new();
    state.set_fact(
        FactScope::World,
        Fact::new("room", FactValue::Text("kitchen".into())),
    );
    state.set_fact(FactScope::World, Fact::bool("ash_is_fresh", true));
    let evaluator = DefaultConditionEvaluator::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(3);

    let say = |caller: &str, progress: &mut TextProgress, rng: &mut SplitMix64| {
        let judge = DefaultJudge::new(&evaluator, &state);
        let s = select_named(
            "keeper_remark",
            &pools,
            caller,
            &query,
            &judge,
            &*progress,
            rng,
        );
        let out = shown(&s);
        commit(&s, &query, progress, &mut |_, _| {});
        out
    };
    let rule = say("kitchen", &mut progress, &mut rng);
    let dialogue = say("kitchen", &mut progress, &mut rng);
    let other_room = say("lock_chamber", &mut progress, &mut rng);
    check!(rule == dialogue);
    check!(rule == other_room);
    // The progress key starts with the pool id, whatever the caller.
    check!(progress.fragment("keeper_remark/2").map(|p| p.shown) == Some(3));
    // The counts are per caller.
    check!(progress.named_resolutions("keeper_remark", "kitchen") == 2);
    check!(progress.named_resolutions("keeper_remark", "lock_chamber") == 1);
    check!(progress.resolutions("keeper_remark") == 0);
}

#[test]
fn first_time_in_named_pool_is_per_caller() {
    let pools = Pools::new().with(
        Pool::new("greeting", Pick::First)
            .with_candidate(
                Candidate::leaf(text("For the first time.")).mode(FragmentMode::FirstTime),
            )
            .with_candidate(Candidate::leaf(text("Again."))),
    );
    let query = Query::new();
    let state = DialogWorldState::new();
    let evaluator = DefaultConditionEvaluator::new();
    let judge = DefaultJudge::new(&evaluator, &state);
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    let mut say = |caller: &str| {
        let s = select_named(
            "greeting", &pools, caller, &query, &judge, &progress, &mut rng,
        );
        let out = shown(&s);
        commit(&s, &query, &mut progress, &mut |_, _| {});
        out
    };
    check!(say("kitchen") == ["For the first time."]);
    check!(say("kitchen") == ["Again."]);
    check!(say("lock_chamber") == ["For the first time."]);
}

#[test]
fn inline_root_counts_per_owner_not_per_caller() {
    // An inline pool has one place: its pool id is the owner, and the
    // caller does not split its first time.
    let pool: Pool<X> = Pool::new("kitchen.description", Pick::All)
        .with_candidate(Candidate::leaf(text("A warm kitchen.")).mode(FragmentMode::FirstTime))
        .with_candidate(Candidate::leaf(text("The kitchen.")));
    let pools = Pools::new();
    let query = Query::new();
    let mut state = DialogWorldState::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    let a = play(
        &pool,
        &pools,
        "rule",
        &query,
        &mut state,
        &mut progress,
        &mut rng,
    );
    let b = play(
        &pool,
        &pools,
        "dialogue",
        &query,
        &mut state,
        &mut progress,
        &mut rng,
    );
    check!(a == ["A warm kitchen.", "The kitchen."]);
    check!(b == ["The kitchen."]);
    check!(progress.resolutions("kitchen.description") == 2);
    check!(
        progress.first_time(Count::Inline {
            owner: "kitchen.description"
        }) == false
    );
}

#[test]
fn counts_list_only_named_pools_that_showed() {
    let pools = Pools::new()
        .with(Pool::new("remark", Pick::First).with_candidate(Candidate::leaf(text("Hm."))))
        .with(
            Pool::new("hidden", Pick::First)
                .with_candidate(Candidate::leaf(text("never")).when(is("nope"))),
        )
        .with(Pool::new("loser", Pick::First).with_candidate(Candidate::leaf(text("lost line"))));
    let root: Pool<X> = Pool::new("talk", Pick::All)
        .with_candidate(Candidate::named("remark"))
        .with_candidate(Candidate::named("hidden"))
        .with_candidate(Candidate::named("remark"))
        .with_candidate(Candidate::pool(
            Pool::new("inner", Pick::Best)
                .with_candidate(Candidate::named("loser"))
                .with_candidate(Candidate::leaf(text("winner")).speaker(keeper())),
        ));
    let state = DialogWorldState::new();
    let evaluator = DefaultConditionEvaluator::new();
    let judge = DefaultJudge::new(&evaluator, &state);
    let progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    let query = Query::new().speaker(keeper());
    let s = select(
        &root, &pools, "kitchen", &query, &judge, &progress, &mut rng,
    );
    check!(shown(&s) == ["Hm.", "Hm.", "winner"]);
    // The root, then `remark` once. Not `hidden` (no leaf) and not `loser`
    // (its candidate lost, so it was not resolved).
    check!(
        s.counts
            == [
                CountKey::Inline("talk".into()),
                CountKey::Named("remark".into())
            ]
    );
    // `remark` showed twice in one selection: its progress steps twice.
    check!(
        s.steps
            .iter()
            .filter(|st| &*st.progress_key == "remark/0")
            .count()
            == 2
    );
    check!(s.steps.last().map(|st| st.progress.shown) == Some(1));
    let remark_steps: Vec<_> = s
        .steps
        .iter()
        .filter(|st| &*st.progress_key == "remark/0")
        .map(|st| st.progress.shown)
        .collect();
    check!(remark_steps == [1, 2]);
}

#[test]
fn missing_and_recursive_pools() {
    let pools: Pools<X> = Pools::new()
        .with(Pool::new("loop", Pick::First).with_candidate(Candidate::named("loop")))
        .with(Pool::new("gap", Pick::First).with_candidate(Candidate::named("nowhere")));
    let state = DialogWorldState::new();
    let evaluator = DefaultConditionEvaluator::new();
    let judge = DefaultJudge::new(&evaluator, &state);
    let progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    let query = Query::new();

    let s = select_named("loop", &pools, "c", &query, &judge, &progress, &mut rng);
    check!(s.is_empty());
    check!(s.trace.rows[0].verdict == Verdict::Recursive);
    // A recursive pool exists: its row has no `missing` trace.
    check!(s.trace.rows[0].nested.is_none());
    check!(!s.trace.to_string().contains("missing"));
    check!(s.counts.is_empty());

    let s = select_named("gap", &pools, "c", &query, &judge, &progress, &mut rng);
    check!(s.trace.rows[0].verdict == Verdict::MissingPool);
    let nested = s.trace.rows[0].nested.as_deref();
    assert!(let Some(nested) = nested);
    check!(nested.missing);

    let s = select_named("nowhere", &pools, "c", &query, &judge, &progress, &mut rng);
    check!(s.trace.missing);
    check!(s.steps.is_empty());
}

#[test]
fn trace_shows_nested_pools() {
    let pools = Pools::new().with(
        Pool::new("remark", Pick::First).with_candidate(
            Candidate::new(vec![
                dramaturge_model::Entry::Leaf(text("a")),
                dramaturge_model::Entry::Leaf(text("b")),
            ])
            .mode(FragmentMode::Cycle),
        ),
    );
    let root: Pool<X> =
        Pool::new("talk", Pick::First).with_candidate(Candidate::named("remark").keyed("say"));
    let state = DialogWorldState::new();
    let evaluator = DefaultConditionEvaluator::new();
    let judge = DefaultJudge::new(&evaluator, &state);
    let progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    let s = select(
        &root,
        &pools,
        "c",
        &Query::new(),
        &judge,
        &progress,
        &mut rng,
    );
    check!(
        s.trace.to_string()
            == "talk  (pick: first)\n  *  say  score 0  won\n    remark  (pick: first)\n      *  #0  score 0  won, variant 1 of 2\n"
    );
}

#[test]
fn unopenable_named_pool_has_no_side_effects() {
    // A `Once` candidate whose variant names a missing or recursive pool
    // shows nothing, so it gets no progress, no count and no `learn`.
    let pools: Pools<X> = Pools::new().with(
        Pool::new("loop", Pick::First).with_candidate(
            Candidate::named("loop")
                .mode(FragmentMode::Once)
                .learn(Fact::bool("looped", true)),
        ),
    );
    let root: Pool<X> = Pool::new("talk", Pick::First).with_candidate(
        Candidate::named("nowhere")
            .mode(FragmentMode::Once)
            .learn(Fact::bool("learned", true)),
    );
    let evaluator = DefaultConditionEvaluator::new();
    let query = Query::new();
    let mut state = DialogWorldState::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    for _ in 0..2 {
        let judge = DefaultJudge::new(&evaluator, &state);
        let s = select(&root, &pools, "c", &query, &judge, &progress, &mut rng);
        check!(s.trace.rows[0].verdict == Verdict::MissingPool);
        check!(s.steps.is_empty());
        commit(&s, &query, &mut progress, &mut world_learner(&mut state));

        let judge = DefaultJudge::new(&evaluator, &state);
        let s = select_named("loop", &pools, "c", &query, &judge, &progress, &mut rng);
        check!(s.trace.rows[0].verdict == Verdict::Recursive);
        check!(s.steps.is_empty());
        commit(&s, &query, &mut progress, &mut world_learner(&mut state));
    }
    check!(progress.fragments().count() == 0);
    check!(progress.named_resolutions("loop", "c") == 0);
    check!(
        state
            .get_fact(&FactScope::World, &FactId::new("learned"))
            .is_none()
    );
    check!(
        state
            .get_fact(&FactScope::World, &FactId::new("looped"))
            .is_none()
    );
}

#[test]
fn named_pool_without_leaves_has_no_side_effects() {
    // `quiet` exists but shows nothing, so the `Once` candidate that names
    // it gets no progress, no count and no `learn`.
    let pools: Pools<X> = Pools::new().with(
        Pool::new("quiet", Pick::First)
            .with_candidate(Candidate::leaf(text("never")).when(is("nope"))),
    );
    let root: Pool<X> = Pool::new("talk", Pick::First).with_candidate(
        Candidate::named("quiet")
            .mode(FragmentMode::Once)
            .learn(Fact::bool("learned", true)),
    );
    let evaluator = DefaultConditionEvaluator::new();
    let query = Query::new();
    let mut state = DialogWorldState::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    let judge = DefaultJudge::new(&evaluator, &state);
    let s = select(&root, &pools, "c", &query, &judge, &progress, &mut rng);
    check!(s.is_empty());
    check!(s.steps.is_empty());
    commit(&s, &query, &mut progress, &mut world_learner(&mut state));
    check!(progress.fragments().count() == 0);
    check!(progress.named_resolutions("quiet", "c") == 0);
    check!(
        state
            .get_fact(&FactScope::World, &FactId::new("learned"))
            .is_none()
    );
}

#[test]
fn unopenable_candidate_does_not_draw() {
    use dramaturge_model::Entry;
    let shuffled = || {
        Candidate::new(vec![Entry::Leaf(text("a")), Entry::Leaf(text("b"))])
            .mode(FragmentMode::Shuffle)
    };
    let with_gap: Pool<X> = Pool::new("p", Pick::All)
        .with_candidate(
            Candidate::new(vec![Entry::named("nowhere"), Entry::named("gone")])
                .mode(FragmentMode::Shuffle),
        )
        .with_candidate(shuffled());
    let without: Pool<X> = Pool::new("p", Pick::All).with_candidate(shuffled());
    let state = DialogWorldState::new();
    let evaluator = DefaultConditionEvaluator::new();
    let judge = DefaultJudge::new(&evaluator, &state);
    let progress = TextProgress::new();
    let pools = Pools::new();
    let (mut r1, mut r2) = (SplitMix64::new(3), SplitMix64::new(3));
    let q = Query::new();
    let a = select(&with_gap, &pools, "c", &q, &judge, &progress, &mut r1);
    let b = select(&without, &pools, "c", &q, &judge, &progress, &mut r2);
    check!(a.trace.rows[0].verdict == Verdict::MissingPool);
    check!(a.trace.rows[0].variant.is_none());
    check!(a.trace.rows[1].variant == b.trace.rows[0].variant);
    check!(r1.state() == r2.state());
}

#[test]
fn colliding_keys_never_show() {
    // `0` is the key of the unkeyed candidate 0; `k` is there twice.
    let once = |s: &str| Candidate::leaf(text(s)).mode(FragmentMode::Once);
    let pool: Pool<X> = Pool::new("p", Pick::All)
        .with_candidate(once("a"))
        .with_candidate(once("b").keyed("0"))
        .with_candidate(once("c").keyed("k"))
        .with_candidate(once("d").keyed("k"))
        .with_candidate(once("e").keyed("x/0"));
    let state = DialogWorldState::new();
    let evaluator = DefaultConditionEvaluator::new();
    let judge = DefaultJudge::new(&evaluator, &state);
    let progress = TextProgress::new();
    let mut rng = SplitMix64::new(0);
    let pools = Pools::new();
    let s = select(
        &pool,
        &pools,
        "c",
        &Query::new(),
        &judge,
        &progress,
        &mut rng,
    );
    let shown: Vec<_> = s.leaves().filter_map(DialogText::as_inline).collect();
    check!(shown == ["a", "c"]);
    let verdicts: Vec<_> = s.trace.rows.iter().map(|r| r.verdict).collect();
    check!(
        verdicts
            == [
                Verdict::Shown,
                Verdict::BadKey,
                Verdict::Shown,
                Verdict::BadKey,
                Verdict::BadKey
            ]
    );
}
