//! Dialogue graph — nodes, options, and branching conversation structure.
//!
//! This module defines the core types for representing branching dialogue.
//! A [`DialogueGraph`] contains a set of [`DialogueNode`]s, each of which
//! presents [`DialogueOption`]s that lead to other nodes or terminate
//! the conversation. Nodes and options can have conditions and outcomes
//! attached.
//!
//! # Type parameters
//!
//! The graph types have two type parameters. Both default to `()`, so
//! `DialogueGraph` is the graph without host data.
//!
//! - `H` is the type of [host outcomes](Outcome::Host).
//! - `T` is the type of [host text](DialogText::Host): text in a format
//!   that only the host knows, for example its own markup.
//!
//! A host with its own outcomes and its own text uses
//! `DialogueGraph<Effect, Markup>`.

use crate::character::CharacterId;
use crate::condition::Condition;
use crate::intent::DialogIntentions;
use crate::outcome::Outcome;
use crate::tag::DialogTagKind;
use std::sync::Arc;

/// Unique numeric identifier for a dialogue node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DialogueNodeId(pub u64);

impl DialogueNodeId {
    /// Creates a new dialogue node identifier from the given `u64` value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique numeric identifier for a dialogue option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DialogueOptionId(pub u64);

impl DialogueOptionId {
    /// Creates a new dialogue option identifier from the given `u64` value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Text content that can be inline, localized, assembled from
/// state-dependent fragments, or host text.
///
/// `T` is the type of [host text](DialogText::Host) (default `()`). A
/// host with its own text format (for example compiled markup) stores
/// it as `DialogText::Host(markup)` and reads it back typed. Host text can
/// also be a variant of a [`TextFragment`], so fragment selection works
/// for host text too.
///
/// The constructors [`inline`](DialogText::inline),
/// [`localized`](DialogText::localized) and
/// [`fragments`](DialogText::fragments) make `DialogText<()>`. For another
/// `T`, use the variants (`DialogText::Inline(..)`) or
/// [`DialogText::host`].
///
/// ```rust
/// use dramaturge_model::DialogText;
///
/// #[derive(Debug, Clone, PartialEq)]
/// struct Markup(Vec<&'static str>);
///
/// let text = DialogText::host(Markup(vec!["Hello, ", "*traveller*"]));
/// assert_eq!(text.as_host(), Some(&Markup(vec!["Hello, ", "*traveller*"])));
///
/// let plain: DialogText<Markup> = DialogText::Inline("Hi.".into());
/// assert!(plain.as_host().is_none());
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DialogText<T = ()> {
    /// Direct string text.
    Inline(String),
    /// Key into a localization table.
    LocalizedKey(Arc<str>),
    /// An ordered list of fragments, each with an optional condition and a
    /// display mode. Resolved at runtime by `dramaturge_runtime::resolve_text`.
    Fragments(Vec<TextFragment<T>>),
    /// Text in a format that only the host knows.
    ///
    /// dramaturge does not look into host text. The validator does not
    /// check it, and `dramaturge_runtime::resolve_text` returns it as it is.
    Host(T),
    /// The id of a named text pool (`say: { text: keeper_remark }`).
    ///
    /// The host selects the pool with `dramaturge_runtime::text::select_named`.
    /// `dramaturge_runtime::resolve_text` has no pools: it returns the id as
    /// a `ResolvedText::Pool` piece.
    Pool(Arc<str>),
}

/// One piece of a [`DialogText::Fragments`] list.
///
/// A fragment contributes text only when its `condition` holds (or has no
/// condition). Which of its `variants` it shows depends on its [`mode`](FragmentMode).
///
/// `T` is the type of [host text](DialogText::Host) (default `()`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextFragment<T = ()> {
    /// Optional stable key, for progress tracking and editor links.
    ///
    /// Without a key, progress is tracked by the fragment's position, which
    /// changes when fragments are inserted before it.
    pub key: Option<Arc<str>>,
    /// The text to show: one entry for `Always`/`FirstTime`/`Once`, the
    /// alternatives for `Cycle`/`Shuffle`/`Sticky`.
    pub variants: Vec<DialogText<T>>,
    /// Shown only if this holds.
    pub condition: Option<Condition>,
    /// How often and which variant to show.
    pub mode: FragmentMode,
    /// Replace the text of the earlier fragments instead of appending to it.
    pub replaces: bool,
}

impl<T> TextFragment<T> {
    /// A fragment that always shows `text`.
    pub fn always(text: DialogText<T>) -> Self {
        Self::with_mode(FragmentMode::Always, vec![text])
    }

    /// A fragment with the given mode and variants.
    pub fn with_mode(mode: FragmentMode, variants: Vec<DialogText<T>>) -> Self {
        Self {
            key: None,
            variants,
            condition: None,
            mode,
            replaces: false,
        }
    }

