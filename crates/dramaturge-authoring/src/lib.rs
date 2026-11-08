//! Authoring-time validation for dialogue graphs.
//!
//! This module provides the [`GraphValidator`] which runs a suite of
//! structural checks on a [`DialogueGraph`] to catch common authoring
//! errors before runtime. Each check produces [`ValidationIssue`]s
//! with a [`ValidationIssueKind`], severity levels ([`ValidationSeverity::Info`],
//! [`ValidationSeverity::Warning`], [`ValidationSeverity::Error`]) and
//! optional source location (node ID, option ID).
//!
//! # Validation Checks
//!
//! Each check reports issues of the kinds in parentheses.
//!
//! 1. **Missing targets** — Options or `next` targets referencing non-existent node IDs
//!    ([`MissingTarget`](ValidationIssueKind::MissingTarget))
//! 2. **Unreachable nodes** — Nodes not reachable from any entry node via BFS
//!    (through option targets and `next` targets)
//!    ([`Unreachable`](ValidationIssueKind::Unreachable),
//!    [`NoEntryNode`](ValidationIssueKind::NoEntryNode),
//!    [`EntryNodeNotFound`](ValidationIssueKind::EntryNodeNotFound))
//! 3. **Dead-end nodes** — Reachable nodes with no options, no `next` and no enter outcomes
//!    ([`DeadEnd`](ValidationIssueKind::DeadEnd))
//! 4. **Self-loops** — Options or `next` targets that point to their own node
//!    ([`SelfLoop`](ValidationIssueKind::SelfLoop))
//! 5. **Cycles** — Nodes participating in cycles, reporting all cycle members
//!    ([`Cycle`](ValidationIssueKind::Cycle)). A cycle of `next` targets
//!    through nodes without options and conditions never stops
//!    ([`NextLoop`](ValidationIssueKind::NextLoop))
//! 6. **Always-false conditions** — Options with conditions that evaluate to always false via constant folding
//!    ([`AlwaysFalse`](ValidationIssueKind::AlwaysFalse))
//! 7. **Orphaned fact references** — Facts referenced in conditions but never set by any outcome
//!    ([`OrphanFact`](ValidationIssueKind::OrphanFact))
//! 8. **Mismatched source node IDs** — Options whose `source_node_id` does not match their parent `DialogueNode`
//!    ([`SourceNodeMismatch`](ValidationIssueKind::SourceNodeMismatch))
//! 9. **Possibly empty text** — Fragment texts of nodes and options where no fragment is guaranteed to show
//!    ([`EmptyText`](ValidationIssueKind::EmptyText)). Host text
//!    (`DialogText::Host`) is not checked: it counts as text. The
//!    [`lines`](dramaturge_model::DialogueNode::lines) of a node are not
//!    checked: a line that resolves to no text is how a line depends on
//!    the state.
//! 10. **Unkeyed progress fragments** — `Once`/`Cycle`/`Sticky` fragments without a key, whose progress resets when a fragment is inserted before them
//!     ([`UnkeyedProgress`](ValidationIssueKind::UnkeyedProgress))
//!
//! # Text pools
//!
//! [`GraphValidator::validate_pools`] checks text pools
//! ([`Pool`](dramaturge_model::Pool)):
//!
//! 1. **Ties** — two candidates of a `best` pool with the same score that
//!    can match the same query ([`PickTie`](ValidationIssueKind::PickTie))
//! 2. **Unkeyed progress** — `once`, `cycle`, `shuffle` and `sticky`
//!    candidates without a key
//!    ([`UnkeyedProgress`](ValidationIssueKind::UnkeyedProgress))
//! 3. **Unknown pools** — a variant names a pool that is not in the pools
//!    ([`UnknownPool`](ValidationIssueKind::UnknownPool))
//! 4. **Bad keys** — a candidate key that is empty, digits only, has a
//!    `/` or repeats on its progress path
//!    ([`BadKey`](ValidationIssueKind::BadKey))
//!
//! The static check does not report a candidate that can never show: that
//! needs the explored states.
//!
//! # Filtering
//!
//! Filter issues by [`ValidationIssue::kind`], not by message text: the
//! messages can change. To switch checks off, pass [`ValidationOptions`] to
//! [`GraphValidator::with_options`]:
//!
//! ```rust
//! use dramaturge_authoring::{GraphValidator, ValidationIssueKind, ValidationOptions};
//!
//! // Talks loop on purpose, and leaves end the talk.
//! let options = ValidationOptions::new()
//!     .allow_cycles()
//!     .allow_dead_ends()
//!     .skip(ValidationIssueKind::SelfLoop);
//! let validator = GraphValidator::with_options(options);
//! # let _ = validator;
//! ```
//!
//! # When to Use
//!
//! Run the validator at authoring time — in CI, in an editor plugin, or as
//! a build step — not at game runtime. Validation catches structural problems
//! early, before they manifest as silent failures during play.
//!
//! # Severity Levels in Practice
//!
//! - [`ValidationSeverity::Error`] — graph is broken; the dialogue cannot
//!   run correctly. Ship-blocker. Examples: option targets a non-existent
//!   node, entry node missing.
//! - [`ValidationSeverity::Warning`] — likely a mistake but not fatal.
//!   Examples: unreachable node, always-false condition, orphaned fact reference.
//! - [`ValidationSeverity::Info`] — informational. Dead-end nodes at the
//!   narrative leaves are common and expected.
//!
//! # Example
//!
//! ```rust
//! use dramaturge_authoring::GraphValidator;
//! use dramaturge_model::DialogueGraph;
//!
//! let graph = DialogueGraph::default();
//! let validator = GraphValidator::new();
//! let issues = validator.validate(&graph);
//! // An empty graph has no issues.
//! assert!(issues.is_empty());
//! ```

use dramaturge_model::{
    Condition, DialogTarget, DialogText, DialogueGraph, DialogueNode, DialogueNodeId,
    DialogueOptionId, FragmentMode, SymbolTable,
};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

mod pools;

/// What a [`ValidationIssue`] is about. One kind per check, or more for a
/// check that finds different problems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ValidationIssueKind {
    /// An option targets a node that does not exist. (Error)
    MissingTarget,
    /// The graph has no entry nodes, so no node is reachable. Reported once
    /// per node. (Warning)
    NoEntryNode,
    /// An entry node does not exist in the graph. Reported once per missing
    /// entry node. (Error)
    EntryNodeNotFound,
    /// A node is not reachable from any entry node. (Warning)
    Unreachable,
    /// A reachable node has no options, no `next` and no enter outcomes.
    /// (Info)
    DeadEnd,
    /// An option targets its own node. (Warning)
    SelfLoop,
    /// A node is part of a cycle. Reported once per node. (Warning)
    Cycle,
    /// An option has a condition that is always false. (Warning)
    AlwaysFalse,
    /// A condition reads a fact that no outcome writes. (Info)
    OrphanFact,
    /// An option's `source_node_id` is not its parent node. (Error)
    SourceNodeMismatch,
    /// A fragment text can resolve to no text. (Warning)
    EmptyText,
    /// A `Once`/`Cycle`/`Sticky` fragment has no key. (Warning)
    UnkeyedProgress,
    /// A node is part of a loop of `next` targets through nodes without
    /// options and without conditions: the talk never stops for a choice.
    /// Reported once per node. (Error; Warning if no entry reaches the node)
    ///
    /// The check reports only loops that never stop, whatever the state.
    /// It does not report a loop through a node with options (they can all
    /// be unavailable) or with conditions (they can block it), and it does
    /// not follow `Return` targets (they depend on the return stack).
    /// `DialogueRuntime::advance` stops such loops at its limit.
    NextLoop,
    /// Two candidates of a `Pick::Best` text pool have the same score and
    /// can match the same query: the first in file order always wins the
    /// tie. Add a test to one of them, or change the order. (Warning)
    PickTie,
    /// A text pool names a pool (`Entry::Named`) that is not in the
    /// pools. (Error)
    UnknownPool,
    /// A text pool candidate key would share progress with another
    /// candidate (see `Pool::bad_key`). Reported once per pool. (Error)
    BadKey,
}

/// Switches for [`GraphValidator`]: which kinds of issues to report.
///
/// All checks are on by default. A skipped kind is not reported.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ValidationOptions {
    skipped: BTreeSet<ValidationIssueKind>,
}

