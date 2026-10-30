//! # Host State: Custom Conditions Against Your Own Game State
//!
//! A game has more state than dialogue: rooms, a clock, inventories. This
//! example embeds [`DialogWorldState`] in a host `GameState` with a
//! `current_room` field and answers `Condition::Custom { key: "in_room", .. }`
//! from it, while every built-in condition still goes to the embedded
//! dialog state.
//!
//! ## Run the Example
//!
//! ```bash
//! cargo run -p dramaturge --example host_state
//! ```

use dramaturge::*;

/// The host's game state. Dialogue state is one field among others.
struct GameState {
    current_room: String,
    dialog: DialogWorldState,
}

impl HasDialogWorldState for GameState {
    fn dialog(&self) -> &DialogWorldState {
        &self.dialog
    }

    fn dialog_mut(&mut self) -> &mut DialogWorldState {
        &mut self.dialog
    }
}

/// Answers `in_room` conditions and handles `move_to` outcomes.
#[derive(Clone, Default)]
struct Rooms;

impl CustomHandler<GameState> for Rooms {
    fn condition(&self, key: &str, args: &[FactValue], state: &GameState) -> bool {
        match (key, args) {
            ("in_room", [FactValue::Text(room)]) => *room == state.current_room,
            _ => false,
        }
    }

    fn outcome(&mut self, key: &str, args: &[FactValue], state: &mut GameState) {
        if let ("move_to", [FactValue::Text(room)]) = (key, args) {
            state.current_room = room.clone();
        }
    }
}

fn in_room(room: &str) -> Condition {
    Condition::Custom {
        key: "in_room".into(),
        args: vec![FactValue::Text(room.into())],
    }
}

fn build_node() -> DialogueNode {
    let keeper = CharacterId::new(EntityId::new(1));
    let mut node = DialogueNode::new(1, keeper, DialogText::inline("The keeper looks up."));

    // Only on the shore, and only once the lamp is lit: the custom leaf sits
    // inside `All`, and the default evaluator still routes it to `Rooms`.
    let mut light = DialogueOption::new(
        1,
        DialogText::inline("Signal the boat."),
        DialogTarget::End,
        node.id,
    );
    light.conditions.push(Condition::All(vec![
        in_room("shore"),
        Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new("lamp_lit"),
        },
    ]));

    let mut walk = DialogueOption::new(
        2,
        DialogText::inline("Walk down to the shore."),
        DialogTarget::End,
        node.id,
    );
    walk.conditions.push(Condition::negate(in_room("shore")));
    walk.outcomes.push(Outcome::Custom {
        key: "move_to".into(),
        args: vec![FactValue::Text("shore".into())],
    });

    node.options.push(light);
    node.options.push(walk);
    node
}

fn print_options(
    runtime: &DefaultRuntime<DefaultConditionEvaluator<Rooms>, DefaultOutcomeApplier<Rooms>>,
    node: &DialogueNode,
    state: &GameState,
) {
    println!("In room '{}':", state.current_room);
    for option in runtime.available_options(node, state) {
        if let DialogText::Inline(text) = &option.text {
            println!("  - {text}");
        }
    }
}

fn main() {
    let mut state = GameState {
        current_room: "cottage".into(),
        dialog: DialogWorldState::new(),
    };
    state
        .dialog
        .set_fact(FactScope::World, Fact::bool("lamp_lit", true));

    let mut runtime = DefaultRuntime::with_parts(
        DefaultConditionEvaluator::with_custom(Rooms),
        DefaultOutcomeApplier::with_custom(Rooms),
    );
    let node = build_node();

    print_options(&runtime, &node, &state);

    let walk = runtime
        .available_options(&node, &state)
        .find(|o| o.id == DialogueOptionId::new(2))
        .cloned()
        .expect("walk option is available in the cottage");
    runtime.choose_option(&walk, &mut state);

    print_options(&runtime, &node, &state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn custom_condition_reads_host_room() {
        let mut state = GameState {
            current_room: "shore".into(),
            dialog: DialogWorldState::new(),
        };
        let runtime = DefaultRuntime::with_parts(
            DefaultConditionEvaluator::with_custom(Rooms),
            DefaultOutcomeApplier::with_custom(Rooms),
        );
        let node = build_node();

        // Lamp not lit yet: the nested fact check fails.
        check!(runtime.available_options(&node, &state).count() == 0);

        state
            .dialog
            .set_fact(FactScope::World, Fact::bool("lamp_lit", true));
        let ids: Vec<_> = runtime
            .available_options(&node, &state)
            .map(|o| o.id)
            .collect();
        check!(ids == vec![DialogueOptionId::new(1)]);
    }

    #[test]
    fn custom_outcome_moves_host() {
        let mut state = GameState {
            current_room: "cottage".into(),
            dialog: DialogWorldState::new(),
        };
        let mut runtime = DefaultRuntime::with_parts(
            DefaultConditionEvaluator::with_custom(Rooms),
            DefaultOutcomeApplier::with_custom(Rooms),
        );
        let node = build_node();
        runtime.choose_option(&node.options[1], &mut state);
        check!(state.current_room == "shore");
        // The built-in DialogOptionChosen event lands in the embedded state.
        check!(state.dialog.events().len() == 1);
    }
}
