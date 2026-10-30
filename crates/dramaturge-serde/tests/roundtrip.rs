//! JSON round-trip tests for the model and runtime DTOs.

use assert2::check;
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, CharacterId, Condition, Confidence, CustomDialogTag,
    DialogIntentions, DialogTarget, DialogText, DialogueGraph, DialogueNode, DialogueNodeId,
    DialogueOption, DialogueOptionId, EntityId, EventKind, Fact, FactId, FactScope, FactValue,
    Intention, Memory, MemoryId, MemoryQuery, MemoryTag, NavigationHint, Outcome,
    RelationshipAxisId, SymbolTable,
};
use dramaturge_runtime::DialogWorldState;
use dramaturge_serde::dto::{ConditionDto, DialogueGraphDto, IntoModel, OutcomeDto, WorldStateDto};
use dramaturge_serde::save::SavedWorldState;
use proptest::collection::vec;
use proptest::prelude::*;
use serde::{Deserialize, Serialize};

fn key() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_.]{0,8}"
}

fn entity() -> impl Strategy<Value = EntityId> {
    (0u64..8).prop_map(EntityId::new)
}

fn fact_value() -> impl Strategy<Value = FactValue> {
    let leaf = prop_oneof![
        any::<bool>().prop_map(FactValue::Bool),
        any::<i64>().prop_map(FactValue::Int),
        // JSON has no NaN or infinity.
        any::<f64>()
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(FactValue::Float),
        ".{0,12}".prop_map(FactValue::Text),
        entity().prop_map(FactValue::Entity),
    ];
    leaf.prop_recursive(2, 8, 4, |inner| vec(inner, 0..4).prop_map(FactValue::Set))
}

fn fact_scope() -> impl Strategy<Value = FactScope> {
    prop_oneof![
        Just(FactScope::World),
        Just(FactScope::Player),
        Just(FactScope::Party),
        key().prop_map(|k| FactScope::Faction(k.into())),
        entity().prop_map(|e| FactScope::Character(CharacterId::new(e))),
    ]
}

fn tag() -> impl Strategy<Value = CustomDialogTag> {
    (key(), key(), proptest::option::of(key()), any::<bool>()).prop_map(|(a, k, p, r)| {
        let mut t = CustomDialogTag::new(a, k).with_requires_trait(r);
        t.display_prefix = p.map(Into::into);
        t
    })
}

fn memory_tag() -> impl Strategy<Value = MemoryTag> {
    prop_oneof![
        Just(MemoryTag::HelpedMe),
        Just(MemoryTag::BetrayedMe),
        Just(MemoryTag::SavedLife),
        key().prop_map(|k| MemoryTag::Custom(k.into())),
    ]
}

fn confidence() -> impl Strategy<Value = Confidence> {
    prop_oneof![
        Just(Confidence::Rumor),
        Just(Confidence::Suspected),
        Just(Confidence::Believed),
        Just(Confidence::Known),
        Just(Confidence::Witnessed),
    ]
}

fn memory_query() -> impl Strategy<Value = MemoryQuery> {
    (
        proptest::option::of(memory_tag()),
        proptest::option::of(confidence()),
        proptest::option::of(any::<i32>()),
        proptest::option::of(entity()),
        vec(key().prop_map(FactId::new), 0..3),
    )
        .prop_map(
            |(tag, min_confidence, min_emotional_weight, subject, facts)| MemoryQuery {
                tag,
                min_confidence,
                min_emotional_weight,
                subject,
                facts,
            },
        )
}

fn memory() -> impl Strategy<Value = Memory> {
    (
        0u64..16,
        proptest::option::of(entity()),
        vec(entity(), 0..3),
        vec(memory_tag(), 0..3),
        any::<i32>(),
        confidence(),
    )
        .prop_map(|(id, subject, about, tags, weight, conf)| {
            let mut m = Memory::new(MemoryId::new(id))
                .with_about(about)
                .with_tags(tags)
                .with_emotional_weight(weight)
                .with_confidence(conf);
            m.subject = subject;
            m
        })
}