    /// Sets the stable progress key.
    pub fn keyed(mut self, key: impl Into<Arc<str>>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Sets the condition.
    pub fn when(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Makes this fragment replace the text before it.
    pub fn replacing(mut self) -> Self {
        self.replaces = true;
        self
    }

    /// Converts the host text in the variants with `f`.
    ///
    /// See [`DialogText::map_text`].
    pub fn map_text<U>(self, mut f: impl FnMut(T) -> U) -> TextFragment<U> {
        self.map_text_with(&mut f)
    }

    fn map_text_with<U>(self, f: &mut impl FnMut(T) -> U) -> TextFragment<U> {
        TextFragment {
            key: self.key,
            variants: self
                .variants
                .into_iter()
                .map(|v| v.map_text_with(f))
                .collect(),
            condition: self.condition,
            mode: self.mode,
            replaces: self.replaces,
        }
    }
}

/// How a [`TextFragment`] picks its text over repeated resolutions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum FragmentMode {
    /// Show the first variant every time.
    #[default]
    Always,
    /// Show the first variant only the first time the owning text resolves.
    FirstTime,
    /// Show the first variant once: the first time the condition holds.
    Once,
    /// Show the variants in order, one per resolution, then start over.
    Cycle,
    /// Show a random variant each time, avoiding an immediate repeat.
    Shuffle,
    /// Pick a random variant the first time, then always show that one.
    Sticky,
}

impl DialogText {
    /// Creates inline text from the given string-like value.
    pub fn inline(text: impl Into<String>) -> Self {
        Self::Inline(text.into())
    }

    /// Creates a localized text reference from the given key.
    pub fn localized(key: impl Into<Arc<str>>) -> Self {
        Self::LocalizedKey(key.into())
    }

    /// Creates fragment text from the given fragments.
    pub fn fragments(fragments: Vec<TextFragment>) -> Self {
        Self::Fragments(fragments)
    }
}

impl<T> DialogText<T> {
    /// Creates host text.
    pub fn host(text: T) -> Self {
        Self::Host(text)
    }

    /// Creates a reference to a named text pool.
    pub fn pool(id: impl Into<Arc<str>>) -> Self {
        Self::Pool(id.into())
    }

    /// Returns the first fragment key, here or in nested fragments, that
    /// would share progress with another fragment, by the rule of
    /// [`Pool::bad_key`](crate::Pool::bad_key): fragments are the
    /// candidates of a pool. `None` for text without fragments.
    pub fn bad_key(&self) -> Option<&str> {
        fn first<'a, T: 'a>(
            fragments: &mut dyn Iterator<Item = &'a TextFragment<T>>,
        ) -> Option<&'a str> {
            let mut seen = std::collections::BTreeSet::new();
            for f in fragments {
                if let Some(key) = f.key.as_deref()
                    && (!crate::Candidate::<crate::DialogTypes>::is_valid_key(key)
                        || !seen.insert(key))
                {
                    return Some(key);
                }
                // The nested lists of one fragment share its progress path.
                let mut inner = f
                    .variants
                    .iter()
                    .filter_map(|v| match v {
                        DialogText::Fragments(inner) => Some(inner.iter()),
                        _ => None,
                    })
                    .flatten();
                if let Some(key) = first(&mut inner) {
                    return Some(key);
                }
            }
            None
        }
        match self {
            DialogText::Fragments(fragments) => first(&mut fragments.iter()),
            _ => None,
        }
    }

    /// Returns the inline text, or `None` for every other variant.
    pub fn as_inline(&self) -> Option<&str> {
        match self {
            Self::Inline(text) => Some(text),
            _ => None,
        }
    }

    /// Returns the host text, or `None` for every other variant.
    ///
    /// Host text inside a fragment list is not returned. Use
    /// `dramaturge_runtime::resolve_text` to select it.
    pub fn as_host(&self) -> Option<&T> {
        match self {
            Self::Host(text) => Some(text),
            _ => None,
        }
    }

    /// Converts the host text with `f`, also inside fragment lists, and
    /// keeps every other variant.
    ///
    /// Use it to move text between host text types, for example from
    /// `DialogText<()>` (loaded from YAML) to `DialogText<Markup>`.
    pub fn map_text<U>(self, mut f: impl FnMut(T) -> U) -> DialogText<U> {
        self.map_text_with(&mut f)
    }

    fn map_text_with<U>(self, f: &mut impl FnMut(T) -> U) -> DialogText<U> {
        match self {
            Self::Inline(s) => DialogText::Inline(s),
            Self::LocalizedKey(k) => DialogText::LocalizedKey(k),
            Self::Fragments(fragments) => DialogText::Fragments(
                fragments
                    .into_iter()
                    .map(|f2| f2.map_text_with(f))
                    .collect(),
            ),
            Self::Host(text) => DialogText::Host(f(text)),
            Self::Pool(id) => DialogText::Pool(id),
        }
    }
}

/// Where a dialogue option leads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DialogTarget {
    /// Transitions to another dialogue node.
    Node(DialogueNodeId),
    /// Ends the conversation entirely.
    End,
    /// Returns to the calling context (e.g., from a sub-dialogue).
    Return,
    /// Game-specific custom target.
    Custom(Arc<str>),
}

