use std::sync::Arc;

use crate::alignment::{AlignmentAxisId, AlignmentPoleId};
use crate::character::EntityId;
use crate::dialogue::{DialogueNodeId, DialogueOptionId};
use crate::fact::{FactId, FactScope};
use crate::intent::DialogIntentions;
use crate::relationship::RelationshipAxisId;
use crate::tag::DialogTagKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(pub u64);

impl EventId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventContext;

#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    DialogOptionChosen {
        node_id: DialogueNodeId,
        option_id: DialogueOptionId,
        tags: Vec<DialogTagKind>,
        intentions: DialogIntentions,
    },
    FactLearned {
        scope: FactScope,
        fact_id: FactId,
    },
    FactRevealed {
        from: EntityId,
        to: EntityId,
        fact_id: FactId,
    },
    RelationshipChanged {
        a: EntityId,
        b: EntityId,
        axis: RelationshipAxisId,
        delta: i32,
    },
    AlignmentChanged {
        axis: AlignmentAxisId,
        pole: AlignmentPoleId,
        delta: i32,
    },
    Custom {
        kind: Arc<str>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub id: EventId,
    pub kind: EventKind,
    pub actor: Option<EntityId>,
    pub target: Option<EntityId>,
}

impl Event {
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

    #[test]
    fn event_id_construction() {
        let id = EventId::new(1);
        assert_eq!(id.0, 1);
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
        assert!(matches!(kind, EventKind::DialogOptionChosen { .. }));
    }

    #[test]
    fn event_kind_fact_learned() {
        let kind = EventKind::FactLearned {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
        };
        assert!(matches!(kind, EventKind::FactLearned { .. }));
    }

    #[test]
    fn event_kind_custom() {
        let kind = EventKind::Custom {
            kind: Arc::from("custom_event"),
        };
        assert!(matches!(kind, EventKind::Custom { .. }));
    }

    #[test]
    fn event_construction() {
        let kind = EventKind::Custom {
            kind: Arc::from("test"),
        };
        let event = Event::new(EventId::new(1), kind);
        assert_eq!(event.id.0, 1);
        assert!(event.actor.is_none());
        assert!(event.target.is_none());
    }
}
