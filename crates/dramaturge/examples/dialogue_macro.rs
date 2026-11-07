//! # A dialogue written with `dialogue!`
//!
//! `dialogue!` reads a dialogue in Rust tokens, checks it at compile time
//! (targets, keys, conditions) and builds its graph. The YAML loader makes
//! the same graph from the same dialogue in YAML.
//!
//! ```bash
//! cargo run -p dramaturge --example dialogue_macro --features macros
//! ```

use dramaturge::{
    AdvanceStop, DefaultRuntime, DialogCursor, DialogText, DialogWorldState, DialogueRuntime,
    dialogue,
};

fn main() {
    let loaded = dialogue! {
        with mara start [again, intro] {
            intro: "If you came about the shipment, I told the Wardens everything." {
                set met_mara = true;
                "You treated someone with that medicine." => soft { relation trust += 10; }
                "Tell me where the crates went." => forced { relation trust -= 10; }
                "Never mind." => end;
            }
            again: "You again." {
                when met_mara;
                "Goodbye." => end;
            }
            soft {
                mara: "Yes. The refugee camp beyond the east wall.";
                player: "I will speak carefully.";
                mara: "Thank you.";
                -> end;
            }
            forced: "The east wall. Take what you want." {
                remember was_threatened;
                -> end;
            }
        }
    };

    let mut symbols = loaded.symbols;
    let player = symbols.player();
    let graph = loaded.graph;
    let issues =
        dramaturge_authoring::GraphValidator::new().validate_with_symbols(&graph, &symbols);
    assert!(issues.is_empty(), "{issues:?}");

    let mut runtime = DefaultRuntime::new();
    let mut state = DialogWorldState::new();
    let line = |text: &DialogText| match text {
        DialogText::Inline(text) => text.clone(),
        _ => "...".to_string(),
    };

    // Two talks, each takes the first option. The second talk starts at
    // `again`, because `intro` set `met_mara`.
    for talk in 1..=2 {
        println!("--- talk {talk} ---");
        let start = runtime.select_entry(&graph, &state).expect("an entry node");
        let mut cursor = DialogCursor::new(start);
        loop {
            let advance = runtime.advance(&graph, &mut cursor, &mut state);
            for id in &advance.entered {
                let node = graph.node(*id).expect("entered nodes exist");
                for (speaker, text) in node.all_lines() {
                    let who = if speaker.0 == player {
                        "You"
                    } else {
                        symbols.entity_key(speaker.0).unwrap_or("?")
                    };
                    println!("{who:>5}: {}", line(text));
                }
            }
            if advance.stop != AdvanceStop::Choice {
                break;
            }
            let node = graph.node(cursor.current).expect("the cursor is at a node");
            let option = runtime
                .available_options(node, &state)
                .next()
                .cloned()
                .expect("an option is available");
            println!("    > {}", line(&option.text));
            let target = runtime.choose_option(&option, &mut state);
            if cursor.follow(&target).is_none() {
                break;
            }
        }
    }
}