fn intentions() -> impl Strategy<Value = DialogIntentions> {
    let intention = prop_oneof![
        Just(Intention::Persuade),
        Just(Intention::Exit),
        key().prop_map(|k| Intention::Learn(FactId::new(k))),
        key().prop_map(|k| Intention::Custom(k.into())),
    ];
    (vec(intention.clone(), 0..3), vec(intention, 0..3))
        .prop_map(|(explicit, implicit)| DialogIntentions { explicit, implicit })
}

fn event_kind() -> impl Strategy<Value = EventKind> {
    prop_oneof![
        (0u64..8, 0u64..8, vec(tag(), 0..2), intentions()).prop_map(|(n, o, tags, i)| {
            EventKind::DialogOptionChosen {
                node_id: DialogueNodeId::new(n),
                option_id: DialogueOptionId::new(o),
                tags,
                intentions: i,
            }
        }),
        (fact_scope(), key()).prop_map(|(scope, f)| EventKind::FactLearned {
            scope,
            fact_id: FactId::new(f),
        }),
        (entity(), entity(), key(), any::<i32>()).prop_map(|(a, b, axis, delta)| {
            EventKind::RelationshipChanged {
                a,
                b,
                axis: RelationshipAxisId::new(axis),
                delta,
            }
        }),
        key().prop_map(|k| EventKind::Custom { kind: k.into() }),
    ]
}

fn condition() -> impl Strategy<Value = Condition> {
    let leaf = prop_oneof![
        Just(Condition::Always),
        Just(Condition::Never),
        (fact_scope(), key(), fact_value()).prop_map(|(scope, f, value)| {
            Condition::FactEquals {
                scope,
                fact_id: FactId::new(f),
                value,
            }
        }),
        (fact_scope(), key()).prop_map(|(scope, f)| Condition::FactExists {
            scope,
            fact_id: FactId::new(f),
        }),
        (fact_scope(), key(), any::<i64>()).prop_map(|(scope, f, value)| {
            Condition::FactAtMost {
                scope,
                fact_id: FactId::new(f),
                value,
            }
        }),
        (key(), key(), any::<i32>()).prop_map(|(a, p, value)| Condition::AlignmentAtLeast {
            axis: AlignmentAxisId::new(a),
            pole: AlignmentPoleId::new(p),
            value,
        }),
        (entity(), entity(), key(), any::<i32>()).prop_map(|(source, target, a, value)| {
            Condition::RelationshipAtMost {
                source,
                target,
                axis: RelationshipAxisId::new(a),
                value,
            }
        }),
        (entity(), memory_query()).prop_map(|(owner, query)| Condition::HasMemory { owner, query }),
        tag().prop_map(|tag| Condition::HasTagAvailable { tag }),
        (key(), vec(fact_value(), 0..3)).prop_map(|(key, args)| Condition::Custom { key, args }),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            vec(inner.clone(), 0..4).prop_map(Condition::All),
            vec(inner.clone(), 0..4).prop_map(Condition::Any),
            inner.prop_map(Condition::negate),
        ]
    })
}

fn outcome() -> impl Strategy<Value = Outcome> {
    prop_oneof![
        (fact_scope(), key(), fact_value()).prop_map(|(scope, id, value)| Outcome::SetFact {
            scope,
            fact: Fact {
                id: FactId::new(id),
                value,
            },
        }),
        (fact_scope(), key()).prop_map(|(scope, f)| Outcome::RemoveFact {
            scope,
            fact_id: FactId::new(f),
        }),
        (key(), key(), any::<i32>()).prop_map(|(a, p, delta)| Outcome::AddAlignment {
            axis: AlignmentAxisId::new(a),
            pole: AlignmentPoleId::new(p),
            delta,
        }),
        (entity(), entity(), key(), any::<i32>()).prop_map(|(source, target, a, delta)| {
            Outcome::AddRelationship {
                source,
                target,
                axis: RelationshipAxisId::new(a),
                delta,
            }
        }),
        (entity(), memory()).prop_map(|(owner, memory)| Outcome::AddMemory { owner, memory }),
        (entity(), 0u64..16).prop_map(|(owner, m)| Outcome::RemoveMemory {
            owner,
            memory_id: MemoryId::new(m),
        }),
        event_kind().prop_map(|kind| Outcome::EmitEvent { kind }),
        key().prop_map(|quest_id| Outcome::StartQuest { quest_id }),
        (key(), key()).prop_map(|(quest_id, stage)| Outcome::AdvanceQuest { quest_id, stage }),
        (key(), vec(fact_value(), 0..3)).prop_map(|(key, args)| Outcome::Custom { key, args }),
    ]
}

