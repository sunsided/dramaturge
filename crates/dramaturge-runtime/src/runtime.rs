use dramaturge_model::{DialogTarget, DialogueNode, DialogueOption, DialogueOptionId};

use crate::context::DialogWorldState;
use crate::evaluation::{
    ConditionEvaluator, DefaultConditionEvaluator, DefaultOutcomeApplier, OutcomeApplier,
};

pub trait DialogueRuntime {
    fn available_options(
        &self,
        node: &DialogueNode,
        state: &DialogWorldState,
    ) -> Vec<DialogueOptionId>;

    fn choose_option(
        &mut self,
        option: &DialogueOption,
        state: &mut DialogWorldState,
    ) -> DialogTarget;
}

#[derive(Debug, Clone, Default)]
pub struct DefaultRuntime {
    pub evaluator: DefaultConditionEvaluator,
    pub applier: DefaultOutcomeApplier,
}

impl DefaultRuntime {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DialogueRuntime for DefaultRuntime {
    fn available_options(
        &self,
        node: &DialogueNode,
        state: &DialogWorldState,
    ) -> Vec<DialogueOptionId> {
        node.options
            .iter()
            .filter(|option| {
                option.conditions.is_empty()
                    || option
                        .conditions
                        .iter()
                        .all(|c| self.evaluator.evaluate(c, state))
            })
            .map(|option| option.id)
            .collect()
    }

    fn choose_option(
        &mut self,
        option: &DialogueOption,
        state: &mut DialogWorldState,
    ) -> DialogTarget {
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
    use dramaturge_model::{
        CharacterId, Condition, DialogText, DialogueOption, EntityId, Fact, FactId, FactScope,
    };

    #[test]
    fn default_runtime_construction() {
        let rt = DefaultRuntime::new();
        assert!(matches!(rt.evaluator, DefaultConditionEvaluator));
        assert!(matches!(rt.applier, DefaultOutcomeApplier));
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
                },
                DialogueOption {
                    id: DialogueOptionId::new(3),
                    text: DialogText::inline("Impossible"),
                    tags: vec![],
                    intentions: dramaturge_model::DialogIntentions::new(),
                    conditions: vec![Condition::Never],
                    outcomes: vec![],
                    target: DialogTarget::node(4),
                },
            ],
        };

        let available = rt.available_options(&node, &state);
        assert_eq!(available.len(), 2);
        assert!(available.contains(&DialogueOptionId::new(1)));
        assert!(available.contains(&DialogueOptionId::new(2)));
        assert!(!available.contains(&DialogueOptionId::new(3)));
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
        };

        let target = rt.choose_option(&option, &mut state);
        assert!(matches!(target, DialogTarget::Node(n) if n.0 == 5));
        assert!(state.fact_exists(&FactScope::World, &FactId::new("agreed")));
    }
}
