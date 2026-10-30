//! Dialogue runtime trait and default implementation for driving conversations.
//!
//! The [`DialogueRuntime`] trait defines the interface for driving a
//! dialogue session: filtering available options by conditions and
//! applying outcomes when an option is chosen. [`DefaultRuntime`]
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
use dramaturge_model::{DialogTarget, DialogueNode, DialogueOption, EventKind};

/// Trait defining the interface for driving a dialogue session.
///
/// Implementations determine which options are available at a given
/// node and handle the consequences of choosing an option.
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

        for outcome in &option.outcomes {
            self.applier.apply(outcome, state);
        }
        option.target.clone()
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
