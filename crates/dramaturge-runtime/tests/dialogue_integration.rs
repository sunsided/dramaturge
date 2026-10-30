use assert2::check;
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, CharacterId, Condition, DialogTarget, DialogText,
    DialogueNode, DialogueNodeId, DialogueOption, EntityId, Fact, FactId, FactScope, FactValue,
    Memory, MemoryId, MemoryQuery, MemoryTag, Outcome, RelationshipAxisId,
};
use dramaturge_runtime::{DialogueRuntime, context::DialogWorldState, runtime::DefaultRuntime};

fn npc() -> CharacterId {
    CharacterId::new(EntityId::new(1))
}

fn make_node(id: u64, options: Vec<DialogueOption>) -> DialogueNode {
    DialogueNode {
        id: DialogueNodeId::new(id),
        speaker: npc(),
        text: DialogText::inline("..."),
        conditions: vec![],
        outcomes_on_enter: vec![],
        options,
    }
}

fn make_opt(id: u64, target: DialogTarget, source_node: u64) -> DialogueOption {
    DialogueOption::new(
        id,
        DialogText::inline("choice"),
        target,
        DialogueNodeId::new(source_node),
    )
}

fn make_opt_with_conditions(
    id: u64,
    target: DialogTarget,
    source_node: u64,
    conditions: Vec<Condition>,
) -> DialogueOption {
    let mut o = make_opt(id, target, source_node);
    o.conditions = conditions;
    o
}

fn make_opt_with_outcomes(
    id: u64,
    target: DialogTarget,
    source_node: u64,
    outcomes: Vec<Outcome>,
) -> DialogueOption {
    let mut o = make_opt(id, target, source_node);
    o.outcomes = outcomes;
    o
}

// ── Full dialog flow ──────────────────────────────────────────────────────────

#[test]
fn full_dialog_flow_three_nodes_to_end() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let node1 = make_node(1, vec![make_opt(1, DialogTarget::node(2), 1)]);
    let node2 = make_node(2, vec![make_opt(2, DialogTarget::node(3), 2)]);
    let node3 = make_node(3, vec![make_opt(3, DialogTarget::end(), 3)]);

    // Enter node 1 — one option available
    let opts1: Vec<DialogueOption> = rt.available_options(&node1, &state).cloned().collect();
    check!(opts1.len() == 1);

    let target = rt.choose_option(&opts1[0], &mut state);
    check!(target == DialogTarget::node(2));

    // Enter node 2 — one option available
    let opts2: Vec<DialogueOption> = rt.available_options(&node2, &state).cloned().collect();
    check!(opts2.len() == 1);

    let target = rt.choose_option(&opts2[0], &mut state);
    check!(target == DialogTarget::node(3));

    // Enter node 3 — ends conversation
    let opts3: Vec<DialogueOption> = rt.available_options(&node3, &state).cloned().collect();
    check!(opts3.len() == 1);

    let target = rt.choose_option(&opts3[0], &mut state);
    check!(target == DialogTarget::end());
}

// ── Condition evaluation: fact checks ────────────────────────────────────────

#[test]
fn fact_exists_gates_option_until_fact_set() {
    let rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::node(3),
        1,
        vec![Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new("has_key"),
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::node(2), 1), gated_opt]);

    // Before fact: only unconditional option visible
    let visible: Vec<_> = rt.available_options(&node, &state).collect();
    check!(visible.len() == 1);
    check!(visible[0].id.0 == 1);

    // After fact: both options visible
    state.set_fact(FactScope::World, Fact::bool("has_key", true));
    let visible: Vec<_> = rt.available_options(&node, &state).collect();
    check!(visible.len() == 2);
}