fn text() -> impl Strategy<Value = DialogText> {
    prop_oneof![
        ".{0,16}".prop_map(DialogText::Inline),
        key().prop_map(DialogText::localized),
    ]
}

fn target() -> impl Strategy<Value = DialogTarget> {
    prop_oneof![
        (0u64..8).prop_map(DialogTarget::node),
        Just(DialogTarget::End),
        Just(DialogTarget::Return),
        key().prop_map(DialogTarget::custom),
    ]
}

fn hint() -> impl Strategy<Value = NavigationHint> {
    prop_oneof![
        Just(NavigationHint::Continue),
        Just(NavigationHint::End),
        key().prop_map(NavigationHint::GoTo),
        key().prop_map(NavigationHint::Custom),
    ]
}

fn option(node: u64) -> impl Strategy<Value = DialogueOption> {
    (
        0u64..32,
        text(),
        vec(tag(), 0..2),
        intentions(),
        vec(condition(), 0..2),
        vec(outcome(), 0..2),
        target(),
        proptest::option::of(hint()),
    )
        .prop_map(
            move |(id, text, tags, intentions, conditions, outcomes, target, hint)| {
                DialogueOption {
                    id: DialogueOptionId::new(id),
                    text,
                    tags,
                    intentions,
                    conditions,
                    outcomes,
                    target,
                    source_node_id: DialogueNodeId::new(node),
                    navigation_hint: hint,
                }
            },
        )
}

fn node() -> impl Strategy<Value = DialogueNode> {
    (0u64..8).prop_flat_map(|id| {
        (
            entity(),
            text(),
            vec(condition(), 0..2),
            vec(outcome(), 0..2),
            vec(option(id), 0..3),
        )
            .prop_map(
                move |(speaker, text, conditions, outcomes, options)| DialogueNode {
                    id: DialogueNodeId::new(id),
                    speaker: CharacterId::new(speaker),
                    text,
                    conditions,
                    outcomes_on_enter: outcomes,
                    options,
                },
            )
    })
}

fn graph() -> impl Strategy<Value = DialogueGraph> {
    (vec(node(), 0..4), proptest::option::of(0u64..8)).prop_map(|(nodes, entry)| DialogueGraph {
        nodes,
        entry_node: entry.map(DialogueNodeId::new),
    })
}

fn world_state() -> impl Strategy<Value = DialogWorldState> {
    (
        vec((fact_scope(), key(), fact_value()), 0..8),
        vec((key(), key(), any::<i16>()), 0..4),
        vec((entity(), entity(), key(), any::<i16>()), 0..4),
        vec((entity(), memory()), 0..4),
        vec(tag(), 0..3),
        vec(event_kind(), 0..4),
        any::<bool>(),
    )
        .prop_map(|(facts, aligns, rels, mems, tags, events, drain)| {
            let mut s = DialogWorldState::new();
            for (scope, id, value) in facts {
                s.set_fact(
                    scope,
                    Fact {
                        id: FactId::new(id),
                        value,
                    },
                );
            }
            for (a, p, v) in aligns {
                s.add_alignment(AlignmentAxisId::new(a), AlignmentPoleId::new(p), v.into());
            }
            for (a, b, axis, v) in rels {
                s.add_relationship(a, b, RelationshipAxisId::new(axis), v.into());
            }
            for (owner, m) in mems {
                s.add_memory(owner, m);
            }
            for t in tags {
                s.add_available_tag(t);
            }
            for k in events {
                s.emit_event(k);
            }
            if drain {
                s.drain_events();
            }
            s
        })
}

