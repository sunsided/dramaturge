//! Dialog intentions — the communicative purpose behind a dialogue option.
//!
//! Intentions describe what a character aims to achieve through a dialogue
//! option. They complement [`DialogTagKind`](crate::tag::DialogTagKind)
//! tags by capturing the *purpose* rather than the *category* of an
//! utterance. Intentions can be explicit (set by the authoring tool) or
//! implicit (inferred by the runtime or other systems).

use std::sync::Arc;

use crate::fact::FactId;

/// What a character aims to achieve through a dialogue option.
///
/// Each variant represents a distinct communicative goal. Variants that
/// reference facts use [`FactId`] to identify the specific fact involved.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Intention {
    /// Attempt to learn a fact from the other party.
    Learn(FactId),
    /// Attempt to reveal a fact to the other party.
    Reveal(FactId),
    /// Attempt to conceal a fact from the other party.
    Hide(FactId),
    /// Attempt to persuade the other party.
    Persuade,
    /// Attempt to intimidate the other party.
    Intimidate,
    /// Attempt to comfort the other party.
    Comfort,
    /// Attempt to deceive the other party.
    Deceive,
    /// Attempt to deflect the conversation away from a topic.
    Deflect,
    /// Attempt to escalate the emotional intensity of the conversation.
    Escalate,
    /// Attempt to deescalate the emotional intensity of the conversation.
    Deescalate,
    /// Attempt to negotiate a trade or exchange.
    Trade,
    /// Attempt to recruit the other party.
    Recruit,
    /// Attempt to romantically engage the other party.
    Romance,
    /// Attempt to exit the conversation.
    Exit,
    /// User-defined custom intention with an arbitrary identifier.
    Custom(Arc<str>),
}

/// Container for explicit and implicit dialogue intentions.
///
/// Separates intentions that the authoring tool attached explicitly from
/// those inferred by the runtime or other systems.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DialogIntentions {
    /// Intentions the authoring tool explicitly attached.
    pub explicit: Vec<Intention>,
    /// Intentions inferred by the runtime or other systems.
    pub implicit: Vec<Intention>,
}

impl DialogIntentions {
    /// Creates empty intentions with no explicit or implicit entries.
    pub fn new() -> Self {
        Self {
            explicit: Vec::new(),
            implicit: Vec::new(),
        }
    }

    /// Builder method — adds an explicit intention and returns self.
    pub fn with_explicit(mut self, intention: Intention) -> Self {
        self.explicit.push(intention);
        self
    }

    /// Builder method — adds an implicit intention and returns self.
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
    use assert2::check;

    #[test]
    fn intention_fact_variants() {
        let id = FactId::new("met_hero");
        let learn = Intention::Learn(id.clone());
        let reveal = Intention::Reveal(id.clone());
        let hide = Intention::Hide(id);
        check!(let Intention::Learn(_) = learn);
        check!(let Intention::Reveal(_) = reveal);
        check!(let Intention::Hide(_) = hide);
    }

    #[test]
    fn intention_action_variants() {
        check!(let Intention::Persuade = Intention::Persuade);
        check!(let Intention::Intimidate = Intention::Intimidate);
        check!(let Intention::Exit = Intention::Exit);
    }

    #[test]
    fn intention_custom() {
        let c = Intention::Custom(Arc::from("my_action"));
        check!(let Intention::Custom(_) = c);
    }

    #[test]
    fn dialog_intentions_default() {
        let intentions = DialogIntentions::default();
        check!(intentions.explicit.is_empty());
        check!(intentions.implicit.is_empty());
    }

    #[test]
    fn dialog_intentions_builder() {
        let id = FactId::new("secret");
        let intentions = DialogIntentions::new()
            .with_explicit(Intention::Learn(id.clone()))
            .with_implicit(Intention::Hide(id));
        check!(intentions.explicit.len() == 1);
        check!(intentions.implicit.len() == 1);
    }
}
