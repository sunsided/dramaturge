//! Dialogue runtime trait and default implementation for driving conversations.
//!
//! The [`DialogueRuntime`] trait defines the interface for driving a
//! dialogue session: entering a node ([`DialogueRuntime::enter_node`]),
//! filtering available options by conditions and applying outcomes when an
//! option is chosen. [`DefaultRuntime`]
//! combines a condition evaluator and an outcome applier (by default
//! [`DefaultConditionEvaluator`] and [`DefaultOutcomeApplier`]).
//!
//! Both the trait and the default runtime are generic over the state type
//! `S`, which defaults to [`DialogWorldState`], and over the host outcome
//! type `H` of [`Outcome::Host`], which defaults to `()`. The methods that
//! take a node or an option accept any host text type `T` of
//! [`DialogText::Host`](dramaturge_model::DialogText::Host): the runtime
//! does not look at the text.

use std::sync::Arc;

use crate::context::DialogWorldState;
use crate::cursor::DialogCursor;
use crate::evaluation::{
    ConditionEvaluator, DefaultConditionEvaluator, DefaultOutcomeApplier, OutcomeApplier,
};
use crate::state::NarrativeState;
use dramaturge_model::{
    Condition, DialogTarget, DialogueGraph, DialogueNode, DialogueNodeId, DialogueOption,
    EventKind, Outcome,
};

/// What [`DialogueRuntime::enter_node`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EnterResult {
    /// The node's conditions do not hold. No outcome was applied.
    Blocked,
    /// The node's conditions hold and its outcomes on enter were applied.
    Entered {
        /// `true` if at least one option of the node is available after
        /// the outcomes on enter were applied. `false` means the talk ends
        /// at this node, unless the host continues it.
        has_options: bool,
    },
}

impl EnterResult {
    /// Returns `true` if the node was entered.
    pub fn is_entered(&self) -> bool {
        matches!(self, Self::Entered { .. })
    }

    /// Returns `true` if the node was entered and has an available option.
    pub fn has_options(&self) -> bool {
        matches!(self, Self::Entered { has_options: true })
    }
}

/// How many nodes [`DialogueRuntime::advance`] enters at most in one call.
///
/// The limit stops a loop of nodes that continue by themselves
/// ([`DialogueNode::next`]) and never offer a choice.
pub const ADVANCE_LIMIT: usize = 1024;

/// What [`DialogueRuntime::advance`] did: the nodes it entered and why it
/// stopped.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Advance {
    /// The nodes that were entered, in order. Their outcomes on enter were
    /// applied. The host shows the lines of each node
    /// ([`DialogueNode::all_lines`]) in this order.
    pub entered: Vec<DialogueNodeId>,
    /// Why the advance stopped.
    pub stop: AdvanceStop,
}

/// Why [`DialogueRuntime::advance`] stopped.
///
/// In every case, the cursor is at the node where the advance stopped.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AdvanceStop {
    /// The node at the cursor has an available option. The host shows the
    /// options and calls [`choose_option`](DialogueRuntime::choose_option).
    Choice,
    /// The talk ends at the node at the cursor: its
    /// [`next`](DialogueNode::next) is `None`, [`DialogTarget::End`], or
    /// [`DialogTarget::Return`] with an empty return stack.
    End,
    /// The `next` target of the node at the cursor is a
    /// [`DialogTarget::Custom`] target. The host handles it.
    Custom(Arc<str>),
    /// The conditions of the node at the cursor do not hold. The node was
    /// not entered and no outcome of it was applied.
    Blocked,
    /// The graph has no node with the id at the cursor.
    Missing,
    /// The advance entered the limit of nodes and stopped. The last
    /// entered node continues by itself, so the graph probably has a loop
    /// of `next` targets.
    Limit,
}

/// Trait defining the interface for driving a dialogue session.
///
/// Implementations determine which options are available at a given
/// node and handle the consequences of choosing an option.
///
/// An implementation gives the two primitives
/// [`conditions_hold`](Self::conditions_hold) and
/// [`apply_outcomes`](Self::apply_outcomes). [`enter_node`](Self::enter_node)
/// has a default implementation that uses them.
pub trait DialogueRuntime<S: NarrativeState + ?Sized = DialogWorldState, H = ()> {
    /// The iterator type returned by [`available_options`](Self::available_options).
    ///
    /// This associated type allows implementors to choose their own iterator
    /// type while maintaining the trait interface. Implementors can return
    /// `std::slice::Iter`, `std::iter::Filter`, custom iterators, or
    /// `Box<dyn Iterator>` as needed.
    ///
    /// `T` is the host text type of the node (see
    /// [`DialogText::Host`](dramaturge_model::DialogText::Host)). The
    /// runtime does not look at the text, so every method takes nodes with
    /// any `T`.
    type OptionsIter<'a, T>: Iterator<Item = &'a DialogueOption<H, T>>
    where
        Self: 'a,
        S: 'a,
        H: 'a,
        T: 'a;

