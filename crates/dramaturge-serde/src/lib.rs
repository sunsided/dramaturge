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

#[cfg(feature = "serde")]
pub use serde;

use dramaturge_model as model;

/// Re-export the model crate for downstream convenience.
pub use model::*;

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {
        // Verify that the crate compiles and the workspace is wired correctly.
    }
}
