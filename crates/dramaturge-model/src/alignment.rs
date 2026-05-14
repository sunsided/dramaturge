//! Alignment axes and scoring for moral or relational positioning.
//!
//! This module defines types for tracking alignment — the positioning of
//! characters along moral, reputational, or other axes. An [`AlignmentAxis`]
//! defines the available poles (e.g., Paragon/Renegade), while an
//! [`AlignmentScore`] tracks a character's current position on that axis.

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
}