    /// Returns references to options whose conditions are satisfied.
    ///
    /// Options with no conditions are always included. Options with
    /// conditions are included only if all their conditions evaluate
    /// to `true`.
    fn available_options<'a, T>(
        &'a self,
        node: &'a DialogueNode<H, T>,
        state: &'a S,
    ) -> Self::OptionsIter<'a, T>;

    /// Applies the option's outcomes and returns the target to transition to.
    ///
    /// This mutates the state by applying each outcome in order,
    /// then returns the option's target for the next navigation step.
    fn choose_option<T>(&mut self, option: &DialogueOption<H, T>, state: &mut S) -> DialogTarget;

    /// Returns `true` if all `conditions` hold for `state`.
    ///
    /// An empty list holds.
    fn conditions_hold(&self, conditions: &[Condition], state: &S) -> bool;

    /// Applies `outcomes` to `state`, in order.
    fn apply_outcomes(&mut self, outcomes: &[Outcome<H>], state: &mut S);

    /// Enters `node`: checks its conditions, applies its outcomes on enter
    /// and reports if it has an available option.
    ///
    /// If the node's [`conditions`](DialogueNode::conditions) do not hold,
    /// nothing changes and the result is [`EnterResult::Blocked`].
    /// Otherwise the [`outcomes_on_enter`](DialogueNode::outcomes_on_enter)
    /// are applied once per call, in order, and the result is
    /// [`EnterResult::Entered`]. `has_options` is computed after the
    /// outcomes, so an outcome on enter can make an option available.
    ///
    /// `enter_node` is not idempotent: every call applies the outcomes
    /// again. Call it once per visit of a node. A host that renders the
    /// same node again (for example after a screen redraw) does not call
    /// it again, or the outcomes apply twice.
    ///
    /// ```rust
    /// use dramaturge_model::{
    ///     CharacterId, DialogText, DialogueNode, EntityId, Fact, FactId, FactScope, Outcome,
    /// };
    /// use dramaturge_runtime::{DefaultRuntime, DialogWorldState, DialogueRuntime, EnterResult};
    ///
    /// let mut node = DialogueNode::new(1, CharacterId::new(EntityId::new(1)), DialogText::inline("Hi."));
    /// node.outcomes_on_enter.push(Outcome::SetFact {
    ///     scope: FactScope::World,
    ///     fact: Fact::bool("met_mara", true),
    /// });
    ///
    /// let mut runtime = DefaultRuntime::new();
    /// let mut state = DialogWorldState::new();
    /// let result = runtime.enter_node(&node, &mut state);
    /// assert_eq!(result, EnterResult::Entered { has_options: false });
    /// assert!(state.fact_exists(&FactScope::World, &FactId::new("met_mara")));
    /// ```
    fn enter_node<T>(&mut self, node: &DialogueNode<H, T>, state: &mut S) -> EnterResult {
        if !self.conditions_hold(&node.conditions, state) {
            return EnterResult::Blocked;
        }
        self.apply_outcomes(&node.outcomes_on_enter, state);
        let has_options = self.available_options(node, state).next().is_some();
        EnterResult::Entered { has_options }
    }

    /// Enters the node at the cursor and continues through
    /// [`next`](DialogueNode::next) targets until a node has a choice or
    /// the talk ends.
    ///
    /// Each node is entered with [`enter_node`](Self::enter_node), so its
    /// conditions are checked and its outcomes on enter apply once. A node
    /// with an available option stops the advance
    /// ([`AdvanceStop::Choice`]). A node without one follows its `next`
    /// target through [`DialogCursor::follow`]: a `Node` target pushes on
    /// the return stack, `Return` pops it. See [`AdvanceStop`] for the other
    /// stops. The advance enters at most [`ADVANCE_LIMIT`] nodes.
    ///
    /// Call it once to start a talk (with the cursor at the entry node),
    /// and once after each choice whose target moved the cursor.
    ///
    /// ```rust
    /// use dramaturge_model::{
    ///     CharacterId, DialogTarget, DialogText, DialogueGraph, DialogueNode, DialogueNodeId,
    ///     DialogueOption, EntityId,
    /// };
    /// use dramaturge_runtime::{
    ///     AdvanceStop, DefaultRuntime, DialogCursor, DialogWorldState, DialogueRuntime,
    /// };
    ///
    /// let mara = CharacterId::new(EntityId::new(1));
    /// let player = CharacterId::new(EntityId::new(2));
    ///
    /// // A scripted exchange, then a choice.
    /// let meet = DialogueNode::new(1, mara, DialogText::inline("Sit, if you like."))
    ///     .with_line(player, DialogText::inline("Who are you?"))
    ///     .with_next(DialogTarget::node(2));
    /// let mut names = DialogueNode::new(2, mara, DialogText::inline("They call me Mara."));
    /// names.options.push(DialogueOption::new(
    ///     1,
    ///     DialogText::inline("Goodbye."),
    ///     DialogTarget::End,
    ///     names.id,
    /// ));
    /// let mut graph = DialogueGraph::new().with_entry_node(meet.id);
    /// graph.nodes = vec![meet, names];
    ///
    /// let mut runtime = DefaultRuntime::new();
    /// let mut state = DialogWorldState::new();
    /// let mut cursor = DialogCursor::new(DialogueNodeId::new(1));
    ///
    /// let advance = runtime.advance(&graph, &mut cursor, &mut state);
    /// assert_eq!(advance.entered, [DialogueNodeId::new(1), DialogueNodeId::new(2)]);
    /// assert_eq!(advance.stop, AdvanceStop::Choice);
    /// assert_eq!(cursor.current, DialogueNodeId::new(2));
    /// ```
    fn advance<T>(
        &mut self,
        graph: &DialogueGraph<H, T>,
        cursor: &mut DialogCursor,
        state: &mut S,
    ) -> Advance {
        self.advance_limited(graph, cursor, state, ADVANCE_LIMIT)
    }

    /// Like [`advance`](Self::advance), but enters at most `limit` nodes.
    ///
    /// With `limit` 0, nothing is entered and the result is
    /// [`AdvanceStop::Limit`].
    fn advance_limited<T>(
        &mut self,
        graph: &DialogueGraph<H, T>,
        cursor: &mut DialogCursor,
        state: &mut S,
        limit: usize,
    ) -> Advance {
        let mut entered = Vec::new();
        let stop = loop {
            if entered.len() >= limit {
                break AdvanceStop::Limit;
            }
            let Some(node) = graph.node(cursor.current) else {
                break AdvanceStop::Missing;
            };
            match self.enter_node(node, state) {
                EnterResult::Blocked => break AdvanceStop::Blocked,
                EnterResult::Entered { has_options } => {
                    entered.push(node.id);
                    if has_options {
                        break AdvanceStop::Choice;
                    }
                }
            }
            let Some(next) = &node.next else {
                break AdvanceStop::End;
            };
            if cursor.follow(next).is_none() {
                break match next {
                    DialogTarget::Custom(key) => AdvanceStop::Custom(key.clone()),
                    _ => AdvanceStop::End,
                };
            }
        };
        Advance { entered, stop }
    }
}

