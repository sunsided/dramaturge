//! Lowering: from the AST to a model [`DialogueGraph`].
//!
//! [`lower`] walks the AST and builds the graph. It asks the host, through
//! the [`Lower`] trait, for each payload: conditions, text and `then`
//! effects. `set`, `add`, `relation` and `remember` have default
//! implementations; a host can replace them (for example to check that a
//! name exists).
//!
//! [`Plain`] is the host for plain dramaturge: the
//! [default condition language](crate::condition), [`Text`] and [`Effect`].

use crate::ast::{Dialogue, Effect, Fragment, FragmentMode, Hint, Target, Text, Value};
use crate::check::{Issue, OutcomePlace, Place, Severity, check};
use crate::condition::{self, ParseError};
use dramaturge_model::{
    CharacterId, Condition, CustomDialogTag, DialogLine, DialogTarget, DialogText, DialogueGraph,
    DialogueNode, DialogueOption, EntityId, Fact, FactId, FactValue, Intention, Memory,
    NavigationHint, Outcome, RelationshipAxisId, SymbolTable, TextFragment,
};
use std::fmt;
use std::sync::Arc;

/// The context of one lowering step.
#[non_exhaustive]
pub struct Cx<'a> {
    /// The symbol table. Node, option, entity and memory keys go in here.
    pub symbols: &'a mut SymbolTable,
    /// The player ([`SymbolTable::player`]).
    pub player: EntityId,
    /// The `with` character of the dialogue.
    pub with: EntityId,
    /// The speaker of the current node (`with`, or the node's `speaker`).
    /// For a line, the speaker of the line.
    pub speaker: EntityId,
    /// Where in the AST the step is.
    pub place: Place,
}

impl Cx<'_> {
    /// Returns the entity id of a character key: `player` is the player,
    /// other keys are interned.
    pub fn entity(&mut self, key: &str) -> EntityId {
        condition::entity(self.symbols, key)
    }
}

/// The hooks of a host: how its payloads become model values.
///
/// `C`, `X` and `E` are the condition, text and effect payloads of the
/// [`Dialogue`]. [`Host`](Self::Host) and [`Text`](Self::Text) are the `H`
/// and `T` of the [`DialogueGraph<H, T>`] that [`lower`] makes.
pub trait Lower<C, X, E> {
    /// The host outcome type (`Outcome::Host`).
    type Host;
    /// The host text type (`DialogText::Host`).
    type Text;
    /// The error of a hook.
    type Error;

