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

/// `with` module for [`DialogText<T>`](dramaturge_model::DialogText), for
/// any host text type `T` that is `Clone` and serializable.
pub mod dialog_text {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::dto::{DialogTextDto, IntoModel};
    use dramaturge_model::DialogText;

    /// Serializes the text through its DTO. Clones host text.
    pub fn serialize<S, T>(value: &DialogText<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Clone + Serialize,
    {
        DialogTextDto::<T>::from(value).serialize(serializer)
    }

    /// Deserializes the text through its DTO.
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<DialogText<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        DialogTextDto::<T>::deserialize(deserializer).map(IntoModel::into_model)
    }
}

/// `with` module for [`DialogueGraph<H, T>`](dramaturge_model::DialogueGraph),
/// for any host outcome type `H` and host text type `T` that are `Clone`
/// and serializable.
pub mod dialogue_graph {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::dto::{DialogueGraphDto, IntoModel};
    use dramaturge_model::DialogueGraph;

    /// Serializes the graph through its DTO. Clones host outcomes and host
    /// text.
    pub fn serialize<S, H, T>(value: &DialogueGraph<H, T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        H: Clone + Serialize,
        T: Clone + Serialize,
    {
        DialogueGraphDto::<H, T>::from(value).serialize(serializer)
    }

    /// Deserializes the graph through its DTO.
    pub fn deserialize<'de, D, H, T>(deserializer: D) -> Result<DialogueGraph<H, T>, D::Error>
    where
        D: Deserializer<'de>,
        H: Deserialize<'de>,
        T: Deserialize<'de>,
    {
        DialogueGraphDto::<H, T>::deserialize(deserializer).map(IntoModel::into_model)
    }
}
