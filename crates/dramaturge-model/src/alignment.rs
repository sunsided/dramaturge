//! Alignment axes and scoring for moral or relational positioning.
//!
//! This module defines types for tracking alignment — the positioning of
//! characters along moral, reputational, or other axes. An [`AlignmentAxis`]
//! defines the available poles (e.g., Paragon/Renegade), while an
//! [`AlignmentScore`] tracks a character's current position on that axis.
//!
//! # Multi-Game Examples
//!
//! **Mass Effect** — moral alignment with Paragon and Renegade poles:
//!
//! ```rust
//! use dramaturge_model::{AlignmentAxis, AlignmentPole, AlignmentAxisId, AlignmentPoleId, Condition};
//!
//! let mut morality = AlignmentAxis::new("morality", "Moral Alignment");
//! morality.add_pole(AlignmentPole::new("paragon", "Paragon"));
//! morality.add_pole(AlignmentPole::new("renegade", "Renegade"));
//!
//! // Check if the player has enough Paragon score to unlock the option
//! let condition = Condition::AlignmentAtLeast {
//!     axis: AlignmentAxisId::new("morality"),
//!     pole: AlignmentPoleId::new("paragon"),
//!     value: 60,
//! };
//! ```
//!
//! **Generic RPG** — faction reputation axis:
//!
//! ```rust
//! use dramaturge_model::{AlignmentAxis, AlignmentPole};
//!
//! let mut rep = AlignmentAxis::new("reputation", "Guild Reputation");
//! rep.add_pole(AlignmentPole::new("honored", "Honored"));
//! rep.add_pole(AlignmentPole::new("reviled", "Reviled"));
//! ```

use std::sync::Arc;

/// Identifier for an alignment axis (e.g., `"moral"`, `"reputation"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlignmentAxisId(pub Arc<str>);

impl AlignmentAxisId {
    /// Creates a new alignment axis identifier from the given string-like value.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// Definition of an alignment axis with its available poles.
///
/// An axis represents a dimension along which alignment is measured,
/// such as morality or faction reputation. Poles are added separately
/// and start empty on construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentAxis {
    /// The unique identifier for this axis.
    pub id: AlignmentAxisId,
    /// Human-facing axis name (e.g., "Moral Alignment").
    pub display_name: Arc<str>,
    /// Available poles on this axis (e.g., Paragon, Renegade).
    /// Starts empty when created via [`AlignmentAxis::new`].
    pub poles: Vec<AlignmentPole>,
}

impl AlignmentAxis {
    /// Creates a new alignment axis with the given ID and display name.
    ///
    /// The poles vector starts empty.
    pub fn new(id: impl Into<Arc<str>>, display_name: impl Into<Arc<str>>) -> Self {
        Self {
            id: AlignmentAxisId::new(id),
            display_name: display_name.into(),
            poles: Vec::new(),
        }
    }

    /// Adds a pole to this axis.
    pub fn add_pole(&mut self, pole: AlignmentPole) {
        self.poles.push(pole);
    }

    /// Finds a pole by its ID.
    pub fn find_pole(&self, id: &AlignmentPoleId) -> Option<&AlignmentPole> {
        self.poles.iter().find(|pole| pole.id == *id)
    }
}

/// Identifier for a specific pole on an alignment axis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlignmentPoleId(pub Arc<str>);

impl AlignmentPoleId {
    /// Creates a new alignment pole identifier from the given string-like value.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

/// A named endpoint on an alignment axis.
///
/// Poles represent the named positions along an axis. For example, on
/// a "moral" axis, poles might be "Paragon" and "Renegade".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentPole {
    /// Unique identifier for this pole.
    pub id: AlignmentPoleId,
    /// Human-facing pole name.
    pub display_name: Arc<str>,
}

impl AlignmentPole {
    /// Creates a new alignment pole with the given ID and display name.
    pub fn new(id: impl Into<Arc<str>>, display_name: impl Into<Arc<str>>) -> Self {
        Self {
            id: AlignmentPoleId::new(id),
            display_name: display_name.into(),
        }
    }
}

