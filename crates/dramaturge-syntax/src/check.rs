//! Checks on the AST that need no host: keys, targets and the shape.
//!
//! [`check`] returns [`Issue`]s. Each issue has a [`Place`] in the AST. The
//! YAML loader shows the place as a path (`nodes.meet.options[1].go`); the
//! token parser maps it to the span of the token.

use crate::ast::{Dialogue, Target};
use std::collections::HashSet;
use std::fmt;

/// A place in a [`Dialogue`]: indices into `start`, `nodes`, `lines` and
/// `options`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Place {
    /// The dialogue as a whole.
    Dialogue,
    /// `with`.
    With,
    /// `start[i]`.
    Start(usize),
    /// The key of node `n`.
    Node(usize),
    /// The `speaker` of node `n`.
    NodeSpeaker(usize),
    /// The `when` of node `n`.
    NodeWhen(usize),
    /// The `say` of node `n`.
    NodeSay(usize),
    /// Line `l` of node `n`.
    Line(usize, usize),
    /// The `next` of node `n`.
    Next(usize),
    /// An outcome of node `n`.
    NodeOutcome(usize, OutcomePlace),
    /// Option `o` of node `n` (its line).
    Option(usize, usize),
    /// The `id` of option `o` of node `n`.
    OptionId(usize, usize),
    /// The `when` of option `o` of node `n`.
    OptionWhen(usize, usize),
    /// The `go` of option `o` of node `n`.
    OptionGo(usize, usize),
    /// An outcome of option `o` of node `n`.
    OptionOutcome(usize, usize, OutcomePlace),
}

/// An entry in [`Outcomes`](crate::Outcomes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OutcomePlace {
    /// `set.<name>`.
    Set(String),
    /// `add.<name>`.
    Add(String),
    /// `relation.<axis>`.
    Relation(String),
    /// `remember[i]`.
    Remember(usize),
    /// `then[i]`, or the `then` list as a whole for `None`.
    Then(Option<usize>),
}

impl Place {
    /// Writes the place as a path, with the node keys of `dialogue`, for
    /// example `nodes.meet.options[1].when`.
    pub fn path<C, X, E, M>(&self, dialogue: &Dialogue<C, X, E, M>) -> String {
        let node = |n: &usize| {
            dialogue.nodes.get(*n).map_or_else(
                || format!("nodes[{n}]"),
                |node| format!("nodes.{}", node.key),
            )
        };
        match self {
            Place::Dialogue => String::new(),
            Place::With => "with".into(),
            Place::Start(i) => format!("start[{i}]"),
            Place::Node(n) => node(n),
            Place::NodeSpeaker(n) => format!("{}.speaker", node(n)),
            Place::NodeWhen(n) => format!("{}.when", node(n)),
            Place::NodeSay(n) => format!("{}.say", node(n)),
            Place::Line(n, l) => format!("{}.lines[{l}]", node(n)),
            Place::Next(n) => format!("{}.next", node(n)),
            Place::NodeOutcome(n, o) => format!("{}.{o}", node(n)),
            Place::Option(n, o) => format!("{}.options[{o}]", node(n)),
            Place::OptionId(n, o) => format!("{}.options[{o}].id", node(n)),
            Place::OptionWhen(n, o) => format!("{}.options[{o}].when", node(n)),
            Place::OptionGo(n, o) => format!("{}.options[{o}].go", node(n)),
            Place::OptionOutcome(n, o, out) => format!("{}.options[{o}].{out}", node(n)),
        }
    }
}

impl fmt::Display for OutcomePlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutcomePlace::Set(name) => write!(f, "set.{name}"),
            OutcomePlace::Add(name) => write!(f, "add.{name}"),
            OutcomePlace::Relation(axis) => write!(f, "relation.{axis}"),
            OutcomePlace::Remember(i) => write!(f, "remember[{i}]"),
            OutcomePlace::Then(Some(i)) => write!(f, "then[{i}]"),
            OutcomePlace::Then(None) => f.write_str("then"),
        }
    }
}

/// How bad an [`Issue`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Severity {
    /// The dialogue works, but probably not as the author wants.
    Warning,
    /// The dialogue can not be lowered.
    Error,
}

