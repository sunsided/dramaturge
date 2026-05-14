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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CustomDialogTag {
    /// Custom axis identifier.
    pub axis: Arc<str>,
    /// Custom key within the axis.
    pub key: Arc<str>,
    /// Optional display prefix for rendering (e.g., "[Medic]").
    #[cfg_attr(feature = "serde", serde(default))]
    pub display_prefix: Option<Arc<str>>,
    /// Whether this tag represents a required player trait/skill.
    /// When true, the option is only available if the player has this trait.
    #[cfg_attr(feature = "serde", serde(default))]
    pub requires_trait: bool,
}

impl CustomDialogTag {
    /// Creates a new custom dialog tag with the given axis and key.
    pub fn new(axis: impl Into<Arc<str>>, key: impl Into<Arc<str>>) -> Self {
        Self {
            axis: axis.into(),
            key: key.into(),
            display_prefix: None,
            requires_trait: false,
        }
    }

    /// Creates a new skill tag with the given key.
    /// Automatically sets `requires_trait: true` and generates a display prefix from the key.
    pub fn skill(key: impl Into<Arc<str>>) -> Self {
        let key = key.into();
        let display_prefix = format!("[{key}]");
        Self {
            axis: Arc::from("skill"),
            key: key.clone(),
            display_prefix: Some(Arc::from(display_prefix)),
            requires_trait: true,
        }
    }

    /// Creates a new tag with display prefix.
    pub fn with_prefix(
        axis: impl Into<Arc<str>>,
        key: impl Into<Arc<str>>,
        prefix: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            axis: axis.into(),
            key: key.into(),
            display_prefix: Some(prefix.into()),
            requires_trait: false,
        }
    }

    /// Sets whether this tag requires the player to have the trait.
    pub fn with_requires_trait(mut self, requires: bool) -> Self {
        self.requires_trait = requires;
        self
    }

    /// Returns the display prefix for this tag, if any.
    pub fn display_prefix(&self) -> Option<&str> {
        self.display_prefix.as_deref()
    }

    /// Returns true if this tag represents a skill requirement.
    pub fn is_skill_requirement(&self) -> bool {
        self.requires_trait && self.axis.as_ref() == "skill"
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
        check!(tag.display_prefix.is_none());
        check!(!tag.requires_trait);
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

    #[test]
    fn skill_tag_auto_configures() {
        let tag = CustomDialogTag::skill("medic");
        check!(&*tag.axis == "skill");
        check!(&*tag.key == "medic");
        check!(tag.display_prefix.as_deref() == Some("[medic]"));
        check!(tag.requires_trait);
        check!(tag.is_skill_requirement());
    }

    #[test]
    fn tag_with_prefix() {
        let tag = CustomDialogTag::with_prefix("moral", "paragon", "[Paragon]");
        check!(&*tag.axis == "moral");
        check!(&*tag.key == "paragon");
        check!(tag.display_prefix.as_deref() == Some("[Paragon]"));
        check!(!tag.requires_trait);
    }

    #[test]
    fn tag_with_requires_trait() {
        let tag = CustomDialogTag::new("skill", "investigator").with_requires_trait(true);
        check!(tag.requires_trait);
        check!(tag.is_skill_requirement());
    }

    #[test]
    fn non_skill_tag_with_requires_trait() {
        let tag = CustomDialogTag::new("moral", "paragon").with_requires_trait(true);
        check!(tag.requires_trait);
        check!(!tag.is_skill_requirement()); // Not a skill axis
    }

    #[test]
    fn display_prefix_method() {
        let tag = CustomDialogTag::with_prefix("skill", "medic", "[Medic]");
        check!(tag.display_prefix() == Some("[Medic]"));

        let tag_no_prefix = CustomDialogTag::new("moral", "paragon");
        check!(tag_no_prefix.display_prefix().is_none());
    }
}