impl ValidationOptions {
    /// Creates options with all checks on.
    pub fn new() -> Self {
        Self::default()
    }

    /// Does not report issues of `kind`.
    pub fn skip(mut self, kind: ValidationIssueKind) -> Self {
        self.skipped.insert(kind);
        self
    }

    /// Reports issues of `kind` again.
    pub fn report(mut self, kind: ValidationIssueKind) -> Self {
        self.skipped.remove(&kind);
        self
    }

    /// Allows cycles: skips [`ValidationIssueKind::Cycle`].
    ///
    /// Self-loops are a separate kind ([`ValidationIssueKind::SelfLoop`]).
    pub fn allow_cycles(self) -> Self {
        self.skip(ValidationIssueKind::Cycle)
    }

    /// Allows dead ends at the leaves: skips [`ValidationIssueKind::DeadEnd`].
    pub fn allow_dead_ends(self) -> Self {
        self.skip(ValidationIssueKind::DeadEnd)
    }

    /// Switches the orphaned-fact check off: skips [`ValidationIssueKind::OrphanFact`].
    pub fn skip_orphan_facts(self) -> Self {
        self.skip(ValidationIssueKind::OrphanFact)
    }

    /// Returns `true` if issues of `kind` are reported.
    pub fn reports(&self, kind: ValidationIssueKind) -> bool {
        !self.skipped.contains(&kind)
    }
}

/// Classification of issue importance.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationSeverity {
    /// Informational note — no action required.
    Info,
    /// Potential issue worth reviewing.
    Warning,
    /// Definite error that should be fixed.
    Error,
}

/// A problem found during graph validation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ValidationIssue {
    /// What the issue is about. Use it to filter issues.
    pub kind: ValidationIssueKind,
    /// How important this issue is.
    pub severity: ValidationSeverity,
    /// Human-readable description of the problem.
    pub message: String,
    /// Optional node ID where the issue was detected.
    pub node_id: Option<DialogueNodeId>,
    /// Optional option ID where the issue was detected.
    pub option_id: Option<DialogueOptionId>,
    /// Optional path in a text pool where the issue was detected, for
    /// example `keeper_remark/2` (pool id, then candidate key or index).
    pub path: Option<String>,
}

impl ValidationIssue {
    /// Creates a new issue without source location.
    pub fn new(
        kind: ValidationIssueKind,
        severity: ValidationSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            severity,
            message: message.into(),
            node_id: None,
            option_id: None,
            path: None,
        }
    }

    /// Builder method — attaches a text pool path to the issue.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Builder method — attaches a node ID to the issue.
    pub fn with_node(mut self, node_id: DialogueNodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    /// Builder method — attaches an option ID to the issue.
    pub fn with_option(mut self, option_id: DialogueOptionId) -> Self {
        self.option_id = Some(option_id);
        self
    }
}

/// Validates a [`DialogueGraph`] for structural correctness.
///
/// Runs validation checks covering graph integrity, reachability,
/// condition validity, and fact reference consistency. [`ValidationOptions`]
/// switch checks off.
///
/// The validator accepts graphs with any host outcome type `H`
/// ([`Outcome::Host`](dramaturge_model::Outcome::Host)). It does not look
/// into host outcomes: a fact that only a host outcome sets is an orphan
/// fact.
///
/// The validator also accepts any host text type `T`
/// ([`DialogText::Host`]). It does not
/// check host text: [`EmptyText`](ValidationIssueKind::EmptyText) counts a
/// host text leaf as never empty, and
/// [`UnkeyedProgress`](ValidationIssueKind::UnkeyedProgress) checks only the
/// fragments around it.
#[derive(Debug, Clone, Default)]
pub struct GraphValidator {
    options: ValidationOptions,
}

impl GraphValidator {
    /// Creates a graph validator with all checks on.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a graph validator with the given options.
    pub fn with_options(options: ValidationOptions) -> Self {
        Self { options }
    }

    /// Returns the options of this validator.
    pub fn options(&self) -> &ValidationOptions {
        &self.options
    }

    /// Runs all validation checks and returns the list of issues found.
    ///
    /// Messages show raw numeric ids (`#3`). Use
    /// [`validate_with_symbols`](Self::validate_with_symbols) to show author keys.
    pub fn validate<H, T>(&self, graph: &DialogueGraph<H, T>) -> Vec<ValidationIssue> {
        self.validate_with_symbols(graph, &SymbolTable::new())
    }

    /// Runs all validation checks, naming nodes and options by the
    /// keys in `symbols` (falling back to `#<number>` for unknown ids).
    ///
    /// Issues of kinds that the [options](ValidationOptions) skip are not
    /// returned.
    pub fn validate_with_symbols<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let on = |kind| self.options.reports(kind);

        let node_map: HashMap<DialogueNodeId, &DialogueNode<H, T>> =
            graph.nodes.iter().map(|n| (n.id, n)).collect();

        use ValidationIssueKind as K;
        if on(K::MissingTarget) {
            self.check_missing_targets(graph, symbols, &node_map, &mut issues);
        }
        // Reports `NoEntryNode`, `EntryNodeNotFound` and `Unreachable`;
        // skipped kinds are dropped below.
        self.check_unreachable_nodes(graph, symbols, &node_map, &mut issues);
        if on(K::DeadEnd) {
            self.check_dead_end_nodes(graph, symbols, &node_map, &mut issues);
        }
        if on(K::SelfLoop) {
            self.check_self_loops(graph, symbols, &node_map, &mut issues);
        }
        if on(K::Cycle) {
            self.check_cycles(graph, symbols, &node_map, &mut issues);
        }
        if on(K::NextLoop) {
            self.check_next_loops(graph, symbols, &node_map, &mut issues);
        }
        if on(K::AlwaysFalse) {
            self.check_always_false_conditions(graph, symbols, &node_map, &mut issues);
        }
        if on(K::OrphanFact) {
            self.check_orphaned_fact_references(graph, &mut issues);
        }
        if on(K::SourceNodeMismatch) {
            self.check_source_node_ids(graph, symbols, &mut issues);
        }
        if on(K::EmptyText) {
            self.check_possibly_empty_text(graph, symbols, &mut issues);
        }
        if on(K::UnkeyedProgress) {
            self.check_unkeyed_progress_fragments(graph, symbols, &mut issues);
        }

