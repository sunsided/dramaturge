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
//! `S`, which defaults to [`DialogWorldState`].

use crate::context::DialogWorldState;
use crate::evaluation::{
    ConditionEvaluator, DefaultConditionEvaluator, DefaultOutcomeApplier, OutcomeApplier,
};
use crate::state::NarrativeState;
use dramaturge_model::{Condition, DialogTarget, DialogueNode, DialogueOption, EventKind, Outcome};

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

/// Trait defining the interface for driving a dialogue session.
///
/// Implementations determine which options are available at a given
/// node and handle the consequences of choosing an option.
///
/// An implementation gives the two primitives
/// [`conditions_hold`](Self::conditions_hold) and
/// [`apply_outcomes`](Self::apply_outcomes). [`enter_node`](Self::enter_node)
/// has a default implementation that uses them.
pub trait DialogueRuntime<S: NarrativeState + ?Sized = DialogWorldState> {
    /// The iterator type returned by [`available_options`](Self::available_options).
    ///
    /// This associated type allows implementors to choose their own iterator
    /// type while maintaining the trait interface. Implementors can return
    /// `std::slice::Iter`, `std::iter::Filter`, custom iterators, or
    /// `Box<dyn Iterator>` as needed.
    type OptionsIter<'a>: Iterator<Item = &'a DialogueOption>
    where
        Self: 'a,
        S: 'a;

    /// Returns references to options whose conditions are satisfied.
    ///
    /// Options with no conditions are always included. Options with
    /// conditions are included only if all their conditions evaluate
    /// to `true`.
    fn available_options<'a>(
        &'a self,
        node: &'a DialogueNode,
        state: &'a S,
    ) -> Self::OptionsIter<'a>;

    /// Applies the option's outcomes and returns the target to transition to.
    ///
    /// This mutates the state by applying each outcome in order,
    /// then returns the option's target for the next navigation step.
    fn choose_option(&mut self, option: &DialogueOption, state: &mut S) -> DialogTarget;

    /// Returns `true` if all `conditions` hold for `state`.
    ///
    /// An empty list holds.
    fn conditions_hold(&self, conditions: &[Condition], state: &S) -> bool;

    /// Applies `outcomes` to `state`, in order.
    fn apply_outcomes(&mut self, outcomes: &[Outcome], state: &mut S);

    /// Enters `node`: checks its conditions, applies its outcomes on enter
    /// and reports if it has an available option.
    ///
    /// If the node's [`conditions`](DialogueNode::conditions) do not hold,
    /// nothing changes and the result is [`EnterResult::Blocked`].
    /// Otherwise the [`outcomes_on_enter`](DialogueNode::outcomes_on_enter)
    /// are applied once, in order, and the result is
    /// [`EnterResult::Entered`]. `has_options` is computed after the
    /// outcomes, so an outcome on enter can make an option available.
    ///
    /// Call this once per visit of a node. A host that renders the same
    /// node again (for example after a screen redraw) does not call it
    /// again, or the outcomes apply twice.
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
    fn enter_node(&mut self, node: &DialogueNode, state: &mut S) -> EnterResult {
        if !self.conditions_hold(&node.conditions, state) {
            return EnterResult::Blocked;
        }
        self.apply_outcomes(&node.outcomes_on_enter, state);
        let has_options = self.available_options(node, state).next().is_some();
        EnterResult::Entered { has_options }
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
}

/// Iterator over available dialogue options.
///
/// This struct is returned by [`DefaultRuntime::available_options`] and
/// yields references to options whose conditions are satisfied.
pub struct AvailableOptionsIter<'a, E, S: ?Sized = DialogWorldState> {
    inner: std::slice::Iter<'a, DialogueOption>,
    evaluator: &'a E,
    state: &'a S,
}

impl<'a, E, S: ?Sized> AvailableOptionsIter<'a, E, S> {
    /// Creates an iterator over the options of `node` that pass `evaluator`.
    pub fn new(node: &'a DialogueNode, evaluator: &'a E, state: &'a S) -> Self {
        Self {
            inner: node.options.iter(),
            evaluator,
            state,
        }
    }
}

impl<'a, E, S> Iterator for AvailableOptionsIter<'a, E, S>
where
    E: ConditionEvaluator<S>,
    S: NarrativeState + ?Sized,
{
    type Item = &'a DialogueOption;

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

impl<S, E, A> DialogueRuntime<S> for DefaultRuntime<E, A>
where
    S: NarrativeState + ?Sized,
    E: ConditionEvaluator<S>,
    A: OutcomeApplier<S>,
{
    type OptionsIter<'a>
        = AvailableOptionsIter<'a, E, S>
    where
        Self: 'a,
        S: 'a;

    fn available_options<'a>(
        &'a self,
        node: &'a DialogueNode,
        state: &'a S,
    ) -> Self::OptionsIter<'a> {
        AvailableOptionsIter::new(node, &self.evaluator, state)
    }

    fn choose_option(&mut self, option: &DialogueOption, state: &mut S) -> DialogTarget {
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

    fn apply_outcomes(&mut self, outcomes: &[Outcome], state: &mut S) {
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
    fn enter_node_applies_outcomes_once() {
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