/// What an [`Issue`] is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IssueKind {
    /// `start` is empty.
    NoStart,
    /// Two nodes have the same key.
    DuplicateNode,
    /// A node has the key `end` or `back`.
    ReservedKey,
    /// Two options have the same key (the `id`, or `<node>#<index>`).
    DuplicateOption,
    /// `start`, `go` or `next` names a node that does not exist.
    UnknownTarget,
    /// A node has no options and no `next`: the talk ends there without
    /// saying so.
    NoWayOn,
    /// An option has no line (or an empty line) and no hint. [`check`]
    /// sees only a missing `say`; [`lower`](crate::lower()) also reports
    /// a `say` that lowers to empty text.
    NoLabel,
}

/// A problem in a dialogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// How bad it is.
    pub severity: Severity,
    /// What it is about.
    pub kind: IssueKind,
    /// Where it is.
    pub place: Place,
    /// The path of the place, for example `nodes.meet.options[1].go`.
    pub path: String,
    /// The message, without the path.
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            f.write_str(&self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

/// Checks the keys, the targets and the shape of a dialogue.
///
/// Errors (the lowering refuses the dialogue):
///
/// - `start` is empty, or names a node that does not exist.
/// - Two nodes have the same key, or a node has the key `end` or `back`.
/// - Two options have the same key: the `id`, or `<node>#<index>` for an
///   option without `id`.
/// - A `go` or `next` names a node that does not exist.
/// - An option has no `say` and no `hint`.
///
/// Warnings:
///
/// - A node has no options and no `next`. The talk ends at this node.
///   Write `next: end` to say so.
pub fn check<C, X, E, M>(dialogue: &Dialogue<C, X, E, M>) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |severity, kind, place: Place, message: String| {
        let path = place.path(dialogue);
        issues.push(Issue {
            severity,
            kind,
            place,
            path,
            message,
        });
    };

    if dialogue.start.is_empty() {
        push(
            Severity::Error,
            IssueKind::NoStart,
            Place::Dialogue,
            "`start` is empty; name at least one node".into(),
        );
    }

    let mut keys = HashSet::new();
    for (n, node) in dialogue.nodes.iter().enumerate() {
        if node.key == "end" || node.key == "back" {
            push(
                Severity::Error,
                IssueKind::ReservedKey,
                Place::Node(n),
                format!("`{}` is a reserved target; use another node key", node.key),
            );
        } else if !keys.insert(node.key.as_str()) {
            push(
                Severity::Error,
                IssueKind::DuplicateNode,
                Place::Node(n),
                format!("duplicate node key `{}`", node.key),
            );
        }
    }

    for (i, key) in dialogue.start.iter().enumerate() {
        if !keys.contains(key.as_str()) {
            push(
                Severity::Error,
                IssueKind::UnknownTarget,
                Place::Start(i),
                format!("no node `{key}`"),
            );
        }
    }

    let unknown = |target: &Target| match target {
        Target::Node(key) if !keys.contains(key.as_str()) => Some(key.clone()),
        _ => None,
    };
    let mut option_keys = HashSet::new();
    for (n, node) in dialogue.nodes.iter().enumerate() {
        if let Some(key) = node.next.as_ref().and_then(unknown) {
            push(
                Severity::Error,
                IssueKind::UnknownTarget,
                Place::Next(n),
                format!("no node `{key}`"),
            );
        }
        if node.options.is_empty() && node.next.is_none() {
            push(
                Severity::Warning,
                IssueKind::NoWayOn,
                Place::Node(n),
                format!(
                    "node `{}` has no options and no `next`; the talk ends here \
                     (write `next: end` to say so)",
                    node.key
                ),
            );
        }
        for (o, option) in node.options.iter().enumerate() {
            if let Some(key) = unknown(&option.go) {
                push(
                    Severity::Error,
                    IssueKind::UnknownTarget,
                    Place::OptionGo(n, o),
                    format!("no node `{key}`"),
                );
            }
            if option.say.is_none() && option.hint.is_none() {
                push(
                    Severity::Error,
                    IssueKind::NoLabel,
                    Place::Option(n, o),
                    "the option has no `say` and no `hint`".into(),
                );
            }
            // The key that the lowering interns: the `id`, or `<node>#<index>`.
            let key = option
                .id
                .clone()
                .unwrap_or_else(|| format!("{}#{o}", node.key));
            if !option_keys.insert(key.clone()) {
                push(
                    Severity::Error,
                    IssueKind::DuplicateOption,
                    if option.id.is_some() {
                        Place::OptionId(n, o)
                    } else {
                        Place::Option(n, o)
                    },
                    format!(
                        "duplicate option key `{key}` (an option without `id` has the key \
                         `<node>#<index>`)"
                    ),
                );
            }
        }
    }
    issues
}
