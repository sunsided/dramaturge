//! Authoring-time validation for dialogue graphs.
//!
//! This module provides the [`GraphValidator`] which runs a suite of
//! structural checks on a [`DialogueGraph`] to catch common authoring
//! errors before runtime. Each check produces [`ValidationIssue`]s
//! with severity levels ([`ValidationSeverity::Info`], [`ValidationSeverity::Warning`],
//! [`ValidationSeverity::Error`]) and optional
//! source location (node ID, option ID).
//!
//! # Validation Checks
//!
//! 1. **Missing targets** — Options referencing non-existent node IDs
//! 2. **Unreachable nodes** — Nodes not reachable from the entry node via BFS
//! 3. **Dead-end nodes** — Reachable nodes with no options or enter outcomes
//! 4. **Self-loops** — Options whose target is their own parent node
//! 5. **Cycles** — Nodes participating in cycles, reporting all cycle members
//! 6. **Always-false conditions** — Options with conditions that evaluate to always false via constant folding
//! 7. **Orphaned fact references** — Facts referenced in conditions but never set by any outcome
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

use std::collections::{HashMap, HashSet, VecDeque};

use dramaturge_model::{
    Condition, DialogTarget, DialogueGraph, DialogueNode, DialogueNodeId, DialogueOptionId,
};

/// Classification of issue importance.
#[derive(Debug, Clone, PartialEq, Eq)]
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
pub struct ValidationIssue {
    /// How important this issue is.
    pub severity: ValidationSeverity,
    /// Human-readable description of the problem.
    pub message: String,
    /// Optional node ID where the issue was detected.
    pub node_id: Option<DialogueNodeId>,
    /// Optional option ID where the issue was detected.
    pub option_id: Option<DialogueOptionId>,
}

impl ValidationIssue {
    /// Creates a new issue without source location.
    pub fn new(severity: ValidationSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            node_id: None,
            option_id: None,
        }
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
/// Runs seven validation checks covering graph integrity, reachability,
/// condition validity, and fact reference consistency.
#[derive(Debug, Clone, Default)]
pub struct GraphValidator;

impl GraphValidator {
    /// Creates a new graph validator.
    pub fn new() -> Self {
        Self
    }

    /// Runs all seven validation checks and returns the list of issues found.
    pub fn validate(&self, graph: &DialogueGraph) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let node_map: HashMap<DialogueNodeId, &DialogueNode> =
            graph.nodes.iter().map(|n| (n.id, n)).collect();

        self.check_missing_targets(graph, &node_map, &mut issues);
        self.check_unreachable_nodes(graph, &node_map, &mut issues);
        self.check_dead_end_nodes(graph, &node_map, &mut issues);
        self.check_self_loops(graph, &node_map, &mut issues);
        self.check_cycles(graph, &node_map, &mut issues);
        self.check_always_false_conditions(graph, &node_map, &mut issues);
        self.check_orphaned_fact_references(graph, &mut issues);

        issues
    }

    /// Detects options that target non-existent node IDs.
    /// Uses set difference between all target IDs and existing node IDs.
    fn check_missing_targets(
        &self,
        graph: &DialogueGraph,
        node_map: &HashMap<DialogueNodeId, &DialogueNode>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let existing_ids: HashSet<DialogueNodeId> = node_map.keys().copied().collect();

        for node in &graph.nodes {
            for option in &node.options {
                if let DialogTarget::Node(target_id) = &option.target
                    && !existing_ids.contains(target_id)
                {
                    issues.push(
                        ValidationIssue::new(
                            ValidationSeverity::Error,
                            format!("Option targets nonexistent node ID {}", target_id.0),
                        )
                        .with_node(node.id)
                        .with_option(option.id),
                    );
                }
            }
        }
    }

