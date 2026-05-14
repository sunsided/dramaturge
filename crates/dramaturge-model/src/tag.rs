//! Typed dialog tags for annotating dialogue options with metadata.
//!
//! Tags provide categorization for dialogue options, enabling the runtime
//! or authoring tools to filter, score, or react to options based on their
//! semantic properties. The [`DialogTag`] trait defines the contract for
//! any tag type: it must provide `axis()` and `key()` methods.
//!
//! The primary tag type is [`CustomDialogTag`], which supports arbitrary
//! axis/key combinations via the [`DialogTag`] trait. Users can also
//! implement the [`DialogTag`] trait on their own domain-specific enums.
//! [`DialogTagKind`] is a type alias for [`CustomDialogTag`].

use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

/// Trait for typed dialog tags.
///
/// Requires `axis()` and `key()` methods for categorization. The axis
/// identifies the tag category (e.g., `"moral"`, `"skill"`), while the
/// key identifies the specific value within that category (e.g.,
/// `"paragon"`, `"medic"`).
pub trait DialogTag: Debug + Clone + PartialEq + Eq + Hash {
    fn axis(&self) -> &str;
    fn key(&self) -> &str;
}

impl DialogTag for CustomDialogTag {
    fn axis(&self) -> &str {
        &self.axis
    }

    fn key(&self) -> &str {
        &self.key
    }
}

/// User-defined tag with arbitrary axis and key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomDialogTag {
    /// Custom axis identifier.
    pub axis: Arc<str>,
    /// Custom key within the axis.
    pub key: Arc<str>,
}

impl CustomDialogTag {
    /// Creates a new custom dialog tag with the given axis and key.
    pub fn new(axis: impl Into<Arc<str>>, key: impl Into<Arc<str>>) -> Self {
        Self {
            axis: axis.into(),
            key: key.into(),
        }
    }
}

/// Primary dialog tag type — an alias for [`CustomDialogTag`].
pub type DialogTagKind = CustomDialogTag;

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn custom_dialog_tag_construction() {
        let tag = CustomDialogTag::new("faction", "rebel");
        check!(&*tag.axis == "faction");
        check!(&*tag.key == "rebel");
    }

    #[test]
    fn custom_dialog_tag_equality() {
        let a = CustomDialogTag::new("moral", "paragon");
        let b = CustomDialogTag::new("moral", "paragon");
        let c = CustomDialogTag::new("moral", "renegade");
        check!(a == b);
        check!(a != c);
    }

    #[test]
    fn dialog_tag_kind_is_custom() {
        let tag: DialogTagKind = DialogTagKind::new("axis", "key");
        check!(&*tag.axis == "axis");
        check!(&*tag.key == "key");
    }
}
