//! # Exchange: Lines That Continue Without a Choice
//!
//! A scripted exchange: two characters talk, and the player only watches
//! until a choice comes. Each node holds more than one line, each with its
//! own speaker, and continues to its `next` node by itself.
//! [`DialogueRuntime::advance`] enters the nodes one after the other,
//! applies their outcomes on enter and stops at the first choice.
//!
//! ## Run the Example
//!
//! ```bash
//! cargo run -p dramaturge --example exchange
//! ```

use dramaturge::*;

const MARA: CharacterId = CharacterId(EntityId(1));
const PLAYER: CharacterId = CharacterId(EntityId(2));

fn name(speaker: CharacterId) -> &'static str {
    if speaker == MARA { "Mara" } else { "You" }
}

fn build_graph() -> DialogueGraph {
    // Node 1: three lines, then on to node 2.
    let meet = DialogueNode::new(1, MARA, DialogText::inline("Sit, if you like."))
        .with_line(PLAYER, DialogText::inline("Who are you?"))
        .with_line(MARA, DialogText::inline("They call me Mara."))
        .with_next(DialogTarget::node(2));

    // Node 2: records the meeting on enter, then on to node 3.
    let mut names = DialogueNode::new(2, MARA, DialogText::inline("And you are the courier."))
        .with_next(DialogTarget::node(3));
    names.outcomes_on_enter.push(Outcome::SetFact {
        scope: FactScope::World,
        fact: Fact::bool("met_mara", true),
    });

    // Node 3: a choice. One option asks for more, then the talk returns here.
    let mut ask = DialogueNode::new(3, MARA, DialogText::inline("What do you want?"));
    ask.options.push(DialogueOption::new(
        1,
        DialogText::inline("Tell me about the seal."),
        DialogTarget::node(4),
        ask.id,
    ));
    ask.options.push(DialogueOption::new(
        2,
        DialogText::inline("Nothing. Goodbye."),
        DialogTarget::End,
        ask.id,
    ));

    // Node 4: an aside that returns to the node before it.
    let seal = DialogueNode::new(4, MARA, DialogText::inline("It broke last night."))
        .with_line(PLAYER, DialogText::inline("By itself?"))
        .with_line(MARA, DialogText::inline("Nothing breaks by itself."))
        .with_next(DialogTarget::Return);

    let mut graph = DialogueGraph::new().with_entry_node(meet.id);
    graph.nodes = vec![meet, names, ask, seal];
    graph
}

/// Prints every line of the entered nodes, in order.
fn play(graph: &DialogueGraph, advance: &Advance) {
    for id in &advance.entered {
        let node = graph.node(*id).expect("entered nodes exist");
        for (speaker, text) in node.all_lines() {
            if let DialogText::Inline(line) = text {
                println!("{:>5}: {line}", name(speaker));
            }
        }
    }
}

fn main() {
    let graph = build_graph();
    let issues = dramaturge_authoring::GraphValidator::new().validate(&graph);
    assert!(issues.is_empty(), "{issues:?}");

    let mut runtime = DefaultRuntime::new();
    let mut state = DialogWorldState::new();
    let start = runtime.select_entry(&graph, &state).expect("an entry node");
    let mut cursor = DialogCursor::new(start);

    // The exchange runs to the first choice.
    let advance = runtime.advance(&graph, &mut cursor, &mut state);
    play(&graph, &advance);
    assert_eq!(advance.stop, AdvanceStop::Choice);
    assert!(state.fact_exists(&FactScope::World, &FactId::new("met_mara")));

    // Choose the first option, then the second; the aside returns to node 3.
    for choice in [0, 1] {
        let node = graph.node(cursor.current).expect("the cursor is at a node");
        let option = runtime
            .available_options(node, &state)
            .nth(choice)
            .cloned()
            .expect("the option is available");
        println!(
            "  > {}",
            match &option.text {
                DialogText::Inline(text) => text.as_str(),
                _ => "...",
            }
        );
        let target = runtime.choose_option(&option, &mut state);
        if cursor.follow(&target).is_none() {
            println!("(The talk ends.)");
            break;
        }
        let advance = runtime.advance(&graph, &mut cursor, &mut state);
        play(&graph, &advance);
        assert_eq!(advance.stop, AdvanceStop::Choice);
        assert_eq!(cursor.current, DialogueNodeId::new(3));
    }
}
