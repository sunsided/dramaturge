use std::sync::Arc;

use crate::fact::FactId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Intention {
    Learn(FactId),
    Reveal(FactId),
    Hide(FactId),
    Persuade,
    Intimidate,
    Comfort,
    Deceive,
    Deflect,
    Escalate,
    Deescalate,
    Trade,
    Recruit,
    Romance,
    Exit,
    Custom(Arc<str>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogIntentions {
    pub explicit: Vec<Intention>,
    pub implicit: Vec<Intention>,
}

impl DialogIntentions {
    pub fn new() -> Self {
        Self {
            explicit: Vec::new(),
            implicit: Vec::new(),
        }
    }

    pub fn with_explicit(mut self, intention: Intention) -> Self {
        self.explicit.push(intention);
        self
    }

    pub fn with_implicit(mut self, intention: Intention) -> Self {
        self.implicit.push(intention);
        self
    }
}

impl Default for DialogIntentions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intention_fact_variants() {
        let id = FactId::new("met_hero");
        let learn = Intention::Learn(id.clone());
        let reveal = Intention::Reveal(id.clone());
        let hide = Intention::Hide(id);
        assert!(matches!(learn, Intention::Learn(_)));
        assert!(matches!(reveal, Intention::Reveal(_)));
        assert!(matches!(hide, Intention::Hide(_)));
    }

    #[test]
    fn intention_action_variants() {
        assert!(matches!(Intention::Persuade, Intention::Persuade));
        assert!(matches!(Intention::Intimidate, Intention::Intimidate));
        assert!(matches!(Intention::Exit, Intention::Exit));
    }

    #[test]
    fn intention_custom() {
        let c = Intention::Custom(Arc::from("my_action"));
        assert!(matches!(c, Intention::Custom(_)));
    }

    #[test]
    fn dialog_intentions_default() {
        let intentions = DialogIntentions::default();
        assert!(intentions.explicit.is_empty());
        assert!(intentions.implicit.is_empty());
    }

    #[test]
    fn dialog_intentions_builder() {
        let id = FactId::new("secret");
        let intentions = DialogIntentions::new()
            .with_explicit(Intention::Learn(id.clone()))
            .with_implicit(Intention::Hide(id));
        assert_eq!(intentions.explicit.len(), 1);
        assert_eq!(intentions.implicit.len(), 1);
    }
}
