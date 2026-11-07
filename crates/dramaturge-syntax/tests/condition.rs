//! The default condition language.

use assert2::{assert, check};
use dramaturge_model::{Condition, FactId, FactScope, FactValue, RelationshipAxisId, SymbolTable};
use dramaturge_syntax::condition::{Expr, parse};
use dramaturge_syntax::{Choice, Dialogue, Node, Target, lower_plain};

/// Lowers `src` as the `when` of an option in a dialogue with `mara`.
fn lower(src: &str) -> (Condition, SymbolTable) {
    let d: Dialogue = Dialogue::new("mara").with_start("a").with_node(
        Node::new("a", "Hi.".into())
            .with_option(Choice::new("Go.".into(), Target::End).with_when(src.to_string())),
    );
    let loaded = lower_plain(&d).unwrap_or_else(|e| panic!("{src}: {e}"));
    let c = loaded.graph.nodes[0].options[0].conditions[0].clone();
    (c, loaded.symbols)
}

fn eq(name: &str, value: FactValue) -> Condition {
    Condition::FactEquals {
        scope: FactScope::World,
        fact_id: FactId::new(name),
        value,
    }
}

#[test]
fn facts_and_comparisons() {
    check!(lower("met").0 == eq("met", FactValue::Bool(true)));
    check!(lower("mara.met == false").0 == eq("mara.met", FactValue::Bool(false)));
    check!(lower("name == \"Ada\"").0 == eq("name", FactValue::Text("Ada".into())));
    check!(lower("gold != 0").0 == Condition::Not(Box::new(eq("gold", FactValue::Int(0)))));
    let at_least = |scope, value| Condition::FactAtLeast {
        scope,
        fact_id: FactId::new("gold"),
        value,
    };
    check!(lower("gold >= 10").0 == at_least(FactScope::World, 10));
    check!(lower("gold > 9").0 == at_least(FactScope::World, 10));
    check!(lower("player.gold >= -3").0 == at_least(FactScope::Player, -3));
    assert!(let Condition::FactAtMost { value: 3, .. } = lower("gold < 4").0);
    assert!(let Condition::FactExists { scope: FactScope::Party, .. } = lower("exists(party.flag)").0);
    check!(lower("true").0 == Condition::Always);
}

#[test]
fn logic_and_precedence() {
    let c = lower("a or b and not c").0;
    assert!(let Condition::Any(items) = c);
    assert!(let Condition::All(inner) = &items[1]);
    assert!(let Condition::Not(_) = &inner[1]);
    let c = lower("!(a || b) && c").0;
    assert!(let Condition::All(items) = c);
    assert!(let Condition::Not(_) = &items[0]);
}

#[test]
fn relations_and_memories() {
    let (c, s) = lower("relation(trust) >= 3");
    let player = s.entity(SymbolTable::PLAYER_KEY).unwrap();
    let mara = s.entity("mara").unwrap();
    check!(
        c == Condition::RelationshipAtLeast {
            source: player,
            target: mara,
            axis: RelationshipAxisId::new("trust"),
            value: 3,
        }
    );
    let (c, s) = lower("relation(trust, orren) < 0");
    assert!(let Condition::RelationshipAtMost { target, value: -1, .. } = c);
    check!(target == s.entity("orren").unwrap());
    let (c, s) = lower("remembers(player, saw_it)");
    assert!(let Condition::HasMemory { owner, .. } = c);
    check!(owner == s.entity(SymbolTable::PLAYER_KEY).unwrap());
    check!(s.memory("saw_it").is_some());
}

#[test]
fn errors_have_a_range() {
    let e = parse("gold >= ").unwrap_err();
    check!(e.message.contains("expected a number"));
    let e = parse("a and (b").unwrap_err();
    check!(e.message == "expected `)`");
    let e = parse("a b").unwrap_err();
    check!(e.range == (2..3));
    let e = parse("weird(x)").unwrap_err();
    check!(e.message.contains("unknown function `weird`"));
    let e = parse("relation(trust)").unwrap_err();
    check!(e.message.contains("compare a relation"));
    let e = parse("gold >= \"x\"").unwrap_err();
    check!(e.message == "expected a number");
    check!(parse("a $ b").unwrap_err().message == "unexpected `$`");
}

#[test]
fn parse_keeps_the_structure() {
    check!(parse("player . gold >= 3").unwrap() == parse("player.gold >= 3").unwrap());
    assert!(let Expr::Remembers { .. } = parse("remembers ( mara , x )").unwrap());
}

#[test]
fn string_escapes() {
    let text = |src: &str| match parse(src).unwrap() {
        Expr::Compare {
            value: dramaturge_syntax::condition::Literal::Text(t),
            ..
        } => t,
        other => panic!("{other:?}"),
    };
    check!(text(r#"a == "x\ny\t\"\\\u{e9}""#) == "x\ny\t\"\\\u{e9}");
    let e = parse(r#"a == "\q""#).unwrap_err();
    check!(e.message == "unknown escape `\\q`");
    check!(e.range == (6..8));
    check!(parse(r#"a == "\u{zz}""#).is_err());
    check!(parse(r#"a == "\u{d800}""#).unwrap_err().message == "not a unicode character");
}

#[test]
fn strict_comparisons_at_the_bounds_never_hold() {
    check!(lower(&format!("gold > {}", i64::MAX)).0 == Condition::Never);
    check!(lower(&format!("gold < {}", i64::MIN + 1)).0 != Condition::Never);
    check!(lower(&format!("relation(trust) > {}", i32::MAX)).0 == Condition::Never);
    check!(lower(&format!("relation(trust) < {}", i32::MIN)).0 == Condition::Never);
}
