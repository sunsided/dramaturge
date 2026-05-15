//! Optional serialization integration for the `dramaturge` toolkit.
//!
//! # Features
//!
//! - `serde` — activates [`serde`](https://docs.rs/serde) support across the
//!   model types. Enable with `dramaturge-serde = { features = ["serde"] }`.
//! - `yaml` — enables the YAML dialogue loader (implies `serde`). Requires the
//!   `yaml` feature: `dramaturge-serde = { features = ["yaml"] }`.
//!
//! # YAML Loader
//!
//! With the `yaml` feature, load a [`DialogueGraph`] from a YAML file:
//!
//! ```ignore
//! use dramaturge_serde::loader::YamlDialogueGraph;
//!
//! let raw = std::fs::read_to_string("dialogue.yaml").unwrap();
//! let graph = YamlDialogueGraph::from_yaml(&raw)
//!     .expect("invalid YAML")
//!     .to_model()
//!     .expect("invalid graph structure");
//! ```
//!
//! See `crates/dramaturge/examples/dialogue.yaml` for the full YAML format.
//!
//! # Why a Separate Crate?
//!
//! `dramaturge-model` intentionally has no serde dependency so that game
//! engines that serialize state through other means (binary formats, custom
//! reflection, etc.) pay zero cost. Add this crate only when you need it.

#[cfg(feature = "serde")]
pub use serde;

#[cfg(feature = "yaml")]
pub mod loader;

use dramaturge_model as model;

/// Re-export the model crate for downstream convenience.
pub use model::*;
