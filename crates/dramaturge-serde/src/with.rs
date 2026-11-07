//! `#[serde(with = ...)]` modules for model and runtime types in host structs.
//!
//! ```rust
//! use dramaturge_runtime::DialogWorldState;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct GameState {
//!     room: String,
//!     #[serde(with = "dramaturge_serde::world_state")]
//!     dialog: DialogWorldState,
//! }
//!
//! let game = GameState { room: "shore".into(), dialog: DialogWorldState::new() };
//! let json = serde_json::to_string(&game).unwrap();
//! let back: GameState = serde_json::from_str(&json).unwrap();
//! assert_eq!(back.room, "shore");
//! ```
//!
//! These modules store raw ids. For saves that must survive content
//! changes, use [`SavedWorldState`](crate::save::SavedWorldState) and
//! [`SavedDialogCursor`](crate::save::SavedDialogCursor).

/// Defines a `with` module for a type with a host outcome parameter `H`
/// (`$model<H>`, serialized through `$dto<H>`).
macro_rules! with_module_host {
    ($(#[$doc:meta])* $name:ident, $($model:ident)::+, $($dto:ident)::+) => {
        $(#[$doc])*
        pub mod $name {
            use serde::{Deserialize, Deserializer, Serialize, Serializer};

            use crate::dto::IntoModel;

            /// Serializes the value through its DTO. Clones host outcomes.
            pub fn serialize<S, H>(value: &$($model)::+<H>, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
                H: Clone + Serialize,
            {
                $($dto)::+::<H>::from(value).serialize(serializer)
            }

            /// Deserializes the value through its DTO.
            pub fn deserialize<'de, D, H>(deserializer: D) -> Result<$($model)::+<H>, D::Error>
            where
                D: Deserializer<'de>,
                H: Deserialize<'de>,
            {
                $($dto)::+::<H>::deserialize(deserializer).map(IntoModel::into_model)
            }
        }
    };
}

/// Defines a `with` module that serializes `$model` through `$dto`.
macro_rules! with_module {
    ($(#[$doc:meta])* $name:ident, $model:ty, $dto:ty) => {
        $(#[$doc])*
        pub mod $name {
            use serde::{Deserialize, Deserializer, Serialize, Serializer};

            use crate::dto::IntoModel;

            /// Serializes the value through its DTO.
            pub fn serialize<S: Serializer>(value: &$model, serializer: S) -> Result<S::Ok, S::Error> {
                <$dto>::from(value).serialize(serializer)
            }

            /// Deserializes the value through its DTO.
            pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<$model, D::Error> {
                <$dto>::deserialize(deserializer).map(IntoModel::into_model)
            }
        }
    };
}

with_module!(
    /// `with` module for [`DialogWorldState`](dramaturge_runtime::DialogWorldState).
    world_state,
    dramaturge_runtime::DialogWorldState,
    crate::dto::WorldStateDto
);
with_module!(
    /// `with` module for [`Condition`](dramaturge_model::Condition).
    condition,
    dramaturge_model::Condition,
    crate::dto::ConditionDto
);
with_module!(
    /// `with` module for [`SymbolTable`](dramaturge_model::SymbolTable).
    symbol_table,
    dramaturge_model::SymbolTable,
    crate::dto::SymbolTableDto
);
with_module!(
    /// `with` module for [`TextProgress`](dramaturge_runtime::TextProgress).
    text_progress,
    dramaturge_runtime::TextProgress,
    crate::dto::TextProgressDto
);
with_module!(
    /// `with` module for [`DialogCursor`](dramaturge_runtime::DialogCursor).
    ///
    /// Stores raw node ids. For a save that must survive content changes,
    /// use [`SavedDialogCursor`](crate::save::SavedDialogCursor).
    dialog_cursor,
    dramaturge_runtime::DialogCursor,
    crate::dto::DialogCursorDto
);
with_module_host!(
    /// `with` module for [`Outcome<H>`](dramaturge_model::Outcome), for any
    /// host outcome type `H` that is `Clone` and serializable.
    outcome,
    dramaturge_model::Outcome,
    crate::dto::OutcomeDto
);
with_module_host!(
    /// `with` module for [`DialogueGraph<H>`](dramaturge_model::DialogueGraph),
    /// for any host outcome type `H` that is `Clone` and serializable.
    dialogue_graph,
    dramaturge_model::DialogueGraph,
    crate::dto::DialogueGraphDto
);
