//! Outcomes — state changes triggered by dialogue choices.
//!
//! Outcomes represent effects applied to the dialogue world state when
//! a dialogue option is chosen or a node is entered. They cover fact
//! manipulation, alignment and relationship adjustments, memory creation,
//! event emission, quest management, and custom extension points.
//!
//! # Host outcomes
//!
//! [`Outcome`] has a type parameter `H` for typed host outcomes, with the
//! default `()`. A host puts its own effects in [`Outcome::Host`] instead
//! of encoding them as [`Outcome::Custom`] keys:
//!
//! ```rust
//! use dramaturge_model::{Outcome, FactScope, Fact};
//!
//! #[derive(Debug, Clone, PartialEq)]
//! enum Effect {
//!     Goto(&'static str),
//! }
//!
//! let outcomes: Vec<Outcome<Effect>> = vec![
//!     Outcome::SetFact { scope: FactScope::World, fact: Fact::bool("left", true) },
//!     Outcome::Host(Effect::Goto("street")),
//! ];
//! assert!(matches!(outcomes[1], Outcome::Host(Effect::Goto("street"))));
//! ```
//!
//! The graph types ([`DialogueGraph`](crate::DialogueGraph),
//! [`DialogueNode`](crate::DialogueNode),
//! [`DialogueOption`](crate::DialogueOption)) carry the same parameter. The
//! runtime hands `Host` outcomes to a `HostHandler` (in
//! `dramaturge-runtime`).
//!
//! Code that does not use host outcomes does not name `H`: `Outcome` is
//! `Outcome<()>`. Where the compiler can not infer `H` from the context,
//! for example `let o = Outcome::EmitEvent { kind };` alone, write the type:
//! `let o: Outcome = ...`.

use crate::alignment::{AlignmentAxisId, AlignmentPoleId};
use crate::character::EntityId;
use crate::event::EventKind;
use crate::fact::{Fact, FactId, FactScope, FactValue};
use crate::memory::Memory;
use crate::memory::MemoryId;
use crate::relationship::RelationshipAxisId;

/// Effect applied to the dialogue world state when triggered.
///
/// Outcomes are attached to [`DialogueOption`](crate::DialogueOption)
/// (applied when the option is chosen) or to [`DialogueNode`](crate::DialogueNode)
/// (applied when the node is entered).
///
/// `H` is the type of [host outcomes](Outcome::Host). The default `()`
/// means "no host outcomes". See the [module documentation](self).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Outcome<H = ()> {
    /// Sets or updates a scoped fact in the world state.
    SetFact {
        /// The scope in which to apply the fact.
        scope: FactScope,
        /// The fact data to set.
        fact: Fact,
    },
    /// Removes a scoped fact from the world state.
    RemoveFact {
        /// The scope from which to remove the fact.
        scope: FactScope,
        /// The identifier of the fact to remove.
        fact_id: FactId,
    },
    /// Adds `delta` to an integer fact.
    ///
    /// A missing fact starts at `0`. A fact that holds a value other than
    /// [`FactValue::Int`] does not change. The sum saturates at the `i64`
    /// limits.
    AddFact {
        /// The scope of the fact.
        scope: FactScope,
        /// The identifier of the fact to change.
        fact_id: FactId,
        /// The amount to add (positive or negative).
        delta: i64,
    },
    /// Adjusts an alignment score on a specific axis/pole.
    AddAlignment {
        /// The alignment axis to adjust.
        axis: AlignmentAxisId,
        /// The pole on the axis to adjust.
        pole: AlignmentPoleId,
        /// The amount to add (positive or negative).
        delta: i32,
    },
    /// Adjusts a relationship score between two entities.
    AddRelationship {
        /// The source entity in the relationship.
        source: EntityId,
        /// The target entity in the relationship.
        target: EntityId,
        /// The relationship axis to adjust.
        axis: RelationshipAxisId,
        /// The amount to add (positive or negative).
        delta: i32,
    },
    /// Adds a memory to an entity's memory bank.
    AddMemory {
        /// The entity that owns the memory.
        owner: EntityId,
        /// The memory to add.
        memory: Memory,
    },
    /// Removes a memory from an entity's memory bank.
    RemoveMemory {
        /// The entity that owns the memory.
        owner: EntityId,
        /// The identifier of the memory to remove.
        memory_id: MemoryId,
    },
    /// Emits a narrative event into the world state's event log.
    EmitEvent {
        /// The event kind to emit.
        kind: EventKind,
    },
    /// Marks a quest as started.
    /// Implemented via a fact-based placeholder (`quest_started_{quest_id}`).
    StartQuest {
        /// The identifier of the quest to start.
        quest_id: String,
    },
    /// Advances a quest to a new stage.
    /// Implemented via a fact-based placeholder (`quest_stage_{quest_id}`).
    AdvanceQuest {
        /// The identifier of the quest to advance.
        quest_id: String,
        /// The new stage identifier.
        stage: String,
    },
    /// User-defined custom outcome with arbitrary key and arguments.
    ///
    /// The default outcome applier hands it to its `CustomHandler`. For
    /// typed host data, use [`Outcome::Host`].
    Custom {
        /// Custom outcome identifier.
        key: String,
        /// Arbitrary fact value arguments for the outcome.
        args: Vec<FactValue>,
    },
    /// A typed host outcome.
    ///
    /// dramaturge does not look into the value. The default outcome
    /// applier hands it to its `HostHandler`. The validator does not know
    /// what it changes (for example, it does not count it as a writer of
    /// a fact).
    Host(H),
}

