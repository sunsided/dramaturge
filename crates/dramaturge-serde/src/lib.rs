//! Optional serialization integration for the `dramaturge` toolkit.
//!
//! # Features
//!
//! - `yaml` — enables the YAML dialogue loader. Requires the `yaml` feature:
//!   `dramaturge-serde = { features = ["yaml"] }`.
//!
//! # YAML Loader
//!
//! With the `yaml` feature, load a [`DialogueGraph`] from a YAML file:
//!
//! ```ignore
//! use dramaturge_serde::loader::YamlDialogueGraph;
//!
//! let raw = std::fs::read_to_string("dialogue.yaml").unwrap();
//! let loaded = YamlDialogueGraph::from_yaml(&raw)
//!     .expect("invalid YAML")
//!     .to_model()
//!     .expect("invalid graph structure");
//! let graph = loaded.graph;
//! ```
//!
//! See `crates/dramaturge/examples/dialogue.yaml` for the full YAML format.
//!
//! # Model and State Serialization
//!
//! [`dto`] has a lossless serde DTO for every public model type and for
//! `DialogWorldState`. Host structs use the `with` modules
//! ([`world_state`], [`condition`], [`outcome`], [`dialogue_graph`],
//! [`symbol_table`], [`text_progress`]). [`save::SavedWorldState`] stores keys next to the
//! state so saves survive content changes.
//!
//! # Why a Separate Crate?
//!
//! `dramaturge-model` intentionally has no serde dependency so that game
//! engines that serialize state through other means (binary formats, custom
//! reflection, etc.) pay zero cost. Add this crate only when you need it.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub use serde;

pub mod dto;
pub mod save;
mod with;
pub use with::{condition, dialogue_graph, outcome, symbol_table, text_progress, world_state};

#[cfg(feature = "yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml")))]
pub mod loader;
use dramaturge_model as model;

/// Re-export the model crate for downstream convenience.
pub use model::*;
