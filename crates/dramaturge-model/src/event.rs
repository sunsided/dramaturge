//! Narrative events that record state changes and player choices.
//!
//! Events capture moments in the narrative timeline — dialogue choices,
//! facts learned, relationship changes, and more. Each [`Event`] has a
//! unique [`EventId`], a classified [`EventKind`], and optional actor
//! and target entities.

use std::sync::Arc;

use crate::alignment::{AlignmentAxisId, AlignmentPoleId};
use crate::character::EntityId;
use crate::dialogue::{DialogueNodeId, DialogueOptionId};
use crate::fact::{FactId, FactScope};
use crate::intent::DialogIntentions;
use crate::relationship::RelationshipAxisId;
use crate::tag::DialogTagKind;

/// Unique numeric identifier for an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(pub u64);

impl EventId {
    /// Creates a new event identifier from the given `u64` value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Placeholder for future context data attached to events.
///
/// Currently a unit struct with no fields. It exists as a placeholder
/// for additional context information that may be attached to events
/// in future iterations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventContext;

/// Classification of what kind of narrative event occurred.
///
/// Each variant captures a different type of narrative occurrence,
/// from player choices to state mutations.
#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    /// A dialogue option was chosen by the player.
    DialogOptionChosen {
        /// The dialogue node where the choice was made.
        node_id: DialogueNodeId,
        /// The dialogue option that was selected.
        option_id: DialogueOptionId,
        /// Tags attached to the chosen option.
        tags: Vec<DialogTagKind>,
        /// Intentions associated with the choice.
        intentions: DialogIntentions,
    },
    /// A fact was learned within a specific scope.
    FactLearned {
        /// The scope in which the fact was learned.
        scope: FactScope,
        /// The identifier of the fact that was learned.
        fact_id: FactId,
    },
    /// A fact was revealed from one entity to another.
    FactRevealed {
        /// The entity revealing the fact.
        from: EntityId,
        /// The entity receiving the revelation.
        to: EntityId,
        /// The identifier of the fact being revealed.
        fact_id: FactId,
    },
    /// A relationship score changed between two entities.
    RelationshipChanged {
        /// The first entity in the relationship.
        a: EntityId,
        /// The second entity in the relationship.
        b: EntityId,
        /// The relationship axis that changed.
        axis: RelationshipAxisId,
        /// The amount of change (positive or negative).
        delta: i32,
    },
    /// An alignment score changed on a specific axis/pole.
    AlignmentChanged {
        /// The alignment axis that changed.
        axis: AlignmentAxisId,
        /// The pole on the axis that changed.
        pole: AlignmentPoleId,
        /// The amount of change (positive or negative).
        delta: i32,
    },
    /// A user-defined custom event with an arbitrary kind identifier.
    Custom {
        /// Custom event kind identifier.
        kind: Arc<str>,
    },
}

/// A recorded event in the narrative timeline.
///
/// Events are immutable records of what occurred. They are appended to
/// the world state's event log as the dialogue progresses.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Unique identifier for this event.
    pub id: EventId,
    /// Classification of what kind of event this is.
    pub kind: EventKind,
    /// The entity that caused or initiated this event, if any.
    pub actor: Option<EntityId>,
    /// The entity targeted by this event, if any.
    pub target: Option<EntityId>,
}

impl Event {
    /// Creates a new event with the given ID and kind.
    ///
    /// The actor and target both default to `None`.
    pub fn new(id: EventId, kind: EventKind) -> Self {
        Self {
            id,
            kind,
            actor: None,
            target: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn event_id_construction() {
        let id = EventId::new(1);
        check!(id.0 == 1);
    }

    #[test]
    fn event_context_is_unit() {
        let ctx = EventContext;
        let _ = ctx;
    }

    #[test]
    fn event_kind_dialog_option_chosen() {
        let kind = EventKind::DialogOptionChosen {
            node_id: DialogueNodeId::new(1),
            option_id: DialogueOptionId::new(1),
            tags: Vec::new(),
            intentions: DialogIntentions::new(),
        };
        check!(let EventKind::DialogOptionChosen { .. } = kind);
    }

    #[test]
    fn event_kind_fact_learned() {
        let kind = EventKind::FactLearned {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
        };
        check!(let EventKind::FactLearned { .. } = kind);
    }

    #[test]
    fn event_kind_custom() {
        let kind = EventKind::Custom {
            kind: Arc::from("custom_event"),
        };
        check!(let EventKind::Custom { .. } = kind);
    }

    #[test]
    fn event_construction() {
        let kind = EventKind::Custom {
            kind: Arc::from("test"),
        };
        let event = Event::new(EventId::new(1), kind);
        check!(event.id.0 == 1);
        check!(event.actor.is_none());
        check!(event.target.is_none());
    }
}
