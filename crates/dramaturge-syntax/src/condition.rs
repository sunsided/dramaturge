//! The default condition language.
//!
//! A host with its own condition language (for example loremaster) parses
//! `when` itself in [`Lower::condition`](crate::Lower::condition). Plain
//! dramaturge uses this small language:
//!
//! | Form | Condition |
//! | --- | --- |
//! | `met_mara` | the fact is `true` |
//! | `gold >= 10`, `gold > 9`, `gold <= 3`, `gold < 4` | integer fact compare |
//! | `name == "Ada"`, `met == false`, `gold != 0` | fact equals (or not) a value |
//! | `"a\"b\n"` | string escapes: `\n`, `\t`, `\r`, `\0`, `\\`, `\"`, `\'`, `\u{..}` |
//! | `exists(met_mara)` | the fact exists |
//! | `player.gold >= 10` | a fact in the player scope (also `world.`, `party.`) |
//! | `relation(trust) >= 3` | relation of the player to the speaker of the node |
//! | `relation(trust, orren) >= 3` | relation of the player to `orren` |
//! | `remembers(mara, saw_the_seal)` | `mara` has the memory |
//! | `seen_by(player)` | the perceiver of a text is `player` (`Condition::SeenBy`) |
//! | `true`, `false` | always, never |
//! | `not a`, `a and b`, `a or b`, `( a )` | also `!`, `&&`, `\|\|` |
//!
//! Names are identifiers joined with dots: `mara.met` is the world fact
//! `mara.met`. The key `player` is the player in `remembers`, `relation`
//! and `seen_by`.
//!
//! [`parse`] reads the text into an [`Expr`] without the model, so the
//! `dialogue!` macro checks a condition at compile time.
//! [`Expr::lower`] makes the model [`Condition`].

use crate::lower::Cx;
use dramaturge_model::{
    CharacterId, Condition, FactId, FactScope, FactValue, MemoryQuery, RelationshipAxisId,
    SymbolTable,
};
use std::fmt;
use std::ops::Range;

/// A condition in the default language, before lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Expr {
    /// `true` or `false`.
    Bool(bool),
    /// `not a`.
    Not(Box<Expr>),
    /// `a and b and ...`.
    And(Vec<Expr>),
    /// `a or b or ...`.
    Or(Vec<Expr>),
    /// A name alone: the fact is `true`.
    Truthy(Fact),
    /// `exists(name)`.
    Exists(Fact),
    /// `left op value`.
    Compare {
        /// What is compared.
        left: Operand,
        /// The operator.
        op: Op,
        /// The value.
        value: Literal,
    },
    /// `remembers(owner, memory)`.
    Remembers {
        /// The owner key.
        owner: String,
        /// The memory key.
        memory: String,
    },
    /// `seen_by(actor)`: the perceiver of a text is this character.
    SeenBy(String),
}

/// A fact name with its scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// The scope prefix: `world` (the default), `player` or `party`.
    pub scope: ScopeName,
    /// The fact name without the prefix.
    pub name: String,
}

/// A scope prefix of a fact name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScopeName {
    /// `world.` or no prefix.
    World,
    /// `player.`.
    Player,
    /// `party.`.
    Party,
}

impl ScopeName {
    /// The model scope.
    pub fn to_model(self) -> FactScope {
        match self {
            ScopeName::World => FactScope::World,
            ScopeName::Player => FactScope::Player,
            ScopeName::Party => FactScope::Party,
        }
    }
}

impl Fact {
    /// Reads a name with an optional scope prefix: `player.gold`.
    pub fn parse(name: &str) -> Self {
        let (scope, rest) = match name.split_once('.') {
            Some(("player", rest)) => (ScopeName::Player, rest),
            Some(("world", rest)) => (ScopeName::World, rest),
            Some(("party", rest)) => (ScopeName::Party, rest),
            _ => (ScopeName::World, name),
        };
        Fact {
            scope,
            name: rest.to_string(),
        }
    }
}