    /// Detects nodes not reachable from the entry node via BFS traversal.
    fn check_unreachable_nodes(
        &self,
        graph: &DialogueGraph,
        node_map: &HashMap<DialogueNodeId, &DialogueNode>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let Some(entry_id) = graph.entry_node else {
            for node in &graph.nodes {
                issues.push(
                    ValidationIssue::new(
                        ValidationSeverity::Warning,
                        "No entry node defined; all nodes are unreachable",
                    )
                    .with_node(node.id),
                );
            }
            return;
        };

        if !node_map.contains_key(&entry_id) {
            issues.push(ValidationIssue::new(
                ValidationSeverity::Error,
                format!("Entry node ID {} does not exist in the graph", entry_id.0),
            ));
            return;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(entry_id);
        visited.insert(entry_id);

        while let Some(current_id) = queue.pop_front() {
            if let Some(node) = node_map.get(&current_id) {
                for option in &node.options {
                    if let DialogTarget::Node(target_id) = &option.target
                        && !visited.contains(target_id)
                    {
                        visited.insert(*target_id);
                        queue.push_back(*target_id);
                    }
                }
            }
        }

        for node in &graph.nodes {
            if !visited.contains(&node.id) {
                issues.push(
                    ValidationIssue::new(
                        ValidationSeverity::Warning,
                        format!("Node {} is unreachable from the entry node", node.id.0),
                    )
                    .with_node(node.id),
                );
            }
        }
    }

    /// Detects nodes with no options and no enter outcomes that are reachable
    /// from the entry node. These are potential dead ends in the conversation flow.
    fn check_dead_end_nodes(
        &self,
        graph: &DialogueGraph,
        node_map: &HashMap<DialogueNodeId, &DialogueNode>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let Some(entry_id) = graph.entry_node else {
            return;
        };

        if !node_map.contains_key(&entry_id) {
            return;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(entry_id);
        visited.insert(entry_id);

        while let Some(current_id) = queue.pop_front() {
            if let Some(node) = node_map.get(&current_id) {
                for option in &node.options {
                    if let DialogTarget::Node(target_id) = &option.target
                        && !visited.contains(target_id)
                    {
                        visited.insert(*target_id);
                        queue.push_back(*target_id);
                    }
                }
            }
        }

        for node_id in &visited {
            if let Some(node) = node_map.get(node_id) {
                let has_options = !node.options.is_empty();
                let has_enter_outcomes = !node.outcomes_on_enter.is_empty();

                if !has_options && !has_enter_outcomes {
                    issues.push(
                        ValidationIssue::new(
                            ValidationSeverity::Info,
                            format!(
                                "Node {} has no options or enter outcomes (potential dead end)",
                                node.id.0
                            ),
                        )
                        .with_node(node.id),
                    );
                }
            }
        }
    }

    /// Detects options whose target is the same node as their parent (self-loop).
    fn check_self_loops(
        &self,
        graph: &DialogueGraph,
        _node_map: &HashMap<DialogueNodeId, &DialogueNode>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        for node in &graph.nodes {
            for option in &node.options {
                if let DialogTarget::Node(target_id) = &option.target
                    && *target_id == node.id
                {
                    issues.push(
                        ValidationIssue::new(
                            ValidationSeverity::Warning,
                            format!(
                                "Option {} in node {} targets the same node (self-loop)",
                                option.id.0, node.id.0
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
    fn check_cycles(
        &self,
        graph: &DialogueGraph,
        node_map: &HashMap<DialogueNodeId, &DialogueNode>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let Some(entry_id) = graph.entry_node else {
            return;
        };

        let mut visited = HashSet::new();
        let mut cycle_nodes = HashSet::new();

        fn dfs(
            current: DialogueNodeId,
            node_map: &HashMap<DialogueNodeId, &DialogueNode>,
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
                for option in &node.options {
                    if let DialogTarget::Node(target_id) = &option.target {
                        dfs(*target_id, node_map, visited, stack, cycle_nodes);
                    }
                }
            }

            stack.pop();
        }

        dfs(
            entry_id,
            node_map,
            &mut visited,
            &mut Vec::new(),
            &mut cycle_nodes,
        );

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
                        ValidationSeverity::Warning,
                        format!("Node {} is part of a cycle", node.id.0),
                    )
                    .with_node(node.id),
                );
            }
        }
    }

    /// Detects options with conditions that are always false.
    /// Uses constant folding to properly evaluate composite conditions:
    /// `Any([Never, Always])` is satisfiable, `Not(Never)` is always true.
    fn check_always_false_conditions(
        &self,
        graph: &DialogueGraph,
        _node_map: &HashMap<DialogueNodeId, &DialogueNode>,
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
                            ValidationSeverity::Warning,
                            format!(
                                "Option {} in node {} has an always-false condition (unreachable option)",
                                option.id.0, node.id.0
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
    /// never set by any `SetFact`, `StartQuest`, or `AdvanceQuest` outcome.
    fn check_orphaned_fact_references(
        &self,
        graph: &DialogueGraph,
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

    #[test]
    fn validator_empty_graph() {
        let validator = GraphValidator::new();
        let graph = DialogueGraph::default();
        let issues = validator.validate(&graph);
        check!(issues.is_empty());
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
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
            entry_node: Some(DialogueNodeId::new(1)),
        };
        let issues = validator.validate(&graph);
        check!(
            issues
                .iter()
                .any(|i| i.message.contains("potential dead end"))
        );
    }
}
