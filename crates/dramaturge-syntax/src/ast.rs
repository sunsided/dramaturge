//! The dialogue AST.
//!
//! One [`Dialogue`] is a talk with one character (`with`). It has a list of
//! start nodes and a list of [`Node`]s in source order. Each node has a line
//! (`say`), more [`Line`]s with their own speakers, a `next` target, the
//! [`Outcomes`] that apply on enter, and the player's [`Choice`]s.
//!
//! The AST is generic over three host payloads:
//!
//! - `C`: a condition (`when`). Default: [`String`], in the
//!   [default condition language](crate::condition).
//! - `X`: a text (`say`). Default: [`Text`], a string or a list of
//!   fragments.
//! - `E`: a world effect (`then`). Default: [`Effect`].
//!
//! The parsers ([serde](crate#serde) and [`tokens`](crate#tokens)) read
//! the payloads as they are. [`lower`](crate::lower()) turns them into model
//! values through the [`Lower`](crate::Lower) hooks of the host.

use std::collections::BTreeMap;

/// A dialogue: a talk with one character.
///
/// `C`, `X` and `E` are the host's condition, text and effect payloads.
/// See the [module documentation](self).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Dialogue<C = String, X = Text, E = Effect> {
    /// An optional id. When it is set, every node and option key is
    /// interned as `<id>/<key>`, so the keys of many dialogues can share one
    /// `SymbolTable`.
    pub id: Option<String>,
    /// The key of the character that the player talks to. Nodes without
    /// their own `speaker` are spoken by this character.
    pub with: String,
    /// The start nodes, in order. The talk starts at the first one whose
    /// `when` holds. These become the entry nodes of the graph.
    pub start: Vec<String>,
    /// The nodes, in source order.
    pub nodes: Vec<Node<C, X, E>>,
}

impl<C, X, E> Dialogue<C, X, E> {
    /// Creates a dialogue with `with`, no start nodes and no nodes.
    pub fn new(with: impl Into<String>) -> Self {
        Self {
            id: None,
            with: with.into(),
            start: Vec::new(),
            nodes: Vec::new(),
        }
    }

    /// Sets the [`id`](Self::id) and returns the dialogue.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Adds a start node key and returns the dialogue.
    pub fn with_start(mut self, key: impl Into<String>) -> Self {
        self.start.push(key.into());
        self
    }

    /// Adds a node and returns the dialogue.
    pub fn with_node(mut self, node: Node<C, X, E>) -> Self {
        self.nodes.push(node);
        self
    }

    /// Returns the node with the given key.
    pub fn node(&self, key: &str) -> Option<&Node<C, X, E>> {
        self.nodes.iter().find(|n| n.key == key)
    }
}

/// One line of a character, the lines after it, and the player's choices.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Node<C = String, X = Text, E = Effect> {
    /// The key of the node. It is unique in the dialogue. `end` and `back`
    /// are reserved.
    pub key: String,
    /// The character that says the line. `None` means the `with` character
    /// of the dialogue.
    pub speaker: Option<String>,
    /// The node can start the talk, or be entered, only when this holds.
    pub when: Option<C>,
    /// The line.
    pub say: X,
    /// More lines after `say`, each with its own speaker (auto-continue).
    pub lines: Vec<Line<X>>,
    /// Where the talk goes on without a choice, when no option is
    /// available (auto-continue).
    pub next: Option<Target>,
    /// What changes when the talk enters the node.
    pub outcomes: Outcomes<E>,
    /// The player's choices.
    pub options: Vec<Choice<C, X, E>>,
}

impl<C, X, E> Node<C, X, E> {
    /// Creates a node with the given key and line, and nothing else.
    pub fn new(key: impl Into<String>, say: X) -> Self {
        Self {
            key: key.into(),
            speaker: None,
            when: None,
            say,
            lines: Vec::new(),
            next: None,
            outcomes: Outcomes::default(),
            options: Vec::new(),
        }
    }

    /// Sets the [`speaker`](Self::speaker) and returns the node.
    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.speaker = Some(speaker.into());
        self
    }

    /// Sets the condition and returns the node.
    pub fn with_when(mut self, when: C) -> Self {
        self.when = Some(when);
        self
    }

    /// Adds a line and returns the node.
    pub fn with_line(mut self, speaker: impl Into<String>, say: X) -> Self {
        self.lines.push(Line::new(speaker, say));
        self
    }

    /// Sets the [`next`](Self::next) target and returns the node.
    pub fn with_next(mut self, next: Target) -> Self {
        self.next = Some(next);
        self
    }

    /// Sets the outcomes on enter and returns the node.
    pub fn with_outcomes(mut self, outcomes: Outcomes<E>) -> Self {
        self.outcomes = outcomes;
        self
    }

    /// Adds a choice and returns the node.
    pub fn with_option(mut self, option: Choice<C, X, E>) -> Self {
        self.options.push(option);
        self
    }
}