/// The left side of a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Operand {
    /// A fact.
    Fact(Fact),
    /// `relation(axis)` or `relation(axis, target)`.
    Relation {
        /// The axis.
        axis: String,
        /// The target key. `None` is the speaker of the node.
        target: Option<String>,
    },
}

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Op {
    Eq,
    Ne,
    Ge,
    Le,
    Gt,
    Lt,
}

/// A literal value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Text(String),
}

/// An error in a condition, with the byte range in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The message.
    pub message: String,
    /// The byte range in the condition text.
    pub range: Range<usize>,
}

impl ParseError {
    fn new(message: impl Into<String>, range: Range<usize>) -> Self {
        Self {
            message: message.into(),
            range,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Dot,
    Int(i64),
    Str(String),
    LParen,
    RParen,
    Comma,
    Op(Op),
    And,
    Or,
    Not,
}

fn lex(src: &str) -> Result<Vec<(Tok, Range<usize>)>, ParseError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let start = i;
        let two = |s: &str| src[i..].starts_with(s);
        let tok = match c {
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
                continue;
            }
            b'(' => Tok::LParen,
            b')' => Tok::RParen,
            b',' => Tok::Comma,
            b'.' => Tok::Dot,
            _ if two("==") => Tok::Op(Op::Eq),
            _ if two("!=") => Tok::Op(Op::Ne),
            _ if two(">=") => Tok::Op(Op::Ge),
            _ if two("<=") => Tok::Op(Op::Le),
            _ if two("&&") => Tok::And,
            _ if two("||") => Tok::Or,
            b'>' => Tok::Op(Op::Gt),
            b'<' => Tok::Op(Op::Lt),
            b'!' => Tok::Not,
            b'"' => {
                let mut text = String::new();
                let mut j = i + 1;
                let body = j;
                let mut chars = src[body..].char_indices().peekable();
                let unclosed = || ParseError::new("unclosed string", start..src.len());
                loop {
                    match chars.next() {
                        None => return Err(unclosed()),
                        Some((k, '"')) => {
                            j = body + k + 1;
                            break;
                        }
                        Some((k, '\\')) => {
                            let at = body + k;
                            let escape = match chars.next() {
                                None => return Err(unclosed()),
                                Some((_, 'n')) => '\n',
                                Some((_, 't')) => '\t',
                                Some((_, 'r')) => '\r',
                                Some((_, '0')) => '\0',
                                Some((_, '\\')) => '\\',
                                Some((_, '"')) => '"',
                                Some((_, '\'')) => '\'',
                                Some((_, 'u')) => {
                                    let mut hex = String::new();
                                    let mut end = at + 2;
                                    if chars.next().map(|(_, c)| c) != Some('{') {
                                        return Err(ParseError::new(
                                            "expected `\\u{...}`",
                                            at..end,
                                        ));
                                    }
                                    loop {
                                        match chars.next() {
                                            Some((k, '}')) => {
                                                end = body + k + 1;
                                                break;
                                            }
                                            Some((k, c))
                                                if c.is_ascii_hexdigit() && hex.len() < 6 =>
                                            {
                                                hex.push(c);
                                                end = body + k + 1;
                                            }
                                            _ => {
                                                return Err(ParseError::new(
                                                    "expected `\\u{...}` with 1 to 6 hex digits",
                                                    at..end,
                                                ));
                                            }
                                        }
                                    }
                                    u32::from_str_radix(&hex, 16)
                                        .ok()
                                        .and_then(char::from_u32)
                                        .ok_or_else(|| {
                                            ParseError::new("not a unicode character", at..end)
                                        })?
                                }
                                Some((k, c)) => {
                                    let end = body + k + c.len_utf8();
                                    return Err(ParseError::new(
                                        format!("unknown escape `\\{c}`"),
                                        at..end,
                                    ));
                                }
                            };
                            text.push(escape);
                        }
                        Some((_, c)) => text.push(c),
                    }
                }
                out.push((Tok::Str(text), start..j));
                i = j;
                continue;
            }
            b'-' | b'0'..=b'9' => {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                let n = src[i..j]
                    .parse()
                    .map_err(|_| ParseError::new("not a number", i..j))?;
                out.push((Tok::Int(n), i..j));
                i = j;
                continue;
            }
            c if c == b'_' || c.is_ascii_alphabetic() => {
                let mut j = i + 1;
                while j < bytes.len() && (bytes[j] == b'_' || bytes[j].is_ascii_alphanumeric()) {
                    j += 1;
                }
                let word = &src[i..j];
                out.push((
                    match word {
                        "and" => Tok::And,
                        "or" => Tok::Or,
                        "not" => Tok::Not,
                        _ => Tok::Ident(word.to_string()),
                    },
                    i..j,
                ));
                i = j;
                continue;
            }
            _ => {
                let len = src[i..].chars().next().map_or(1, char::len_utf8);
                return Err(ParseError::new(
                    format!("unexpected `{}`", &src[i..i + len]),
                    i..i + len,
                ));
            }
        };
        i += match tok {
            Tok::Op(Op::Gt | Op::Lt) | Tok::Not => 1,
            Tok::Op(_) | Tok::And | Tok::Or => 2,
            _ => 1,
        };
        out.push((tok, start..i));
    }
    Ok(out)
}

