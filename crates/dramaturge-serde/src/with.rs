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
//! changes, use [`SavedWorldState`](crate::save::SavedWorldState).

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
    /// `with` module for [`Outcome`](dramaturge_model::Outcome).
    outcome,
    dramaturge_model::Outcome,
    crate::dto::OutcomeDto
);
with_module!(
    /// `with` module for [`DialogueGraph`](dramaturge_model::DialogueGraph).
    dialogue_graph,
    dramaturge_model::DialogueGraph,
    crate::dto::DialogueGraphDto
);
with_module!(
    /// `with` module for [`SymbolTable`](dramaturge_model::SymbolTable).
    symbol_table,
    dramaturge_model::SymbolTable,
    crate::dto::SymbolTableDto
);