/// Combines a [`ConditionEvaluator`] and an [`OutcomeApplier`] into a
/// complete runtime.
///
/// With the default type parameters this is the built-in runtime over
/// [`DialogWorldState`]. Use [`DefaultRuntime::with_parts`] to plug in
/// evaluators and appliers that carry a [`CustomHandler`](crate::CustomHandler).
#[derive(Debug, Clone, Default)]
pub struct DefaultRuntime<E = DefaultConditionEvaluator, A = DefaultOutcomeApplier> {
    /// The condition evaluator used to filter available options.
    pub evaluator: E,
    /// The outcome applier used when an option is chosen.
    pub applier: A,
}

impl DefaultRuntime {
    /// Creates a new runtime with the default evaluator and applier.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<E, A> DefaultRuntime<E, A> {
    /// Creates a runtime from the given evaluator and applier.
    pub fn with_parts(evaluator: E, applier: A) -> Self {
        Self { evaluator, applier }
    }

    /// Returns the first entry node of `graph` whose conditions hold.
    ///
    /// Uses the runtime's evaluator. See [`select_entry`].
    pub fn select_entry<S, H, T>(
        &self,
        graph: &DialogueGraph<H, T>,
        state: &S,
    ) -> Option<DialogueNodeId>
    where
        S: NarrativeState + ?Sized,
        E: ConditionEvaluator<S>,
    {
        select_entry(graph, &self.evaluator, state)
    }
}

/// Returns the first entry node of `graph` whose conditions hold.
///
/// The function looks at [`DialogueGraph::entry_nodes`] in order and
/// returns the first node whose [`conditions`](DialogueNode::conditions)
/// all evaluate to `true` with `evaluator`. A node without conditions
/// always holds. Entry node ids without a node in the graph are skipped.
/// Returns `None` when no entry node holds, or when the graph has no
/// entry nodes.
///
/// ```rust
/// use dramaturge_model::{
///     CharacterId, Condition, DialogText, DialogueGraph, DialogueNode, DialogueNodeId, EntityId,
/// };
/// use dramaturge_runtime::{DefaultConditionEvaluator, DialogWorldState, select_entry};
///
/// let speaker = CharacterId::new(EntityId::new(1));
/// let mut locked = DialogueNode::new(1, speaker, DialogText::inline("Welcome back."));
/// locked.conditions.push(Condition::Never);
/// let open = DialogueNode::new(2, speaker, DialogText::inline("Hello, stranger."));
///
/// let mut graph = DialogueGraph::new()
///     .with_entry_node(locked.id)
///     .with_entry_node(open.id);
/// graph.nodes = vec![locked, open];
///
/// let state = DialogWorldState::new();
/// let entry = select_entry(&graph, &DefaultConditionEvaluator::new(), &state);
/// assert_eq!(entry, Some(DialogueNodeId::new(2)));
/// ```
pub fn select_entry<S, E, H, T>(
    graph: &DialogueGraph<H, T>,
    evaluator: &E,
    state: &S,
) -> Option<DialogueNodeId>
where
    S: NarrativeState + ?Sized,
    E: ConditionEvaluator<S> + ?Sized,
{
    graph
        .entries()
        .find(|node| node.conditions.iter().all(|c| evaluator.evaluate(c, state)))
        .map(|node| node.id)
}

