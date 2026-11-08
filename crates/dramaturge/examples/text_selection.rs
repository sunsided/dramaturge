//! The text selection core: one line of a group, the most specific line,
//! replies by speaker and topic, a perceiver test, `learn`, and the trace.
//!
//! This is the stove and the lock-keeper of loremaster's Lock-Keeper slice,
//! with plain dramaturge types.
//!
//! Run with `cargo run --example text_selection`.

use dramaturge::{
    Candidate, CharacterId, Condition, DefaultConditionEvaluator, DefaultJudge, DialogText,
    DialogTypes, DialogWorldState, EntityId, Entry, Fact, FactId, FactScope, FactValue,
    FragmentMode, Pick, Pool, Pools, Query, Selection, SplitMix64, TextProgress, commit, select,
    select_named, world_learner,
};

fn is(name: &str) -> Condition {
    Condition::FactEquals {
        scope: FactScope::World,
        fact_id: FactId::new(name),
        value: FactValue::Bool(true),
    }
}

fn line(text: &str) -> Candidate<DialogTypes> {
    Candidate::leaf(DialogText::inline(text))
}

fn texts(selection: &Selection<'_, DialogTypes>) -> Vec<String> {
    selection
        .leaves()
        .filter_map(DialogText::as_inline)
        .map(str::to_string)
        .collect()
}

fn main() {
    let player = CharacterId::new(EntityId::new(0));
    let keeper = CharacterId::new(EntityId::new(1));

    // The stove: the line with the most tests wins (`pick: best`).
    let stove = Pool::new("stove", Pick::Best)
        .with_candidate(line("The stove is cold."))
        .with_candidate(line("The stove is still too hot to touch.").when(is("stove.hot")))
        .with_candidate(
            line("Someone fed the stove after midnight. The ash on the grate is fresh.")
                .when(Condition::All(vec![
                    is("stove.hot"),
                    Condition::SeenBy(player),
                ]))
                .learn(Fact::bool("ash_is_fresh", true)),
        );

    // Named pools: rules and dialogue call them by id.
    let pools = Pools::new()
        .with(
            Pool::new("keeper_remark", Pick::Best)
                .with_candidate(line("\"Mind your step.\""))
                .with_candidate(
                    Candidate::new(vec![
                        Entry::Leaf(DialogText::inline("\"I let it go out at midnight.\"")),
                        Entry::Leaf(DialogText::inline("\"Cold stove, cold supper.\"")),
                    ])
                    .keyed("midnight")
                    .mode(FragmentMode::Sticky)
                    .when(is("ash_is_fresh")),
                ),
        )
        .with(
            Pool::new("keeper_replies", Pick::Best)
                .with_candidate(
                    line("\"Leave the stove alone.\"")
                        .speaker(keeper)
                        .topic("stove".into()),
                )
                .with_candidate(line("\"I keep the lock. That is all.\"").speaker(keeper))
                .with_candidate(line("Nobody answers.")),
        );

    let evaluator = DefaultConditionEvaluator::new();
    let mut state = DialogWorldState::new();
    let mut progress = TextProgress::new();
    let mut rng = SplitMix64::new(2026);
    let seen = Query::new().perceiver(player);

    // Before midnight.
    let judge = DefaultJudge::new(&evaluator, &state);
    let s = select(
        &stove, &pools, "kitchen", &seen, &judge, &progress, &mut rng,
    );
    println!("> examine stove\n{}\n", texts(&s).join(" "));
    commit(&s, &seen, &mut progress, &mut world_learner(&mut state));

    // The keeper feeds the stove at midnight.
    state.set_fact(FactScope::World, Fact::bool("stove.hot", true));

    // A preview (an editor) selects with a copy of the RNG and does not
    // commit: nothing changes.
    let judge = DefaultJudge::new(&evaluator, &state);
    let mut preview_rng = rng;
    let preview = select(
        &stove,
        &pools,
        "kitchen",
        &seen,
        &judge,
        &progress,
        &mut preview_rng,
    );
    println!("Preview trace:\n{}", preview.trace);

    let s = select(
        &stove, &pools, "kitchen", &seen, &judge, &progress, &mut rng,
    );
    println!("> examine stove\n{}\n", texts(&s).join(" "));
    commit(&s, &seen, &mut progress, &mut world_learner(&mut state));
    println!(
        "ash_is_fresh: {:?}\n",
        state.get_fact(&FactScope::World, &FactId::new("ash_is_fresh"))
    );

    // The keeper's remark: the sticky line stays the same.
    for _ in 0..2 {
        let judge = DefaultJudge::new(&evaluator, &state);
        let q = Query::new();
        let s = select_named(
            "keeper_remark",
            &pools,
            "kitchen",
            &q,
            &judge,
            &progress,
            &mut rng,
        );
        println!("> talk to keeper\n{}\n", texts(&s).join(" "));
        commit(&s, &q, &mut progress, &mut |_, _| {});
    }

    // Ask and show: by speaker and topic, with a default row.
    for topic in ["stove", "lock"] {
        let judge = DefaultJudge::new(&evaluator, &state);
        let q = Query::new().speaker(keeper).topic(topic.into());
        let s = select_named(
            "keeper_replies",
            &pools,
            "kitchen",
            &q,
            &judge,
            &progress,
            &mut rng,
        );
        println!("> ask keeper about {topic}\n{}\n", texts(&s).join(" "));
    }
}