struct Parser<'a> {
    toks: Vec<(Tok, Range<usize>)>,
    pos: usize,
    len: usize,
    _src: &'a str,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(t, _)| t)
    }

    fn range(&self) -> Range<usize> {
        self.toks
            .get(self.pos)
            .map_or(self.len..self.len, |(_, r)| r.clone())
    }

    fn next(&mut self) -> Option<(Tok, Range<usize>)> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn expect(&mut self, tok: Tok, what: &str) -> Result<(), ParseError> {
        if self.peek() == Some(&tok) {
            self.pos += 1;
            Ok(())
        } else {
            Err(ParseError::new(format!("expected {what}"), self.range()))
        }
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut items = vec![self.and()?];
        while self.peek() == Some(&Tok::Or) {
            self.pos += 1;
            items.push(self.and()?);
        }
        Ok(if items.len() == 1 {
            items.remove(0)
        } else {
            Expr::Or(items)
        })
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut items = vec![self.unary()?];
        while self.peek() == Some(&Tok::And) {
            self.pos += 1;
            items.push(self.unary()?);
        }
        Ok(if items.len() == 1 {
            items.remove(0)
        } else {
            Expr::And(items)
        })
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.peek() == Some(&Tok::Not) {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        self.atom()
    }

    fn name(&mut self) -> Result<String, ParseError> {
        let mut name = match self.next() {
            Some((Tok::Ident(s), _)) => s,
            _ => {
                self.pos -= 1;
                return Err(ParseError::new("expected a name", self.range()));
            }
        };
        while self.peek() == Some(&Tok::Dot) {
            self.pos += 1;
            match self.next() {
                Some((Tok::Ident(s), _)) => {
                    name.push('.');
                    name.push_str(&s);
                }
                _ => {
                    self.pos -= 1;
                    return Err(ParseError::new("expected a name after `.`", self.range()));
                }
            }
        }
        Ok(name)
    }

    fn atom(&mut self) -> Result<Expr, ParseError> {
        let start = self.range();
        match self.peek() {
            Some(Tok::LParen) => {
                self.pos += 1;
                let e = self.or()?;
                self.expect(Tok::RParen, "`)`")?;
                return Ok(e);
            }
            Some(Tok::Ident(_)) => {}
            _ => return Err(ParseError::new("expected a condition", start)),
        }
        let name = self.name()?;
        let left = match name.as_str() {
            "true" => return Ok(Expr::Bool(true)),
            "false" => return Ok(Expr::Bool(false)),
            "exists" if self.peek() == Some(&Tok::LParen) => {
                self.pos += 1;
                let fact = self.name()?;
                self.expect(Tok::RParen, "`)`")?;
                return Ok(Expr::Exists(Fact::parse(&fact)));
            }
            "remembers" if self.peek() == Some(&Tok::LParen) => {
                self.pos += 1;
                let owner = self.name()?;
                self.expect(Tok::Comma, "`,`")?;
                let memory = self.name()?;
                self.expect(Tok::RParen, "`)`")?;
                return Ok(Expr::Remembers { owner, memory });
            }
            "seen_by" if self.peek() == Some(&Tok::LParen) => {
                self.pos += 1;
                let actor = self.name()?;
                self.expect(Tok::RParen, "`)`")?;
                return Ok(Expr::SeenBy(actor));
            }
            "relation" if self.peek() == Some(&Tok::LParen) => {
                self.pos += 1;
                let axis = self.name()?;
                let target = if self.peek() == Some(&Tok::Comma) {
                    self.pos += 1;
                    Some(self.name()?)
                } else {
                    None
                };
                self.expect(Tok::RParen, "`)`")?;
                let operand = Operand::Relation { axis, target };
                if !matches!(self.peek(), Some(Tok::Op(_))) {
                    return Err(ParseError::new(
                        "compare a relation with a number, for example `relation(trust) >= 3`",
                        self.range(),
                    ));
                }
                operand
            }
            _ if self.peek() == Some(&Tok::LParen) => {
                return Err(ParseError::new(
                    format!(
                        "unknown function `{name}`; use `exists`, `remembers`, `relation` or `seen_by`"
                    ),
                    start.start..self.range().end,
                ));
            }
            _ => Operand::Fact(Fact::parse(&name)),
        };
        let Some(Tok::Op(op)) = self.peek().cloned() else {
            let Operand::Fact(fact) = left else {
                unreachable!("a relation without an operator returned above")
            };
            return Ok(Expr::Truthy(fact));
        };
        self.pos += 1;
        let value_range = self.range();
        let value = match self.next() {
            Some((Tok::Int(n), _)) => Literal::Int(n),
            Some((Tok::Str(s), _)) => Literal::Text(s),
            Some((Tok::Ident(s), _)) if s == "true" => Literal::Bool(true),
            Some((Tok::Ident(s), _)) if s == "false" => Literal::Bool(false),
            _ => {
                return Err(ParseError::new(
                    "expected a number, a string, `true` or `false`",
                    value_range,
                ));
            }
        };
        let ordered = matches!(op, Op::Ge | Op::Le | Op::Gt | Op::Lt);
        let relation = matches!(left, Operand::Relation { .. });
        if (ordered || relation) && !matches!(value, Literal::Int(_)) {
            return Err(ParseError::new("expected a number", value_range));
        }
        if let (true, Literal::Int(n)) = (relation, &value)
            && i32::try_from(*n).is_err()
        {
            return Err(ParseError::new("the number is too large", value_range));
        }
        Ok(Expr::Compare { left, op, value })
    }
}