fn json_roundtrip<D: Serialize + for<'de> Deserialize<'de>>(dto: &D) -> D {
    let json = serde_json::to_string(dto).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

proptest! {
    #[test]
    fn condition_roundtrips(c in condition()) {
        let back = json_roundtrip(&ConditionDto::from(&c)).into_model();
        prop_assert_eq!(back, c);
    }

    #[test]
    fn outcome_roundtrips(o in outcome()) {
        let back = json_roundtrip(&OutcomeDto::from(&o)).into_model();
        prop_assert_eq!(back, o);
    }

    #[test]
    fn dialogue_graph_roundtrips(g in graph()) {
        let back = json_roundtrip(&DialogueGraphDto::from(&g)).into_model();
        prop_assert_eq!(back, g);
    }

    #[test]
    fn world_state_roundtrips(s in world_state()) {
        let back = json_roundtrip(&WorldStateDto::from(&s)).into_model();
        prop_assert_eq!(back.next_event_id(), s.next_event_id());
        prop_assert_eq!(back, s);
    }

    #[test]
    fn world_state_serializes_deterministically(s in world_state()) {
        let a = serde_json::to_string(&WorldStateDto::from(&s)).unwrap();
        let b = serde_json::to_string(&WorldStateDto::from(&s.clone())).unwrap();
        prop_assert_eq!(a, b);
    }
}

#[test]
fn enums_are_externally_tagged_snake_case() {
    let c = Condition::FactExists {
        scope: FactScope::World,
        fact_id: FactId::new("met_hero"),
    };
    let json = serde_json::to_string(&ConditionDto::from(&c)).unwrap();
    check!(json == r#"{"fact_exists":{"scope":"world","fact_id":"met_hero"}}"#);
}

#[test]
fn with_module_embeds_state_in_host_struct() {
    #[derive(Serialize, Deserialize)]
    struct GameState {
        room: String,
        #[serde(with = "dramaturge_serde::world_state")]
        dialog: DialogWorldState,
    }

    let mut dialog = DialogWorldState::new();
    dialog.set_fact(FactScope::World, Fact::bool("lamp_lit", true));
    let game = GameState {
        room: "shore".into(),
        dialog,
    };
    let json = serde_json::to_string(&game).unwrap();
    let back: GameState = serde_json::from_str(&json).unwrap();
    check!(back.room == "shore");
    check!(back.dialog == game.dialog);
}

#[test]
fn saved_state_survives_reordered_content() {
    let mut v1 = SymbolTable::new();
    let player = v1.intern_entity("player");
    let mara = v1.intern_entity("mara");
    let intro = v1.intern_node("mara.intro");
    let comfort = v1.intern_option("mara.intro.comfort");
    let memory = v1.intern_memory("mara.remembers_kindness");

    let mut state = DialogWorldState::new();
    state.add_relationship(player, mara, RelationshipAxisId::new("trust"), 10);
    state.set_fact(
        FactScope::Character(CharacterId::new(mara)),
        Fact::entity("confided_in", player),
    );
    state.add_memory(mara, Memory::new(memory).with_subject(player));
    state.emit_event(EventKind::DialogOptionChosen {
        node_id: intro,
        option_id: comfort,
        tags: vec![],
        intentions: DialogIntentions::new(),
    });

    let json = serde_json::to_string(&SavedWorldState::new(&state, &v1)).unwrap();

    // New content version: other keys were added first, so every number moved.
    let mut v2 = SymbolTable::new();
    v2.intern_entity("orren");
    v2.intern_node("orren.intro");
    v2.intern_option("orren.intro.hello");
    v2.intern_memory("orren.distrust");
    let player2 = v2.intern_entity("player");
    let mara2 = v2.intern_entity("mara");

    let saved: SavedWorldState = serde_json::from_str(&json).unwrap();
    let restored = saved.restore(&mut v2);

    check!(
        restored.get_relationship(player2, mara2, &RelationshipAxisId::new("trust")) == Some(10)
    );
    check!(
        restored.get_fact(
            &FactScope::Character(CharacterId::new(mara2)),
            &FactId::new("confided_in")
        ) == Some(&FactValue::Entity(player2))
    );
    let memories = restored.get_memories(mara2);
    check!(memories.len() == 1);
    check!(v2.memory_key(memories[0].id) == Some("mara.remembers_kindness"));
    check!(memories[0].subject == Some(player2));
    let EventKind::DialogOptionChosen {
        node_id, option_id, ..
    } = &restored.events()[0].kind
    else {
        panic!("expected DialogOptionChosen");
    };
    check!(v2.node_key(*node_id) == Some("mara.intro"));
    check!(v2.option_key(*option_id) == Some("mara.intro.comfort"));
}