/// Navigation hint for dialogue options.
///
/// Provides typed navigation cues that can be rendered as UI hints
/// (e.g., "[Continue]", "[Leave]", "[End]"). These replace hardcoded
/// prefixes in dialogue text with semantic, localizable navigation markers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NavigationHint {
    /// Continue to the next node (→ Continue / [Continue]).
    Continue,
    /// Leave the current conversation (← Leave / [Leave]).
    Leave,
    /// Go to a specific character or location (→ Go to {name} / [Go to {name}]).
    GoTo(String),
    /// End the conversation (✓ End / [End]).
    End,
    /// Return to a previous menu or caller (↩ Return / [Return]).
    Return,
    /// Custom navigation label.
    Custom(String),
}

impl NavigationHint {
    /// Returns the default display text for this navigation hint.
    pub fn default_text(&self) -> String {
        match self {
            NavigationHint::Continue => "Continue...".to_string(),
            NavigationHint::Leave => "Leave...".to_string(),
            NavigationHint::GoTo(target) => format!("Go to {target}"),
            NavigationHint::End => "End conversation".to_string(),
            NavigationHint::Return => "Return...".to_string(),
            NavigationHint::Custom(label) => label.clone(),
        }
    }

    /// Returns the display prefix for this navigation hint (e.g., "[Continue]").
    pub fn display_prefix(&self) -> String {
        match self {
            NavigationHint::Continue => "[Continue]".to_string(),
            NavigationHint::Leave => "[Leave]".to_string(),
            NavigationHint::GoTo(target) => format!("[Go to {target}]"),
            NavigationHint::End => "[End]".to_string(),
            NavigationHint::Return => "[Return]".to_string(),
            NavigationHint::Custom(label) => format!("[{label}]"),
        }
    }
}

impl DialogTarget {
    /// Creates a target pointing to the given node ID.
    pub fn node(id: u64) -> Self {
        Self::Node(DialogueNodeId::new(id))
    }

    /// Creates an end-of-conversation target.
    pub fn end() -> Self {
        Self::End
    }

    /// Creates a return-to-caller target.
    pub fn r#return() -> Self {
        Self::Return
    }

    /// Creates a custom target with the given identifier.
    pub fn custom(target: impl Into<Arc<str>>) -> Self {
        Self::Custom(target.into())
    }
}

/// One line in a node, with its own speaker.
///
/// A node can hold more than one line: the node's own
/// [`speaker`](DialogueNode::speaker) and [`text`](DialogueNode::text)
/// come first, then the [`lines`](DialogueNode::lines) in order. Use lines
/// for scripted exchanges, cutscenes and background talk, where characters
/// talk to each other without a choice.
///
/// A line has no conditions. For a line that depends on the state, use
/// fragment text ([`DialogText::Fragments`]) with conditions; a host skips a
/// line whose text resolves to nothing.
///
/// `T` is the type of [host text](DialogText::Host) (default `()`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DialogLine<T = ()> {
    /// The character who speaks this line.
    pub speaker: CharacterId,
    /// The spoken text.
    pub text: DialogText<T>,
}

impl<T> DialogLine<T> {
    /// Creates a line spoken by `speaker`.
    pub fn new(speaker: CharacterId, text: DialogText<T>) -> Self {
        Self { speaker, text }
    }

    /// Converts the host text of this line with `f`.
    ///
    /// See [`DialogText::map_text`].
    pub fn map_text<U>(self, mut f: impl FnMut(T) -> U) -> DialogLine<U> {
        self.map_text_with(&mut f)
    }

    fn map_text_with<U>(self, f: &mut impl FnMut(T) -> U) -> DialogLine<U> {
        DialogLine {
            speaker: self.speaker,
            text: self.text.map_text_with(f),
        }
    }
}

/// A single step in a dialogue graph where a character speaks.
///
/// Each node represents a moment in the conversation: a character says
/// something (via `text`, and more [`lines`](Self::lines) with their own
/// speakers), and optionally presents options to continue. Nodes can have
/// conditions that gate their entry and outcomes that apply when the node
/// is entered.
///
/// # Auto-continue
///
/// A node with [`next`](Self::next) continues by itself when it has no
/// available option: the talk goes on to the next target without a
/// choice. `DialogueRuntime::advance` in `dramaturge-runtime` follows these
/// targets and enters each node. A node with an available option waits
/// for the choice; `next` is then not used.
///
/// `H` is the type of [host outcomes](Outcome::Host) and `T` the type of
/// [host text](DialogText::Host). Both default to `()`.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueNode<H = (), T = ()> {
    /// Unique identifier for this node.
    pub id: DialogueNodeId,
    /// The character who speaks in this node.
    pub speaker: CharacterId,
    /// The spoken text.
    pub text: DialogText<T>,
    /// Conditions that must be satisfied for this node to be entered.
    ///
    /// `dramaturge_runtime::select_entry` checks them for the entry nodes,
    /// and `DialogueRuntime::enter_node` checks them when the host enters a
    /// node. `choose_option` does not check the conditions of the target
    /// node.
    pub conditions: Vec<Condition>,
    /// Outcomes that apply when this node is entered.
    pub outcomes_on_enter: Vec<Outcome<H>>,
    /// Branching options presented to the player after this node.
    pub options: Vec<DialogueOption<H, T>>,
    /// Further lines after [`text`](Self::text), each with its own speaker,
    /// in order. Empty for a node with one line.
    pub lines: Vec<DialogLine<T>>,
    /// Where the talk goes on without a choice, when the node has no
    /// available option. `None` ends the talk at such a node.
    ///
    /// See [Auto-continue](Self#auto-continue).
    pub next: Option<DialogTarget>,
}