    /// Lowers a `when`.
    fn condition(&mut self, cx: &mut Cx<'_>, source: &C) -> Result<Condition, Self::Error>;

    /// Lowers a `say`.
    fn text(&mut self, cx: &mut Cx<'_>, source: &X) -> Result<DialogText<Self::Text>, Self::Error>;

    /// Lowers the `then` list of a node or an option. `cx.place` is the
    /// `then` list. The result goes after the other outcomes.
    fn effects(
        &mut self,
        cx: &mut Cx<'_>,
        then: &[E],
    ) -> Result<Vec<Outcome<Self::Host>>, Self::Error>;

    /// Lowers one `set` entry. The default sets the fact in the world
    /// scope, or in the player scope for a `player.` prefix.
    fn set(
        &mut self,
        cx: &mut Cx<'_>,
        name: &str,
        value: &Value,
    ) -> Result<Vec<Outcome<Self::Host>>, Self::Error> {
        let _ = cx;
        let fact = condition::Fact::parse(name);
        Ok(vec![Outcome::SetFact {
            scope: fact.scope.to_model(),
            fact: Fact::new(fact.name.as_str(), fact_value(value)),
        }])
    }

    /// Lowers one `add` entry. The default adds to the fact, in the scope
    /// of its prefix.
    fn add(
        &mut self,
        cx: &mut Cx<'_>,
        name: &str,
        delta: i64,
    ) -> Result<Vec<Outcome<Self::Host>>, Self::Error> {
        let _ = cx;
        let fact = condition::Fact::parse(name);
        Ok(vec![Outcome::AddFact {
            scope: fact.scope.to_model(),
            fact_id: FactId::new(fact.name.as_str()),
            delta,
        }])
    }

    /// Lowers one `relation` entry. The default adds to the relation of the
    /// player to the speaker of the node.
    fn relation(
        &mut self,
        cx: &mut Cx<'_>,
        axis: &str,
        delta: i32,
    ) -> Result<Vec<Outcome<Self::Host>>, Self::Error> {
        Ok(vec![Outcome::AddRelationship {
            source: cx.player,
            target: cx.speaker,
            axis: RelationshipAxisId::new(axis),
            delta,
        }])
    }

    /// Lowers one `remember` entry. The default gives the speaker of the
    /// node the memory (interned by key), with the player as subject.
    fn remember(
        &mut self,
        cx: &mut Cx<'_>,
        key: &str,
    ) -> Result<Vec<Outcome<Self::Host>>, Self::Error> {
        let id = cx.symbols.intern_memory(key);
        Ok(vec![Outcome::AddMemory {
            owner: cx.speaker,
            memory: Memory::new(id).with_subject(cx.player),
        }])
    }

    /// Returns the entity of a character key (`with`, `speaker`). The
    /// default is [`Cx::entity`].
    fn entity(&mut self, cx: &mut Cx<'_>, key: &str) -> Result<EntityId, Self::Error> {
        Ok(cx.entity(key))
    }
}

/// The value of a `set` entry as a model value.
pub fn fact_value(value: &Value) -> FactValue {
    match value {
        Value::Bool(b) => FactValue::Bool(*b),
        Value::Int(n) => FactValue::Int(*n),
        Value::Text(s) => FactValue::Text(s.clone()),
    }
}

/// A lowered dialogue.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Lowered<H = (), T = ()> {
    /// The graph.
    pub graph: DialogueGraph<H, T>,
    /// The warnings of [`check`].
    pub warnings: Vec<Issue>,
}

/// A dialogue lowered with [`Plain`], with its own symbol table.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Loaded {
    /// The graph.
    pub graph: DialogueGraph,
    /// The keys of every entity, node, option and memory in the graph.
    pub symbols: SymbolTable,
    /// The warnings of [`check`].
    pub warnings: Vec<Issue>,
}

/// An error of [`lower`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LowerError<E> {
    /// [`check`] found errors. The list has errors and warnings.
    Check(Vec<Issue>),
    /// A hook failed.
    Host {
        /// Where.
        place: Place,
        /// The path of the place, for example `nodes.meet.when`.
        path: String,
        /// The hook's error.
        error: E,
    },
}

