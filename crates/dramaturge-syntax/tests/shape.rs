//! The serde shape: YAML and JSON round trips, lowering, and the validator
//! on lowered graphs.
#![cfg(feature = "serde")]

use assert2::{assert, check};
use dramaturge_authoring::{GraphValidator, ValidationSeverity};
use dramaturge_model::{
    Condition, DialogTarget, DialogText, FactScope, FactValue, MemoryQuery, NavigationHint,
    Outcome, RelationshipAxisId, SymbolTable,
};
use dramaturge_syntax::{
    Cx, Dialogue, IssueKind, Lower, LowerError, Place, Severity, Target, Text, Value, check, lower,
    lower_plain,
};

const MARA: &str = r#"
with: mara
start: [again, intro]
nodes:
  intro:
    say: "Hello, traveller."
    set: { met_mara: true, player.gold: 5 }
    add: { visits: 1 }
    options:
      - say: "Who are you?"
        go: names
        relation: { trust: 1 }
        remember: [asked_name]
      - say: "Back."
        when: relation(trust) >= 1
        go: back
      - id: bye
        say: "Goodbye."
        tags: [{ axis: skill, key: medic, prefix: "[Medic]" }]
        intentions: { explicit: [Leave] }
        hint: leave
  again:
    when: met_mara and remembers(mara, asked_name)
    say:
      - "Mara looks up."
      - when: visits >= 2
        text: "She smiles."
      - mode: cycle
        key: greet
        variants: ["Again?", "You."]
    options:
      - hint: { go_to: "the gate" }
        go: intro
        then:
          - relation: { from: player, to: orren, axis: trust, delta: -2 }
          - custom: { key: ring_bell, args: [3, true] }
  names:
    say: "They call me Mara."
    lines:
      - { speaker: player, say: "Mara. I will remember." }
      - { speaker: mara, say: "Do." }
    next: end
"#;

fn mara() -> Dialogue {
    serde_saphyr::from_str(MARA).expect("the YAML loads")
}

#[test]
fn yaml_reads_the_shape() {
    let d = mara();
    check!(d.with == "mara");
    check!(d.start == ["again", "intro"]);
    let keys: Vec<_> = d.nodes.iter().map(|n| n.key.as_str()).collect();
    check!(keys == ["intro", "again", "names"], "file order");
    let intro = d.node("intro").unwrap();
    check!(intro.outcomes.set["met_mara"] == Value::Bool(true));
    check!(intro.outcomes.add["visits"] == 1);
    check!(intro.options[0].go == Target::Node("names".into()));
    check!(intro.options[1].go == Target::Back);
    check!(intro.options[2].go == Target::End, "no `go` ends the talk");
    let names = d.node("names").unwrap();
    check!(names.lines.len() == 2);
    check!(names.next == Some(Target::End));
    assert!(let Text::Fragments(f) = &d.node("again").unwrap().say);
    check!(f.len() == 3);
}

#[test]
fn yaml_round_trip() {
    let d = mara();
    let yaml = serde_saphyr::to_string(&d).expect("the dialogue writes");
    let again: Dialogue = serde_saphyr::from_str(&yaml).expect("the written YAML loads");
    check!(again == d, "{yaml}");

    let json = serde_json::to_string(&d).unwrap();
    let again: Dialogue = serde_json::from_str(&json).unwrap();
    check!(again == d);
    // Defaults stay out of the file.
    check!(!json.contains("\"go\":\"end\""));
    check!(!json.contains("\"lines\":[]"));
}

#[test]
fn unknown_fields_and_numeric_keys_are_errors() {
    let typo = MARA.replace("    add: { visits: 1 }", "    ad: { visits: 1 }");
    let err = serde_saphyr::from_str::<Dialogue>(&typo).unwrap_err();
    check!(err.to_string().contains("ad"), "{err}");

    let option_typo = MARA.replace("        go: names", "        goto: names");
    check!(serde_saphyr::from_str::<Dialogue>(&option_typo).is_err());

    let numeric = MARA.replace("        go: names", "        go: 2");
    let err = serde_saphyr::from_str::<Dialogue>(&numeric).unwrap_err();
    check!(err.to_string().contains("numeric id 2"), "{err}");
}

