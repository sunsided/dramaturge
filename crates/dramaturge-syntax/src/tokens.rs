//! The token form: `syn` parsers for `dialogue!` and for `dialogue` blocks
//! in a host macro (loremaster's `world!`).
//!
//! ```text
//! dialogue with mara start [again, intro] {
//!     intro: "Hello, traveller." {
//!         set met_mara = true;
//!         "Who are you?" => names { relation trust += 1; }
//!         "Goodbye." => end;
//!     }
//!     again: "You again." {
//!         when met_mara;
//!         "Goodbye." => end;
//!     }
//!     names {
//!         mara: "They call me Mara.";
//!         player: "Mara. I will remember.";
//!         -> end;
//!     }
//! }
//! ```
//!
//! # Grammar
//!
//! ```text
//! dialogue  = ["dialogue" [id]] "with" key "start" (key | "[" key,* "]") "{" node* "}"
//! node      = key [":" text] ("{" statement* "}" | ";")
//! statement = "when" condition ";"              node condition
//!           | "speaker" key ";"                 the node's speaker
//!           | key ":" text ";"                  a line with its own speaker
//!           | "->" target ";"                   next (auto-continue)
//!           | outcome ";"
//!           | text ["when" condition] "=>" target ("{" option_statement* "}" | ";")
//! option_statement = "id" key ";" | outcome ";"
//! outcome   = "set" key "=" value ("," key "=" value)*
//!           | "add" key ("+=" | "-=") int ("," ...)*
//!           | "relation" key ("+=" | "-=") int ("," ...)*
//!           | "remember" key ("," key)*
//!           | effect                            the host's effect
//! target    = key | "end" | "back"
//! key       = ident ("." ident)* | string
//! value     = "true" | "false" | ["-"] int | string
//! ```
//!
//! A node without `: text` takes its first line as its own line. The
//! [`ParseHost`] reads the payloads: conditions, text and effects.
//! [`Plain`] reads string literals as text, the tokens of a condition as
//! the [default condition language](crate::condition) (or a string literal
//! with the condition), and `then custom key(args)` and
//! `then relation from -> to axis += n` as effects.
//!
//! [`parse_dialogue`] keeps the span of each part. [`Spanned::check`] runs
//! [`check`](crate::check()) and points each issue at its token.

use crate::ast::{Choice, Dialogue, Effect, Line, Node, Outcomes, Target, Text, Value};
use crate::check::{OutcomePlace, Place, check};
use crate::lower::Plain;
use proc_macro2::{Span, TokenStream, TokenTree};
use std::collections::HashMap;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitBool, LitInt, LitStr, Token, braced, bracketed};

/// How a host reads its payloads from tokens.
pub trait ParseHost {
    /// The condition payload.
    type Condition;
    /// The text payload.
    type Text;
    /// The effect payload.
    type Effect;

    /// Reads a condition from its tokens (up to `;` or `=>`, without it).
    /// `span` is the span of the first token.
    fn condition(&mut self, tokens: TokenStream, span: Span) -> syn::Result<Self::Condition>;

