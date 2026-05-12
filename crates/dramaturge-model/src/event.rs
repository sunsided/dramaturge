//! Narrative event types.

/// An event that occurs within the narrative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// A unique name or identifier for this event.
    pub name: String,
}

impl Event {
    /// Create a new [`Event`] with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_construction() {
        let e = Event::new("quest_started");
        assert_eq!(e.name, "quest_started");
    }
}