/// Reads a condition in the default language.
pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        len: src.len(),
        _src: src,
    };
    let e = p.or()?;
    if p.pos < p.toks.len() {
        return Err(ParseError::new(
            "expected the end of the condition, or `and`, `or`",
            p.range(),
        ));
    }
    Ok(e)
}

impl Expr {
    /// Makes the model condition. Entity keys (`remembers`, `relation`) go
    /// through [`Cx::entity`]; memory keys are interned.
    pub fn lower(&self, cx: &mut Cx<'_>) -> Condition {
        match self {
            Expr::Bool(true) => Condition::Always,
            Expr::Bool(false) => Condition::Never,
            Expr::Not(e) => Condition::Not(Box::new(e.lower(cx))),
            Expr::And(es) => Condition::All(es.iter().map(|e| e.lower(cx)).collect()),
            Expr::Or(es) => Condition::Any(es.iter().map(|e| e.lower(cx)).collect()),
            Expr::Truthy(fact) => Condition::FactEquals {
                scope: fact.scope.to_model(),
                fact_id: FactId::new(fact.name.as_str()),
                value: FactValue::Bool(true),
            },
            Expr::Exists(fact) => Condition::FactExists {
                scope: fact.scope.to_model(),
                fact_id: FactId::new(fact.name.as_str()),
            },
            Expr::Remembers { owner, memory } => Condition::HasMemory {
                owner: cx.entity(owner),
                query: MemoryQuery::by_id(cx.symbols.intern_memory(memory)),
            },
            Expr::SeenBy(actor) => Condition::SeenBy(CharacterId::new(cx.entity(actor))),
            Expr::Compare { left, op, value } => match left {
                Operand::Fact(fact) => fact_compare(fact, *op, value),
                Operand::Relation { axis, target } => {
                    let target = target.as_deref().map_or(cx.speaker, |key| cx.entity(key));
                    let n = match value {
                        Literal::Int(n) => *n as i32,
                        _ => 0,
                    };
                    let at = |least: bool, value: i32| {
                        let axis = RelationshipAxisId::new(axis.as_str());
                        if least {
                            Condition::RelationshipAtLeast {
                                source: cx.player,
                                target,
                                axis,
                                value,
                            }
                        } else {
                            Condition::RelationshipAtMost {
                                source: cx.player,
                                target,
                                axis,
                                value,
                            }
                        }
                    };
                    match op {
                        Op::Ge => at(true, n),
                        Op::Gt => n.checked_add(1).map_or(Condition::Never, |v| at(true, v)),
                        Op::Le => at(false, n),
                        Op::Lt => n.checked_sub(1).map_or(Condition::Never, |v| at(false, v)),
                        Op::Eq => Condition::All(vec![at(true, n), at(false, n)]),
                        Op::Ne => Condition::Not(Box::new(Condition::All(vec![
                            at(true, n),
                            at(false, n),
                        ]))),
                    }
                }
            },
        }
    }
}