impl<E: fmt::Display> fmt::Display for LowerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LowerError::Check(issues) => {
                let errors: Vec<String> = issues
                    .iter()
                    .filter(|i| i.severity == Severity::Error)
                    .map(ToString::to_string)
                    .collect();
                f.write_str(&errors.join("; "))
            }
            LowerError::Host { path, error, .. } if path.is_empty() => write!(f, "{error}"),
            LowerError::Host { path, error, .. } => write!(f, "{path}: {error}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for LowerError<E> {}

/// Builds the symbol key of a node or option: `<id>/<key>` when the
/// dialogue has an id.
fn scoped(id: Option<&str>, key: &str) -> String {
    match id {
        Some(id) => format!("{id}/{key}"),
        None => key.to_string(),
    }
}

/// The result of [`lower`] for the host `L`.
pub type LowerResult<C, X, E, L> = Result<
    Lowered<<L as Lower<C, X, E>>::Host, <L as Lower<C, X, E>>::Text>,
    LowerError<<L as Lower<C, X, E>>::Error>,
>;

/// Lowers a dialogue to a graph.
///
/// First runs [`check`]: an error stops the lowering with
/// [`LowerError::Check`]; warnings go into [`Lowered::warnings`]. Then
/// interns the keys in `symbols`:
///
/// - Nodes: the node key (or `<id>/<key>`). Start nodes first, in order.
/// - Options: the option `id`, or `<node>#<index>` (or with the `<id>/`
///   prefix).
/// - Characters: `with` and the speakers through [`Lower::entity`].
///
/// The graph's entry nodes are the start nodes. A choice without `say`
/// gets the default text of its hint. `back` lowers to
/// `DialogTarget::Return`.
pub fn lower<C, X, E, L: Lower<C, X, E>>(
    dialogue: &Dialogue<C, X, E>,
    symbols: &mut SymbolTable,
    host: &mut L,
) -> LowerResult<C, X, E, L> {
    let issues = check(dialogue);
    if issues.iter().any(|i| i.severity == Severity::Error) {
        return Err(LowerError::Check(issues));
    }
    let id = dialogue.id.as_deref();
    let fail = |place: Place, error| LowerError::Host {
        path: place.path(dialogue),
        place,
        error,
    };

    let player = symbols.player();
    let mut cx = Cx {
        symbols,
        player,
        with: player,
        speaker: player,
        place: Place::With,
    };
    let with = host
        .entity(&mut cx, &dialogue.with)
        .map_err(|e| fail(Place::With, e))?;
    cx.with = with;

    let mut graph = DialogueGraph::<L::Host, L::Text>::typed();
    for key in &dialogue.start {
        graph
            .entry_nodes
            .push(cx.symbols.intern_node(&scoped(id, key)));
    }
    let target = |cx: &mut Cx<'_>, target: &Target| match target {
        Target::Node(key) => DialogTarget::Node(cx.symbols.intern_node(&scoped(id, key))),
        Target::End => DialogTarget::End,
        Target::Back => DialogTarget::Return,
    };

    for (n, source) in dialogue.nodes.iter().enumerate() {
        let node_id = cx.symbols.intern_node(&scoped(id, &source.key));
        cx.speaker = match &source.speaker {
            Some(key) => {
                cx.place = Place::NodeSpeaker(n);
                host.entity(&mut cx, key)
                    .map_err(|e| fail(Place::NodeSpeaker(n), e))?
            }
            None => with,
        };
        let speaker = cx.speaker;
        cx.place = Place::NodeSay(n);
        let text = host
            .text(&mut cx, &source.say)
            .map_err(|e| fail(Place::NodeSay(n), e))?;
        let mut node = DialogueNode::typed(node_id.0, CharacterId::new(speaker), text);

        for (l, line) in source.lines.iter().enumerate() {
            cx.place = Place::Line(n, l);
            let who = host
                .entity(&mut cx, &line.speaker)
                .map_err(|e| fail(Place::Line(n, l), e))?;
            cx.speaker = who;
            let text = host
                .text(&mut cx, &line.say)
                .map_err(|e| fail(Place::Line(n, l), e))?;
            cx.speaker = speaker;
            node.lines
                .push(DialogLine::new(CharacterId::new(who), text));
        }
        node.next = source.next.as_ref().map(|t| target(&mut cx, t));

        if let Some(when) = &source.when {
            cx.place = Place::NodeWhen(n);
            node.conditions.push(
                host.condition(&mut cx, when)
                    .map_err(|e| fail(Place::NodeWhen(n), e))?,
            );
        }
        node.outcomes_on_enter = outcomes(host, &mut cx, &source.outcomes, &|o| {
            Place::NodeOutcome(n, o)
        })
        .map_err(|(place, e)| fail(place, e))?;

        for (o, choice) in source.options.iter().enumerate() {
            let key = match &choice.id {
                Some(key) => scoped(id, key),
                None => scoped(id, &format!("{}#{o}", source.key)),
            };
            let option_id = cx.symbols.intern_option(&key);
            let hint = choice.hint.as_ref().map(hint_to_model);
            let text = match &choice.say {
                Some(say) => {
                    cx.place = Place::Option(n, o);
                    host.text(&mut cx, say)
                        .map_err(|e| fail(Place::Option(n, o), e))?
                }
                None => DialogText::Inline(
                    hint.as_ref()
                        .map(NavigationHint::default_text)
                        .unwrap_or_default(),
                ),
            };
            let to = target(&mut cx, &choice.go);
            let mut option = DialogueOption::typed(option_id.0, text, to, node_id);
            option.navigation_hint = hint;
            for tag in &choice.tags {
                let mut t = CustomDialogTag::new(tag.axis.as_str(), tag.key.as_str());
                t.display_prefix = tag.prefix.as_deref().map(Arc::from);
                t.requires_trait = tag.requires_trait.unwrap_or(tag.axis == "skill");
                option.tags.push(t);
            }
            for i in &choice.intentions.explicit {
                option
                    .intentions
                    .explicit
                    .push(Intention::Custom(Arc::from(i.as_str())));
            }
            for i in &choice.intentions.implicit {
                option
                    .intentions
                    .implicit
                    .push(Intention::Custom(Arc::from(i.as_str())));
            }
            if let Some(when) = &choice.when {
                cx.place = Place::OptionWhen(n, o);
                option.conditions.push(
                    host.condition(&mut cx, when)
                        .map_err(|e| fail(Place::OptionWhen(n, o), e))?,
                );
            }
            option.outcomes = outcomes(host, &mut cx, &choice.outcomes, &|out| {
                Place::OptionOutcome(n, o, out)
            })
            .map_err(|(place, e)| fail(place, e))?;
            node.options.push(option);
        }
        graph.nodes.push(node);
    }

    Ok(Lowered {
        graph,
        warnings: issues,
    })
}