    /// Reads a text.
    fn text(&mut self, input: ParseStream<'_>) -> syn::Result<Self::Text>;

    /// Returns `true` if the next tokens are an effect of the host.
    fn peek_effect(&self, input: ParseStream<'_>) -> bool;

    /// Reads an effect, up to the `;` or `,` after it (without it).
    fn effect(&mut self, input: ParseStream<'_>) -> syn::Result<Self::Effect>;
}

impl ParseHost for Plain {
    type Condition = String;
    type Text = Text;
    type Effect = Effect;

    fn condition(&mut self, tokens: TokenStream, span: Span) -> syn::Result<String> {
        let source = condition_source(tokens);
        crate::condition::parse(&source)
            .map_err(|e| syn::Error::new(span, format!("in the condition `{source}`: {e}")))?;
        Ok(source)
    }

    fn text(&mut self, input: ParseStream<'_>) -> syn::Result<Text> {
        let lit: LitStr = input.parse()?;
        Ok(Text::Plain(lit.value()))
    }

    fn peek_effect(&self, input: ParseStream<'_>) -> bool {
        peek_word(input, "then")
    }

    fn effect(&mut self, input: ParseStream<'_>) -> syn::Result<Effect> {
        input.call(Ident::parse_any)?;
        let kind = input.call(Ident::parse_any)?;
        match kind.to_string().as_str() {
            "custom" => {
                let key = parse_key(input)?.0;
                let mut args = Vec::new();
                if input.peek(syn::token::Paren) {
                    let inner;
                    syn::parenthesized!(inner in input);
                    while !inner.is_empty() {
                        args.push(parse_value(&inner)?);
                        if !inner.is_empty() {
                            inner.parse::<Token![,]>()?;
                        }
                    }
                }
                Ok(Effect::Custom { key, args })
            }
            "relation" => {
                let from = parse_key(input)?.0;
                input.parse::<Token![->]>()?;
                let to = parse_key(input)?.0;
                let axis = parse_key(input)?.0;
                let delta = parse_delta(input)?;
                let delta = i32::try_from(delta)
                    .map_err(|_| syn::Error::new(kind.span(), "the number is too large"))?;
                Ok(Effect::Relation {
                    from,
                    to,
                    axis,
                    delta,
                })
            }
            other => Err(syn::Error::new(
                kind.span(),
                format!("unknown effect `then {other}`; use `then custom` or `then relation`"),
            )),
        }
    }
}

/// The text of a condition: the value of a single string literal, or the
/// tokens as text.
pub fn condition_source(tokens: TokenStream) -> String {
    let mut iter = tokens.clone().into_iter();
    if let (Some(TokenTree::Literal(lit)), None) = (iter.next(), iter.next())
        && let Ok(s) = syn::parse2::<LitStr>(TokenTree::Literal(lit).into())
    {
        return s.value();
    }
    tokens.to_string()
}

/// A dialogue with the span of each part.
#[derive(Debug, Clone)]
pub struct Spanned<C = String, X = Text, E = Effect, M = String> {
    /// The dialogue.
    pub dialogue: Dialogue<C, X, E, M>,
    /// The span of each place that the parser saw.
    pub spans: HashMap<Place, Span>,
    /// The span of the whole dialogue (its first token).
    pub span: Span,
}

impl<C, X, E, M> Spanned<C, X, E, M> {
    /// Returns the span of a place, or of the nearest place that has one.
    pub fn span(&self, place: &Place) -> Span {
        let fallback = match place {
            Place::NodeOutcome(n, _)
            | Place::NodeSpeaker(n)
            | Place::NodeWhen(n)
            | Place::NodeSay(n)
            | Place::Line(n, _)
            | Place::Next(n) => Some(Place::Node(*n)),
            Place::OptionOutcome(n, o, _)
            | Place::OptionId(n, o)
            | Place::OptionWhen(n, o)
            | Place::OptionGo(n, o) => Some(Place::Option(*n, *o)),
            _ => None,
        };
        self.spans
            .get(place)
            .or_else(|| fallback.and_then(|f| self.spans.get(&f)))
            .copied()
            .unwrap_or(self.span)
    }

    /// Runs [`check`](crate::check()) and returns every issue as one error,
    /// each at the span of its place. With `warnings_are_errors`, warnings
    /// count too (the `dialogue!` macro does this).
    pub fn check(&self, warnings_are_errors: bool) -> syn::Result<()> {
        let mut out: Option<syn::Error> = None;
        for issue in check(&self.dialogue) {
            if issue.severity == crate::Severity::Warning && !warnings_are_errors {
                continue;
            }
            // The YAML hint (`next: end`) is `-> end;` in the token form.
            let message = issue.message.replace("`next: end`", "`-> end;`");
            let e = syn::Error::new(self.span(&issue.place), message);
            match &mut out {
                Some(all) => all.combine(e),
                None => out = Some(e),
            }
        }
        out.map_or(Ok(()), Err)
    }
}

impl Parse for Spanned {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        parse_dialogue(input, &mut Plain)
    }
}

fn peek_word(input: ParseStream<'_>, word: &str) -> bool {
    input.fork().call(Ident::parse_any).is_ok_and(|i| i == word)
}

/// `true` if the next tokens are a key followed by one `:` (a line).
fn peek_line(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    parse_key(&fork).is_ok() && fork.peek(Token![:]) && !fork.peek(Token![::])
}

/// `true` if the next word is `word` and no `:` follows (so it is not a
/// line spoken by a character with that key).
fn peek_keyword(input: ParseStream<'_>, word: &str) -> bool {
    let fork = input.fork();
    fork.call(Ident::parse_any).is_ok_and(|i| i == word) && !fork.peek(Token![:])
}

/// Reads a key: `ident(.ident)*` or a string literal.
pub fn parse_key(input: ParseStream<'_>) -> syn::Result<(String, Span)> {
    if input.peek(LitStr) {
        let lit: LitStr = input.parse()?;
        return Ok((lit.value(), lit.span()));
    }
    let first = input
        .call(Ident::parse_any)
        .map_err(|e| syn::Error::new(e.span(), "expected a key (an identifier or a string)"))?;
    let span = first.span();
    let mut key = first.to_string();
    while input.peek(Token![.]) && !input.peek(Token![..]) {
        input.parse::<Token![.]>()?;
        key.push('.');
        key.push_str(&input.call(Ident::parse_any)?.to_string());
    }
    Ok((key, span))
}

fn parse_target(input: ParseStream<'_>) -> syn::Result<(Target, Span)> {
    let (key, span) = parse_key(input)?;
    Ok((Target::parse(&key), span))
}

fn parse_value(input: ParseStream<'_>) -> syn::Result<Value> {
    if input.peek(LitStr) {
        return Ok(Value::Text(input.parse::<LitStr>()?.value()));
    }
    if input.peek(LitBool) {
        return Ok(Value::Bool(input.parse::<LitBool>()?.value));
    }
    let negative = input.peek(Token![-]);
    if negative {
        input.parse::<Token![-]>()?;
    }
    let n: LitInt = input
        .parse()
        .map_err(|e| syn::Error::new(e.span(), "expected `true`, `false`, a number or a string"))?;
    let n: i64 = n.base10_parse()?;
    Ok(Value::Int(if negative { -n } else { n }))
}

fn parse_delta(input: ParseStream<'_>) -> syn::Result<i64> {
    let sign = if input.peek(Token![+=]) {
        input.parse::<Token![+=]>()?;
        1
    } else if input.peek(Token![-=]) {
        input.parse::<Token![-=]>()?;
        -1
    } else {
        return Err(input.error("expected `+=` or `-=`"));
    };
    let n: LitInt = input.parse()?;
    Ok(sign * n.base10_parse::<i64>()?)
}

/// Collects tokens up to (without) `;` or `=>`.
fn condition_tokens(input: ParseStream<'_>, until_arrow: bool) -> syn::Result<(TokenStream, Span)> {
    let span = input.span();
    let mut out = TokenStream::new();
    while !input.is_empty() && !input.peek(Token![;]) && !(until_arrow && input.peek(Token![=>])) {
        out.extend([input.parse::<TokenTree>()?]);
    }
    if out.is_empty() {
        return Err(syn::Error::new(span, "expected a condition"));
    }
    Ok((out, span))
}

type HostNode<P, M> =
    Node<<P as ParseHost>::Condition, <P as ParseHost>::Text, <P as ParseHost>::Effect, M>;

type HostChoice<P, M> =
    Choice<<P as ParseHost>::Condition, <P as ParseHost>::Text, <P as ParseHost>::Effect, M>;

/// What [`parse_dialogue`] reads with the parse host `P`, with notes of
/// the type `M`.
pub type Parsed<P, M = String> =
    Spanned<<P as ParseHost>::Condition, <P as ParseHost>::Text, <P as ParseHost>::Effect, M>;

struct Ctx<'a, P: ParseHost> {
    host: &'a mut P,
    spans: HashMap<Place, Span>,
}

impl<P: ParseHost> Ctx<'_, P> {
    /// Reads one outcome statement into `outcomes`. Returns `false` if the
    /// tokens are not an outcome.
    fn outcome(
        &mut self,
        input: ParseStream<'_>,
        outcomes: &mut Outcomes<P::Effect>,
        place: &dyn Fn(OutcomePlace) -> Place,
    ) -> syn::Result<bool> {
        if self.host.peek_effect(input) {
            loop {
                let span = input.span();
                self.spans
                    .insert(place(OutcomePlace::Then(Some(outcomes.then.len()))), span);
                self.spans
                    .entry(place(OutcomePlace::Then(None)))
                    .or_insert(span);
                outcomes.then.push(self.host.effect(input)?);
                if !input.peek(Token![,]) {
                    break;
                }
                input.parse::<Token![,]>()?;
            }
            return Ok(true);
        }
        let word = ["set", "add", "relation", "remember"]
            .into_iter()
            .find(|w| peek_keyword(input, w));
        let Some(word) = word else {
            return Ok(false);
        };
        input.call(Ident::parse_any)?;
        loop {
            let (key, span) = parse_key(input)?;
            match word {
                "set" => {
                    input.parse::<Token![=]>()?;
                    let value = parse_value(input)?;
                    self.spans
                        .insert(place(OutcomePlace::Set(key.clone())), span);
                    outcomes.set.insert(key, value);
                }
                "add" => {
                    let delta = parse_delta(input)?;
                    self.spans
                        .insert(place(OutcomePlace::Add(key.clone())), span);
                    outcomes.add.insert(key, delta);
                }
                "relation" => {
                    let delta = i32::try_from(parse_delta(input)?)
                        .map_err(|_| syn::Error::new(span, "the number is too large"))?;
                    self.spans
                        .insert(place(OutcomePlace::Relation(key.clone())), span);
                    outcomes.relation.insert(key, delta);
                }
                _ => {
                    self.spans
                        .insert(place(OutcomePlace::Remember(outcomes.remember.len())), span);
                    outcomes.remember.push(key);
                }
            }
            if !input.peek(Token![,]) {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(true)
    }

    fn node<M>(&mut self, input: ParseStream<'_>, n: usize) -> syn::Result<HostNode<P, M>> {
        let (key, key_span) = parse_key(input)?;
        self.spans.insert(Place::Node(n), key_span);
        let mut say = None;
        if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            self.spans.insert(Place::NodeSay(n), input.span());
            say = Some(self.host.text(input)?);
        }
        let mut speaker = None;
        let mut when = None;
        let mut lines: Vec<Line<P::Text>> = Vec::new();
        let mut next = None;
        let mut outcomes = Outcomes::default();
        let mut options = Vec::new();
        if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
        } else {
            let body;
            braced!(body in input);
            while !body.is_empty() {
                if body.peek(Token![->]) {
                    body.parse::<Token![->]>()?;
                    let (target, span) = parse_target(&body)?;
                    self.spans.insert(Place::Next(n), span);
                    next = Some(target);
                } else if peek_keyword(&body, "when") {
                    body.call(Ident::parse_any)?;
                    let (tokens, span) = condition_tokens(&body, false)?;
                    self.spans.insert(Place::NodeWhen(n), span);
                    when = Some(self.host.condition(tokens, span)?);
                } else if peek_keyword(&body, "speaker") {
                    body.call(Ident::parse_any)?;
                    let (who, span) = parse_key(&body)?;
                    self.spans.insert(Place::NodeSpeaker(n), span);
                    speaker = Some(who);
                } else if self.outcome(&body, &mut outcomes, &|o| Place::NodeOutcome(n, o))? {
                } else if peek_line(&body) {
                    let (who, span) = parse_key(&body)?;
                    body.parse::<Token![:]>()?;
                    let text = self.host.text(&body)?;
                    self.spans.insert(Place::Line(n, lines.len()), span);
                    lines.push(Line::new(who, text));
                } else {
                    let o = options.len();
                    options.push(self.option(&body, n, o)?);
                    continue;
                }
                body.parse::<Token![;]>()?;
            }
        }
        let say = match say {
            Some(say) => say,
            None if !lines.is_empty() => {
                let first = lines.remove(0);
                // Line indices move down by one.
                let mut moved = HashMap::new();
                for (place, span) in self.spans.drain() {
                    match place {
                        Place::Line(m, 0) if m == n => {
                            moved.insert(Place::NodeSpeaker(n), span);
                            moved.insert(Place::NodeSay(n), span);
                        }
                        Place::Line(m, l) if m == n => {
                            moved.insert(Place::Line(m, l - 1), span);
                        }
                        other => {
                            moved.insert(other, span);
                        }
                    }
                }
                self.spans = moved;
                speaker = speaker.or(Some(first.speaker));
                first.say
            }
            None => {
                return Err(syn::Error::new(
                    key_span,
                    format!(
                        "node `{key}` has no line; write `{key}: \"...\"`, or a line with \
                         its speaker key in the body, for example `mara: \"...\";`"
                    ),
                ));
            }
        };
        Ok(Node {
            key,
            speaker,
            when,
            say,
            lines,
            next,
            outcomes,
            options,
            notes: Vec::new(),
        })
    }