impl DialogueNode {
    /// Creates a node with the given ID, speaker, and text.
    ///
    /// Conditions, outcomes, and options all start empty. For a node with
    /// host outcomes or host text, use [`DialogueNode::typed`].
    pub fn new(id: u64, speaker: CharacterId, text: DialogText) -> Self {
        Self::typed(id, speaker, text)
    }
}

impl<H, T> DialogueNode<H, T> {
    /// Creates a node for the host outcome type `H` and the host text type
    /// `T`, with the given ID, speaker, and text.
    ///
    /// Conditions, outcomes, and options all start empty.
    pub fn typed(id: u64, speaker: CharacterId, text: DialogText<T>) -> Self {
        Self {
            id: DialogueNodeId::new(id),
            speaker,
            text,
            conditions: Vec::new(),
            outcomes_on_enter: Vec::new(),
            options: Vec::new(),
            lines: Vec::new(),
            next: None,
        }
    }

    /// Adds a line spoken by `speaker` and returns the node.
    pub fn with_line(mut self, speaker: CharacterId, text: DialogText<T>) -> Self {
        self.lines.push(DialogLine::new(speaker, text));
        self
    }

    /// Sets the [`next`](Self::next) target and returns the node.
    pub fn with_next(mut self, target: DialogTarget) -> Self {
        self.next = Some(target);
        self
    }

    /// Returns every line of the node in order: first the node's own
    /// speaker and text, then the [`lines`](Self::lines).
    ///
    /// ```rust
    /// use dramaturge_model::{CharacterId, DialogText, DialogueNode, EntityId};
    ///
    /// let mara = CharacterId::new(EntityId::new(1));
    /// let player = CharacterId::new(EntityId::new(2));
    /// let node = DialogueNode::new(1, mara, DialogText::inline("Sit, if you like."))
    ///     .with_line(player, DialogText::inline("Who are you?"))
    ///     .with_line(mara, DialogText::inline("They call me Mara."));
    ///
    /// let speakers: Vec<_> = node.all_lines().map(|(speaker, _)| speaker).collect();
    /// assert_eq!(speakers, [mara, player, mara]);
    /// ```
    pub fn all_lines(&self) -> impl Iterator<Item = (CharacterId, &DialogText<T>)> {
        std::iter::once((self.speaker, &self.text))
            .chain(self.lines.iter().map(|line| (line.speaker, &line.text)))
    }

    /// Converts the host outcomes of this node and its options with `f`.
    ///
    /// See [`Outcome::map_host`].
    pub fn map_host<G>(self, mut f: impl FnMut(H) -> G) -> DialogueNode<G, T> {
        DialogueNode {
            id: self.id,
            speaker: self.speaker,
            text: self.text,
            lines: self.lines,
            next: self.next,
            conditions: self.conditions,
            outcomes_on_enter: self
                .outcomes_on_enter
                .into_iter()
                .map(|o| o.map_host(&mut f))
                .collect(),
            options: self
                .options
                .into_iter()
                .map(|o| o.map_host(&mut f))
                .collect(),
        }
    }

    /// Converts the host text of this node and its options with `f`.
    ///
    /// See [`DialogText::map_text`].
    pub fn map_text<U>(self, mut f: impl FnMut(T) -> U) -> DialogueNode<H, U> {
        DialogueNode {
            id: self.id,
            speaker: self.speaker,
            text: self.text.map_text_with(&mut f),
            lines: self
                .lines
                .into_iter()
                .map(|line| line.map_text_with(&mut f))
                .collect(),
            next: self.next,
            conditions: self.conditions,
            outcomes_on_enter: self.outcomes_on_enter,
            options: self
                .options
                .into_iter()
                .map(|o| o.map_text(&mut f))
                .collect(),
        }
    }
}