#[test]
fn lowering_builds_the_graph() {
    let loaded = lower_plain(&mara()).expect("the dialogue lowers");
    let s = &loaded.symbols;
    let g = &loaded.graph;
    let node = |key: &str| g.node(s.node(key).unwrap()).unwrap();
    let entries: Vec<_> = g
        .entry_nodes
        .iter()
        .map(|&id| s.node_key(id).unwrap())
        .collect();
    check!(entries == ["again", "intro"]);

    let player = s.entity(SymbolTable::PLAYER_KEY).unwrap();
    let mara = s.entity("mara").unwrap();
    let intro = node("intro");
    check!(intro.speaker.0 == mara);
    check!(intro.text == DialogText::inline("Hello, traveller."));
    check!(
        intro.outcomes_on_enter
            == vec![
                Outcome::SetFact {
                    scope: FactScope::World,
                    fact: dramaturge_model::Fact::new("met_mara", FactValue::Bool(true)),
                },
                Outcome::SetFact {
                    scope: FactScope::Player,
                    fact: dramaturge_model::Fact::new("gold", FactValue::Int(5)),
                },
                Outcome::AddFact {
                    scope: FactScope::World,
                    fact_id: "visits".into(),
                    delta: 1,
                },
            ]
    );
    let who = &intro.options[0];
    check!(s.option_key(who.id) == Some("intro#0"));
    check!(who.target == DialogTarget::Node(s.node("names").unwrap()));
    assert!(let [Outcome::AddRelationship { source, target, delta: 1, .. }, Outcome::AddMemory { owner, .. }] = who.outcomes.as_slice());
    check!((*source, *target, *owner) == (player, mara, mara));
    check!(intro.options[1].target == DialogTarget::Return);
    check!(
        intro.options[1].conditions
            == vec![Condition::RelationshipAtLeast {
                source: player,
                target: mara,
                axis: RelationshipAxisId::new("trust"),
                value: 1,
            }]
    );
    let bye = &intro.options[2];
    check!(s.option_key(bye.id) == Some("bye"));
    check!(bye.tags[0].requires_trait, "skill tags need the trait");
    check!(bye.navigation_hint == Some(NavigationHint::Leave));

    let again = node("again");
    assert!(let [Condition::All(parts)] = again.conditions.as_slice());
    check!(
        parts[1]
            == Condition::HasMemory {
                owner: mara,
                query: MemoryQuery::by_id(s.memory("asked_name").unwrap()),
            }
    );
    let hinted = &again.options[0];
    check!(hinted.text == DialogText::inline("Go to the gate"));
    assert!(let [Outcome::AddRelationship { target, delta: -2, .. }, Outcome::Custom { key, args }] = hinted.outcomes.as_slice());
    check!(*target == s.entity("orren").unwrap());
    check!(key == "ring_bell");
    check!(*args == vec![FactValue::Int(3), FactValue::Bool(true)]);

    let names = node("names");
    check!(names.lines[0].speaker.0 == player);
    check!(names.next == Some(DialogTarget::End));
}

#[test]
fn the_validator_accepts_the_lowered_graph() {
    let loaded = lower_plain(&mara()).unwrap();
    let issues = GraphValidator::new().validate_with_symbols(&loaded.graph, &loaded.symbols);
    let bad: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == ValidationSeverity::Error)
        .collect();
    check!(bad.is_empty(), "{bad:?}");
}

#[test]
fn check_finds_key_and_target_errors() {
    let yaml = r#"
with: mara
start: [intro, nowhere]
nodes:
  intro:
    say: "Hi."
    next: gone
    options:
      - { say: "A", go: missing }
      - { id: x, say: "B" }
      - { id: x }
  end:
    say: "Reserved."
  quiet:
    say: "No way on."
"#;
    let d: Dialogue = serde_saphyr::from_str(yaml).unwrap();
    let issues = check(&d);
    let found: Vec<_> = issues
        .iter()
        .map(|i| (i.kind, i.severity, i.path.as_str()))
        .collect();
    check!(found.contains(&(IssueKind::UnknownTarget, Severity::Error, "start[1]")));
    check!(found.contains(&(
        IssueKind::UnknownTarget,
        Severity::Error,
        "nodes.intro.next"
    )));
    check!(found.contains(&(
        IssueKind::UnknownTarget,
        Severity::Error,
        "nodes.intro.options[0].go"
    )));
    check!(found.contains(&(
        IssueKind::DuplicateOption,
        Severity::Error,
        "nodes.intro.options[2].id"
    )));
    check!(found.contains(&(
        IssueKind::NoLabel,
        Severity::Error,
        "nodes.intro.options[2]"
    )));
    check!(found.contains(&(IssueKind::ReservedKey, Severity::Error, "nodes.end")));
    check!(found.contains(&(IssueKind::NoWayOn, Severity::Warning, "nodes.quiet")));

    assert!(let Err(LowerError::Check(_)) = lower_plain(&d));
}

#[test]
fn a_bad_condition_names_its_place() {
    let yaml = MARA.replace("relation(trust) >= 1", "relation(trust) >= high");
    let d: Dialogue = serde_saphyr::from_str(&yaml).unwrap();
    let err = lower_plain(&d).unwrap_err();
    assert!(let LowerError::Host { place: Place::OptionWhen(0, 1), .. } = &err);
    check!(
        err.to_string()
            .starts_with("nodes.intro.options[1].when: expected a number"),
        "{err}"
    );
}

