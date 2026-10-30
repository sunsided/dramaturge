//! Symbol tables — stable author keys mapped to dense runtime ids.
//!
//! Authors refer to content by readable, stable keys such as `npc.mara` or
//! `dlg.mara.intro`. The runtime works with small `Copy` ids
//! ([`EntityId`], [`DialogueNodeId`], [`DialogueOptionId`], [`MemoryId`]),
//! which are cheap to clone, hash and compare.
//!
//! A [`SymbolTable`] bridges the two: a loader (or compile step) interns
//! each key once and hands out a dense number for it. The table maps the
//! number back to its key, so error messages, debug output and save games
//! can show keys instead of numbers.
//!
//! Each id kind has its own namespace: the entity key `mara` and the node
//! key `mara` are unrelated.
//!
//! [`EventId`](crate::EventId) is deliberately not interned. Event ids are
//! allocated by the runtime, not written by authors.
//!
//! # Example
//!
//! ```rust
//! use dramaturge_model::{DialogueNodeId, SymbolTable};
//!
//! let mut symbols = SymbolTable::new();
//! let intro = symbols.intern_node("dlg.mara.intro");
//! let again = symbols.intern_node("dlg.mara.intro");
//! assert_eq!(intro, again);
//!
//! assert_eq!(symbols.node("dlg.mara.intro"), Some(intro));
//! assert_eq!(symbols.node_key(intro), Some("dlg.mara.intro"));
//! assert_eq!(symbols.node_key(DialogueNodeId::new(99)), None);
//! ```

use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::character::EntityId;
use crate::dialogue::{DialogueNodeId, DialogueOptionId};
use crate::memory::MemoryId;

/// A numeric id type that a [`SymbolTable`] can hand out.
///
/// Implemented for [`EntityId`], [`DialogueNodeId`], [`DialogueOptionId`]
/// and [`MemoryId`].
pub trait SymbolId: Copy {
    /// Builds the id from its raw number.
    fn from_raw(raw: u64) -> Self;
    /// Returns the raw number of the id.
    fn raw(self) -> u64;
}

macro_rules! impl_symbol_id {
    ($($ty:ty),* $(,)?) => {
        $(
            impl SymbolId for $ty {
                fn from_raw(raw: u64) -> Self {
                    Self(raw)
                }

                fn raw(self) -> u64 {
                    self.0
                }
            }
        )*
    };
}

impl_symbol_id!(EntityId, DialogueNodeId, DialogueOptionId, MemoryId);

/// Two-way map between string keys and dense numeric ids of one kind.
///
/// Ids are handed out in insertion order, starting at zero. Interning the
/// same key twice returns the same id.
#[derive(Clone)]
pub struct Interner<Id> {
    keys: Vec<Arc<str>>,
    ids: HashMap<Arc<str>, u64>,
    _id: PhantomData<fn() -> Id>,
}

impl<Id> Default for Interner<Id> {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            ids: HashMap::new(),
            _id: PhantomData,
        }
    }
}

impl<Id> fmt::Debug for Interner<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.keys.iter()).finish()
    }
}

impl<Id> PartialEq for Interner<Id> {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys
    }
}

impl<Id> Eq for Interner<Id> {}

impl<Id: SymbolId> Interner<Id> {
    /// Creates an empty interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the id for `key`, allocating the next free id if the key is new.
    pub fn intern(&mut self, key: &str) -> Id {
        if let Some(&raw) = self.ids.get(key) {
            return Id::from_raw(raw);
        }
        let raw = self.keys.len() as u64;
        let key: Arc<str> = Arc::from(key);
        self.keys.push(key.clone());
        self.ids.insert(key, raw);
        Id::from_raw(raw)
    }

    /// Returns the id for `key`, or `None` if the key was never interned.
    pub fn get(&self, key: &str) -> Option<Id> {
        self.ids.get(key).map(|&raw| Id::from_raw(raw))
    }

    /// Returns the key for `id`, or `None` if this interner did not hand it out.
    pub fn key(&self, id: Id) -> Option<&str> {
        usize::try_from(id.raw())
            .ok()
            .and_then(|index| self.keys.get(index))
            .map(|key| &**key)
    }

    /// Returns the number of interned keys.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Returns `true` if no key was interned.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Iterates over `(id, key)` pairs in id order.
    pub fn iter(&self) -> impl Iterator<Item = (Id, &str)> {
        self.keys
            .iter()
            .enumerate()
            .map(|(index, key)| (Id::from_raw(index as u64), &**key))
    }
}