#[test]
fn fact_equals_gates_option_on_value_match() {
    let rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::FactEquals {
            scope: FactScope::World,
            fact_id: FactId::new("chapter"),
            value: FactValue::Int(3),
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    state.set_fact(FactScope::World, Fact::int("chapter", 2));
    check!(rt.available_options(&node, &state).count() == 1);

    state.set_fact(FactScope::World, Fact::int("chapter", 3));
    check!(rt.available_options(&node, &state).count() == 2);
}

#[test]
fn fact_at_least_gates_option_on_int_threshold() {
    let rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::FactAtLeast {
            scope: FactScope::World,
            fact_id: FactId::new("gold"),
            value: 100,
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    state.set_fact(FactScope::World, Fact::int("gold", 50));
    check!(rt.available_options(&node, &state).count() == 1);

    state.set_fact(FactScope::World, Fact::int("gold", 100));
    check!(rt.available_options(&node, &state).count() == 2);

    state.set_fact(FactScope::World, Fact::int("gold", 200));
    check!(rt.available_options(&node, &state).count() == 2);
}

#[test]
fn fact_at_most_gates_option_on_int_ceiling() {
    let rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::FactAtMost {
            scope: FactScope::World,
            fact_id: FactId::new("heat"),
            value: 10,
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    state.set_fact(FactScope::World, Fact::int("heat", 5));
    check!(rt.available_options(&node, &state).count() == 2);

    state.set_fact(FactScope::World, Fact::int("heat", 10));
    check!(rt.available_options(&node, &state).count() == 2);

    state.set_fact(FactScope::World, Fact::int("heat", 11));
    check!(rt.available_options(&node, &state).count() == 1);
}

// ── Condition evaluation: alignment thresholds ───────────────────────────────

#[test]
fn alignment_at_least_unlocks_option_after_alignment_outcome() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let morality = AlignmentAxisId::new("morality");
    let paragon = AlignmentPoleId::new("paragon");

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::AlignmentAtLeast {
            axis: morality.clone(),
            pole: paragon.clone(),
            value: 50,
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    // Below threshold
    check!(rt.available_options(&node, &state).count() == 1);

    // Reach threshold via outcome
    let ramp_opt = make_opt_with_outcomes(
        10,
        DialogTarget::end(),
        99,
        vec![Outcome::AddAlignment {
            axis: morality.clone(),
            pole: paragon.clone(),
            delta: 50,
        }],
    );
    rt.choose_option(&ramp_opt, &mut state);

    // At threshold
    check!(rt.available_options(&node, &state).count() == 2);
}

#[test]
fn alignment_at_most_hides_option_when_alignment_exceeds_ceiling() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let morality = AlignmentAxisId::new("morality");
    let renegade = AlignmentPoleId::new("renegade");

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::AlignmentAtMost {
            axis: morality.clone(),
            pole: renegade.clone(),
            value: 30,
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    // No alignment set — AtMost returns false (unwrap_or(false))
    check!(rt.available_options(&node, &state).count() == 1);

    // Set alignment within ceiling — visible
    state.add_alignment(morality.clone(), renegade.clone(), 20);
    check!(rt.available_options(&node, &state).count() == 2);

    // Push above ceiling via outcome — hidden again
    let push_opt = make_opt_with_outcomes(
        10,
        DialogTarget::end(),
        99,
        vec![Outcome::AddAlignment {
            axis: morality.clone(),
            pole: renegade.clone(),
            delta: 20,
        }],
    );
    rt.choose_option(&push_opt, &mut state);
    check!(rt.available_options(&node, &state).count() == 1);
}

// ── Condition evaluation: relationship checks ─────────────────────────────────

#[test]
fn relationship_at_least_unlocks_option_after_relationship_outcome() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let player = EntityId::new(1);
    let npc_entity = EntityId::new(2);
    let trust = RelationshipAxisId::new("trust");

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::RelationshipAtLeast {
            source: player,
            target: npc_entity,
            axis: trust.clone(),
            value: 30,
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    check!(rt.available_options(&node, &state).count() == 1);

    let bond_opt = make_opt_with_outcomes(
        10,
        DialogTarget::end(),
        99,
        vec![Outcome::AddRelationship {
            source: player,
            target: npc_entity,
            axis: trust.clone(),
            delta: 30,
        }],
    );
    rt.choose_option(&bond_opt, &mut state);

    check!(rt.available_options(&node, &state).count() == 2);
}

#[test]
fn relationship_at_most_hides_option_when_hostility_exceeds_ceiling() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let player = EntityId::new(1);
    let npc_entity = EntityId::new(2);
    let hostility = RelationshipAxisId::new("hostility");

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::RelationshipAtMost {
            source: player,
            target: npc_entity,
            axis: hostility.clone(),
            value: 20,
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    // Relationship within ceiling — visible
    state.add_relationship(player, npc_entity, hostility.clone(), 10);
    check!(rt.available_options(&node, &state).count() == 2);

    // Exceed ceiling — hidden
    let anger_opt = make_opt_with_outcomes(
        10,
        DialogTarget::end(),
        99,
        vec![Outcome::AddRelationship {
            source: player,
            target: npc_entity,
            axis: hostility.clone(),
            delta: 20,
        }],
    );
    rt.choose_option(&anger_opt, &mut state);

    check!(rt.available_options(&node, &state).count() == 1);
}

// ── Outcome application: memory creation and memory-based conditions ──────────

#[test]
fn add_memory_outcome_enables_has_memory_condition() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let npc_entity = EntityId::new(2);

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::HasMemory {
            owner: npc_entity,
            query: MemoryQuery::by_tag(MemoryTag::HelpedMe),
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    // No memory yet — gated option hidden
    check!(rt.available_options(&node, &state).count() == 1);

    // Player helps the NPC — memory stored via outcome
    let help_opt = make_opt_with_outcomes(
        10,
        DialogTarget::end(),
        99,
        vec![Outcome::AddMemory {
            owner: npc_entity,
            memory: Memory::new(MemoryId::new(1)).with_tag(MemoryTag::HelpedMe),
        }],
    );
    rt.choose_option(&help_opt, &mut state);

    // Memory now exists — gated option visible
    check!(rt.available_options(&node, &state).count() == 2);
}

#[test]
fn remove_memory_outcome_hides_previously_visible_option() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let npc_entity = EntityId::new(2);
    let memory_id = MemoryId::new(1);

    state.add_memory(
        npc_entity,
        Memory::new(memory_id).with_tag(MemoryTag::HelpedMe),
    );

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::HasMemory {
            owner: npc_entity,
            query: MemoryQuery::by_tag(MemoryTag::HelpedMe),
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    check!(rt.available_options(&node, &state).count() == 2);

    // Remove the memory via outcome
    let forget_opt = make_opt_with_outcomes(
        10,
        DialogTarget::end(),
        99,
        vec![Outcome::RemoveMemory {
            owner: npc_entity,
            memory_id,
        }],
    );
    rt.choose_option(&forget_opt, &mut state);

    check!(rt.available_options(&node, &state).count() == 1);
}

// ── Outcome application: remove fact locks option ────────────────────────────

#[test]
fn remove_fact_outcome_locks_previously_visible_option() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    state.set_fact(FactScope::World, Fact::bool("has_key", true));

    let gated_opt = make_opt_with_conditions(
        2,
        DialogTarget::end(),
        1,
        vec![Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new("has_key"),
        }],
    );
    let node = make_node(1, vec![make_opt(1, DialogTarget::end(), 1), gated_opt]);

    // Fact present — gated option visible
    check!(rt.available_options(&node, &state).count() == 2);

    let use_key_opt = make_opt_with_outcomes(
        10,
        DialogTarget::end(),
        99,
        vec![Outcome::RemoveFact {
            scope: FactScope::World,
            fact_id: FactId::new("has_key"),
        }],
    );
    rt.choose_option(&use_key_opt, &mut state);

    // Key consumed — gated option hidden
    check!(rt.available_options(&node, &state).count() == 1);
}

// ── Outcome application: multiple outcomes in one option ─────────────────────

#[test]
fn choosing_option_applies_all_outcomes_in_sequence() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    let player = EntityId::new(1);
    let npc_entity = EntityId::new(2);

    let combo_opt = make_opt_with_outcomes(
        1,
        DialogTarget::end(),
        1,
        vec![
            Outcome::SetFact {
                scope: FactScope::World,
                fact: Fact::bool("hero_known", true),
            },
            Outcome::AddAlignment {
                axis: AlignmentAxisId::new("morality"),
                pole: AlignmentPoleId::new("paragon"),
                delta: 20,
            },
            Outcome::AddRelationship {
                source: player,
                target: npc_entity,
                axis: RelationshipAxisId::new("trust"),
                delta: 15,
            },
            Outcome::AddMemory {
                owner: npc_entity,
                memory: Memory::new(MemoryId::new(1)).with_tag(MemoryTag::HelpedMe),
            },
        ],
    );

    rt.choose_option(&combo_opt, &mut state);

    check!(state.fact_exists(&FactScope::World, &FactId::new("hero_known")));
    check!(
        state.get_alignment(
            &AlignmentAxisId::new("morality"),
            &AlignmentPoleId::new("paragon")
        ) == Some(20)
    );
    check!(
        state.get_relationship(player, npc_entity, &RelationshipAxisId::new("trust")) == Some(15)
    );
    check!(state.get_memories(npc_entity).len() == 1);
}

