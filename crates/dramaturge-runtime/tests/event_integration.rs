use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, DialogTarget, DialogText, DialogueNodeId, DialogueOption,
    EntityId, EventKind, Fact, FactScope, Outcome, RelationshipAxisId,
};
use dramaturge_runtime::{DialogueRuntime, context::DialogWorldState, runtime::DefaultRuntime};

fn make_option(id: u64, text: &str, target: DialogTarget, node_id: u64) -> DialogueOption {
    let mut opt = DialogueOption::new(
        id,
        DialogText::inline(text),
        target,
        DialogueNodeId::new(node_id),
    );
    opt.source_node_id = DialogueNodeId::new(node_id);
    opt
}

fn make_option_with_outcomes(
    id: u64,
    text: &str,
    target: DialogTarget,
    node_id: u64,
    outcomes: Vec<Outcome>,
) -> DialogueOption {
    let mut opt = make_option(id, text, target, node_id);
    opt.outcomes = outcomes;
    opt
}

#[test]
fn choosing_option_produces_dialog_option_chosen_event() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let option = make_option(1, "I agree", DialogTarget::node(2), 1);

    rt.choose_option(&option, &mut state);

    assert_eq!(state.events.len(), 1);
    match &state.events[0].kind {
        EventKind::DialogOptionChosen {
            node_id, option_id, ..
        } => {
            assert_eq!(node_id.0, 1);
            assert_eq!(option_id.0, 1);
        }
        other => panic!("expected DialogOptionChosen, got {:?}", other),
    }
}

#[test]
fn setting_fact_produces_fact_learned_event() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let option = make_option_with_outcomes(
        1,
        "Learn a fact",
        DialogTarget::end(),
        1,
        vec![Outcome::SetFact {
            scope: FactScope::World,
            fact: Fact::bool("secret_revealed", true),
        }],
    );

    rt.choose_option(&option, &mut state);

    assert_eq!(state.events.len(), 2);
    match &state.events[1].kind {
        EventKind::FactLearned { scope, fact_id } => {
            assert!(matches!(scope, FactScope::World));
            assert_eq!(fact_id.0.as_ref(), "secret_revealed");
        }
        other => panic!("expected FactLearned, got {:?}", other),
    }
}

#[test]
fn changing_alignment_produces_alignment_changed_event() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let option = make_option_with_outcomes(
        1,
        "Be nice",
        DialogTarget::end(),
        1,
        vec![Outcome::AddAlignment {
            axis: AlignmentAxisId::new("morality"),
            pole: AlignmentPoleId::new("paragon"),
            delta: 25,
        }],
    );

    rt.choose_option(&option, &mut state);

    assert_eq!(state.events.len(), 2);
    match &state.events[1].kind {
        EventKind::AlignmentChanged { axis, pole, delta } => {
            assert_eq!(axis.0.as_ref(), "morality");
            assert_eq!(pole.0.as_ref(), "paragon");
            assert_eq!(*delta, 25);
        }
        other => panic!("expected AlignmentChanged, got {:?}", other),
    }
}

#[test]
fn changing_relationship_produces_relationship_changed_event() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let source = EntityId::new(1);
    let target = EntityId::new(2);

    let option = make_option_with_outcomes(
        1,
        "Make friends",
        DialogTarget::end(),
        1,
        vec![Outcome::AddRelationship {
            source,
            target,
            axis: RelationshipAxisId::new("trust"),
            delta: 15,
        }],
    );

    rt.choose_option(&option, &mut state);

    assert_eq!(state.events.len(), 2);
    match &state.events[1].kind {
        EventKind::RelationshipChanged { a, b, axis, delta } => {
            assert_eq!(*a, source);
            assert_eq!(*b, target);
            assert_eq!(axis.0.as_ref(), "trust");
            assert_eq!(*delta, 15);
        }
        other => panic!("expected RelationshipChanged, got {:?}", other),
    }
}