/// A branching choice presented to the player.
///
/// Options are attached to [`DialogueNode`]s and represent the possible
/// responses or actions available to the player. Each option can have
/// tags, intentions, conditions, and outcomes.
///
/// `H` is the type of [host outcomes](Outcome::Host) and `T` the type of
/// [host text](DialogText::Host). Both default to `()`.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueOption<H = (), T = ()> {
    /// Unique identifier for this option.
    pub id: DialogueOptionId,
    /// The text displayed for this option.
    pub text: DialogText<T>,
    /// Tags categorizing this option.
    pub tags: Vec<DialogTagKind>,
    /// Intentions behind this option.
    pub intentions: DialogIntentions,
    /// Conditions that must be satisfied for this option to be available.
    pub conditions: Vec<Condition>,
    /// Outcomes that apply when this option is chosen.
    pub outcomes: Vec<Outcome<H>>,
    /// Where this option leads when chosen.
    pub target: DialogTarget,
    /// The node this option belongs to.
    pub source_node_id: DialogueNodeId,
    /// Optional navigation hint (e.g., Continue, Leave, End).
    pub navigation_hint: Option<NavigationHint>,
}

impl DialogueOption {
    /// Creates an option with the given ID, text, target, and source node.
    ///
    /// Tags, intentions, conditions, outcomes, and navigation hint all start
    /// empty. For an option with host outcomes or host text, use
    /// [`DialogueOption::typed`].
    pub fn new(
        id: u64,
        text: DialogText,
        target: DialogTarget,
        source_node_id: DialogueNodeId,
    ) -> Self {
        Self::typed(id, text, target, source_node_id)
    }
}

impl<H, T> DialogueOption<H, T> {
    /// Creates an option for the host outcome type `H` and the host text
    /// type `T`, with the given ID, text, target, and source node.
    ///
    /// Tags, intentions, conditions, outcomes, and navigation hint all start empty.
    pub fn typed(
        id: u64,
        text: DialogText<T>,
        target: DialogTarget,
        source_node_id: DialogueNodeId,
    ) -> Self {
        Self {
            id: DialogueOptionId::new(id),
            text,
            tags: Vec::new(),
            intentions: DialogIntentions::new(),
            conditions: Vec::new(),
            outcomes: Vec::new(),
            target,
            source_node_id,
            navigation_hint: None,
        }
    }

    /// Sets the navigation hint for this option.
    pub fn with_navigation_hint(mut self, hint: NavigationHint) -> Self {
        self.navigation_hint = Some(hint);
        self
    }

    /// Converts the host outcomes of this option with `f`.
    ///
    /// See [`Outcome::map_host`].
    pub fn map_host<G>(self, mut f: impl FnMut(H) -> G) -> DialogueOption<G, T> {
        DialogueOption {
            id: self.id,
            text: self.text,
            tags: self.tags,
            intentions: self.intentions,
            conditions: self.conditions,
            outcomes: self
                .outcomes
                .into_iter()
                .map(|o| o.map_host(&mut f))
                .collect(),
            target: self.target,
            source_node_id: self.source_node_id,
            navigation_hint: self.navigation_hint,
        }
    }

    /// Converts the host text of this option with `f`.
    ///
    /// See [`DialogText::map_text`].
    pub fn map_text<U>(self, mut f: impl FnMut(T) -> U) -> DialogueOption<H, U> {
        DialogueOption {
            id: self.id,
            text: self.text.map_text_with(&mut f),
            tags: self.tags,
            intentions: self.intentions,
            conditions: self.conditions,
            outcomes: self.outcomes,
            target: self.target,
            source_node_id: self.source_node_id,
            navigation_hint: self.navigation_hint,
        }
    }
}

/// Root container for a complete dialogue tree.
///
/// Holds all nodes and the entry nodes where a conversation can start.
///
/// # Entry nodes
///
/// A graph can have more than one entry node. The order is the priority:
/// `dramaturge_runtime::select_entry` returns the first entry node whose
/// [`conditions`](DialogueNode::conditions) hold. A graph with one
/// unconditional entry node always starts there. The validator walks from
/// every entry node.
///
/// `H` is the type of [host outcomes](Outcome::Host) and `T` the type of
/// [host text](DialogText::Host). Both default to `()`: a graph without
/// host data is `DialogueGraph`.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueGraph<H = (), T = ()> {
    /// All nodes in the graph.
    pub nodes: Vec<DialogueNode<H, T>>,
    /// The nodes where a conversation can start, in priority order.
    ///
    /// Empty when the graph has no entry node.
    pub entry_nodes: Vec<DialogueNodeId>,
}

impl DialogueGraph {
    /// Creates an empty graph with no nodes and no entry nodes.
    ///
    /// For a graph with host outcomes or host text, use
    /// [`DialogueGraph::typed`].
    pub fn new() -> Self {
        Self::typed()
    }
}

impl<H, T> DialogueGraph<H, T> {
    /// Creates an empty graph for the host outcome type `H` and the host
    /// text type `T`, with no nodes and no entry point.
    pub fn typed() -> Self {
        Self {
            nodes: Vec::new(),
            entry_nodes: Vec::new(),
        }
    }

