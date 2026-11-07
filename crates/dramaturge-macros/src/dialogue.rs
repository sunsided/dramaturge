//! `dialogue!`: the token form of a dialogue, checked at compile time.
//!
//! The parser and the checks are in `dramaturge-syntax` (feature `syn`), so
//! host macros use the same grammar. This module only maps the errors to
//! spans and writes the code.

use dramaturge_syntax::tokens::Spanned;
use dramaturge_syntax::{
    Choice, Dialogue, Effect, LowerError, Node, Outcomes, Target, Text, Value, lower_plain,
};
use proc_macro_crate::FoundCrate;
use proc_macro2::{Span, TokenStream};
use quote::quote;

/// The path of `dramaturge-syntax` in the calling crate: the crate itself,
/// or `dramaturge::syntax` through the facade.
fn syntax_path() -> TokenStream {
    let ident = |name: &str| syn::Ident::new(name, Span::call_site());
    if let Ok(found) = proc_macro_crate::crate_name("dramaturge-syntax") {
        return match found {
            FoundCrate::Itself => quote!(crate),
            FoundCrate::Name(name) => {
                let name = ident(&name);
                quote!(::#name)
            }
        };
    }
    if let Ok(FoundCrate::Name(name)) = proc_macro_crate::crate_name("dramaturge") {
        let name = ident(&name);
        return quote!(::#name::syntax);
    }
    quote!(::dramaturge_syntax)
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let spanned: Spanned = syn::parse2(input)?;
    // A warning of `check` (a node without a way on) is an error here: the
    // macro can not emit warnings, and the fix is one token (`-> end;`).
    spanned.check(true)?;
    // Lower once at compile time. A condition that the default language can
    // not read fails here, at its tokens, and not at run time.
    if let Err(e) = lower_plain(&spanned.dialogue) {
        let (place, message) = match e {
            LowerError::Host { place, error, .. } => (Some(place), error.to_string()),
            other => (None, other.to_string()),
        };
        let span = place.map_or(spanned.span, |p| spanned.span(&p));
        return Err(syn::Error::new(span, message));
    }
    let p = syntax_path();
    let dialogue = dialogue_tokens(&p, &spanned.dialogue);
    Ok(quote! {
        {
            let dialogue: #p::Dialogue = #dialogue;
            #p::lower_plain(&dialogue)
                .expect("dialogue! checked this dialogue at compile time")
        }
    })
}

fn string(s: &str) -> TokenStream {
    quote!(::std::string::String::from(#s))
}

fn dialogue_tokens(p: &TokenStream, d: &Dialogue) -> TokenStream {
    let with = &d.with;
    let mut out = quote!(#p::Dialogue::new(#with));
    if let Some(id) = &d.id {
        out.extend(quote!(.with_id(#id)));
    }
    for key in &d.start {
        out.extend(quote!(.with_start(#key)));
    }
    for node in &d.nodes {
        let node = node_tokens(p, node);
        out.extend(quote!(.with_node(#node)));
    }
    out
}

fn text(p: &TokenStream, t: &Text) -> TokenStream {
    match t {
        Text::Plain(s) => {
            let s = string(s);
            quote!(#p::Text::Plain(#s))
        }
        // The token form reads string literals only.
        _ => quote!(::core::compile_error!(
            "dialogue!: fragment text is not supported"
        )),
    }
}

fn target(p: &TokenStream, t: &Target) -> TokenStream {
    match t {
        Target::Node(key) => {
            let key = string(key);
            quote!(#p::Target::Node(#key))
        }
        Target::Back => quote!(#p::Target::Back),
        _ => quote!(#p::Target::End),
    }
}

fn value(p: &TokenStream, v: &Value) -> TokenStream {
    match v {
        Value::Bool(b) => quote!(#p::Value::Bool(#b)),
        Value::Int(n) => quote!(#p::Value::Int(#n)),
        Value::Text(s) => {
            let s = string(s);
            quote!(#p::Value::Text(#s))
        }
        _ => quote!(::core::compile_error!("dialogue!: unknown value")),
    }
}

fn outcomes(p: &TokenStream, o: &Outcomes) -> TokenStream {
    let mut out = quote!(#p::Outcomes::default());
    for (name, v) in &o.set {
        let v = value(p, v);
        out.extend(quote!(.with_set(#name, #v)));
    }
    for (name, delta) in &o.add {
        out.extend(quote!(.with_add(#name, #delta)));
    }
    for (axis, delta) in &o.relation {
        out.extend(quote!(.with_relation(#axis, #delta)));
    }
    for key in &o.remember {
        out.extend(quote!(.with_remember(#key)));
    }
    for effect in &o.then {
        let effect = match effect {
            Effect::Relation {
                from,
                to,
                axis,
                delta,
            } => {
                let (from, to, axis) = (string(from), string(to), string(axis));
                quote!(#p::Effect::Relation { from: #from, to: #to, axis: #axis, delta: #delta })
            }
            Effect::Custom { key, args } => {
                let key = string(key);
                let args = args.iter().map(|a| value(p, a));
                quote!(#p::Effect::Custom { key: #key, args: ::std::vec![#(#args),*] })
            }
            _ => quote!(::core::compile_error!("dialogue!: unknown effect")),
        };
        out.extend(quote!(.with_then(#effect)));
    }
    out
}

fn node_tokens(p: &TokenStream, n: &Node) -> TokenStream {
    let key = &n.key;
    let say = text(p, &n.say);
    let mut out = quote!(#p::Node::new(#key, #say));
    if let Some(speaker) = &n.speaker {
        out.extend(quote!(.with_speaker(#speaker)));
    }
    if let Some(when) = &n.when {
        let when = string(when);
        out.extend(quote!(.with_when(#when)));
    }
    for line in &n.lines {
        let speaker = &line.speaker;
        let say = text(p, &line.say);
        out.extend(quote!(.with_line(#speaker, #say)));
    }
    if let Some(next) = &n.next {
        let next = target(p, next);
        out.extend(quote!(.with_next(#next)));
    }
    if !n.outcomes.is_empty() {
        let o = outcomes(p, &n.outcomes);
        out.extend(quote!(.with_outcomes(#o)));
    }
    for choice in &n.options {
        let choice = choice_tokens(p, choice);
        out.extend(quote!(.with_option(#choice)));
    }
    out
}

fn choice_tokens(p: &TokenStream, c: &Choice) -> TokenStream {
    let go = target(p, &c.go);
    let say = c.say.as_ref().map_or_else(
        || quote!(#p::Text::Plain(::std::string::String::new())),
        |s| text(p, s),
    );
    let mut out = quote!(#p::Choice::new(#say, #go));
    if let Some(id) = &c.id {
        out.extend(quote!(.with_id(#id)));
    }
    if let Some(when) = &c.when {
        let when = string(when);
        out.extend(quote!(.with_when(#when)));
    }
    if !c.outcomes.is_empty() {
        let o = outcomes(p, &c.outcomes);
        out.extend(quote!(.with_outcomes(#o)));
    }
    out
}
