//! The token form.
#![cfg(feature = "syn")]

use assert2::{assert, check};
use dramaturge_syntax::{Effect, Place, Spanned, Target, Text, Value, lower_plain};

const MARA: &str = r#"
dialogue with mara start [again, intro] {
    intro: "Hello, traveller." {
        set met_mara = true, player.gold = -5;
        add visits += 1;
        "Who are you?" => names { relation trust += 1; remember asked_name; }
        "Back." when relation(trust) >= 1 => back;
        "Goodbye." => end { id bye; then custom ring_bell(3, true); }
    }
    again: "You again." {
        when "met_mara and visits >= 2";
        then relation player -> orren trust -= 2;
        "Hello." => intro;
    }
    names {
        mara: "They call me Mara.";
        player: "Mara. I will remember.";
        -> end;
    }
}
"#;

fn parse(src: &str) -> syn::Result<Spanned> {
    syn::parse_str(src)
}

#[test]
fn the_token_form_reads_the_same_ast() {
    let s = parse(MARA).expect("parses");
    s.check(true).expect("no issues");
    let d = &s.dialogue;
    check!(d.with == "mara");
    check!(d.start == ["again", "intro"]);
    let intro = d.node("intro").unwrap();
    check!(intro.say == Text::Plain("Hello, traveller.".into()));
    check!(intro.outcomes.set["player.gold"] == Value::Int(-5));
    check!(intro.options[0].go == Target::Node("names".into()));
    check!(intro.options[0].outcomes.relation["trust"] == 1);
    check!(intro.options[1].when.as_deref() == Some("relation (trust) >= 1"));
    check!(intro.options[1].go == Target::Back);
    check!(intro.options[2].id.as_deref() == Some("bye"));
    assert!(let [Effect::Custom { key, args }] = intro.options[2].outcomes.then.as_slice());
    check!(key == "ring_bell");
    check!(*args == vec![Value::Int(3), Value::Bool(true)]);

    let again = d.node("again").unwrap();
    check!(again.when.as_deref() == Some("met_mara and visits >= 2"));
    assert!(let [Effect::Relation { delta: -2, .. }] = again.outcomes.then.as_slice());

    let names = d.node("names").unwrap();
    check!(
        names.speaker.as_deref() == Some("mara"),
        "the first line is the node's line"
    );
    check!(names.say == Text::Plain("They call me Mara.".into()));
    check!(names.lines.len() == 1);
    check!(names.lines[0].speaker == "player");
    check!(names.next == Some(Target::End));

    let loaded = lower_plain(d).expect("lowers");
    check!(loaded.graph.nodes.len() == 3);
}

#[test]
fn the_dialogue_keyword_and_an_id_are_optional() {
    let s = parse("with mara start intro { intro: \"Hi.\" { -> end; } }").unwrap();
    check!(s.dialogue.id.is_none());
    let s = parse("dialogue talk with mara start intro { intro: \"Hi.\" { -> end; } }").unwrap();
    check!(s.dialogue.id.as_deref() == Some("talk"));
}

fn messages(src: &str) -> Vec<String> {
    match parse(src) {
        Err(e) => e.into_iter().map(|e| e.to_string()).collect(),
        Ok(s) => match s.check(true) {
            Ok(()) => vec![],
            Err(e) => e.into_iter().map(|e| e.to_string()).collect(),
        },
    }
}

#[test]
fn errors() {
    check!(
        messages("with mara start a { a: \"Hi.\" { \"Go\" => nowhere; } }")
            == ["no node `nowhere`"]
    );
    check!(
        messages("with mara start a { a: \"Hi.\" { -> end; } a: \"Again.\" { -> end; } }")
            == ["duplicate node key `a`"]
    );
    check!(
        messages("with mara start a { a: \"Hi.\"; }")
            == [
                "node `a` has no options and no `next`; the talk ends here (write `-> end;` to say so)"
            ]
    );
    let m = messages("with mara start a { a: \"Hi.\" { when gold >= ; -> end; } }");
    check!(m[0].contains("expected a number"), "{m:?}");
    let m = messages("with mara start a { a { -> end; } }");
    check!(m[0].contains("node `a` has no line"), "{m:?}");
    let m = messages("with mara start a { a: \"Hi.\" { \"Go\" names; } }");
    check!(m[0].contains("expected `=> target`"), "{m:?}");
}

#[test]
fn spans_point_at_the_tokens() {
    let s = parse("with mara start a {\n a: \"Hi.\" {\n \"Go\" => nowhere;\n } }").unwrap();
    let span = s.span(&Place::OptionGo(0, 0));
    check!(span.start().line == 3);
    check!(span.start().column == 9);
}