// ── Full branching flow ───────────────────────────────────────────────────────

#[test]
fn branching_paths_produce_different_world_states() {
    let morality = AlignmentAxisId::new("morality");
    let paragon = AlignmentPoleId::new("paragon");
    let renegade = AlignmentPoleId::new("renegade");

    // "Be kind" path
    let mut rt_good = DefaultRuntime::new();
    let mut state_good = DialogWorldState::new();

    let kind_opt = make_opt_with_outcomes(
        1,
        DialogTarget::end(),
        1,
        vec![
            Outcome::SetFact {
                scope: FactScope::World,
                fact: Fact::bool("chose_mercy", true),
            },
            Outcome::AddAlignment {
                axis: morality.clone(),
                pole: paragon.clone(),
                delta: 50,
            },
        ],
    );
    rt_good.choose_option(&kind_opt, &mut state_good);

    // "Be cruel" path
    let mut rt_evil = DefaultRuntime::new();
    let mut state_evil = DialogWorldState::new();

    let cruel_opt = make_opt_with_outcomes(
        2,
        DialogTarget::end(),
        1,
        vec![
            Outcome::SetFact {
                scope: FactScope::World,
                fact: Fact::bool("chose_cruelty", true),
            },
            Outcome::AddAlignment {
                axis: morality.clone(),
                pole: renegade.clone(),
                delta: 50,
            },
        ],
    );
    rt_evil.choose_option(&cruel_opt, &mut state_evil);

    // Good path unlocks paragon-gated option
    let paragon_opt = make_opt_with_conditions(
        10,
        DialogTarget::end(),
        2,
        vec![Condition::AlignmentAtLeast {
            axis: morality.clone(),
            pole: paragon.clone(),
            value: 50,
        }],
    );
    let node2 = make_node(2, vec![make_opt(9, DialogTarget::end(), 2), paragon_opt]);

    check!(rt_good.available_options(&node2, &state_good).count() == 2);
    check!(rt_evil.available_options(&node2, &state_evil).count() == 1);
}