/// One more line of a node, with its own speaker.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Line<X = Text> {
    /// The character that says the line. The key `player` is the player.
    pub speaker: String,
    /// The line.
    pub say: X,
}

impl<X> Line<X> {
    /// Creates a line.
    pub fn new(speaker: impl Into<String>, say: X) -> Self {
        Self {
            speaker: speaker.into(),
            say,
        }
    }
}

/// A choice of the player: a dialogue option.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Choice<C = String, X = Text, E = Effect> {
    /// An optional key. Without it, the key is `<node>#<index>`.
    pub id: Option<String>,
    /// The player's line, also the label of the choice. Can be empty only
    /// when the choice has a [`hint`](Self::hint): the hint then gives the
    /// label.
    pub say: Option<X>,
    /// The choice shows only when this holds.
    pub when: Option<C>,
    /// Where the choice leads. The default is [`Target::End`].
    pub go: Target,
    /// What changes when the player takes the choice.
    pub outcomes: Outcomes<E>,
    /// Tags of the choice (`DialogueOption::tags`).
    pub tags: Vec<Tag>,
    /// Intentions of the choice (`DialogueOption::intentions`).
    pub intentions: Intentions,
    /// A navigation hint (`DialogueOption::navigation_hint`).
    pub hint: Option<Hint>,
}

impl<C, X, E> Choice<C, X, E> {
    /// Creates a choice with the given line and target.
    pub fn new(say: X, go: Target) -> Self {
        Self {
            id: None,
            say: Some(say),
            when: None,
            go,
            outcomes: Outcomes::default(),
            tags: Vec::new(),
            intentions: Intentions::default(),
            hint: None,
        }
    }

    /// Creates a choice without a line. Its label comes from the hint.
    pub fn hinted(hint: Hint, go: Target) -> Self {
        Self {
            id: None,
            say: None,
            when: None,
            go,
            outcomes: Outcomes::default(),
            tags: Vec::new(),
            intentions: Intentions::default(),
            hint: Some(hint),
        }
    }

    /// Sets the key and returns the choice.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the condition and returns the choice.
    pub fn with_when(mut self, when: C) -> Self {
        self.when = Some(when);
        self
    }

    /// Sets the outcomes and returns the choice.
    pub fn with_outcomes(mut self, outcomes: Outcomes<E>) -> Self {
        self.outcomes = outcomes;
        self
    }

    /// Adds a tag and returns the choice.
    pub fn with_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Sets the hint and returns the choice.
    pub fn with_hint(mut self, hint: Hint) -> Self {
        self.hint = Some(hint);
        self
    }
}

/// Where a choice or a `next` leads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Target {
    /// A node of the same dialogue, by key.
    Node(String),
    /// The end of the talk (`end`).
    #[default]
    End,
    /// The node before (`back`): `DialogTarget::Return`.
    Back,
}

impl Target {
    /// Reads `end`, `back` or a node key.
    pub fn parse(key: &str) -> Self {
        match key {
            "end" => Target::End,
            "back" => Target::Back,
            key => Target::Node(key.to_string()),
        }
    }

    /// Returns the key as it is written: `end`, `back` or the node key.
    pub fn as_key(&self) -> &str {
        match self {
            Target::Node(key) => key,
            Target::End => "end",
            Target::Back => "back",
        }
    }
}

/// What a node (on enter) or a choice (when taken) changes.
///
/// The lowering applies the fields in this order: `set`, `add`, `relation`,
/// `remember`, `then`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Outcomes<E = Effect> {
    /// Set facts: `{ met_mara: true }`. The default lowering reads a
    /// `player.` prefix as the player scope, and all other names as the
    /// world scope.
    pub set: BTreeMap<String, Value>,
    /// Add to integer facts: `{ visits: 1 }`.
    pub add: BTreeMap<String, i64>,
    /// Add to the relation of the player to the speaker of the node:
    /// `{ trust: 1 }`.
    pub relation: BTreeMap<String, i32>,
    /// Memories that the speaker of the node keeps from now on.
    pub remember: Vec<String>,
    /// World effects of the host.
    pub then: Vec<E>,
}

impl<E> Default for Outcomes<E> {
    fn default() -> Self {
        Self {
            set: BTreeMap::new(),
            add: BTreeMap::new(),
            relation: BTreeMap::new(),
            remember: Vec::new(),
            then: Vec::new(),
        }
    }
}

