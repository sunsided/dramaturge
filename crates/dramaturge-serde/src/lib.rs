//! Optional serialization integration for the `dramaturge` toolkit.
//!
//! Enable the `serde` feature to pull in [`serde`](https://docs.rs/serde)
//! support. The core domain model crate (`dramaturge-model`) intentionally
//! avoids a hard serde dependency; this crate is the designated integration
//! point.
//!
//! # Features
//!
//! - `serde` — activates [`serde`](https://docs.rs/serde) as a dependency.
//! - `serde` — enables YAML loader support.

#[cfg(feature = "serde")]
pub use serde;

#[cfg(feature = "serde")]
pub mod loader;

use dramaturge_model as model;

/// Re-export the model crate for downstream convenience.
pub use model::*;