    fn option<M>(
        &mut self,
        input: ParseStream<'_>,
        n: usize,
        o: usize,
    ) -> syn::Result<HostChoice<P, M>> {
        self.spans.insert(Place::Option(n, o), input.span());
        let say = self.host.text(input)?;
        let mut when = None;
        if peek_keyword(input, "when") {
            input.call(Ident::parse_any)?;
            let (tokens, span) = condition_tokens(input, true)?;
            self.spans.insert(Place::OptionWhen(n, o), span);
            when = Some(self.host.condition(tokens, span)?);
        }
        input
            .parse::<Token![=>]>()
            .map_err(|e| syn::Error::new(e.span(), "expected `=> target` after the option"))?;
        let (go, span) = parse_target(input)?;
        self.spans.insert(Place::OptionGo(n, o), span);
        let mut choice = Choice::new(say, go);
        choice.when = when;
        if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
        } else {
            let body;
            braced!(body in input);
            while !body.is_empty() {
                if peek_keyword(&body, "id") {
                    body.call(Ident::parse_any)?;
                    let (id, span) = parse_key(&body)?;
                    self.spans.insert(Place::OptionId(n, o), span);
                    choice.id = Some(id);
                } else if !self.outcome(&body, &mut choice.outcomes, &|out| {
                    Place::OptionOutcome(n, o, out)
                })? {
                    return Err(body.error(
                        "expected `id`, `set`, `add`, `relation`, `remember` or an effect",
                    ));
                }
                body.parse::<Token![;]>()?;
            }
        }
        Ok(choice)
    }
}

