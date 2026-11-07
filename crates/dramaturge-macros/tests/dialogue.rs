//! `dialogue!`: positive cases, and the compile errors through trybuild.

use assert2::{assert, check};
use dramaturge_authoring::{GraphValidator, ValidationSeverity};
use dramaturge_macros::dialogue;
use dramaturge_model::{Condition, DialogTarget, DialogText, Outcome, SymbolTable};

#[test]
fn dialogue_builds_the_graph() {
    let loaded = dialogue! {
        with mara start [again, intro] {
            intro: "Hello, traveller." {
                set met_mara = true;
                add visits += 1;
                "Who are you?" => names { relation trust += 1; remember asked_name; }
                "Goodbye." => end { id intro.bye; }
            }
            again: "You again." {
                when met_mara and relation(trust) >= 1;
                then custom ring_bell(2);
                "Back." => back;
            }
            names {
                mara: "They call me Mara.";
                player: "Mara. I will remember.";
                -> end;
            }
        }
    };
    check!(loaded.warnings.is_empty());
    let s = &loaded.symbols;
    let g = &loaded.graph;
    let entries: Vec<_> = g
        .entry_nodes
        .iter()
        .map(|&id| s.node_key(id).unwrap())
        .collect();
    check!(entries == ["again", "intro"]);

    let intro = g.node(s.node("intro").unwrap()).unwrap();
    check!(intro.text == DialogText::inline("Hello, traveller."));
    check!(intro.outcomes_on_enter.len() == 2);
    check!(intro.options[0].target == DialogTarget::Node(s.node("names").unwrap()));
    assert!(let [Outcome::AddRelationship { delta: 1, .. }, Outcome::AddMemory { .. }] = intro.options[0].outcomes.as_slice());
    check!(s.option_key(intro.options[1].id) == Some("intro.bye"));

    let again = g.node(s.node("again").unwrap()).unwrap();
    assert!(let [Condition::All(_)] = again.conditions.as_slice());
    assert!(let [Outcome::Custom { .. }] = again.outcomes_on_enter.as_slice());
    check!(again.options[0].target == DialogTarget::Return);

    let names = g.node(s.node("names").unwrap()).unwrap();
    check!(names.lines[0].speaker.0 == s.entity(SymbolTable::PLAYER_KEY).unwrap());
    check!(names.next == Some(DialogTarget::End));

    let issues = GraphValidator::new().validate_with_symbols(g, s);
    check!(
        !issues
            .iter()
            .any(|i| i.severity == ValidationSeverity::Error),
        "{issues:?}"
    );
}

#[test]
fn dialogue_and_yaml_make_the_same_graph() {
    let from_macro = dialogue! {
        dialogue talk with mara start intro {
            intro: "Hello." {
                when visits >= 1;
                "Bye." => end { set left = true; }
            }
        }
    };
    let yaml: dramaturge_syntax::Dialogue = dramaturge_syntax::Dialogue::new("mara")
        .with_id("talk")
        .with_start("intro")
        .with_node(
            dramaturge_syntax::Node::new("intro", "Hello.".into())
                .with_when("visits >= 1".to_string())
                .with_option(
                    dramaturge_syntax::Choice::new("Bye.".into(), dramaturge_syntax::Target::End)
                        .with_outcomes(
                            dramaturge_syntax::Outcomes::default()
                                .with_set("left", dramaturge_syntax::Value::Bool(true)),
                        ),
                ),
        );
    let from_ast = dramaturge_syntax::lower_plain(&yaml).unwrap();
    check!(from_macro == from_ast);
}

#[test]
fn compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
