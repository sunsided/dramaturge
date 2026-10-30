//! Save games that store keys, not numbers.
//!
//! Runtime ids are dense numbers handed out by a
//! [`SymbolTable`](model::SymbolTable) when content is loaded. Adding or
//! removing content shifts them, so a save that stores raw ids breaks.
//! [`SavedWorldState`] stores the state together with the keys of every id
//! it was saved with. [`SavedWorldState::restore`] re-resolves each id by
//! key against the *current* symbol table, so the save stays valid when
//! content changes.
//!
//! ```rust
//! use dramaturge_model::{Fact, FactScope, RelationshipAxisId, SymbolTable};
//! use dramaturge_runtime::DialogWorldState;
//! use dramaturge_serde::save::SavedWorldState;
//!
//! // Content v1 knows two characters.
//! let mut v1 = SymbolTable::new();
//! let player = v1.intern_entity("player");
//! let mara = v1.intern_entity("mara");
//! let mut state = DialogWorldState::new();
//! state.add_relationship(player, mara, RelationshipAxisId::new("trust"), 10);
//!
//! let json = serde_json::to_string(&SavedWorldState::new(&state, &v1)).unwrap();
//!
//! // Content v2 added a character before them: the numbers moved.
//! let mut v2 = SymbolTable::new();
//! v2.intern_entity("orren");
//! let player2 = v2.intern_entity("player");
//! let mara2 = v2.intern_entity("mara");
//!
//! let saved: SavedWorldState = serde_json::from_str(&json).unwrap();
//! let (restored, report) = saved.restore(&mut v2);
//! assert!(report.orphaned.is_empty());
//! assert_eq!(restored.get_relationship(player2, mara2, &RelationshipAxisId::new("trust")), Some(10));
//! ```

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use dramaturge_model as model;
use dramaturge_model::{DialogueNodeId, DialogueOptionId, EntityId, MemoryId, SymbolTable};
use dramaturge_runtime::DialogWorldState;

use crate::dto::{IdMap, IntoModel, SymbolTableDto, WorldStateDto};

/// A [`DialogWorldState`] with the keys of the ids it references.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedWorldState {
    /// Keys of the ids at save time.
    pub symbols: SymbolTableDto,
    /// The state, with ids as they were at save time.
    pub state: WorldStateDto,
}

/// The id namespace a [`OrphanedKey`] belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    /// An entity key.
    Entity,
    /// A dialogue node key.
    Node,
    /// A dialogue option key.
    Option,
    /// A memory key.
    Memory,
}

/// A key the saved state references but the current content does not define.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OrphanedKey {
    /// The namespace of the key.
    pub kind: SymbolKind,
    /// The key.
    pub key: String,
}

/// What [`SavedWorldState::restore`] could not match to current content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreReport {
    /// Keys the save references that the current symbol table did not know.
    /// They were interned with fresh ids, so the state stays loadable.
    /// Sorted and without duplicates.
    pub orphaned: Vec<OrphanedKey>,
}

impl SavedWorldState {
    /// Captures `state` together with the keys in `symbols`.
    pub fn new(state: &DialogWorldState, symbols: &SymbolTable) -> Self {
        Self {
            symbols: symbols.into(),
            state: state.into(),
        }
    }

    /// Rebuilds the state, re-resolving every id by key against `current`.
    ///
    /// Keys that `current` does not know (content removed since the save)
    /// are interned, so they get fresh ids that do not collide with current
    /// content, and are listed in the returned [`RestoreReport`]. A host that
    /// wants strict loading rejects the save when `report.orphaned` is not
    /// empty. Ids that had no key at save time are kept as they were.
    pub fn restore(&self, current: &mut SymbolTable) -> (DialogWorldState, RestoreReport) {
        let saved: SymbolTable = self.symbols.clone().into_model();
        let mut remap = KeyRemap::new(&saved, current);
        let state = self.state.clone().into_model_with(&mut remap);
        (state, remap.into_report())
    }
}

/// An [`IdMap`] that translates ids from one symbol table to another by key.
#[derive(Debug)]
pub struct KeyRemap<'a> {
    saved: &'a SymbolTable,
    current: &'a mut SymbolTable,
    orphaned: BTreeSet<OrphanedKey>,
}

impl<'a> KeyRemap<'a> {
    /// Maps ids from `saved` to `current`. Unknown keys are interned into
    /// `current` and recorded as orphaned.
    pub fn new(saved: &'a SymbolTable, current: &'a mut SymbolTable) -> Self {
        Self {
            saved,
            current,
            orphaned: BTreeSet::new(),
        }
    }

    /// Returns the keys that were missing from the current table.
    pub fn into_report(self) -> RestoreReport {
        RestoreReport {
            orphaned: self.orphaned.into_iter().collect(),
        }
    }

    fn note_orphan(&mut self, kind: SymbolKind, key: &str, known: bool) {
        if !known {
            self.orphaned.insert(OrphanedKey {
                kind,
                key: key.to_string(),
            });
        }
    }
}

impl IdMap for KeyRemap<'_> {
    fn entity(&mut self, raw: u64) -> EntityId {
        let Some(key) = self.saved.entity_key(EntityId::new(raw)) else {
            return EntityId::new(raw);
        };
        self.note_orphan(SymbolKind::Entity, key, self.current.entity(key).is_some());
        self.current.intern_entity(key)
    }

    fn node(&mut self, raw: u64) -> DialogueNodeId {
        let Some(key) = self.saved.node_key(DialogueNodeId::new(raw)) else {
            return DialogueNodeId::new(raw);
        };
        self.note_orphan(SymbolKind::Node, key, self.current.node(key).is_some());
        self.current.intern_node(key)
    }

    fn option(&mut self, raw: u64) -> DialogueOptionId {
        let Some(key) = self.saved.option_key(DialogueOptionId::new(raw)) else {
            return DialogueOptionId::new(raw);
        };
        self.note_orphan(SymbolKind::Option, key, self.current.option(key).is_some());
        self.current.intern_option(key)
    }

    fn memory(&mut self, raw: u64) -> MemoryId {
        let Some(key) = self.saved.memory_key(model::MemoryId::new(raw)) else {
            return MemoryId::new(raw);
        };
        self.note_orphan(SymbolKind::Memory, key, self.current.memory(key).is_some());
        self.current.intern_memory(key)
    }
}