type Outcomes<H> = Vec<Outcome<H>>;

fn outcomes<C, X, E, L: Lower<C, X, E>>(
    host: &mut L,
    cx: &mut Cx<'_>,
    source: &crate::ast::Outcomes<E>,
    place: &dyn Fn(OutcomePlace) -> Place,
) -> Result<Outcomes<L::Host>, (Place, L::Error)> {
    let mut out = Vec::new();
    for (name, value) in &source.set {
        let at = place(OutcomePlace::Set(name.clone()));
        cx.place = at.clone();
        out.extend(host.set(cx, name, value).map_err(|e| (at, e))?);
    }
    for (name, delta) in &source.add {
        let at = place(OutcomePlace::Add(name.clone()));
        cx.place = at.clone();
        out.extend(host.add(cx, name, *delta).map_err(|e| (at, e))?);
    }
    for (axis, delta) in &source.relation {
        let at = place(OutcomePlace::Relation(axis.clone()));
        cx.place = at.clone();
        out.extend(host.relation(cx, axis, *delta).map_err(|e| (at, e))?);
    }
    for (i, key) in source.remember.iter().enumerate() {
        let at = place(OutcomePlace::Remember(i));
        cx.place = at.clone();
        out.extend(host.remember(cx, key).map_err(|e| (at, e))?);
    }
    if !source.then.is_empty() {
        let at = place(OutcomePlace::Then(None));
        cx.place = at.clone();
        out.extend(host.effects(cx, &source.then).map_err(|e| (at, e))?);
    }
    Ok(out)
}

fn hint_to_model(hint: &Hint) -> NavigationHint {
    match hint {
        Hint::Continue => NavigationHint::Continue,
        Hint::Leave => NavigationHint::Leave,
        Hint::GoTo(to) => NavigationHint::GoTo(to.clone()),
        Hint::End => NavigationHint::End,
        Hint::Return => NavigationHint::Return,
        Hint::Custom(label) => NavigationHint::Custom(label.clone()),
    }
}