/// Reads a dialogue in the token form with the payload parsers of `host`.
///
/// It reads the grammar only. Call [`Spanned::check`] for the checks on
/// keys and targets. The token form has no author notes: `M` is the note
/// type of the host, and the notes stay empty.
pub fn parse_dialogue<P: ParseHost, M>(
    input: ParseStream<'_>,
    host: &mut P,
) -> syn::Result<Parsed<P, M>> {
    let span = input.span();
    let mut id = None;
    if peek_word(input, "dialogue") {
        input.call(Ident::parse_any)?;
        if !peek_word(input, "with") {
            id = Some(parse_key(input)?.0);
        }
    }
    let with_word = input.call(Ident::parse_any)?;
    if with_word != "with" {
        return Err(syn::Error::new(
            with_word.span(),
            "expected `with <character>`",
        ));
    }
    let mut cx = Ctx {
        host,
        spans: HashMap::new(),
    };
    let (with, with_span) = parse_key(input)?;
    cx.spans.insert(Place::With, with_span);
    let start_word = input.call(Ident::parse_any)?;
    if start_word != "start" {
        return Err(syn::Error::new(
            start_word.span(),
            "expected `start [<node>, ...]`",
        ));
    }
    let mut start = Vec::new();
    if input.peek(syn::token::Bracket) {
        let list;
        bracketed!(list in input);
        while !list.is_empty() {
            let (key, span) = parse_key(&list)?;
            cx.spans.insert(Place::Start(start.len()), span);
            start.push(key);
            if !list.is_empty() {
                list.parse::<Token![,]>()?;
            }
        }
    } else {
        let (key, span) = parse_key(input)?;
        cx.spans.insert(Place::Start(0), span);
        start.push(key);
    }
    let body;
    braced!(body in input);
    let mut nodes = Vec::new();
    while !body.is_empty() {
        let n = nodes.len();
        nodes.push(cx.node(&body, n)?);
    }
    let mut dialogue = Dialogue::new(with);
    dialogue.id = id;
    dialogue.start = start;
    dialogue.nodes = nodes;
    Ok(Spanned {
        dialogue,
        spans: cx.spans,
        span,
    })
}