impl<E> Outcomes<E> {
    /// Returns `true` when nothing changes.
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
            && self.add.is_empty()
            && self.relation.is_empty()
            && self.remember.is_empty()
            && self.then.is_empty()
    }

    /// Adds a `set` entry and returns the outcomes.
    pub fn with_set(mut self, name: impl Into<String>, value: Value) -> Self {
        self.set.insert(name.into(), value);
        self
    }

    /// Adds an `add` entry and returns the outcomes.
    pub fn with_add(mut self, name: impl Into<String>, delta: i64) -> Self {
        self.add.insert(name.into(), delta);
        self
    }

    /// Adds a `relation` entry and returns the outcomes.
    pub fn with_relation(mut self, axis: impl Into<String>, delta: i32) -> Self {
        self.relation.insert(axis.into(), delta);
        self
    }

    /// Adds a `remember` entry and returns the outcomes.
    pub fn with_remember(mut self, key: impl Into<String>) -> Self {
        self.remember.push(key.into());
        self
    }

    /// Adds a `then` effect and returns the outcomes.
    pub fn with_then(mut self, effect: E) -> Self {
        self.then.push(effect);
        self
    }
}

/// A value in `set`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Value {
    /// `true` or `false`.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A string.
    Text(String),
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

/// The default text payload: a string, or a list of fragments.
///
/// ```yaml
/// say:
///   - "Mara looks up."
///   - when: trust >= 3
///     text: "She smiles."
///   - mode: cycle
///     variants: ["\"Again?\"", "\"You.\""]
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Text {
    /// Plain text (`DialogText::Inline`).
    Plain(String),
    /// A list of fragments (`DialogText::Fragments`).
    Fragments(Vec<Fragment>),
}

impl Text {
    /// Returns `true` for an empty string or an empty fragment list.
    pub fn is_empty(&self) -> bool {
        match self {
            Text::Plain(s) => s.is_empty(),
            Text::Fragments(f) => f.is_empty(),
        }
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text::Plain(s.to_string())
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Text::Plain(s)
    }
}

/// One fragment of a [`Text::Fragments`] list (`TextFragment`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Fragment {
    /// A stable progress key.
    pub key: Option<String>,
    /// Show the fragment only when this holds, in the default condition
    /// language.
    pub when: Option<String>,
    /// The display mode.
    pub mode: FragmentMode,
    /// The variants. One for `always`, `first_time` and `once`.
    pub variants: Vec<Text>,
    /// Replace the text of earlier fragments.
    pub replaces: bool,
}

impl Fragment {
    /// A fragment that always shows the given text.
    pub fn always(text: impl Into<Text>) -> Self {
        Self {
            key: None,
            when: None,
            mode: FragmentMode::Always,
            variants: vec![text.into()],
            replaces: false,
        }
    }
}

/// The display mode of a [`Fragment`] (`FragmentMode` in the model).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FragmentMode {
    /// Show the first variant every time.
    #[default]
    Always,
    /// Only the first time the text resolves.
    FirstTime,
    /// Once: the first time the condition holds.
    Once,
    /// The variants in order, then start over.
    Cycle,
    /// A random variant, without an immediate repeat.
    Shuffle,
    /// A random variant the first time, then always that one.
    Sticky,
}

/// The default effect payload, for hosts without their own effects.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Effect {
    /// Add to the relation between two characters, by key
    /// (`Outcome::AddRelationship`). The key `player` is the player.
    Relation {
        /// The character whose relation changes.
        from: String,
        /// The character the relation points to.
        to: String,
        /// The relationship axis.
        axis: String,
        /// The amount to add.
        delta: i32,
    },
    /// A custom outcome (`Outcome::Custom`).
    Custom {
        /// The key.
        key: String,
        /// The arguments.
        args: Vec<Value>,
    },
}

/// A tag of a choice (`CustomDialogTag`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Tag {
    /// The axis, for example `skill`.
    pub axis: String,
    /// The key, for example `medic`.
    pub key: String,
    /// An optional display prefix, for example `[Bluff]`.
    pub prefix: Option<String>,
    /// The option needs the trait. The default is `true` on the `skill`
    /// axis and `false` on other axes.
    pub requires_trait: Option<bool>,
}

impl Tag {
    /// Creates a tag.
    pub fn new(axis: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            axis: axis.into(),
            key: key.into(),
            prefix: None,
            requires_trait: None,
        }
    }
}

/// The intentions of a choice (`DialogIntentions`), as custom intentions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Intentions {
    /// Intentions the author attaches.
    pub explicit: Vec<String>,
    /// Intentions that the choice has without saying it.
    pub implicit: Vec<String>,
}

impl Intentions {
    /// Returns `true` when both lists are empty.
    pub fn is_empty(&self) -> bool {
        self.explicit.is_empty() && self.implicit.is_empty()
    }
}

/// A navigation hint of a choice (`NavigationHint`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Hint {
    /// Continue.
    Continue,
    /// Leave the talk.
    Leave,
    /// Go to a named place or character.
    GoTo(String),
    /// End the talk.
    End,
    /// Return to the menu before.
    Return,
    /// A custom label.
    Custom(String),
}