// ------------------------------------------------- a host with its own payloads

/// A host like loremaster: conditions and text in its own language (here:
/// kept as strings), world effects as typed host outcomes.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum WorldEffect {
    Say(String),
    Goto(String),
}

#[derive(Debug, Clone, PartialEq)]
struct Markup(String);

struct Lore {
    relations: Vec<&'static str>,
}

impl Lower<String, String, WorldEffect> for Lore {
    type Host = Vec<WorldEffect>;
    type Text = Markup;
    type Error = String;

    fn condition(&mut self, _: &mut Cx<'_>, source: &String) -> Result<Condition, String> {
        Ok(Condition::Custom {
            key: source.clone(),
            args: vec![],
        })
    }

    fn text(&mut self, _: &mut Cx<'_>, source: &String) -> Result<DialogText<Markup>, String> {
        Ok(DialogText::host(Markup(source.clone())))
    }

    fn effects(
        &mut self,
        _: &mut Cx<'_>,
        then: &[WorldEffect],
    ) -> Result<Vec<Outcome<Vec<WorldEffect>>>, String> {
        Ok(vec![Outcome::Host(then.to_vec())])
    }

    fn relation(
        &mut self,
        cx: &mut Cx<'_>,
        axis: &str,
        delta: i32,
    ) -> Result<Vec<Outcome<Vec<WorldEffect>>>, String> {
        if !self.relations.contains(&axis) {
            return Err(format!("unknown relation `{axis}`"));
        }
        Ok(vec![Outcome::AddRelationship {
            source: cx.player,
            target: cx.with,
            axis: RelationshipAxisId::new(axis),
            delta,
        }])
    }
}

/// The cantina dialogue of loremaster, as it is in the editor, loads as is.
#[test]
fn a_loremaster_dialogue_lowers_with_host_hooks() {
    let d: Dialogue<String, String, WorldEffect> =
        serde_saphyr::from_str(include_str!("data/blackbeard.yaml")).expect("the YAML loads");
    check!(d.with == "blackbeard");
    check!(d.start == ["again", "meet"]);

    let mut symbols = SymbolTable::new();
    let mut host = Lore {
        relations: vec!["trust"],
    };
    let lowered = lower(&d.clone().with_id("bb"), &mut symbols, &mut host).expect("lowers");
    let g = &lowered.graph;
    check!(g.nodes.len() == 4);
    check!(symbols.node_key(g.entry_nodes[0]) == Some("bb/again"));
    let marigold = g.node(symbols.node("bb/marigold").unwrap()).unwrap();
    check!(let Some(Markup(_)) = marigold.text.as_host());
    let thanks = &marigold.options[0];
    check!(symbols.option_key(thanks.id) == Some("bb/marigold#0"));
    assert!(let [Outcome::AddRelationship { .. }, Outcome::Host(effects)] = thanks.outcomes.as_slice());
    check!(effects[1] == WorldEffect::Goto("street".into()));

    let mut strict = Lore { relations: vec![] };
    let err = lower(&d, &mut SymbolTable::new(), &mut strict).unwrap_err();
    check!(
        err.to_string()
            == "nodes.meet.options[0].relation.trust: unknown relation `trust`"
                .replace("meet.options[0]", "names")
    );
}

// ------------------------------------------------------------- author notes

/// A note type like loremaster's `Note`: a plain string, or a map with an
/// id and a todo flag.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
enum Note {
    Text(String),
    Full {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        text: String,
        #[serde(default)]
        todo: bool,
    },
}

#[test]
fn notes_load_with_a_host_note_type_and_do_not_reach_the_graph() {
    let base = include_str!("data/blackbeard.yaml");
    let yaml = format!(
        "{base}\nnotes:\n  - \"Blackbeard knows more than he says.\"\n  - {{ id: marigold_hint, text: \"Pay off the Marigold in act 2.\", todo: true }}\n"
    )
    .replace(
        "  meet:\n",
        "  meet:\n    notes: [\"First meeting.\"]\n",
    )
    .replace(
        "      - say: '\"Not tonight.\"'\n",
        "      - say: '\"Not tonight.\"'\n        notes: [{ text: \"A way out.\" }]\n",
    );
    let d: Dialogue<String, String, WorldEffect, Note> =
        serde_saphyr::from_str(&yaml).expect("the YAML with notes loads");
    check!(d.notes.len() == 2);
    check!(
        d.notes[1]
            == Note::Full {
                id: Some("marigold_hint".into()),
                text: "Pay off the Marigold in act 2.".into(),
                todo: true,
            }
    );
    let meet = d.node("meet").unwrap();
    check!(meet.notes == vec![Note::Text("First meeting.".into())]);
    check!(meet.options[2].notes.len() == 1);
    check!(d.node("names").unwrap().notes.is_empty());

    // Notes survive a round trip and are left out where there are none.
    let out = serde_saphyr::to_string(&d).unwrap();
    let again: Dialogue<String, String, WorldEffect, Note> = serde_saphyr::from_str(&out).unwrap();
    check!(again == d);
    check!(out.matches("notes:").count() == 3, "{out}");

    // The lowering ignores the notes.
    let mut host = Lore {
        relations: vec!["trust"],
    };
    let with_notes = lower(&d, &mut SymbolTable::new(), &mut host).unwrap();
    let mut plain = d.clone();
    plain.notes.clear();
    for node in &mut plain.nodes {
        node.notes.clear();
        for option in &mut node.options {
            option.notes.clear();
        }
    }
    let without = lower(&plain, &mut SymbolTable::new(), &mut host).unwrap();
    check!(with_notes.graph == without.graph);
}

