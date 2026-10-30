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
//! let restored = saved.restore(&mut v2);
//! assert_eq!(restored.get_relationship(player2, mara2, &RelationshipAxisId::new("trust")), Some(10));
//! ```

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
    /// Keys that `current` does not know yet (content removed since the
    /// save) are interned, so they get fresh ids that do not collide with
    /// current content. Ids that had no key at save time are kept as they
    /// were.
    pub fn restore(self, current: &mut SymbolTable) -> DialogWorldState {
        let saved: SymbolTable = self.symbols.into_model();
        self.state.into_model_with(&mut KeyRemap {
            saved: &saved,
            current,
        })
    }
}

/// An [`IdMap`] that translates ids from one symbol table to another by key.
#[derive(Debug)]
pub struct KeyRemap<'a> {
    /// The table the ids come from.
    pub saved: &'a SymbolTable,
    /// The table the ids go to. Unknown keys are interned here.
    pub current: &'a mut SymbolTable,
}

impl IdMap for KeyRemap<'_> {
    fn entity(&mut self, raw: u64) -> EntityId {
        match self.saved.entity_key(EntityId::new(raw)) {
            Some(key) => self.current.intern_entity(key),
            None => EntityId::new(raw),
        }
    }

    fn node(&mut self, raw: u64) -> DialogueNodeId {
        match self.saved.node_key(DialogueNodeId::new(raw)) {
            Some(key) => self.current.intern_node(key),
            None => DialogueNodeId::new(raw),
        }
    }

    fn option(&mut self, raw: u64) -> DialogueOptionId {
        match self.saved.option_key(DialogueOptionId::new(raw)) {
            Some(key) => self.current.intern_option(key),
            None => DialogueOptionId::new(raw),
        }
    }

    fn memory(&mut self, raw: u64) -> MemoryId {
        match self.saved.memory_key(model::MemoryId::new(raw)) {
            Some(key) => self.current.intern_memory(key),
            None => MemoryId::new(raw),
        }
    }
}