/// A character's current score on a specific axis/pole combination.
///
/// Positive values favor the pole, negative values oppose it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentScore {
    /// Which axis this score belongs to.
    pub axis: AlignmentAxisId,
    /// Which pole on the axis is being scored.
    pub pole: AlignmentPoleId,
    /// Numeric score; positive favors the pole, negative opposes it.
    pub value: i32,
}

impl AlignmentScore {
    /// Creates a new alignment score.
    pub fn new(axis: impl Into<Arc<str>>, pole: impl Into<Arc<str>>, value: i32) -> Self {
        Self {
            axis: AlignmentAxisId::new(axis),
            pole: AlignmentPoleId::new(pole),
            value,
        }
    }

    /// Creates a positive alignment score (absolute value is taken).
    pub fn positive(axis: impl Into<Arc<str>>, pole: impl Into<Arc<str>>, value: i32) -> Self {
        Self::new(axis, pole, value.saturating_abs())
    }

    /// Creates a negative alignment score (absolute value is taken and negated).
    pub fn negative(axis: impl Into<Arc<str>>, pole: impl Into<Arc<str>>, value: i32) -> Self {
        Self::new(axis, pole, -(value.saturating_abs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn alignment_axis_construction() {
        let axis = AlignmentAxis::new("moral", "Moral Alignment");
        check!(&*axis.id.0 == "moral");
        check!(&*axis.display_name == "Moral Alignment");
        check!(axis.poles.is_empty());
    }

    #[test]
    fn alignment_pole_construction() {
        let pole = AlignmentPole::new("paragon", "Paragon");
        check!(&*pole.id.0 == "paragon");
        check!(&*pole.display_name == "Paragon");
    }

    #[test]
    fn alignment_score_construction() {
        let score = AlignmentScore {
            axis: AlignmentAxisId::new("moral"),
            pole: AlignmentPoleId::new("paragon"),
            value: 50,
        };
        check!(score.value == 50);
    }

    #[test]
    fn alignment_axis_add_pole() {
        let mut axis = AlignmentAxis::new("moral", "Moral Alignment");
        check!(axis.poles.is_empty());
        axis.add_pole(AlignmentPole::new("paragon", "Paragon"));
        axis.add_pole(AlignmentPole::new("renegade", "Renegade"));
        check!(axis.poles.len() == 2);
        check!(&*axis.poles[0].id.0 == "paragon");
        check!(&*axis.poles[1].id.0 == "renegade");
    }

    #[test]
    fn alignment_axis_find_pole() {
        let mut axis = AlignmentAxis::new("moral", "Moral Alignment");
        axis.add_pole(AlignmentPole::new("paragon", "Paragon"));
        axis.add_pole(AlignmentPole::new("renegade", "Renegade"));
        let found = axis.find_pole(&AlignmentPoleId::new("paragon"));
        check!(found.is_some());
        check!(&*found.unwrap().display_name == "Paragon");
    }

    #[test]
    fn alignment_axis_find_pole_not_found() {
        let axis = AlignmentAxis::new("moral", "Moral Alignment");
        let found = axis.find_pole(&AlignmentPoleId::new("nonexistent"));
        check!(found.is_none());
    }

    #[test]
    fn alignment_score_new() {
        let score = AlignmentScore::new("moral", "paragon", 50);
        check!(&*score.axis.0 == "moral");
        check!(&*score.pole.0 == "paragon");
        check!(score.value == 50);
    }

    #[test]
    fn alignment_score_positive() {
        let score = AlignmentScore::positive("moral", "paragon", 50);
        check!(score.value == 50);
        let score = AlignmentScore::positive("moral", "paragon", -50);
        check!(score.value == 50);
    }

    #[test]
    fn alignment_score_negative() {
        let score = AlignmentScore::negative("moral", "paragon", 50);
        check!(score.value == -50);
        let score = AlignmentScore::negative("moral", "paragon", -50);
        check!(score.value == -50);
    }

    #[test]
    fn alignment_score_positive_i32_min() {
        let score = AlignmentScore::positive("moral", "paragon", i32::MIN);
        check!(score.value == i32::MAX);
    }

    #[test]
    fn alignment_score_negative_i32_min() {
        let score = AlignmentScore::negative("moral", "paragon", i32::MIN);
        check!(score.value == i32::MIN + 1);
    }
}