#[test]
fn event_ids_are_monotonically_increasing() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let _node1 = DialogueNodeId::new(1);
    let option1 = make_option_with_outcomes(
        1,
        "First choice",
        DialogTarget::node(2),
        1,
        vec![Outcome::SetFact {
            scope: FactScope::World,
            fact: Fact::bool("step1", true),
        }],
    );

    rt.choose_option(&option1, &mut state);

    let option2 = make_option_with_outcomes(
        2,
        "Second choice",
        DialogTarget::end(),
        2,
        vec![
            Outcome::AddAlignment {
                axis: AlignmentAxisId::new("morality"),
                pole: AlignmentPoleId::new("renegade"),
                delta: -10,
            },
            Outcome::AddRelationship {
                source: EntityId::new(1),
                target: EntityId::new(2),
                axis: RelationshipAxisId::new("trust"),
                delta: 5,
            },
        ],
    );

    rt.choose_option(&option2, &mut state);

    assert_eq!(state.events.len(), 5);
    let ids: Vec<u64> = state.events.iter().map(|e| e.id.0).collect();
    for i in 1..ids.len() {
        assert!(
            ids[i] > ids[i - 1],
            "Event IDs should be monotonically increasing"
        );
    }
}

#[test]
fn emit_event_outcome_still_works() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let custom_kind = EventKind::Custom {
        kind: "player_defeated_boss".into(),
    };

    let option = make_option_with_outcomes(
        1,
        "Boss defeated",
        DialogTarget::end(),
        1,
        vec![
            Outcome::SetFact {
                scope: FactScope::World,
                fact: Fact::bool("boss_dead", true),
            },
            Outcome::EmitEvent {
                kind: custom_kind.clone(),
            },
        ],
    );

    rt.choose_option(&option, &mut state);

    assert_eq!(state.events.len(), 3);
    assert!(matches!(
        state.events[0].kind,
        EventKind::DialogOptionChosen { .. }
    ));
    assert!(matches!(
        state.events[1].kind,
        EventKind::FactLearned { .. }
    ));
    assert_eq!(state.events[2].kind, custom_kind);
}

#[test]
fn event_queries_return_correct_results() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let _node1 = DialogueNodeId::new(1);
    let option1 = make_option_with_outcomes(
        1,
        "First",
        DialogTarget::node(2),
        1,
        vec![Outcome::SetFact {
            scope: FactScope::World,
            fact: Fact::bool("met_npc", true),
        }],
    );
    rt.choose_option(&option1, &mut state);

    let option2 = make_option_with_outcomes(
        2,
        "Second",
        DialogTarget::end(),
        2,
        vec![Outcome::AddAlignment {
            axis: AlignmentAxisId::new("morality"),
            pole: AlignmentPoleId::new("paragon"),
            delta: 10,
        }],
    );
    rt.choose_option(&option2, &mut state);

    assert_eq!(state.events.len(), 4);

    let chosen = state.events_of_kind("DialogOptionChosen");
    assert_eq!(chosen.len(), 2);

    let learned = state.events_of_kind("FactLearned");
    assert_eq!(learned.len(), 1);

    let alignment = state.events_of_kind("AlignmentChanged");
    assert_eq!(alignment.len(), 1);

    let recent = state.recent_events(1);
    assert_eq!(recent.len(), 1);
    assert!(matches!(recent[0].kind, EventKind::AlignmentChanged { .. }));

    let found = state.find_events(|e| {
        matches!(&e.kind, EventKind::FactLearned { fact_id, .. } if fact_id.0.as_ref() == "met_npc")
    });
    assert_eq!(found.len(), 1);
}

#[test]
fn start_quest_produces_custom_event() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let option = make_option_with_outcomes(
        1,
        "Start quest",
        DialogTarget::end(),
        1,
        vec![Outcome::StartQuest {
            quest_id: "main_quest_1".to_string(),
        }],
    );

    rt.choose_option(&option, &mut state);

    assert_eq!(state.events.len(), 2);
    match &state.events[1].kind {
        EventKind::Custom { kind } => {
            assert_eq!(kind.as_ref(), "quest_started_main_quest_1");
        }
        other => panic!("expected Custom event, got {:?}", other),
    }
    assert!(state.fact_exists(
        &FactScope::World,
        &dramaturge_model::FactId::new("quest_started_main_quest_1")
    ));
}

#[test]
fn advance_quest_produces_custom_event() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let option = make_option_with_outcomes(
        1,
        "Advance quest",
        DialogTarget::end(),
        1,
        vec![Outcome::AdvanceQuest {
            quest_id: "main_quest_1".to_string(),
            stage: "complete".to_string(),
        }],
    );

    rt.choose_option(&option, &mut state);

    assert_eq!(state.events.len(), 2);
    match &state.events[1].kind {
        EventKind::Custom { kind } => {
            assert_eq!(kind.as_ref(), "quest_advanced_main_quest_1_complete");
        }
        other => panic!("expected Custom event, got {:?}", other),
    }
}