    /// Returns the first configured entry node id, if the graph has one.
    ///
    /// This is the first id in [`entry_nodes`](Self::entry_nodes), as it is:
    /// the graph may have no node with this id (the validator reports it as
    /// `EntryNodeNotFound`). For the first entry node that exists, use
    /// [`entries`](Self::entries); to select an entry node by its
    /// conditions, use `dramaturge_runtime::select_entry`.
    pub fn entry_node(&self) -> Option<DialogueNodeId> {
        self.entry_nodes.first().copied()
    }

    /// Adds `id` to the end of the entry nodes and returns the graph.
    pub fn with_entry_node(mut self, id: DialogueNodeId) -> Self {
        self.entry_nodes.push(id);
        self
    }

    /// Returns the node with the given id, if the graph has one.
    ///
    /// This is a linear search over [`nodes`](Self::nodes).
    pub fn node(&self, id: DialogueNodeId) -> Option<&DialogueNode<H, T>> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Returns the entry nodes that exist in the graph, in priority order.
    ///
    /// Entry node ids without a node in the graph are skipped.
    pub fn entries(&self) -> impl Iterator<Item = &DialogueNode<H, T>> {
        self.entry_nodes.iter().filter_map(|&id| self.node(id))
    }

    /// Converts the host outcomes of every node and option with `f`.
    ///
    /// Use it to give a graph without host outcomes (for example one
    /// loaded from YAML, a `DialogueGraph<()>`) a host outcome type:
    ///
    /// ```rust
    /// use dramaturge_model::DialogueGraph;
    ///
    /// enum Effect {}
    ///
    /// let loaded = DialogueGraph::new();
    /// let typed: DialogueGraph<Effect> = loaded.map_host(|()| unreachable!());
    /// assert!(typed.nodes.is_empty());
    /// ```
    pub fn map_host<G>(self, mut f: impl FnMut(H) -> G) -> DialogueGraph<G, T> {
        DialogueGraph {
            nodes: self.nodes.into_iter().map(|n| n.map_host(&mut f)).collect(),
            entry_nodes: self.entry_nodes,
        }
    }

    /// Converts the host text of every node and option with `f`, also
    /// inside fragment lists.
    ///
    /// Use it to give a graph without host text (for example one loaded
    /// from YAML) a host text type:
    ///
    /// ```rust
    /// use dramaturge_model::DialogueGraph;
    ///
    /// struct Markup(String);
    ///
    /// let loaded = DialogueGraph::new();
    /// let typed: DialogueGraph<(), Markup> = loaded.map_text(|()| unreachable!());
    /// assert!(typed.nodes.is_empty());
    /// ```
    pub fn map_text<U>(self, mut f: impl FnMut(T) -> U) -> DialogueGraph<H, U> {
        DialogueGraph {
            nodes: self.nodes.into_iter().map(|n| n.map_text(&mut f)).collect(),
            entry_nodes: self.entry_nodes,
        }
    }
}