        issues.retain(|issue| on(issue.kind));
        issues
    }

    /// Detects options that target non-existent node IDs.
    /// Uses set difference between all target IDs and existing node IDs.
    fn check_missing_targets<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let existing_ids: HashSet<DialogueNodeId> = node_map.keys().copied().collect();

        for node in &graph.nodes {
            if let Some(DialogTarget::Node(target_id)) = &node.next
                && !existing_ids.contains(target_id)
            {
                issues.push(
                    ValidationIssue::new(
                        ValidationIssueKind::MissingTarget,
                        ValidationSeverity::Error,
                        format!(
                            "Next of node {} targets nonexistent node {}",
                            symbols.display_node(node.id),
                            symbols.display_node(*target_id)
                        ),
                    )
                    .with_node(node.id),
                );
            }
            for option in &node.options {
                if let DialogTarget::Node(target_id) = &option.target
                    && !existing_ids.contains(target_id)
                {
                    issues.push(
                        ValidationIssue::new(
                            ValidationIssueKind::MissingTarget,
                            ValidationSeverity::Error,
                            format!(
                                "Option {} in node {} targets nonexistent node {}",
                                symbols.display_option(option.id),
                                symbols.display_node(node.id),
                                symbols.display_node(*target_id)
                            ),
                        )
                        .with_node(node.id)
                        .with_option(option.id),
                    );
                }
            }
        }
    }

    /// Detects nodes not reachable from any entry node via BFS traversal.
    fn check_unreachable_nodes<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        if graph.entry_nodes.is_empty() {
            for node in &graph.nodes {
                issues.push(
                    ValidationIssue::new(
                        ValidationIssueKind::NoEntryNode,
                        ValidationSeverity::Warning,
                        "No entry node defined; all nodes are unreachable",
                    )
                    .with_node(node.id),
                );
            }
            return;
        }

        // A duplicated missing entry is reported once.
        let mut reported = HashSet::new();
        for entry_id in &graph.entry_nodes {
            if !node_map.contains_key(entry_id) && reported.insert(*entry_id) {
                issues.push(ValidationIssue::new(
                    ValidationIssueKind::EntryNodeNotFound,
                    ValidationSeverity::Error,
                    format!(
                        "Entry node {} does not exist in the graph",
                        symbols.display_node(*entry_id)
                    ),
                ));
            }
        }

        // Without an existing entry node, every node would be unreachable;
        // the `EntryNodeNotFound` errors already say why.
        if !graph.entry_nodes.iter().any(|id| node_map.contains_key(id)) {
            return;
        }

        let visited = reachable_from_entries(graph, node_map);
        for node in &graph.nodes {
            if !visited.contains(&node.id) {
                issues.push(
                    ValidationIssue::new(
                        ValidationIssueKind::Unreachable,
                        ValidationSeverity::Warning,
                        format!(
                            "Node {} is unreachable from the entry nodes",
                            symbols.display_node(node.id)
                        ),
                    )
                    .with_node(node.id),
                );
            }
        }
    }

    /// Detects nodes with no options and no enter outcomes that are reachable
    /// from an entry node. These are potential dead ends in the conversation flow.
    fn check_dead_end_nodes<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let mut visited: Vec<_> = reachable_from_entries(graph, node_map)
            .into_iter()
            .collect();
        visited.sort();

        for node_id in &visited {
            if let Some(node) = node_map.get(node_id) {
                let has_options = !node.options.is_empty();
                let has_enter_outcomes = !node.outcomes_on_enter.is_empty();
                let has_next = node.next.is_some();

                if !has_options && !has_enter_outcomes && !has_next {
                    issues.push(
                        ValidationIssue::new(
                            ValidationIssueKind::DeadEnd,
                            ValidationSeverity::Info,
                            format!(
                                "Node {} has no options, next or enter outcomes (potential dead end)",
                                symbols.display_node(node.id)
                            ),
                        )
                        .with_node(node.id),
                    );
                }
            }
        }
    }

    /// Detects options whose target is the same node as their parent (self-loop).
    fn check_self_loops<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        _node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        for node in &graph.nodes {
            if let Some(DialogTarget::Node(target_id)) = &node.next
                && *target_id == node.id
            {
                issues.push(
                    ValidationIssue::new(
                        ValidationIssueKind::SelfLoop,
                        ValidationSeverity::Warning,
                        format!(
                            "Next of node {} targets the same node (self-loop)",
                            symbols.display_node(node.id)
                        ),
                    )
                    .with_node(node.id),
                );
            }
            for option in &node.options {
                if let DialogTarget::Node(target_id) = &option.target
                    && *target_id == node.id
                {
                    issues.push(
                        ValidationIssue::new(
                            ValidationIssueKind::SelfLoop,
                            ValidationSeverity::Warning,
                            format!(
                                "Option {} in node {} targets the same node (self-loop)",
                                symbols.display_option(option.id),
                                symbols.display_node(node.id)
                            ),
                        )
                        .with_node(node.id)
                        .with_option(option.id),
                    );
                }
            }
        }
    }

    /// Detects cycles using DFS with a recursion stack that tracks path order.
    /// When a back-edge is found, all nodes in the cycle are reported.
    fn check_cycles<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        if graph.entry_nodes.is_empty() {
            return;
        }

        let mut visited = HashSet::new();
        let mut cycle_nodes = HashSet::new();

        fn dfs<H, T>(
            current: DialogueNodeId,
            node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
            visited: &mut HashSet<DialogueNodeId>,
            stack: &mut Vec<DialogueNodeId>,
            cycle_nodes: &mut HashSet<DialogueNodeId>,
        ) {
            if let Some(pos) = stack.iter().position(|&n| n == current) {
                for &node_id in &stack[pos..] {
                    cycle_nodes.insert(node_id);
                }
                return;
            }
            if visited.contains(&current) {
                return;
            }

            visited.insert(current);
            stack.push(current);

            if let Some(node) = node_map.get(&current) {
                for target_id in node_targets(node) {
                    dfs(target_id, node_map, visited, stack, cycle_nodes);
                }
            }

            stack.pop();
        }

        for &entry_id in &graph.entry_nodes {
            dfs(
                entry_id,
                node_map,
                &mut visited,
                &mut Vec::new(),
                &mut cycle_nodes,
            );
        }

        for node in &graph.nodes {
            if !visited.contains(&node.id) {
                dfs(
                    node.id,
                    node_map,
                    &mut visited,
                    &mut Vec::new(),
                    &mut cycle_nodes,
                );
            }
        }

        for node_id in &cycle_nodes {
            if let Some(node) = node_map.get(node_id) {
                issues.push(
                    ValidationIssue::new(
                        ValidationIssueKind::Cycle,
                        ValidationSeverity::Warning,
                        format!("Node {} is part of a cycle", symbols.display_node(node.id)),
                    )
                    .with_node(node.id),
                );
            }
        }
    }

    /// Detects loops of `next` targets through nodes without options and
    /// without conditions. Such a loop never stops for a choice, whatever
    /// the state: `DialogueRuntime::advance` runs to its limit. Each node
    /// has at most one `next`, so following `next` from each node finds
    /// every such loop.
    ///
    /// The check is sound, not complete. Options can be unavailable and
    /// conditions can block a node only depending on the state, and a
    /// `Return` target depends on the return stack. The check can not
    /// decide these statically, so it stops at such nodes and targets.
    fn check_next_loops<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        /// The `next` node of a node that always continues to it, if any.
        fn auto_next<H, T>(
            id: DialogueNodeId,
            node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
        ) -> Option<DialogueNodeId> {
            let node = node_map.get(&id)?;
            if !node.options.is_empty() || !node.conditions.is_empty() {
                return None;
            }
            match node.next {
                Some(DialogTarget::Node(next)) if node_map.contains_key(&next) => Some(next),
                _ => None,
            }
        }

        let mut done: HashSet<DialogueNodeId> = HashSet::new();
        let mut in_loop: BTreeSet<DialogueNodeId> = BTreeSet::new();
        for node in &graph.nodes {
            let mut path: Vec<DialogueNodeId> = Vec::new();
            let mut current = Some(node.id);
            while let Some(id) = current {
                if done.contains(&id) {
                    break;
                }
                if let Some(pos) = path.iter().position(|&p| p == id) {
                    in_loop.extend(&path[pos..]);
                    break;
                }
                path.push(id);
                current = auto_next(id, node_map);
            }
            done.extend(path);
        }

        if in_loop.is_empty() {
            return;
        }
        // As with `Cycle`, a loop that `advance` can never enter is only a
        // warning; `Unreachable`, `EntryNodeNotFound` or `NoEntryNode` says why.
        let reachable = reachable_from_entries(graph, node_map);
        for node_id in in_loop {
            let severity = if reachable.contains(&node_id) {
                ValidationSeverity::Error
            } else {
                ValidationSeverity::Warning
            };
            issues.push(
                ValidationIssue::new(
                    ValidationIssueKind::NextLoop,
                    severity,
                    format!(
                        "Node {} is part of a loop of next targets without a choice",
                        symbols.display_node(node_id)
                    ),
                )
                .with_node(node_id),
            );
        }
    }

    /// Detects options with conditions that are always false.
    /// Uses constant folding to properly evaluate composite conditions:
    /// `Any([Never, Always])` is satisfiable, `Not(Never)` is always true.
    fn check_always_false_conditions<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        _node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        fn is_always_false(cond: &Condition) -> bool {
            match cond {
                Condition::Never => true,
                Condition::Always => false,
                Condition::All(conds) => conds.iter().any(is_always_false),
                Condition::Any(conds) => conds.iter().all(is_always_false),
                Condition::Not(inner) => is_always_true(inner),
                _ => false,
            }
        }

        fn is_always_true(cond: &Condition) -> bool {
            match cond {
                Condition::Always => true,
                Condition::Never => false,
                Condition::All(conds) => conds.iter().all(is_always_true),
                Condition::Any(conds) => conds.iter().any(is_always_true),
                Condition::Not(inner) => is_always_false(inner),
                _ => false,
            }
        }

        for node in &graph.nodes {
            for option in &node.options {
                if option.conditions.iter().any(is_always_false) {
                    issues.push(
                        ValidationIssue::new(
                            ValidationIssueKind::AlwaysFalse,
                            ValidationSeverity::Warning,
                            format!(
                                "Option {} in node {} has an always-false condition (unreachable option)",
                                symbols.display_option(option.id),
                                symbols.display_node(node.id)
                            ),
                        )
                        .with_node(node.id)
                        .with_option(option.id),
                    );
                }
            }
        }
    }

    /// Detects fact IDs referenced in conditions but never set by any outcome.
    /// Collects fact IDs from conditions only (not outcomes), then flags those
    /// never set by any `SetFact`, `AddFact`, `StartQuest`, or `AdvanceQuest` outcome.
    fn check_orphaned_fact_references<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        struct FactRef {
            fact_id: String,
            node_id: DialogueNodeId,
            option_id: Option<DialogueOptionId>,
        }

        let mut condition_fact_refs: Vec<FactRef> = Vec::new();
        let mut used_fact_ids: HashSet<String> = HashSet::new();

        fn collect_fact_ids_from_condition(
            cond: &Condition,
            refs: &mut Vec<FactRef>,
            node_id: DialogueNodeId,
            option_id: Option<DialogueOptionId>,
        ) {
            match cond {
                Condition::FactEquals { fact_id, .. } | Condition::FactExists { fact_id, .. } => {
                    refs.push(FactRef {
                        fact_id: fact_id.0.to_string(),
                        node_id,
                        option_id,
                    });
                }
                Condition::FactAtLeast { fact_id, .. } | Condition::FactAtMost { fact_id, .. } => {
                    refs.push(FactRef {
                        fact_id: fact_id.0.to_string(),
                        node_id,
                        option_id,
                    });
                }
                Condition::All(conds) | Condition::Any(conds) => {
                    for c in conds {
                        collect_fact_ids_from_condition(c, refs, node_id, option_id);
                    }
                }
                Condition::Not(inner) => {
                    collect_fact_ids_from_condition(inner, refs, node_id, option_id);
                }
                Condition::Custom { args, .. } => {
                    for arg in args {
                        if let dramaturge_model::FactValue::Text(id) = arg {
                            refs.push(FactRef {
                                fact_id: id.to_string(),
                                node_id,
                                option_id,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        for node in &graph.nodes {
            for cond in &node.conditions {
                collect_fact_ids_from_condition(cond, &mut condition_fact_refs, node.id, None);
            }
            for option in &node.options {
                for cond in &option.conditions {
                    collect_fact_ids_from_condition(
                        cond,
                        &mut condition_fact_refs,
                        node.id,
                        Some(option.id),
                    );
                }
            }
        }

        for node in &graph.nodes {
            for outcome in &node.outcomes_on_enter {
                match outcome {
                    dramaturge_model::Outcome::SetFact { fact, .. } => {
                        used_fact_ids.insert(fact.id.0.to_string());
                    }
                    dramaturge_model::Outcome::AddFact { fact_id, .. } => {
                        used_fact_ids.insert(fact_id.0.to_string());
                    }
                    dramaturge_model::Outcome::StartQuest { quest_id } => {
                        used_fact_ids.insert(format!("quest_started_{quest_id}"));
                    }
                    dramaturge_model::Outcome::AdvanceQuest { quest_id, .. } => {
                        used_fact_ids.insert(format!("quest_stage_{quest_id}"));
                    }
                    _ => {}
                }
            }
            for option in &node.options {
                for outcome in &option.outcomes {
                    match outcome {
                        dramaturge_model::Outcome::SetFact { fact, .. } => {
                            used_fact_ids.insert(fact.id.0.to_string());
                        }
                        dramaturge_model::Outcome::AddFact { fact_id, .. } => {
                            used_fact_ids.insert(fact_id.0.to_string());
                        }
                        dramaturge_model::Outcome::StartQuest { quest_id } => {
                            used_fact_ids.insert(format!("quest_started_{quest_id}"));
                        }
                        dramaturge_model::Outcome::AdvanceQuest { quest_id, .. } => {
                            used_fact_ids.insert(format!("quest_stage_{quest_id}"));
                        }
                        _ => {}
                    }
                }
            }
        }

        let mut reported = HashSet::new();
        for fact_ref in &condition_fact_refs {
            if !used_fact_ids.contains(&fact_ref.fact_id) && !reported.contains(&fact_ref.fact_id) {
                reported.insert(fact_ref.fact_id.clone());
                let mut issue = ValidationIssue::new(
                    ValidationIssueKind::OrphanFact,
                    ValidationSeverity::Info,
                    format!(
                        "Fact ID '{}' is referenced in conditions but never set by any outcome",
                        fact_ref.fact_id,
                    ),
                )
                .with_node(fact_ref.node_id);
                if let Some(option_id) = fact_ref.option_id {
                    issue = issue.with_option(option_id);
                }
                issues.push(issue);
            }
        }
    }

    /// Detects fragment texts that can resolve to no text at all: no fragment
    /// is guaranteed to show (unconditional, repeatable mode) with non-empty text.
    fn check_possibly_empty_text<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        issues: &mut Vec<ValidationIssue>,
    ) {
        fn always_true(cond: &Condition) -> bool {
            match cond {
                Condition::Always => true,
                Condition::All(conds) => conds.iter().all(always_true),
                Condition::Any(conds) => conds.iter().any(always_true),
                _ => false,
            }
        }

        /// Returns `true` if `text` always resolves to some non-empty text.
        fn never_empty<T>(text: &DialogText<T>) -> bool {
            match text {
                DialogText::Inline(s) => !s.is_empty(),
                DialogText::LocalizedKey(_) => true,
                DialogText::Fragments(fragments) => fragments.iter().any(|f| {
                    let repeatable = matches!(
                        f.mode,
                        FragmentMode::Always
                            | FragmentMode::Cycle
                            | FragmentMode::Shuffle
                            | FragmentMode::Sticky
                    );
                    repeatable
                        && f.condition.as_ref().is_none_or(always_true)
                        && !f.variants.is_empty()
                        && f.variants.iter().all(never_empty)
                }),
                // Host text belongs to the host; it is not checked.
                DialogText::Host(_) => true,
                // Text kinds this validator does not know are not checked.
                _ => true,
            }
        }

        fn is_fragments<T>(t: &DialogText<T>) -> bool {
            matches!(t, DialogText::Fragments(_))
        }
        for node in &graph.nodes {
            if is_fragments(&node.text) && !never_empty(&node.text) {
                issues.push(
                    ValidationIssue::new(
                        ValidationIssueKind::EmptyText,
                        ValidationSeverity::Warning,
                        format!(
                            "Text of node {} can resolve to empty text",
                            symbols.display_node(node.id)
                        ),
                    )
                    .with_node(node.id),
                );
            }
            for option in &node.options {
                if is_fragments(&option.text) && !never_empty(&option.text) {
                    issues.push(
                        ValidationIssue::new(
                            ValidationIssueKind::EmptyText,
                            ValidationSeverity::Warning,
                            format!(
                                "Text of option {} in node {} can resolve to empty text",
                                symbols.display_option(option.id),
                                symbols.display_node(node.id)
                            ),
                        )
                        .with_node(node.id)
                        .with_option(option.id),
                    );
                }
            }
        }
    }

    /// Detects `Once`, `Cycle` and `Sticky` fragments without a key. Their
    /// progress is tracked by position, so inserting a fragment before them
    /// resets it.
    fn check_unkeyed_progress_fragments<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        issues: &mut Vec<ValidationIssue>,
    ) {
        fn count_unkeyed<T>(text: &DialogText<T>) -> usize {
            match text {
                DialogText::Inline(_) | DialogText::LocalizedKey(_) | DialogText::Host(_) => 0,
                DialogText::Fragments(fragments) => fragments
                    .iter()
                    .map(|f| {
                        let tracked = matches!(
                            f.mode,
                            FragmentMode::Once | FragmentMode::Cycle | FragmentMode::Sticky
                        );
                        usize::from(tracked && f.key.is_none())
                            + f.variants.iter().map(count_unkeyed).sum::<usize>()
                    })
                    .sum(),
                _ => 0,
            }
        }

        const HINT: &str = "has no key; inserting a fragment before it resets its progress";
        for node in &graph.nodes {
            for _ in 0..count_unkeyed(&node.text) {
                issues.push(
                    ValidationIssue::new(
                        ValidationIssueKind::UnkeyedProgress,
                        ValidationSeverity::Warning,
                        format!(
                            "A fragment in the text of node {} {HINT}",
                            symbols.display_node(node.id)
                        ),
                    )
                    .with_node(node.id),
                );
            }
            for (index, line) in node.lines.iter().enumerate() {
                for _ in 0..count_unkeyed(&line.text) {
                    issues.push(
                        ValidationIssue::new(
                            ValidationIssueKind::UnkeyedProgress,
                            ValidationSeverity::Warning,
                            format!(
                                "A fragment in line {} of node {} {HINT}",
                                index + 1,
                                symbols.display_node(node.id)
                            ),
                        )
                        .with_node(node.id),
                    );
                }
            }
            for option in &node.options {
                for _ in 0..count_unkeyed(&option.text) {
                    issues.push(
                        ValidationIssue::new(
                            ValidationIssueKind::UnkeyedProgress,
                            ValidationSeverity::Warning,
                            format!(
                                "A fragment in the text of option {} in node {} {HINT}",
                                symbols.display_option(option.id),
                                symbols.display_node(node.id)
                            ),
                        )
                        .with_node(node.id)
                        .with_option(option.id),
                    );
                }
            }
        }
    }

    /// Detects options whose `source_node_id` does not match their parent node's ID.
    /// A mismatch corrupts runtime event history recorded by `DefaultRuntime::choose_option`.
    fn check_source_node_ids<H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        symbols: &SymbolTable,
        issues: &mut Vec<ValidationIssue>,
    ) {
        for node in &graph.nodes {
            for option in &node.options {
                if option.source_node_id != node.id {
                    issues.push(
                        ValidationIssue::new(
                            ValidationIssueKind::SourceNodeMismatch,
                            ValidationSeverity::Error,
                            format!(
                                "Option {} in node {} has wrong source node {} (parent is node {})",
                                symbols.display_option(option.id),
                                symbols.display_node(node.id),
                                symbols.display_node(option.source_node_id),
                                symbols.display_node(node.id)
                            ),
                        )
                        .with_node(node.id)
                        .with_option(option.id),
                    );
                }
            }
        }
    }
}

/// Returns the node targets of `node`: its `next` target, then the targets
/// of its options. Only `DialogTarget::Node` targets count.
fn node_targets<H, T>(node: &DialogueNode<H, T>) -> impl Iterator<Item = DialogueNodeId> + '_ {
    node.next
        .iter()
        .chain(node.options.iter().map(|option| &option.target))
        .filter_map(|target| match target {
            DialogTarget::Node(id) => Some(*id),
            _ => None,
        })
}

/// Returns the nodes reachable from the entry nodes (breadth-first, through
/// `DialogTarget::Node` targets of options and `next`). Only ids with a node in `node_map` are
/// returned: missing entry nodes and dangling option targets are left out.
fn reachable_from_entries<H, T>(
    graph: &DialogueGraph<H, T>,
    node_map: &HashMap<DialogueNodeId, &DialogueNode<H, T>>,
) -> HashSet<DialogueNodeId> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    for &entry_id in &graph.entry_nodes {
        if node_map.contains_key(&entry_id) && visited.insert(entry_id) {
            queue.push_back(entry_id);
        }
    }

    while let Some(current_id) = queue.pop_front() {
        if let Some(node) = node_map.get(&current_id) {
            for target_id in node_targets(node) {
                if node_map.contains_key(&target_id) && visited.insert(target_id) {
                    queue.push_back(target_id);
                }
            }
        }
    }
    visited
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;
    use dramaturge_model::{
        CharacterId, DialogIntentions, DialogText, DialogueGraph, DialogueNode, DialogueOption,
        EntityId, FactId, FactScope, Outcome,
    };

    fn make_node(id: u64, options: Vec<DialogueOption>) -> DialogueNode {
        DialogueNode {
            id: DialogueNodeId::new(id),
            speaker: CharacterId::new(EntityId::new(1)),
            text: DialogText::inline("test"),
            conditions: vec![],
            outcomes_on_enter: vec![],
            options,
            lines: vec![],
            next: None,
        }
    }

    fn make_option(
        id: u64,
        target: DialogTarget,
        source_node_id: DialogueNodeId,
    ) -> DialogueOption {
        DialogueOption {
            id: DialogueOptionId::new(id),
            text: DialogText::inline("opt"),
            tags: vec![],
            intentions: DialogIntentions::new(),
            conditions: vec![],
            outcomes: vec![],
            target,
            source_node_id,
            navigation_hint: None,
        }
    }

    fn graph(nodes: Vec<DialogueNode>, entry: Option<u64>) -> DialogueGraph {
        DialogueGraph {
            nodes,
            entry_nodes: entry.map(DialogueNodeId::new).into_iter().collect(),
        }
    }

    /// Returns the issues of `kind` that `validator` reports for `graph`.
    fn of_kind(
        validator: &GraphValidator,
        graph: &DialogueGraph,
        kind: ValidationIssueKind,
    ) -> Vec<ValidationIssue> {
        validator
            .validate(graph)
            .into_iter()
            .filter(|i| i.kind == kind)
            .collect()
    }

    /// Checks that `graph` has an issue of `kind` with `severity`.
    fn assert_kind(graph: &DialogueGraph, kind: ValidationIssueKind, severity: ValidationSeverity) {
        let found = of_kind(&GraphValidator::new(), graph, kind);
        check!(!found.is_empty(), "no {kind:?} issue");
        check!(found.iter().all(|i| i.severity == severity), "{found:?}");
    }

    fn fact_exists(id: &str) -> Condition {
        Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new(id),
        }
    }

    /// A graph that has an issue of every kind.
    fn graph_with_every_kind() -> DialogueGraph {
        use dramaturge_model::TextFragment;
        // 1 -> 2 -> 1 (cycle), 1 -> 1 (self-loop), 1 -> 99 (missing),
        // 1 -> 3 (dead end), 4 unreachable.
        let mut always_false = make_option(5, DialogTarget::node(2), DialogueNodeId::new(1));
        always_false.conditions = vec![Condition::Never, fact_exists("orphan")];
        let node1 = make_node(
            1,
            vec![
                make_option(1, DialogTarget::node(2), DialogueNodeId::new(1)),
                make_option(2, DialogTarget::node(1), DialogueNodeId::new(1)),
                make_option(3, DialogTarget::node(99), DialogueNodeId::new(1)),
                make_option(4, DialogTarget::node(3), DialogueNodeId::new(7)),
                always_false,
            ],
        );
        let node2 = make_node(
            2,
            vec![make_option(
                6,
                DialogTarget::node(1),
                DialogueNodeId::new(2),
            )],
        );
        let mut node3 = make_node(3, vec![]);
        node3.text = DialogText::fragments(vec![TextFragment::with_mode(
            FragmentMode::Once,
            vec![DialogText::inline("Once.")],
        )]);
        let node4 = make_node(4, vec![]);
        graph(vec![node1, node2, node3, node4], Some(1))
    }

    #[test]
    fn every_issue_has_its_kind() {
        use ValidationIssueKind as K;
        use ValidationSeverity as S;
        let g = graph_with_every_kind();
        assert_kind(&g, K::MissingTarget, S::Error);
        assert_kind(&g, K::Unreachable, S::Warning);
        assert_kind(&g, K::DeadEnd, S::Info);
        assert_kind(&g, K::SelfLoop, S::Warning);
        assert_kind(&g, K::Cycle, S::Warning);
        assert_kind(&g, K::AlwaysFalse, S::Warning);
        assert_kind(&g, K::OrphanFact, S::Info);
        assert_kind(&g, K::SourceNodeMismatch, S::Error);
        assert_kind(&g, K::EmptyText, S::Warning);
        assert_kind(&g, K::UnkeyedProgress, S::Warning);

        let no_entry = graph(vec![make_node(1, vec![])], None);
        assert_kind(&no_entry, K::NoEntryNode, S::Warning);
        let bad_entry = graph(vec![make_node(1, vec![])], Some(5));
        assert_kind(&bad_entry, K::EntryNodeNotFound, S::Error);
    }

    #[test]
    fn reachability_starts_from_every_entry_node() {
        use ValidationIssueKind as K;
        // 1 -> 2, 3 -> 4; 5 is not reachable.
        let mut g = graph(
            vec![
                make_node(
                    1,
                    vec![make_option(
                        1,
                        DialogTarget::node(2),
                        DialogueNodeId::new(1),
                    )],
                ),
                make_node(2, vec![]),
                make_node(
                    3,
                    vec![make_option(
                        2,
                        DialogTarget::node(4),
                        DialogueNodeId::new(3),
                    )],
                ),
                make_node(4, vec![]),
                make_node(5, vec![]),
            ],
            Some(1),
        );
        g.entry_nodes.push(DialogueNodeId::new(3));

        let validator = GraphValidator::new();
        let unreachable: Vec<_> = of_kind(&validator, &g, K::Unreachable)
            .into_iter()
            .map(|i| i.node_id.map(|n| n.0))
            .collect();
        check!(unreachable == vec![Some(5)]);

        // Dead ends are searched from every entry node, too.
        let mut dead_ends: Vec<_> = of_kind(&validator, &g, K::DeadEnd)
            .into_iter()
            .filter_map(|i| i.node_id.map(|n| n.0))
            .collect();
        dead_ends.sort();
        check!(dead_ends == vec![2, 4]);
    }

    #[test]
    fn missing_entry_node_among_others() {
        use ValidationIssueKind as K;
        let mut g = graph(vec![make_node(1, vec![]), make_node(2, vec![])], Some(1));
        g.entry_nodes.push(DialogueNodeId::new(9));
        g.entry_nodes.push(DialogueNodeId::new(2));

        // The missing entry is listed twice but reported once.
        g.entry_nodes.push(DialogueNodeId::new(9));

        let validator = GraphValidator::new();
        check!(of_kind(&validator, &g, K::EntryNodeNotFound).len() == 1);
        check!(of_kind(&validator, &g, K::Unreachable).is_empty());
        check!(of_kind(&validator, &g, K::NoEntryNode).is_empty());
    }

    #[test]
    fn validates_graphs_with_host_outcomes() {
        use ValidationIssueKind as K;
        use dramaturge_model::{FactValue, Outcome};

        #[derive(Debug, Clone, PartialEq)]
        enum Effect {
            SetsDoorOpen,
        }

        let mut node = DialogueNode::<Effect>::typed(
            1,
            CharacterId::new(EntityId::new(1)),
            DialogText::inline("Hi."),
        );
        node.outcomes_on_enter
            .push(Outcome::Host(Effect::SetsDoorOpen));
        let mut option =
            DialogueOption::typed(1, DialogText::inline("Go."), DialogTarget::End, node.id);
        option.conditions.push(Condition::FactEquals {
            scope: dramaturge_model::FactScope::World,
            fact_id: dramaturge_model::FactId::new("door_open"),
            value: FactValue::Bool(true),
        });
        node.options.push(option);
        let mut g = DialogueGraph::typed();
        g.nodes.push(node);
        g.entry_nodes.push(DialogueNodeId::new(1));

        let issues = GraphValidator::new().validate(&g);
        check!(issues.iter().all(|i| i.kind == K::OrphanFact), "{issues:?}");
        check!(issues.len() == 1);
    }

    fn next_graph(nexts: &[(u64, Option<DialogTarget>)]) -> DialogueGraph {
        let mut g = DialogueGraph::new().with_entry_node(DialogueNodeId::new(nexts[0].0));
        for (id, next) in nexts {
            let mut node = make_node(*id, vec![]);
            node.next = next.clone();
            g.nodes.push(node);
        }
        g
    }

    #[test]
    fn next_counts_for_reachability_and_dead_ends() {
        use ValidationIssueKind as K;
        let g = next_graph(&[
            (1, Some(DialogTarget::node(2))),
            (2, Some(DialogTarget::End)),
            (3, None),
        ]);
        let validator = GraphValidator::new();
        let unreachable = of_kind(&validator, &g, K::Unreachable);
        check!(unreachable.len() == 1);
        check!(unreachable[0].node_id == Some(DialogueNodeId::new(3)));
        // Nodes with `next` are not dead ends; node 3 is not reachable.
        check!(of_kind(&validator, &g, K::DeadEnd).is_empty());

        let g = next_graph(&[(1, Some(DialogTarget::node(2))), (2, None)]);
        let dead = of_kind(&validator, &g, K::DeadEnd);
        check!(dead.len() == 1);
        check!(dead[0].node_id == Some(DialogueNodeId::new(2)));
    }

    #[test]
    fn next_targets_are_checked() {
        use ValidationIssueKind as K;
        let validator = GraphValidator::new();

        let g = next_graph(&[(1, Some(DialogTarget::node(9)))]);
        let missing = of_kind(&validator, &g, K::MissingTarget);
        check!(missing.len() == 1);
        check!(missing[0].node_id == Some(DialogueNodeId::new(1)));
        check!(missing[0].option_id.is_none());
        check!(missing[0].message.contains("Next of node #1"));

        let g = next_graph(&[(1, Some(DialogTarget::node(1)))]);
        check!(of_kind(&validator, &g, K::SelfLoop).len() == 1);
        check!(of_kind(&validator, &g, K::NextLoop).len() == 1);
    }

    #[test]
    fn detects_next_loops_without_a_choice() {
        use ValidationIssueKind as K;
        let validator = GraphValidator::new();

        // 1 -> 2 -> 3 -> 2: nodes 2 and 3 loop; 1 only leads in.
        let g = next_graph(&[
            (1, Some(DialogTarget::node(2))),
            (2, Some(DialogTarget::node(3))),
            (3, Some(DialogTarget::node(2))),
        ]);
        let loops = of_kind(&validator, &g, K::NextLoop);
        let ids: Vec<_> = loops.iter().filter_map(|i| i.node_id).collect();
        check!(ids == vec![DialogueNodeId::new(2), DialogueNodeId::new(3)]);
        check!(loops[0].severity == ValidationSeverity::Error);
        check!(of_kind(&validator, &g, K::Cycle).len() == 2);

        // An option on the loop gives the talk a place to stop.
        let mut g = g;
        let option = make_option(1, DialogTarget::End, DialogueNodeId::new(3));
        g.nodes[2].options.push(option);
        check!(of_kind(&validator, &g, K::NextLoop).is_empty());
    }

    #[test]
    fn unreachable_next_loops_are_warnings() {
        use ValidationIssueKind as K;
        let validator = GraphValidator::new();
        // Entry 1 ends; 2 <-> 3 loop but can not be entered.
        let g = next_graph(&[
            (1, Some(DialogTarget::End)),
            (2, Some(DialogTarget::node(3))),
            (3, Some(DialogTarget::node(2))),
        ]);
        let loops = of_kind(&validator, &g, K::NextLoop);
        check!(loops.len() == 2);
        check!(
            loops
                .iter()
                .all(|i| i.severity == ValidationSeverity::Warning)
        );
    }

    #[test]
    fn next_loop_check_reports_only_loops_that_never_stop() {
        use ValidationIssueKind as K;
        let validator = GraphValidator::new();
        let looping = || {
            next_graph(&[
                (1, Some(DialogTarget::node(2))),
                (2, Some(DialogTarget::node(1))),
            ])
        };
        check!(of_kind(&validator, &looping(), K::NextLoop).len() == 2);

        // A condition can block a node, so the loop can stop.
        let mut g = looping();
        g.nodes[1].conditions.push(Condition::Never);
        check!(of_kind(&validator, &g, K::NextLoop).is_empty());

        // An option, also one that is never available, is not followed.
        let mut g = looping();
        let mut option = make_option(1, DialogTarget::End, DialogueNodeId::new(2));
        option.conditions.push(Condition::Never);
        g.nodes[1].options.push(option);
        check!(of_kind(&validator, &g, K::NextLoop).is_empty());

        // `Return` depends on the return stack; it is not followed.
        let g = next_graph(&[
            (1, Some(DialogTarget::node(2))),
            (2, Some(DialogTarget::Return)),
        ]);
        check!(of_kind(&validator, &g, K::NextLoop).is_empty());
    }

    #[test]
    fn checks_progress_keys_in_lines() {
        use ValidationIssueKind as K;
        use dramaturge_model::TextFragment;

        let mut g = next_graph(&[(1, Some(DialogTarget::End))]);
        g.nodes[0] = g.nodes[0].clone().with_line(
            CharacterId::new(EntityId::new(2)),
            DialogText::fragments(vec![TextFragment::with_mode(
                FragmentMode::Once,
                vec![DialogText::inline("Only once.")],
            )]),
        );
        let validator = GraphValidator::new();
        let unkeyed = of_kind(&validator, &g, K::UnkeyedProgress);
        check!(unkeyed.len() == 1);
        check!(unkeyed[0].message.contains("line 1 of node #1"));
        // A line may resolve to no text.
        check!(of_kind(&validator, &g, K::EmptyText).is_empty());
    }

    #[test]
    fn does_not_check_host_text() {
        use ValidationIssueKind as K;
        use dramaturge_model::TextFragment;

        /// Host markup; an empty string is valid host text.
        #[derive(Debug, Clone, PartialEq)]
        struct Markup(&'static str);

        let speaker = CharacterId::new(EntityId::new(1));
        let mut node = DialogueNode::<(), Markup>::typed(1, speaker, DialogText::host(Markup("")));
        node.options.push(DialogueOption::typed(
            1,
            DialogText::Fragments(vec![TextFragment::always(DialogText::host(Markup("")))]),
            DialogTarget::End,
            node.id,
        ));
        let mut g = DialogueGraph::typed().with_entry_node(node.id);
        g.nodes.push(node);

        let issues = GraphValidator::new().validate(&g);
        check!(issues.is_empty(), "{issues:?}");

        // The fragments around host text are still checked.
        g.nodes[0].text = DialogText::Fragments(vec![TextFragment::with_mode(
            FragmentMode::Once,
            vec![DialogText::host(Markup("Once."))],
        )]);
        let kinds: Vec<_> = GraphValidator::new()
            .validate(&g)
            .into_iter()
            .map(|i| i.kind)
            .collect();
        check!(kinds == vec![K::EmptyText, K::UnkeyedProgress]);
    }

    #[test]
    fn issue_kinds_match_messages() {
        use ValidationIssueKind as K;
        let g = graph_with_every_kind();
        let issues = GraphValidator::new().validate(&g);
        for (kind, text) in [
            (K::MissingTarget, "targets nonexistent node"),
            (K::Unreachable, "is unreachable"),
            (K::DeadEnd, "potential dead end"),
            (K::SelfLoop, "self-loop"),
            (K::Cycle, "is part of a cycle"),
            (K::AlwaysFalse, "always-false condition"),
            (K::OrphanFact, "never set by any outcome"),
            (K::SourceNodeMismatch, "wrong source node"),
            (K::EmptyText, "can resolve to empty text"),
            (K::UnkeyedProgress, "has no key"),
        ] {
            for issue in issues.iter().filter(|i| i.kind == kind) {
                check!(issue.message.contains(text), "{kind:?}: {}", issue.message);
            }
        }
    }

    #[test]
    fn options_skip_kinds() {
        use ValidationIssueKind as K;
        let g = graph_with_every_kind();
        let all = GraphValidator::new().validate(&g);

        let options = ValidationOptions::new()
            .allow_cycles()
            .allow_dead_ends()
            .skip_orphan_facts();
        check!(!options.reports(K::Cycle));
        check!(options.reports(K::SelfLoop));
        let validator = GraphValidator::with_options(options);
        check!(validator.options().reports(K::Unreachable));
        let issues = validator.validate(&g);
        check!(
            issues
                .iter()
                .all(|i| !matches!(i.kind, K::Cycle | K::DeadEnd | K::OrphanFact))
        );
        let kept = all
            .iter()
            .filter(|i| !matches!(i.kind, K::Cycle | K::DeadEnd | K::OrphanFact))
            .count();
        check!(issues.len() == kept);
        check!(issues.iter().any(|i| i.kind == K::SelfLoop));
    }

    #[test]
    fn options_skip_every_kind() {
        use ValidationIssueKind as K;
        let g = graph_with_every_kind();
        for kind in [
            K::MissingTarget,
            K::Unreachable,
            K::DeadEnd,
            K::SelfLoop,
            K::Cycle,
            K::AlwaysFalse,
            K::OrphanFact,
            K::SourceNodeMismatch,
            K::EmptyText,
            K::UnkeyedProgress,
        ] {
            let validator = GraphValidator::with_options(ValidationOptions::new().skip(kind));
            check!(of_kind(&validator, &g, kind).is_empty(), "{kind:?}");
            check!(!validator.validate(&g).is_empty());
        }
        let no_entry = graph(vec![make_node(1, vec![])], None);
        let validator = GraphValidator::with_options(ValidationOptions::new().skip(K::NoEntryNode));
        check!(validator.validate(&no_entry).is_empty());
        let bad_entry = graph(vec![make_node(1, vec![])], Some(5));
        let validator =
            GraphValidator::with_options(ValidationOptions::new().skip(K::EntryNodeNotFound));
        check!(validator.validate(&bad_entry).is_empty());
    }

    #[test]
    fn options_report_undoes_skip() {
        let g = graph_with_every_kind();
        let options = ValidationOptions::new()
            .allow_cycles()
            .report(ValidationIssueKind::Cycle);
        check!(options == ValidationOptions::new());
        let validator = GraphValidator::with_options(options);
        check!(!of_kind(&validator, &g, ValidationIssueKind::Cycle).is_empty());
    }

    #[test]
    fn validator_empty_graph() {
        let validator = GraphValidator::new();
        let graph = DialogueGraph::default();
        let issues = validator.validate(&graph);
        check!(issues.is_empty());
    }

    #[test]
    fn detects_possibly_empty_fragment_text() {
        use dramaturge_model::{FactId, FactScope, TextFragment};
        let flag = Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new("flag"),
        };
        let mut risky = make_node(1, vec![]);
        risky.text = DialogText::fragments(vec![
            TextFragment::always(DialogText::inline("Only sometimes.")).when(flag),
            TextFragment::with_mode(FragmentMode::Once, vec![DialogText::inline("Once.")]),
        ]);
        let mut safe = make_node(2, vec![]);
        safe.text = DialogText::fragments(vec![
            TextFragment::with_mode(FragmentMode::FirstTime, vec![DialogText::inline("Hi.")]),
            TextFragment::always(DialogText::inline("Always.")),
        ]);
        let graph = DialogueGraph {
            nodes: vec![risky, safe],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let empty: Vec<_> = GraphValidator::new()
            .validate(&graph)
            .into_iter()
            .filter(|i| i.message.contains("empty text"))
            .collect();
        check!(empty.len() == 1);
        check!(empty[0].node_id == Some(DialogueNodeId::new(1)));
        check!(empty[0].severity == ValidationSeverity::Warning);
    }

    #[test]
    fn warns_about_unkeyed_progress_fragments() {
        use dramaturge_model::TextFragment;
        let mut node = make_node(1, vec![]);
        node.text = DialogText::fragments(vec![
            TextFragment::always(DialogText::inline("Fine without a key.")),
            TextFragment::with_mode(FragmentMode::Cycle, vec![DialogText::inline("a")]),
            TextFragment::with_mode(FragmentMode::Once, vec![DialogText::inline("b")])
                .keyed("greeting"),
        ]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let unkeyed: Vec<_> = GraphValidator::new()
            .validate(&graph)
            .into_iter()
            .filter(|i| i.message.contains("has no key"))
            .collect();
        check!(unkeyed.len() == 1);
        check!(unkeyed[0].severity == ValidationSeverity::Warning);
    }

    #[test]
    fn validate_with_symbols_names_keys() {
        let mut symbols = SymbolTable::new();
        let intro = symbols.intern_node("dlg.intro");
        let missing = symbols.intern_node("dlg.missing");
        let option = symbols.intern_option("dlg.intro.go");
        let node = make_node(
            intro.0,
            vec![make_option(option.0, DialogTarget::Node(missing), intro)],
        );
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_nodes: vec![intro],
        };
        let issues = GraphValidator::new().validate_with_symbols(&graph, &symbols);
        check!(issues.iter().any(|i| i.message
            == "Option 'dlg.intro.go' in node 'dlg.intro' targets nonexistent node 'dlg.missing'"));
    }

    #[test]
    fn detects_missing_targets() {
        let validator = GraphValidator::new();
        let node = make_node(
            1,
            vec![make_option(
                1,
                DialogTarget::Node(DialogueNodeId::new(999)),
                DialogueNodeId::new(1),
            )],
        );
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(
            issues
                .iter()
                .any(|i| i.message.contains("targets nonexistent node"))
        );
    }

    #[test]
    fn detects_unreachable_nodes() {
        let validator = GraphValidator::new();
        let node1 = make_node(1, vec![]);
        let node2 = make_node(2, vec![]);
        let graph = DialogueGraph {
            nodes: vec![node1, node2],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(issues.iter().any(|i| i.message.contains("unreachable")));
    }

    #[test]
    fn detects_self_loops() {
        let validator = GraphValidator::new();
        let option = make_option(
            1,
            DialogTarget::Node(DialogueNodeId::new(1)),
            DialogueNodeId::new(1),
        );
        let node = make_node(1, vec![option]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(issues.iter().any(|i| i.message.contains("self-loop")));
    }

    #[test]
    fn detects_never_conditions() {
        let validator = GraphValidator::new();
        let option = DialogueOption {
            id: DialogueOptionId::new(1),
            text: DialogText::inline("opt"),
            tags: vec![],
            intentions: DialogIntentions::new(),
            conditions: vec![Condition::Never],
            outcomes: vec![],
            target: DialogTarget::node(2),
            source_node_id: DialogueNodeId::new(1),
            navigation_hint: None,
        };
        let node = make_node(1, vec![option]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(
            issues
                .iter()
                .any(|i| i.message.contains("always-false condition"))
        );
    }

    #[test]
    fn any_with_never_and_always_not_flagged() {
        let validator = GraphValidator::new();
        let option = DialogueOption {
            id: DialogueOptionId::new(1),
            text: DialogText::inline("opt"),
            tags: vec![],
            intentions: DialogIntentions::new(),
            conditions: vec![Condition::Any(vec![Condition::Never, Condition::Always])],
            outcomes: vec![],
            target: DialogTarget::node(2),
            source_node_id: DialogueNodeId::new(1),
            navigation_hint: None,
        };
        let node = make_node(1, vec![option]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(
            !issues
                .iter()
                .any(|i| i.message.contains("always-false condition"))
        );
    }

    #[test]
    fn not_never_not_flagged() {
        let validator = GraphValidator::new();
        let option = DialogueOption {
            id: DialogueOptionId::new(1),
            text: DialogText::inline("opt"),
            tags: vec![],
            intentions: DialogIntentions::new(),
            conditions: vec![Condition::Not(Box::new(Condition::Never))],
            outcomes: vec![],
            target: DialogTarget::node(2),
            source_node_id: DialogueNodeId::new(1),
            navigation_hint: None,
        };
        let node = make_node(1, vec![option]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(
            !issues
                .iter()
                .any(|i| i.message.contains("always-false condition"))
        );
    }

    #[test]
    fn detects_cycle_reports_all_members() {
        let validator = GraphValidator::new();
        let node1 = make_node(
            1,
            vec![make_option(
                1,
                DialogTarget::node(2),
                DialogueNodeId::new(1),
            )],
        );
        let node2 = make_node(
            2,
            vec![make_option(
                2,
                DialogTarget::node(1),
                DialogueNodeId::new(2),
            )],
        );
        let graph = DialogueGraph {
            nodes: vec![node1, node2],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        let cycle_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("cycle"))
            .collect();
        check!(cycle_issues.len() == 2);
        check!(
            cycle_issues
                .iter()
                .any(|i| i.node_id == Some(DialogueNodeId::new(1)))
        );
        check!(
            cycle_issues
                .iter()
                .any(|i| i.node_id == Some(DialogueNodeId::new(2)))
        );
    }

    #[test]
    fn detects_dead_end_entry_node() {
        let validator = GraphValidator::new();
        let node1 = make_node(1, vec![]);
        let graph = DialogueGraph {
            nodes: vec![node1],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(
            issues
                .iter()
                .any(|i| i.message.contains("potential dead end")
                    && i.node_id == Some(DialogueNodeId::new(1)))
        );
    }

    #[test]
    fn orphaned_facts_only_from_conditions() {
        let validator = GraphValidator::new();
        let option = DialogueOption {
            id: DialogueOptionId::new(1),
            text: DialogText::inline("opt"),
            tags: vec![],
            intentions: DialogIntentions::new(),
            conditions: vec![
                Condition::FactExists {
                    scope: FactScope::World,
                    fact_id: FactId::new("only_removed"),
                },
                Condition::FactExists {
                    scope: FactScope::World,
                    fact_id: FactId::new("never_mentioned"),
                },
            ],
            outcomes: vec![Outcome::RemoveFact {
                scope: FactScope::World,
                fact_id: FactId::new("only_removed"),
            }],
            target: DialogTarget::node(2),
            source_node_id: DialogueNodeId::new(1),
            navigation_hint: None,
        };
        let node = make_node(1, vec![option]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        let orphaned: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("referenced in conditions but never set"))
            .collect();
        check!(orphaned.len() == 2);
        check!(orphaned.iter().any(|i| i.message.contains("only_removed")));
        check!(
            orphaned
                .iter()
                .any(|i| i.message.contains("never_mentioned"))
        );
    }

    #[test]
    fn add_fact_counts_as_writer() {
        let mut option = make_option(1, DialogTarget::End, DialogueNodeId::new(1));
        option.conditions = vec![Condition::FactAtLeast {
            scope: FactScope::World,
            fact_id: FactId::new("visits"),
            value: 2,
        }];
        let mut node = make_node(1, vec![option]);
        node.outcomes_on_enter = vec![Outcome::AddFact {
            scope: FactScope::World,
            fact_id: FactId::new("visits"),
            delta: 1,
        }];
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = GraphValidator::new().validate(&graph);
        check!(
            !issues
                .iter()
                .any(|i| i.message.contains("never set by any outcome"))
        );
    }

    #[test]
    fn detects_cycles() {
        let validator = GraphValidator::new();
        let node1 = make_node(
            1,
            vec![make_option(
                1,
                DialogTarget::node(2),
                DialogueNodeId::new(1),
            )],
        );
        let node2 = make_node(
            2,
            vec![make_option(
                2,
                DialogTarget::node(1),
                DialogueNodeId::new(2),
            )],
        );
        let graph = DialogueGraph {
            nodes: vec![node1, node2],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(issues.iter().any(|i| i.message.contains("cycle")));
    }

    #[test]
    fn detects_dead_end_nodes() {
        let validator = GraphValidator::new();
        let node1 = make_node(
            1,
            vec![make_option(
                1,
                DialogTarget::node(2),
                DialogueNodeId::new(1),
            )],
        );
        let node2 = make_node(2, vec![]);
        let graph = DialogueGraph {
            nodes: vec![node1, node2],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(
            issues
                .iter()
                .any(|i| i.message.contains("potential dead end"))
        );
    }

    #[test]
    fn valid_source_node_id_no_issue() {
        let validator = GraphValidator::new();
        let option = make_option(
            1,
            DialogTarget::Node(DialogueNodeId::new(2)),
            DialogueNodeId::new(1), // matches parent node id=1
        );
        let node = make_node(1, vec![option]);
        let node2 = make_node(2, vec![]);
        let graph = DialogueGraph {
            nodes: vec![node, node2],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        check!(
            !issues
                .iter()
                .any(|i| i.message.contains("wrong source node"))
        );
    }

    #[test]
    fn detects_mismatched_source_node_id() {
        let validator = GraphValidator::new();
        let option = make_option(
            1,
            DialogTarget::Node(DialogueNodeId::new(2)),
            DialogueNodeId::new(99), // wrong: parent node id=1, not 99
        );
        let node = make_node(1, vec![option]);
        let node2 = make_node(2, vec![]);
        let graph = DialogueGraph {
            nodes: vec![node, node2],
            entry_nodes: vec![DialogueNodeId::new(1)],
        };
        let issues = validator.validate(&graph);
        let mismatch_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("wrong source node"))
            .collect();
        check!(mismatch_issues.len() == 1);
        check!(mismatch_issues[0].severity == ValidationSeverity::Error);
        check!(mismatch_issues[0].node_id == Some(DialogueNodeId::new(1)));
        check!(mismatch_issues[0].option_id == Some(DialogueOptionId::new(1)));
    }
}