/// Maps author keys to dense runtime ids, one namespace per id kind.
///
/// Loaders return a `SymbolTable` together with the model they build.
/// Hosts use it to look up ids by key, to print keys in diagnostics, and
/// to store keys (not numbers) in save games, so a save stays valid when
/// content is added or removed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolTable {
    /// Entity keys (characters and other participants).
    pub entities: Interner<EntityId>,
    /// Dialogue node keys.
    pub nodes: Interner<DialogueNodeId>,
    /// Dialogue option keys.
    pub options: Interner<DialogueOptionId>,
    /// Memory keys.
    pub memories: Interner<MemoryId>,
}

macro_rules! symbol_accessors {
    ($field:ident, $id:ty, $get:ident, $key:ident, $intern:ident, $what:literal) => {
        #[doc = concat!("Returns the ", $what, " id for `key`, if interned.")]
        pub fn $get(&self, key: &str) -> Option<$id> {
            self.$field.get(key)
        }

        #[doc = concat!("Returns the key of the ", $what, " id, if known.")]
        pub fn $key(&self, id: $id) -> Option<&str> {
            self.$field.key(id)
        }

        #[doc = concat!("Returns the ", $what, " id for `key`, allocating one if needed.")]
        pub fn $intern(&mut self, key: &str) -> $id {
            self.$field.intern(key)
        }
    };
}

impl SymbolTable {
    /// Creates an empty symbol table.
    pub fn new() -> Self {
        Self::default()
    }

    symbol_accessors!(
        entities,
        EntityId,
        entity,
        entity_key,
        intern_entity,
        "entity"
    );
    symbol_accessors!(nodes, DialogueNodeId, node, node_key, intern_node, "node");
    symbol_accessors!(
        options,
        DialogueOptionId,
        option,
        option_key,
        intern_option,
        "option"
    );
    symbol_accessors!(
        memories,
        MemoryId,
        memory,
        memory_key,
        intern_memory,
        "memory"
    );

    /// Formats an entity id as its key, or as `#<number>` if unknown.
    pub fn display_entity(&self, id: EntityId) -> String {
        display(self.entity_key(id), id.0)
    }

    /// Formats a node id as its key, or as `#<number>` if unknown.
    pub fn display_node(&self, id: DialogueNodeId) -> String {
        display(self.node_key(id), id.0)
    }

    /// Formats an option id as its key, or as `#<number>` if unknown.
    pub fn display_option(&self, id: DialogueOptionId) -> String {
        display(self.option_key(id), id.0)
    }

    /// Formats a memory id as its key, or as `#<number>` if unknown.
    pub fn display_memory(&self, id: MemoryId) -> String {
        display(self.memory_key(id), id.0)
    }
}

fn display(key: Option<&str>, raw: u64) -> String {
    match key {
        Some(key) => format!("'{key}'"),
        None => format!("#{raw}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn intern_is_idempotent_and_dense() {
        let mut symbols = SymbolTable::new();
        let a = symbols.intern_entity("npc.mara");
        let b = symbols.intern_entity("npc.orren");
        check!(a == EntityId::new(0));
        check!(b == EntityId::new(1));
        check!(symbols.intern_entity("npc.mara") == a);
        check!(symbols.entities.len() == 2);
    }

    #[test]
    fn namespaces_are_independent() {
        let mut symbols = SymbolTable::new();
        let entity = symbols.intern_entity("mara");
        let node = symbols.intern_node("mara");
        check!(entity.0 == 0);
        check!(node.0 == 0);
        check!(symbols.option("mara").is_none());
    }

    #[test]
    fn reverse_lookup() {
        let mut symbols = SymbolTable::new();
        let id = symbols.intern_option("opt.comfort");
        check!(symbols.option_key(id) == Some("opt.comfort"));
        check!(symbols.option_key(DialogueOptionId::new(7)).is_none());
    }

    #[test]
    fn display_falls_back_to_number() {
        let mut symbols = SymbolTable::new();
        let id = symbols.intern_node("dlg.intro");
        check!(symbols.display_node(id) == "'dlg.intro'");
        check!(symbols.display_node(DialogueNodeId::new(42)) == "#42");
    }

    #[test]
    fn iter_in_id_order() {
        let mut symbols = SymbolTable::new();
        symbols.intern_memory("m.a");
        symbols.intern_memory("m.b");
        let pairs: Vec<_> = symbols.memories.iter().map(|(id, k)| (id.0, k)).collect();
        check!(pairs == vec![(0, "m.a"), (1, "m.b")]);
    }
}