#[test]
fn plain_notes_are_strings() {
    let yaml = MARA.replace(
        "with: mara\n",
        "with: mara\nnotes: [\"Mara is the clerk.\"]\n",
    );
    let d: Dialogue = serde_saphyr::from_str(&yaml).unwrap();
    check!(d.notes == ["Mara is the clerk."]);
    check!(lower_plain(&d).is_ok());
}

// ------------------------------------------------------------- review fixes

#[test]
fn an_empty_say_needs_a_hint() {
    for say in ["\"\"", "[]"] {
        let yaml = format!(
            "with: m\nstart: a\nnodes:\n  a:\n    say: Hi.\n    options:\n      - say: {say}\n"
        );
        let d: Dialogue = serde_saphyr::from_str(&yaml).unwrap();
        let err = lower_plain(&d).unwrap_err();
        assert!(let LowerError::Check(issues) = &err);
        check!(issues[0].kind == IssueKind::NoLabel);
        check!(issues[0].path == "nodes.a.options[0]");

        let hinted = yaml.replace("      - say:", "      - hint: leave\n        say:");
        let d: Dialogue = serde_saphyr::from_str(&hinted).unwrap();
        let loaded = lower_plain(&d).unwrap();
        check!(loaded.graph.nodes[0].options[0].text == DialogText::inline("Leave..."));
    }
}

#[test]
fn a_generated_option_key_can_not_collide_with_an_id() {
    let yaml = r#"
with: m
start: a
nodes:
  a:
    say: Hi.
    options:
      - { id: "b#0", say: "One.", go: b }
  b:
    say: Bye.
    options:
      - { say: "Two." }
"#;
    let d: Dialogue = serde_saphyr::from_str(yaml).unwrap();
    let issues = check(&d);
    check!(issues.iter().any(|i| i.kind == IssueKind::DuplicateOption
        && i.path == "nodes.b.options[0]"
        && i.message.contains("`b#0`")));
}

#[test]
fn a_single_mode_fragment_takes_one_text() {
    let yaml = r#"
with: m
start: a
nodes:
  a:
    say:
      - variants: ["One.", "Two."]
    next: end
"#;
    let d: Dialogue = serde_saphyr::from_str(yaml).unwrap();
    let err = lower_plain(&d).unwrap_err();
    check!(err.to_string().contains("shows one text"), "{err}");
    let cycle = yaml.replace(
        "      - variants:",
        "      - mode: cycle\n        variants:",
    );
    let d: Dialogue = serde_saphyr::from_str(&cycle).unwrap();
    check!(lower_plain(&d).is_ok());
}

#[test]
fn an_explicit_null_key_is_no_key() {
    let yaml = "with: m\nstart: a\nnodes:\n  a:\n    speaker:\n    say: Hi.\n    options:\n      - { id: ~, say: Bye. }\n";
    let d: Dialogue = serde_saphyr::from_str(yaml).unwrap();
    check!(d.nodes[0].speaker.is_none());
    check!(d.nodes[0].options[0].id.is_none());
    let json: Dialogue = serde_json::from_str(
        r#"{"with":"m","start":["a"],"nodes":{"a":{"speaker":null,"say":"Hi.","next":"end"}}}"#,
    )
    .unwrap();
    check!(json.nodes[0].speaker.is_none());
}

#[test]
fn a_duplicate_node_key_does_not_serialize() {
    let d: Dialogue = Dialogue::new("m")
        .with_start("a")
        .with_node(dramaturge_syntax::Node::new("a", "One.".into()))
        .with_node(dramaturge_syntax::Node::new("a", "Two.".into()));
    let err = serde_json::to_string(&d).unwrap_err();
    check!(err.to_string().contains("duplicate node key `a`"));
}