/// Iterator over available dialogue options.
///
/// This struct is returned by [`DefaultRuntime::available_options`] and
/// yields references to options whose conditions are satisfied.
pub struct AvailableOptionsIter<'a, E, S: ?Sized = DialogWorldState, H = (), T = ()> {
    inner: std::slice::Iter<'a, DialogueOption<H, T>>,
    evaluator: &'a E,
    state: &'a S,
}

impl<'a, E, S: ?Sized, H, T> AvailableOptionsIter<'a, E, S, H, T> {
    /// Creates an iterator over the options of `node` that pass `evaluator`.
    pub fn new(node: &'a DialogueNode<H, T>, evaluator: &'a E, state: &'a S) -> Self {
        Self {
            inner: node.options.iter(),
            evaluator,
            state,
        }
    }
}

impl<'a, E, S, H, T> Iterator for AvailableOptionsIter<'a, E, S, H, T>
where
    E: ConditionEvaluator<S>,
    S: NarrativeState + ?Sized,
{
    type Item = &'a DialogueOption<H, T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let option = self.inner.next()?;
            if option
                .conditions
                .iter()
                .all(|c| self.evaluator.evaluate(c, self.state))
            {
                return Some(option);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // We can't know exactly how many will pass the filter,
        // but we can provide an upper bound
        (0, self.inner.size_hint().1)
    }
}