impl Default for DialogueGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn fragment_bad_keys() {
        let keyed = |k: &str| {
            let mut f = TextFragment::always(DialogText::Inline("a".into()));
            f.key = Some(k.into());
            f
        };
        let list =
            |keys: &[&str]| DialogText::<()>::Fragments(keys.iter().map(|k| keyed(k)).collect());
        check!(list(&["a", "b"]).bad_key().is_none());
        check!(list(&["0"]).bad_key() == Some("0"));
        check!(list(&[""]).bad_key() == Some(""));
        check!(list(&["a/b"]).bad_key() == Some("a/b"));
        check!(list(&["a", "a"]).bad_key() == Some("a"));
        let siblings =
            TextFragment::with_mode(FragmentMode::Cycle, vec![list(&["x"]), list(&["x"])]);
        check!(DialogText::Fragments(vec![siblings]).bad_key() == Some("x"));
        check!(DialogText::<()>::Inline("0".into()).bad_key().is_none());
    }

    #[test]
    fn dialogue_node_id_construction() {
        let id = DialogueNodeId::new(1);
        check!(id.0 == 1);
    }

    #[test]
    fn dialogue_node_id_is_copy() {
        let a = DialogueNodeId::new(1);
        let b = a;
        check!(a == b);
    }

    #[test]
    fn dialogue_option_id_construction() {
        let id = DialogueOptionId::new(2);
        check!(id.0 == 2);
    }

    #[test]
    fn dialog_text_variants() {
        let inline = DialogText::inline("Hello");
        let localized = DialogText::localized("greeting_key");
        check!(let DialogText::Inline(_) = inline);
        check!(let DialogText::LocalizedKey(_) = localized);
    }

    #[test]
    fn dialog_target_variants() {
        let node = DialogTarget::node(1);
        let end = DialogTarget::end();
        let ret = DialogTarget::r#return();
        let custom = DialogTarget::custom("custom_target");
        check!(let DialogTarget::Node(_) = node);
        check!(let DialogTarget::End = end);
        check!(let DialogTarget::Return = ret);
        check!(let DialogTarget::Custom(_) = custom);
    }

    #[test]
    fn dialogue_node_construction() {
        let speaker = CharacterId::new(crate::character::EntityId::new(1));
        let text = DialogText::inline("Hello, traveller.");
        let node = DialogueNode::new(1, speaker, text);
        check!(node.id.0 == 1);
        check!(node.conditions.is_empty());
        check!(node.outcomes_on_enter.is_empty());
        check!(node.options.is_empty());
    }

    #[test]
    fn dialogue_option_construction() {
        let text = DialogText::inline("Yes, I'll help.");
        let option = DialogueOption::new(1, text, DialogTarget::node(2), DialogueNodeId::new(1));
        check!(option.id.0 == 1);
        check!(option.tags.is_empty());
        check!(option.conditions.is_empty());
        check!(option.outcomes.is_empty());
        check!(let DialogTarget::Node(_) = option.target);
    }

    #[test]
    fn graph_with_host_outcomes() {
        #[derive(Debug, Clone, PartialEq)]
        enum Effect {
            Goto(&'static str),
        }

        let speaker = CharacterId::new(crate::character::EntityId::new(1));
        let mut node = DialogueNode::<Effect>::typed(1, speaker, DialogText::inline("Bye."));
        node.outcomes_on_enter
            .push(Outcome::Host(Effect::Goto("hall")));
        let mut option =
            DialogueOption::typed(1, DialogText::inline("Leave."), DialogTarget::End, node.id);
        option.outcomes.push(Outcome::Host(Effect::Goto("street")));
        node.options.push(option);
        let mut graph = DialogueGraph::typed();
        graph.nodes.push(node);
        graph.entry_nodes.push(DialogueNodeId::new(1));

        let copy = graph.clone();
        check!(copy == graph);
        check!(graph.nodes[0].options[0].outcomes[0].as_host() == Some(&Effect::Goto("street")));

        let lengths = graph.map_host(|Effect::Goto(room)| room.len());
        check!(lengths.nodes[0].outcomes_on_enter[0] == Outcome::Host(4));
        check!(lengths.nodes[0].options[0].outcomes[0] == Outcome::Host(6));
        check!(lengths.entry_nodes == vec![DialogueNodeId::new(1)]);
    }

    #[test]
    fn graph_with_host_text() {
        #[derive(Debug, Clone, PartialEq)]
        struct Markup(&'static str);

        let speaker = CharacterId::new(crate::character::EntityId::new(1));
        let mut node =
            DialogueNode::<(), Markup>::typed(1, speaker, DialogText::host(Markup("*Hi.*")));
        node.options.push(DialogueOption::typed(
            1,
            DialogText::Fragments(vec![
                TextFragment::always(DialogText::Inline("Go ".into())),
                TextFragment::always(DialogText::host(Markup("_now_"))),
            ]),
            DialogTarget::End,
            node.id,
        ));
        let mut graph = DialogueGraph::typed().with_entry_node(node.id);
        graph.nodes.push(node);

        // The host reads its text back, typed.
        check!(graph.nodes[0].text.as_host() == Some(&Markup("*Hi.*")));
        check!(graph.nodes[0].options[0].text.as_host().is_none());

        let lengths = graph.clone().map_text(|Markup(m)| m.len());
        check!(lengths.nodes[0].text == DialogText::Host(5));
        let DialogText::Fragments(fragments) = &lengths.nodes[0].options[0].text else {
            panic!("fragments expected");
        };
        check!(fragments[0].variants[0] == DialogText::Inline("Go ".into()));
        check!(fragments[1].variants[0] == DialogText::Host(5));

        // Host outcomes and host text map apart.
        let unit: DialogueGraph<(), ()> = graph.map_text(|_| ());
        check!(unit.nodes[0].text == DialogText::Host(()));
    }

    #[test]
    fn node_with_lines_and_next() {
        let mara = CharacterId::new(crate::character::EntityId::new(1));
        let player = CharacterId::new(crate::character::EntityId::new(2));
        let node = DialogueNode::new(1, mara, DialogText::inline("Sit."))
            .with_line(player, DialogText::inline("Who are you?"))
            .with_line(mara, DialogText::inline("Mara."))
            .with_next(DialogTarget::node(2));
        check!(node.lines.len() == 2);
        check!(node.next == Some(DialogTarget::node(2)));
        let lines: Vec<_> = node.all_lines().collect();
        check!(lines.len() == 3);
        check!(lines[1] == (player, &DialogText::inline("Who are you?")));

        // Lines keep their speakers through both maps.
        let typed: DialogueNode<u8, &str> = node.clone().map_host(|()| 0).map_text(|()| "never");
        check!(typed.lines[1].speaker == mara);
        check!(typed.next == Some(DialogTarget::node(2)));

        let plain = DialogueNode::new(2, mara, DialogText::inline("Hi."));
        check!(plain.lines.is_empty());
        check!(plain.next.is_none());
        check!(plain.all_lines().count() == 1);
    }

    #[test]
    fn dialogue_graph_default() {
        let graph = DialogueGraph::default();
        check!(graph.nodes.is_empty());
        check!(graph.entry_nodes.is_empty());
        check!(graph.entry_node().is_none());
    }

    #[test]
    fn dialogue_graph_entries_skip_missing_nodes() {
        let speaker = CharacterId::new(crate::character::EntityId::new(1));
        let mut graph = DialogueGraph::new()
            .with_entry_node(DialogueNodeId::new(9))
            .with_entry_node(DialogueNodeId::new(2))
            .with_entry_node(DialogueNodeId::new(1));
        graph
            .nodes
            .push(DialogueNode::new(1, speaker, DialogText::inline("a")));
        graph
            .nodes
            .push(DialogueNode::new(2, speaker, DialogText::inline("b")));

        check!(graph.entry_node() == Some(DialogueNodeId::new(9)));
        let entries: Vec<_> = graph.entries().map(|n| n.id.0).collect();
        check!(entries == vec![2, 1]);
        check!(graph.node(DialogueNodeId::new(2)).is_some());
        check!(graph.node(DialogueNodeId::new(9)).is_none());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use assert2::check;
    use proptest::prelude::*;

    fn arb_dialogue_node_id() -> impl Strategy<Value = DialogueNodeId> {
        any::<u64>().prop_map(DialogueNodeId::new)
    }

    #[allow(dead_code)]
    fn arb_dialogue_option_id() -> impl Strategy<Value = DialogueOptionId> {
        any::<u64>().prop_map(DialogueOptionId::new)
    }

    #[allow(dead_code)]
    fn arb_dialog_text() -> impl Strategy<Value = DialogText> {
        prop_oneof![
            ".*".prop_map(DialogText::inline),
            ".*".prop_map(|s: String| DialogText::localized(s)),
        ]
    }

    fn arb_dialog_target() -> impl Strategy<Value = DialogTarget> {
        prop_oneof![
            arb_dialogue_node_id().prop_map(DialogTarget::Node),
            Just(DialogTarget::End),
            Just(DialogTarget::Return),
            ".*".prop_map(|s: String| DialogTarget::custom(s)),
        ]
    }

    #[test]
    fn prop_node_ids_unique() {
        proptest!(|(node_ids in proptest::collection::hash_set(any::<u64>(), 1..20))| {
            let mut seen = std::collections::HashSet::new();
            for id in &node_ids {
                prop_assert!(seen.insert(*id));
            }
        });
    }

    #[test]
    fn prop_option_targets_valid_or_terminal() {
        proptest!(|(target in arb_dialog_target())| {
            match &target {
                DialogTarget::Node(_) | DialogTarget::End | DialogTarget::Return | DialogTarget::Custom(_) => {}
            }
        });
    }

    #[test]
    fn prop_dialogue_graph_default_entry_none() {
        let graph = DialogueGraph::new();
        check!(graph.entry_nodes.is_empty());
    }

    #[test]
    fn prop_dialog_text_roundtrip() {
        proptest!(|(text in ".*")| {
            let dt = DialogText::inline(&text);
            if let DialogText::Inline(s) = dt {
                prop_assert_eq!(s, text);
            } else {
                prop_assert!(false, "expected Inline variant");
            }
        });
    }

    #[test]
    fn prop_dialog_target_node_id_preserved() {
        proptest!(|(id: u64)| {
            let target = DialogTarget::node(id);
            if let DialogTarget::Node(node_id) = target {
                prop_assert_eq!(node_id.0, id);
            } else {
                prop_assert!(false, "expected Node variant");
            }
        });
    }

    #[test]
    fn prop_dialogue_node_construction_preserves_fields() {
        proptest!(|(
            id in any::<u64>(),
            speaker in any::<u64>(),
            text in ".*"
        )| {
            let speaker = CharacterId::new(crate::character::EntityId::new(speaker));
            let node = DialogueNode::new(id, speaker, DialogText::inline(&text));
            prop_assert_eq!(node.id.0, id);
            prop_assert!(node.conditions.is_empty());
            prop_assert!(node.outcomes_on_enter.is_empty());
            prop_assert!(node.options.is_empty());
        });
    }

    #[test]
    fn prop_dialogue_option_construction_preserves_fields() {
        proptest!(|(
            id in any::<u64>(),
            text in ".*",
            target_id in any::<u64>(),
            source_node_id in any::<u64>()
        )| {
            let option = DialogueOption::new(id, DialogText::inline(&text), DialogTarget::node(target_id), DialogueNodeId::new(source_node_id));
            prop_assert_eq!(option.id.0, id);
            prop_assert!(option.tags.is_empty());
            prop_assert!(option.conditions.is_empty());
            prop_assert!(option.outcomes.is_empty());
            if let DialogTarget::Node(n) = option.target {
                prop_assert_eq!(n.0, target_id);
            } else {
                prop_assert!(false, "expected Node target");
            }
            prop_assert_eq!(option.source_node_id.0, source_node_id);
        });
    }
}