#[test]
fn full_dialog_with_condition_and_outcomes_across_nodes() {
    let mut rt = DefaultRuntime::new();
    let mut state = DialogWorldState::new();

    // Node 1: choose to help → sets fact + adds alignment
    let node1 = make_node(
        1,
        vec![make_opt_with_outcomes(
            1,
            DialogTarget::node(2),
            1,
            vec![
                Outcome::SetFact {
                    scope: FactScope::World,
                    fact: Fact::bool("helped_merchant", true),
                },
                Outcome::AddAlignment {
                    axis: AlignmentAxisId::new("morality"),
                    pole: AlignmentPoleId::new("paragon"),
                    delta: 25,
                },
            ],
        )],
    );

    // Node 2: one option always visible, one gated on helped_merchant fact
    let node2 = make_node(
        2,
        vec![
            make_opt(2, DialogTarget::end(), 2),
            make_opt_with_conditions(
                3,
                DialogTarget::end(),
                2,
                vec![Condition::FactExists {
                    scope: FactScope::World,
                    fact_id: FactId::new("helped_merchant"),
                }],
            ),
        ],
    );

    // Before choosing: node 2 has 1 visible option
    check!(rt.available_options(&node2, &state).count() == 1);

    // Choose node 1's option
    let opts1: Vec<DialogueOption> = rt.available_options(&node1, &state).cloned().collect();
    check!(opts1.len() == 1);
    let target = rt.choose_option(&opts1[0], &mut state);
    check!(target == DialogTarget::node(2));

    // After choosing: fact set, node 2 now shows both options
    check!(rt.available_options(&node2, &state).count() == 2);
    check!(state.fact_exists(&FactScope::World, &FactId::new("helped_merchant")));
    check!(
        state.get_alignment(
            &AlignmentAxisId::new("morality"),
            &AlignmentPoleId::new("paragon")
        ) == Some(25)
    );
}
