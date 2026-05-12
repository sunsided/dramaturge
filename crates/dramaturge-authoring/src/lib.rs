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
    /// never set by any `SetFact` outcome.
    fn check_orphaned_fact_references(
        &self,
        graph: &DialogueGraph,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let mut condition_fact_ids: HashSet<&str> = HashSet::new();
        let mut used_fact_ids: HashSet<&str> = HashSet::new();

        /// Extracts fact IDs from a condition tree.
        fn collect_fact_ids_from_condition<'a>(cond: &'a Condition, set: &mut HashSet<&'a str>) {
            match cond {
                Condition::FactEquals { fact_id, .. } | Condition::FactExists { fact_id, .. } => {
                    set.insert(&fact_id.0);
                }
                Condition::All(conds) | Condition::Any(conds) => {
                    for c in conds {
                        collect_fact_ids_from_condition(c, set);
                    }
                }
                Condition::Not(inner) => collect_fact_ids_from_condition(inner, set),
                Condition::Custom { args, .. } => {
                    for arg in args {
                        if let dramaturge_model::FactValue::Text(id) = arg {
                            set.insert(id);
                        }
                    }
                }
                _ => {}
            }
        }

        for node in &graph.nodes {
            for cond in &node.conditions {
                collect_fact_ids_from_condition(cond, &mut condition_fact_ids);
            }
            for option in &node.options {
                for cond in &option.conditions {
                    collect_fact_ids_from_condition(cond, &mut condition_fact_ids);
                }
            }
        }

        for node in &graph.nodes {
            for outcome in &node.outcomes_on_enter {
                if let dramaturge_model::Outcome::SetFact { fact, .. } = outcome {
                    used_fact_ids.insert(&fact.id.0);
                }
            }
            for option in &node.options {
                for outcome in &option.outcomes {
                    if let dramaturge_model::Outcome::SetFact { fact, .. } = outcome {
                        used_fact_ids.insert(&fact.id.0);
                    }
                }
            }
        }

        for fact_id in &condition_fact_ids {
            if !used_fact_ids.contains(fact_id) {
                issues.push(ValidationIssue::new(
                    ValidationSeverity::Info,
                    format!(
                        "Fact ID '{fact_id}' is referenced in conditions but never set by any outcome",
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dramaturge_model::{
        CharacterId, DialogIntentions, DialogText, DialogueGraph, DialogueNode, DialogueOption,
        EntityId, FactId, FactScope, Outcome,
    };

    /// Test helper — creates a minimal dialogue node with the given options.
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

    /// Test helper — creates a minimal dialogue option with the given target.
    fn make_option(id: u64, target: DialogTarget) -> DialogueOption {
        DialogueOption {
            id: DialogueOptionId::new(id),
            text: DialogText::inline("opt"),
            tags: vec![],
            intentions: DialogIntentions::new(),
            conditions: vec![],
            outcomes: vec![],
            target,
        }
    }

    #[test]
    fn validator_empty_graph() {
        let validator = GraphValidator::new();
        let graph = DialogueGraph::default();
        let issues = validator.validate(&graph);
        assert!(issues.is_empty());
    }

    #[test]
    fn detects_missing_targets() {
        let validator = GraphValidator::new();
        let node = make_node(
            1,
            vec![make_option(1, DialogTarget::Node(DialogueNodeId::new(999)))],
        );
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_node: Some(DialogueNodeId::new(1)),
        };
        let issues = validator.validate(&graph);
        assert!(
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
        assert!(issues.iter().any(|i| i.message.contains("unreachable")));
    }

    #[test]
    fn detects_self_loops() {
        let validator = GraphValidator::new();
        let option = make_option(1, DialogTarget::Node(DialogueNodeId::new(1)));
        let node = make_node(1, vec![option]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_node: Some(DialogueNodeId::new(1)),
        };
        let issues = validator.validate(&graph);
        assert!(issues.iter().any(|i| i.message.contains("self-loop")));
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
        };
        let node = make_node(1, vec![option]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_node: Some(DialogueNodeId::new(1)),
        };
        let issues = validator.validate(&graph);
        assert!(
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
        };
        let node = make_node(1, vec![option]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_node: Some(DialogueNodeId::new(1)),
        };
        let issues = validator.validate(&graph);
        assert!(
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
        };
        let node = make_node(1, vec![option]);
        let graph = DialogueGraph {
            nodes: vec![node],
            entry_node: Some(DialogueNodeId::new(1)),
        };
        let issues = validator.validate(&graph);
        assert!(
            !issues
                .iter()
                .any(|i| i.message.contains("always-false condition"))
        );
    }

    #[test]
    fn detects_cycle_reports_all_members() {
        let validator = GraphValidator::new();
        let node1 = make_node(1, vec![make_option(1, DialogTarget::node(2))]);
        let node2 = make_node(2, vec![make_option(2, DialogTarget::node(1))]);
        let graph = DialogueGraph {
            nodes: vec![node1, node2],
            entry_node: Some(DialogueNodeId::new(1)),
        };
        let issues = validator.validate(&graph);
        let cycle_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("cycle"))
            .collect();
        assert_eq!(cycle_issues.len(), 2);
        assert!(
            cycle_issues
                .iter()
                .any(|i| i.node_id == Some(DialogueNodeId::new(1)))
        );
        assert!(
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
        assert!(
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
        assert_eq!(orphaned.len(), 2);
        assert!(orphaned.iter().any(|i| i.message.contains("only_removed")));
        assert!(
            orphaned
                .iter()
                .any(|i| i.message.contains("never_mentioned"))
        );
    }

    #[test]
    fn detects_cycles() {
        let validator = GraphValidator::new();
        let node1 = make_node(1, vec![make_option(1, DialogTarget::node(2))]);
        let node2 = make_node(2, vec![make_option(2, DialogTarget::node(1))]);
        let graph = DialogueGraph {
            nodes: vec![node1, node2],
            entry_node: Some(DialogueNodeId::new(1)),
        };
        let issues = validator.validate(&graph);
        assert!(issues.iter().any(|i| i.message.contains("cycle")));
    }

    #[test]
    fn detects_dead_end_nodes() {
        let validator = GraphValidator::new();
        let node1 = make_node(1, vec![make_option(1, DialogTarget::node(2))]);
        let node2 = make_node(2, vec![]);
        let graph = DialogueGraph {
            nodes: vec![node1, node2],
            entry_node: Some(DialogueNodeId::new(1)),
        };
        let issues = validator.validate(&graph);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("potential dead end"))
        );
    }
}
