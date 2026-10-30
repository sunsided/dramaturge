//! Lookup and clone costs of `DialogWorldState` with many entries.
//!
//! Run with `cargo bench -p dramaturge-runtime --bench world_state`.

use criterion::{Criterion, criterion_group, criterion_main};
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, EntityId, Fact, FactId, FactScope, RelationshipAxisId,
};
use dramaturge_runtime::DialogWorldState;
use std::hint::black_box;

const FACTS: usize = 10_000;
const ENTITIES: u64 = 100;

fn populated() -> DialogWorldState {
    let mut state = DialogWorldState::new();
    for i in 0..FACTS {
        let scope = if i % 2 == 0 {
            FactScope::World
        } else {
            FactScope::Player
        };
        state.set_fact(scope, Fact::int(format!("fact.{i}"), i as i64));
    }
    for source in 0..ENTITIES {
        for target in 0..ENTITIES {
            state.add_relationship(
                EntityId::new(source),
                EntityId::new(target),
                RelationshipAxisId::new("trust"),
                1,
            );
        }
    }
    for i in 0..100 {
        state.add_alignment(
            AlignmentAxisId::new(format!("axis.{i}")),
            AlignmentPoleId::new("pole"),
            i,
        );
    }
    state
}

fn bench_lookups(c: &mut Criterion) {
    let state = populated();
    let hit = FactId::new(format!("fact.{}", FACTS - 2));
    let miss = FactId::new("fact.missing");
    let trust = RelationshipAxisId::new("trust");
    let axis = AlignmentAxisId::new("axis.99");
    let pole = AlignmentPoleId::new("pole");

    c.bench_function("get_fact hit (10k facts)", |b| {
        b.iter(|| black_box(state.get_fact(black_box(&FactScope::World), black_box(&hit))))
    });
    c.bench_function("get_fact miss (10k facts)", |b| {
        b.iter(|| black_box(state.get_fact(black_box(&FactScope::World), black_box(&miss))))
    });
    c.bench_function("get_relationship (10k pairs)", |b| {
        b.iter(|| {
            black_box(state.get_relationship(
                black_box(EntityId::new(ENTITIES - 1)),
                black_box(EntityId::new(ENTITIES - 1)),
                black_box(&trust),
            ))
        })
    });
    c.bench_function("get_alignment (100 axes)", |b| {
        b.iter(|| black_box(state.get_alignment(black_box(&axis), black_box(&pole))))
    });
    c.bench_function("set_fact overwrite (10k facts)", |b| {
        let mut state = state.clone();
        b.iter(|| state.set_fact(FactScope::World, Fact::int(hit.0.clone(), 1)))
    });
    c.bench_function("clone (10k facts, 10k pairs)", |b| {
        b.iter(|| black_box(state.clone()))
    });
}

criterion_group!(benches, bench_lookups);
criterion_main!(benches);