impl<H> Outcome<H> {
    /// Returns the host outcome, if this is [`Outcome::Host`].
    pub fn as_host(&self) -> Option<&H> {
        match self {
            Self::Host(host) => Some(host),
            _ => None,
        }
    }

    /// Converts the host outcome with `f` and keeps every other variant.
    ///
    /// Use it to move outcomes between host types, for example from
    /// `Outcome<()>` (a graph loaded from YAML) to `Outcome<Effect>`.
    pub fn map_host<G>(self, f: impl FnOnce(H) -> G) -> Outcome<G> {
        match self {
            Self::SetFact { scope, fact } => Outcome::SetFact { scope, fact },
            Self::RemoveFact { scope, fact_id } => Outcome::RemoveFact { scope, fact_id },
            Self::AddFact {
                scope,
                fact_id,
                delta,
            } => Outcome::AddFact {
                scope,
                fact_id,
                delta,
            },
            Self::AddAlignment { axis, pole, delta } => Outcome::AddAlignment { axis, pole, delta },
            Self::AddRelationship {
                source,
                target,
                axis,
                delta,
            } => Outcome::AddRelationship {
                source,
                target,
                axis,
                delta,
            },
            Self::AddMemory { owner, memory } => Outcome::AddMemory { owner, memory },
            Self::RemoveMemory { owner, memory_id } => Outcome::RemoveMemory { owner, memory_id },
            Self::EmitEvent { kind } => Outcome::EmitEvent { kind },
            Self::StartQuest { quest_id } => Outcome::StartQuest { quest_id },
            Self::AdvanceQuest { quest_id, stage } => Outcome::AdvanceQuest { quest_id, stage },
            Self::Custom { key, args } => Outcome::Custom { key, args },
            Self::Host(host) => Outcome::Host(f(host)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryTag;
    use assert2::check;

    #[test]
    fn outcome_set_fact() {
        let fact = Fact::bool("met_hero", true);
        let o: Outcome = Outcome::SetFact {
            scope: FactScope::World,
            fact,
        };
        check!(let Outcome::SetFact { .. } = o);
    }

    #[test]
    fn outcome_remove_fact() {
        let o: Outcome = Outcome::RemoveFact {
            scope: FactScope::World,
            fact_id: FactId::new("met_hero"),
        };
        check!(let Outcome::RemoveFact { .. } = o);
    }

    #[test]
    fn outcome_add_fact() {
        let o: Outcome = Outcome::AddFact {
            scope: FactScope::World,
            fact_id: FactId::new("gold"),
            delta: -3,
        };
        check!(let Outcome::AddFact { delta: -3, .. } = o);
    }

    #[test]
    fn outcome_add_alignment() {
        let o: Outcome = Outcome::AddAlignment {
            axis: AlignmentAxisId::new("moral"),
            pole: AlignmentPoleId::new("paragon"),
            delta: 10,
        };
        check!(let Outcome::AddAlignment { .. } = o);
    }

    #[test]
    fn outcome_add_memory() {
        let memory = Memory::new(MemoryId::new(1)).with_tag(MemoryTag::HelpedMe);
        let o: Outcome = Outcome::AddMemory {
            owner: EntityId::new(1),
            memory,
        };
        check!(let Outcome::AddMemory { owner, memory: m } = o && owner.0 == 1 && m.id.0 == 1);
    }

    #[test]
    fn outcome_remove_memory() {
        let o: Outcome = Outcome::RemoveMemory {
            owner: EntityId::new(1),
            memory_id: MemoryId::new(42),
        };
        check!(let Outcome::RemoveMemory { owner, memory_id } = o && owner.0 == 1 && memory_id.0 == 42);
    }

    #[test]
    fn outcome_add_relationship() {
        let o: Outcome = Outcome::AddRelationship {
            source: EntityId::new(1),
            target: EntityId::new(2),
            axis: RelationshipAxisId::new("trust"),
            delta: 5,
        };
        check!(let Outcome::AddRelationship { .. } = o);
    }

    #[test]
    fn outcome_emit_event() {
        use std::sync::Arc;
        let kind = EventKind::Custom {
            kind: Arc::from("test"),
        };
        let o: Outcome = Outcome::EmitEvent { kind };
        check!(let Outcome::EmitEvent { .. } = o);
    }

    #[test]
    fn outcome_start_quest() {
        let o: Outcome = Outcome::StartQuest {
            quest_id: "main_quest".to_string(),
        };
        check!(let Outcome::StartQuest { .. } = o);
    }

    #[test]
    fn outcome_advance_quest() {
        let o: Outcome = Outcome::AdvanceQuest {
            quest_id: "main_quest".to_string(),
            stage: "complete".to_string(),
        };
        check!(let Outcome::AdvanceQuest { .. } = o);
    }

    #[test]
    fn outcome_custom() {
        let o: Outcome = Outcome::Custom {
            key: "custom_outcome".to_string(),
            args: vec![],
        };
        check!(let Outcome::Custom { .. } = o);
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Effect {
        Goto(&'static str),
    }

    #[test]
    fn outcome_host() {
        let o = Outcome::Host(Effect::Goto("street"));
        check!(o.as_host() == Some(&Effect::Goto("street")));
        let unit: Outcome = Outcome::Host(());
        check!(unit.as_host() == Some(&()));
        let custom: Outcome<Effect> = Outcome::Custom {
            key: "k".into(),
            args: vec![],
        };
        check!(custom.as_host().is_none());
    }

    #[test]
    fn outcome_map_host() {
        let o = Outcome::Host(Effect::Goto("street"));
        let mapped = o.map_host(|Effect::Goto(room)| room.len());
        check!(mapped == Outcome::Host(6));

        let plain: Outcome = Outcome::AddFact {
            scope: FactScope::World,
            fact_id: FactId::new("gold"),
            delta: 2,
        };
        let typed: Outcome<Effect> = plain.clone().map_host(|()| Effect::Goto("never"));
        check!(let Outcome::AddFact { delta: 2, .. } = typed);
    }
}