fn fact_compare(fact: &Fact, op: Op, value: &Literal) -> Condition {
    let scope = fact.scope.to_model();
    let fact_id = FactId::new(fact.name.as_str());
    let equals = |value: &Literal| Condition::FactEquals {
        scope: scope.clone(),
        fact_id: fact_id.clone(),
        value: match value {
            Literal::Bool(b) => FactValue::Bool(*b),
            Literal::Int(n) => FactValue::Int(*n),
            Literal::Text(s) => FactValue::Text(s.clone()),
        },
    };
    let n = match value {
        Literal::Int(n) => *n,
        _ => 0,
    };
    match op {
        Op::Eq => equals(value),
        Op::Ne => Condition::Not(Box::new(equals(value))),
        Op::Ge => Condition::FactAtLeast {
            scope,
            fact_id,
            value: n,
        },
        // `x > MAX` and `x < MIN` never hold.
        Op::Gt => match n.checked_add(1) {
            Some(value) => Condition::FactAtLeast {
                scope,
                fact_id,
                value,
            },
            None => Condition::Never,
        },
        Op::Le => Condition::FactAtMost {
            scope,
            fact_id,
            value: n,
        },
        Op::Lt => match n.checked_sub(1) {
            Some(value) => Condition::FactAtMost {
                scope,
                fact_id,
                value,
            },
            None => Condition::Never,
        },
    }
}

/// Parses and lowers a condition in the default language.
pub fn lower_str(src: &str, cx: &mut Cx<'_>) -> Result<Condition, ParseError> {
    Ok(parse(src)?.lower(cx))
}

/// Returns the entity id of a key: `player` is the player
/// ([`SymbolTable::player`]); other keys are interned.
pub fn entity(symbols: &mut SymbolTable, key: &str) -> dramaturge_model::EntityId {
    if key == "player" {
        symbols.player()
    } else {
        symbols.intern_entity(key)
    }
}