/// The host for plain dramaturge.
///
/// - Conditions: [`String`] in the [default condition
///   language](crate::condition).
/// - Text: [`Text`], lowered to `DialogText::Inline` or
///   `DialogText::Fragments` (fragment `when` in the default condition
///   language).
/// - Effects: [`Effect`], lowered to `AddRelationship` or `Custom`.
///
/// The graph has no host outcomes and no host text: `DialogueGraph<(), ()>`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Plain;

impl Plain {
    /// Lowers a [`Text`] in the default form. Hosts with their own
    /// conditions but the default text can call it.
    pub fn text_with<T>(
        cx: &mut Cx<'_>,
        text: &Text,
        condition: &mut dyn FnMut(&mut Cx<'_>, &str) -> Result<Condition, ParseError>,
    ) -> Result<DialogText<T>, ParseError> {
        match text {
            Text::Plain(s) => Ok(DialogText::Inline(s.clone())),
            Text::Fragments(fragments) => fragments
                .iter()
                .map(|f| fragment(cx, f, condition))
                .collect::<Result<_, _>>()
                .map(DialogText::Fragments),
        }
    }
}

fn fragment<T>(
    cx: &mut Cx<'_>,
    f: &Fragment,
    condition: &mut dyn FnMut(&mut Cx<'_>, &str) -> Result<Condition, ParseError>,
) -> Result<TextFragment<T>, ParseError> {
    if f.variants.is_empty() {
        return Err(ParseError {
            message: "a fragment needs `text` or `variants`".into(),
            range: 0..0,
        });
    }
    let variants = f
        .variants
        .iter()
        .map(|v| Plain::text_with(cx, v, condition))
        .collect::<Result<Vec<_>, _>>()?;
    let mode = match f.mode {
        FragmentMode::Always => dramaturge_model::FragmentMode::Always,
        FragmentMode::FirstTime => dramaturge_model::FragmentMode::FirstTime,
        FragmentMode::Once => dramaturge_model::FragmentMode::Once,
        FragmentMode::Cycle => dramaturge_model::FragmentMode::Cycle,
        FragmentMode::Shuffle => dramaturge_model::FragmentMode::Shuffle,
        FragmentMode::Sticky => dramaturge_model::FragmentMode::Sticky,
    };
    let mut out = TextFragment::with_mode(mode, variants);
    out.key = f.key.as_deref().map(Arc::from);
    out.replaces = f.replaces;
    if let Some(when) = &f.when {
        out.condition = Some(condition(cx, when)?);
    }
    Ok(out)
}

impl Lower<String, Text, Effect> for Plain {
    type Host = ();
    type Text = ();
    type Error = ParseError;

    fn condition(&mut self, cx: &mut Cx<'_>, source: &String) -> Result<Condition, ParseError> {
        condition::lower_str(source, cx)
    }

    fn text(&mut self, cx: &mut Cx<'_>, source: &Text) -> Result<DialogText, ParseError> {
        Plain::text_with(cx, source, &mut |cx, s| condition::lower_str(s, cx))
    }

    fn effects(&mut self, cx: &mut Cx<'_>, then: &[Effect]) -> Result<Vec<Outcome>, ParseError> {
        Ok(then
            .iter()
            .map(|e| match e {
                Effect::Relation {
                    from,
                    to,
                    axis,
                    delta,
                } => Outcome::AddRelationship {
                    source: cx.entity(from),
                    target: cx.entity(to),
                    axis: RelationshipAxisId::new(axis.as_str()),
                    delta: *delta,
                },
                Effect::Custom { key, args } => Outcome::Custom {
                    key: key.clone(),
                    args: args.iter().map(fact_value).collect(),
                },
            })
            .collect())
    }
}

/// Lowers a plain dialogue into a new symbol table.
pub fn lower_plain(dialogue: &Dialogue) -> Result<Loaded, LowerError<ParseError>> {
    let mut symbols = SymbolTable::new();
    let lowered = lower(dialogue, &mut symbols, &mut Plain)?;
    Ok(Loaded {
        graph: lowered.graph,
        symbols,
        warnings: lowered.warnings,
    })
}