impl<S, H, E, A> DialogueRuntime<S, H> for DefaultRuntime<E, A>
where
    S: NarrativeState + ?Sized,
    E: ConditionEvaluator<S>,
    A: OutcomeApplier<S, H>,
{
    type OptionsIter<'a, T>
        = AvailableOptionsIter<'a, E, S, H, T>
    where
        Self: 'a,
        S: 'a,
        H: 'a,
        T: 'a;

    fn available_options<'a, T>(
        &'a self,
        node: &'a DialogueNode<H, T>,
        state: &'a S,
    ) -> Self::OptionsIter<'a, T> {
        AvailableOptionsIter::new(node, &self.evaluator, state)
    }

    fn choose_option<T>(&mut self, option: &DialogueOption<H, T>, state: &mut S) -> DialogTarget {
        state.emit_event(EventKind::DialogOptionChosen {
            node_id: option.source_node_id,
            option_id: option.id,
            tags: option.tags.clone(),
            intentions: option.intentions.clone(),
        });

        self.apply_outcomes(&option.outcomes, state);
        option.target.clone()
    }

    fn conditions_hold(&self, conditions: &[Condition], state: &S) -> bool {
        conditions.iter().all(|c| self.evaluator.evaluate(c, state))
    }

    fn apply_outcomes(&mut self, outcomes: &[Outcome<H>], state: &mut S) {
        for outcome in outcomes {
            self.applier.apply(outcome, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::DialogWorldState;
    use crate::evaluation::NoCustom;
    use assert2::check;
    use dramaturge_model::{
        CharacterId, Condition, DialogText, DialogueOption, DialogueOptionId, EntityId, Fact,
        FactId, FactScope,
    };

    fn speaker() -> CharacterId {
        CharacterId::new(EntityId::new(1))
    }

    fn add_visits() -> dramaturge_model::Outcome {
        dramaturge_model::Outcome::AddFact {
            scope: FactScope::World,
            fact_id: FactId::new("visits"),
            delta: 1,
        }
    }

    fn visits(state: &DialogWorldState) -> Option<i64> {
        match state.get_fact(&FactScope::World, &FactId::new("visits")) {
            Some(dramaturge_model::FactValue::Int(n)) => Some(*n),
            _ => None,
        }
    }

    #[test]
    fn enter_node_blocked_when_conditions_are_false() {
        let mut node = DialogueNode::new(1, speaker(), DialogText::inline("Hi."));
        node.conditions.push(Condition::Never);
        node.outcomes_on_enter.push(add_visits());
        node.options.push(DialogueOption::new(
            1,
            DialogText::inline("Bye."),
            DialogTarget::End,
            node.id,
        ));

        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        let result = rt.enter_node(&node, &mut state);
        check!(result == EnterResult::Blocked);
        check!(!result.is_entered());
        check!(!result.has_options());
        check!(visits(&state).is_none());
        check!(state.events().is_empty());
    }

    #[test]
    fn enter_node_applies_outcomes_per_call() {
        let mut node = DialogueNode::new(1, speaker(), DialogText::inline("Hi."));
        node.outcomes_on_enter.push(add_visits());
        node.options.push(DialogueOption::new(
            1,
            DialogText::inline("Bye."),
            DialogTarget::End,
            node.id,
        ));

        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        let result = rt.enter_node(&node, &mut state);
        check!(result == EnterResult::Entered { has_options: true });
        check!(result.has_options());
        check!(visits(&state) == Some(1));

        // A second visit applies the outcomes a second time.
        rt.enter_node(&node, &mut state);
        check!(visits(&state) == Some(2));
    }

    #[test]
    fn enter_node_without_options() {
        let node = DialogueNode::new(1, speaker(), DialogText::inline("The end."));
        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        let result = rt.enter_node(&node, &mut state);
        check!(result == EnterResult::Entered { has_options: false });
        check!(result.is_entered());
        check!(!result.has_options());
    }

    #[test]
    fn enter_node_checks_options_after_the_outcomes() {
        let mut node = DialogueNode::new(1, speaker(), DialogText::inline("Hi."));
        node.outcomes_on_enter
            .push(dramaturge_model::Outcome::SetFact {
                scope: FactScope::World,
                fact: Fact::bool("has_key", true),
            });
        let mut option =
            DialogueOption::new(1, DialogText::inline("Open."), DialogTarget::End, node.id);
        option.conditions.push(Condition::FactEquals {
            scope: FactScope::World,
            fact_id: FactId::new("has_key"),
            value: dramaturge_model::FactValue::Bool(true),
        });
        node.options.push(option);

        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        check!(rt.available_options(&node, &state).next().is_none());
        check!(rt.enter_node(&node, &mut state) == EnterResult::Entered { has_options: true });
    }

    #[test]
    fn enter_node_without_available_options() {
        let mut node = DialogueNode::new(1, speaker(), DialogText::inline("Hi."));
        let mut option =
            DialogueOption::new(1, DialogText::inline("Never."), DialogTarget::End, node.id);
        option.conditions.push(Condition::Never);
        node.options.push(option);

        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        check!(rt.enter_node(&node, &mut state) == EnterResult::Entered { has_options: false });
    }

    fn id(n: u64) -> DialogueNodeId {
        DialogueNodeId::new(n)
    }

    /// A node that adds 1 to `visits` on enter and continues to `next`.
    fn auto(n: u64, next: Option<DialogTarget>) -> DialogueNode {
        let mut node = DialogueNode::new(n, speaker(), DialogText::inline("..."));
        node.outcomes_on_enter.push(add_visits());
        node.next = next;
        node
    }

    fn with_choice(mut node: DialogueNode) -> DialogueNode {
        node.options.push(DialogueOption::new(
            node.id.0,
            DialogText::inline("Bye."),
            DialogTarget::End,
            node.id,
        ));
        node
    }

    fn graph_of(nodes: Vec<DialogueNode>) -> DialogueGraph {
        let mut graph = DialogueGraph::new().with_entry_node(nodes[0].id);
        graph.nodes = nodes;
        graph
    }

    fn advance_from(
        graph: &DialogueGraph,
        start: u64,
    ) -> (Advance, DialogCursor, DialogWorldState) {
        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        let mut cursor = DialogCursor::new(id(start));
        let advance = rt.advance(graph, &mut cursor, &mut state);
        (advance, cursor, state)
    }

    #[test]
    fn advance_follows_next_and_applies_enter_outcomes_per_node() {
        let graph = graph_of(vec![
            auto(1, Some(DialogTarget::node(2))),
            auto(2, Some(DialogTarget::node(3))),
            with_choice(auto(3, Some(DialogTarget::node(4)))),
            auto(4, None),
        ]);
        let (advance, cursor, state) = advance_from(&graph, 1);
        check!(advance.entered == vec![id(1), id(2), id(3)]);
        check!(advance.stop == AdvanceStop::Choice);
        check!(cursor.current == id(3));
        check!(cursor.back == vec![id(1), id(2)]);
        check!(visits(&state) == Some(3));
    }

    #[test]
    fn advance_ends_without_next_or_at_end() {
        let graph = graph_of(vec![auto(1, Some(DialogTarget::node(2))), auto(2, None)]);
        let (advance, cursor, _) = advance_from(&graph, 1);
        check!(advance.entered == vec![id(1), id(2)]);
        check!(advance.stop == AdvanceStop::End);
        check!(cursor.current == id(2));

        let graph = graph_of(vec![auto(1, Some(DialogTarget::End))]);
        let (advance, cursor, _) = advance_from(&graph, 1);
        check!(advance.entered == vec![id(1)]);
        check!(advance.stop == AdvanceStop::End);
        check!(cursor.current == id(1));
    }

    #[test]
    fn advance_returns_through_the_stack() {
        // 1 -> 2 -> return to 1? No: 1 has a choice, so the talk waits there.
        let graph = graph_of(vec![
            with_choice(auto(1, None)),
            auto(2, Some(DialogTarget::Return)),
        ]);
        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        let mut cursor = DialogCursor::new(id(1));
        cursor.follow(&DialogTarget::node(2));
        let advance = rt.advance(&graph, &mut cursor, &mut state);
        check!(advance.entered == vec![id(2), id(1)]);
        check!(advance.stop == AdvanceStop::Choice);
        check!(cursor.current == id(1));
        check!(cursor.back.is_empty());

        // `Return` with an empty stack ends the talk.
        let graph = graph_of(vec![auto(1, Some(DialogTarget::Return))]);
        let (advance, _, _) = advance_from(&graph, 1);
        check!(advance.stop == AdvanceStop::End);
    }

    #[test]
    fn advance_hands_custom_targets_to_the_host() {
        let graph = graph_of(vec![auto(1, Some(DialogTarget::custom("shop")))]);
        let (advance, cursor, _) = advance_from(&graph, 1);
        check!(advance.entered == vec![id(1)]);
        check!(advance.stop == AdvanceStop::Custom("shop".into()));
        check!(cursor.current == id(1));
    }

    #[test]
    fn advance_stops_at_a_blocked_or_missing_node() {
        let mut blocked = auto(2, None);
        blocked.conditions.push(Condition::Never);
        let graph = graph_of(vec![auto(1, Some(DialogTarget::node(2))), blocked]);
        let (advance, cursor, state) = advance_from(&graph, 1);
        check!(advance.entered == vec![id(1)]);
        check!(advance.stop == AdvanceStop::Blocked);
        check!(cursor.current == id(2));
        // The outcomes of the blocked node did not apply.
        check!(visits(&state) == Some(1));

        let graph = graph_of(vec![auto(1, Some(DialogTarget::node(9)))]);
        let (advance, cursor, _) = advance_from(&graph, 1);
        check!(advance.entered == vec![id(1)]);
        check!(advance.stop == AdvanceStop::Missing);
        check!(cursor.current == id(9));
    }

    #[test]
    fn advance_uses_next_when_no_option_is_available() {
        let mut gated = auto(1, Some(DialogTarget::node(2)));
        let mut option =
            DialogueOption::new(1, DialogText::inline("Never."), DialogTarget::End, gated.id);
        option.conditions.push(Condition::Never);
        gated.options.push(option);
        let graph = graph_of(vec![gated, auto(2, None)]);
        let (advance, _, _) = advance_from(&graph, 1);
        check!(advance.entered == vec![id(1), id(2)]);
        check!(advance.stop == AdvanceStop::End);
    }

    #[test]
    fn advance_stops_a_loop_at_the_limit() {
        let graph = graph_of(vec![
            auto(1, Some(DialogTarget::node(2))),
            auto(2, Some(DialogTarget::node(1))),
        ]);
        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        let mut cursor = DialogCursor::new(id(1));
        let advance = rt.advance_limited(&graph, &mut cursor, &mut state, 5);
        check!(advance.entered == vec![id(1), id(2), id(1), id(2), id(1)]);
        check!(advance.stop == AdvanceStop::Limit);
        check!(visits(&state) == Some(5));

        let advance = rt.advance_limited(&graph, &mut cursor, &mut state, 0);
        check!(advance.entered.is_empty());
        check!(advance.stop == AdvanceStop::Limit);

        let (advance, _, _) = advance_from(&graph, 1);
        check!(advance.entered.len() == ADVANCE_LIMIT);
    }

    fn entry_graph(conditions: Vec<(u64, Vec<Condition>)>) -> DialogueGraph {
        let speaker = CharacterId::new(EntityId::new(1));
        let mut graph = DialogueGraph::new();
        for (id, conds) in conditions {
            let mut node = DialogueNode::new(id, speaker, DialogText::inline("..."));
            node.conditions = conds;
            graph.entry_nodes.push(node.id);
            graph.nodes.push(node);
        }
        graph
    }

    fn has_key() -> Condition {
        Condition::FactEquals {
            scope: FactScope::World,
            fact_id: FactId::new("has_key"),
            value: dramaturge_model::FactValue::Bool(true),
        }
    }

    #[test]
    fn select_entry_first_that_holds_wins() {
        let graph = entry_graph(vec![(1, vec![has_key()]), (2, vec![]), (3, vec![])]);
        let rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        check!(rt.select_entry(&graph, &state) == Some(DialogueNodeId::new(2)));

        state.set_fact(FactScope::World, Fact::bool("has_key", true));
        check!(rt.select_entry(&graph, &state) == Some(DialogueNodeId::new(1)));
    }

    #[test]
    fn select_entry_needs_all_conditions_of_an_entry() {
        // Entry 1 has one true and one false condition, so it does not hold.
        let mut state = DialogWorldState::new();
        state.set_fact(FactScope::World, Fact::bool("has_key", true));
        let evaluator = DefaultConditionEvaluator::new();

        let graph = entry_graph(vec![
            (1, vec![has_key(), Condition::Never]),
            (2, vec![has_key(), Condition::Always]),
        ]);
        check!(select_entry(&graph, &evaluator, &state) == Some(DialogueNodeId::new(2)));

        // Every entry mixes a true and a false condition: none holds.
        let graph = entry_graph(vec![
            (1, vec![has_key(), Condition::Never]),
            (2, vec![Condition::Never, Condition::Always]),
        ]);
        check!(select_entry(&graph, &evaluator, &state).is_none());
    }

    #[test]
    fn select_entry_none_holds() {
        let graph = entry_graph(vec![(1, vec![has_key()]), (2, vec![Condition::Never])]);
        let state = DialogWorldState::new();
        check!(select_entry(&graph, &DefaultConditionEvaluator::new(), &state).is_none());
    }

    #[test]
    fn select_entry_skips_missing_nodes_and_empty_graph() {
        let state = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator::new();
        check!(select_entry(&DialogueGraph::new(), &evaluator, &state).is_none());

        let mut graph = entry_graph(vec![(2, vec![])]);
        graph.entry_nodes.insert(0, DialogueNodeId::new(7));
        check!(select_entry(&graph, &evaluator, &state) == Some(DialogueNodeId::new(2)));
    }

    #[test]
    fn host_outcomes_go_to_the_host_handler() {
        use crate::evaluation::{CustomHandler, DefaultOutcomeApplier, HostHandler};
        use crate::state::HasDialogWorldState;
        use dramaturge_model::Outcome;

        #[derive(Debug, Clone, PartialEq)]
        enum Effect {
            Goto(&'static str),
            Give(u32),
        }

        #[derive(Default)]
        struct Game {
            room: &'static str,
            items: Vec<u32>,
            dialog: DialogWorldState,
        }

        impl HasDialogWorldState for Game {
            fn dialog(&self) -> &DialogWorldState {
                &self.dialog
            }
            fn dialog_mut(&mut self) -> &mut DialogWorldState {
                &mut self.dialog
            }
        }

        #[derive(Default)]
        struct Effects;

        impl CustomHandler<Game> for Effects {}

        impl HostHandler<Effect, Game> for Effects {
            fn host_outcome(&mut self, outcome: &Effect, game: &mut Game) {
                match outcome {
                    Effect::Goto(room) => game.room = room,
                    Effect::Give(item) => game.items.push(*item),
                }
            }
        }

        let speaker = CharacterId::new(EntityId::new(1));
        let mut node = DialogueNode::<Effect>::typed(1, speaker, DialogText::inline("Leave?"));
        let mut option =
            DialogueOption::typed(1, DialogText::inline("Yes."), DialogTarget::End, node.id);
        option.outcomes = vec![
            Outcome::Host(Effect::Give(7)),
            Outcome::SetFact {
                scope: FactScope::World,
                fact: Fact::bool("left", true),
            },
            Outcome::Host(Effect::Goto("street")),
        ];
        node.options.push(option);

        let mut rt = DefaultRuntime::with_parts(
            DefaultConditionEvaluator::new(),
            DefaultOutcomeApplier::with_custom(Effects),
        );
        let mut game = Game::default();
        let chosen = rt.available_options(&node, &game).next().cloned().unwrap();
        let target = rt.choose_option(&chosen, &mut game);

        check!(target == DialogTarget::End);
        check!(game.room == "street");
        check!(game.items == vec![7]);
        check!(
            game.dialog
                .fact_exists(&FactScope::World, &FactId::new("left"))
        );
    }

    #[test]
    fn enter_node_applies_host_outcomes() {
        use crate::evaluation::{CustomHandler, DefaultOutcomeApplier, HostHandler};

        #[derive(Default)]
        struct Rooms(Vec<&'static str>);

        impl<S: ?Sized> CustomHandler<S> for Rooms {}

        impl<S: ?Sized> HostHandler<&'static str, S> for Rooms {
            fn host_outcome(&mut self, room: &&'static str, _state: &mut S) {
                self.0.push(room);
            }
        }

        let mut node = DialogueNode::<&'static str>::typed(1, speaker(), DialogText::inline("Hi."));
        node.outcomes_on_enter
            .push(dramaturge_model::Outcome::Host("hall"));
        node.outcomes_on_enter
            .push(add_visits().map_host(|()| "never"));

        let mut rt = DefaultRuntime::with_parts(
            DefaultConditionEvaluator::new(),
            DefaultOutcomeApplier::with_custom(Rooms::default()),
        );
        let mut state = DialogWorldState::new();
        check!(rt.enter_node(&node, &mut state) == EnterResult::Entered { has_options: false });
        check!(rt.applier.custom.0 == vec!["hall"]);
        check!(visits(&state) == Some(1));
    }

    #[test]
    fn unit_host_outcome_is_a_no_op() {
        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        let mut option = DialogueOption::new(
            1,
            DialogText::inline("..."),
            DialogTarget::End,
            dramaturge_model::DialogueNodeId::new(1),
        );
        option.outcomes.push(dramaturge_model::Outcome::Host(()));
        let target = rt.choose_option(&option, &mut state);
        check!(target == DialogTarget::End);
        // Only the DialogOptionChosen event; the host outcome changes nothing.
        check!(state.events().len() == 1);
        check!(let dramaturge_model::EventKind::DialogOptionChosen { .. } = &state.events()[0].kind);
    }

    #[test]
    fn default_runtime_construction() {
        let rt = DefaultRuntime::new();
        check!(let DefaultConditionEvaluator { custom: NoCustom } = rt.evaluator);
        check!(let DefaultOutcomeApplier { custom: NoCustom } = rt.applier);
    }

    #[test]
    fn available_options_filters_by_condition() {
        let rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();
        state.set_fact(FactScope::World, Fact::bool("has_key", true));

        let speaker = CharacterId::new(EntityId::new(1));
        let node = DialogueNode {
            id: dramaturge_model::DialogueNodeId::new(1),
            speaker,
            text: DialogText::inline("Hello"),
            conditions: vec![],
            outcomes_on_enter: vec![],
            lines: vec![],
            next: None,
            options: vec![
                DialogueOption {
                    id: DialogueOptionId::new(1),
                    text: DialogText::inline("Always available"),
                    tags: vec![],
                    intentions: dramaturge_model::DialogIntentions::new(),
                    conditions: vec![],
                    outcomes: vec![],
                    target: DialogTarget::node(2),
                    source_node_id: dramaturge_model::DialogueNodeId::new(1),
                    navigation_hint: None,
                },
                DialogueOption {
                    id: DialogueOptionId::new(2),
                    text: DialogText::inline("Conditional"),
                    tags: vec![],
                    intentions: dramaturge_model::DialogIntentions::new(),
                    conditions: vec![Condition::FactEquals {
                        scope: FactScope::World,
                        fact_id: FactId::new("has_key"),
                        value: dramaturge_model::FactValue::Bool(true),
                    }],
                    outcomes: vec![],
                    target: DialogTarget::node(3),
                    source_node_id: dramaturge_model::DialogueNodeId::new(1),
                    navigation_hint: None,
                },
                DialogueOption {
                    id: DialogueOptionId::new(3),
                    text: DialogText::inline("Impossible"),
                    tags: vec![],
                    intentions: dramaturge_model::DialogIntentions::new(),
                    conditions: vec![Condition::Never],
                    outcomes: vec![],
                    target: DialogTarget::node(4),
                    source_node_id: dramaturge_model::DialogueNodeId::new(1),
                    navigation_hint: None,
                },
            ],
        };

        let available: Vec<_> = rt.available_options(&node, &state).collect();
        check!(available.len() == 2);
        check!(available.iter().any(|o| o.id == DialogueOptionId::new(1)));
        check!(available.iter().any(|o| o.id == DialogueOptionId::new(2)));
        check!(!available.iter().any(|o| o.id == DialogueOptionId::new(3)));
    }

    #[test]
    fn choose_option_applies_outcomes_and_returns_target() {
        let mut rt = DefaultRuntime::new();
        let mut state = DialogWorldState::new();

        let option = DialogueOption {
            id: DialogueOptionId::new(1),
            text: DialogText::inline("Yes"),
            tags: vec![],
            intentions: dramaturge_model::DialogIntentions::new(),
            conditions: vec![],
            outcomes: vec![dramaturge_model::Outcome::SetFact {
                scope: FactScope::World,
                fact: Fact::bool("agreed", true),
            }],
            target: DialogTarget::node(5),
            source_node_id: dramaturge_model::DialogueNodeId::new(1),
            navigation_hint: None,
        };

        let target = rt.choose_option(&option, &mut state);
        check!(let DialogTarget::Node(n) = target && n.0 == 5);
        check!(state.fact_exists(&FactScope::World, &FactId::new("agreed")));
        check!(state.events().len() == 2);
        match &state.events()[0].kind {
            dramaturge_model::EventKind::DialogOptionChosen {
                node_id, option_id, ..
            } => {
                check!(node_id.0 == 1);
                check!(option_id.0 == 1);
            }
            _ => panic!("expected DialogOptionChosen"),
        }
        match &state.events()[1].kind {
            dramaturge_model::EventKind::FactLearned { scope, fact_id } => {
                check!(matches!(scope, FactScope::World));
                check!(fact_id.0.as_ref() == "agreed");
            }
            _ => panic!("expected FactLearned"),
        }
    }
}
