//! Typed dialog tags for annotating dialogue options with metadata.
//!
//! Tags provide categorization for dialogue options, enabling the runtime
//! or authoring tools to filter, score, or react to options based on their
//! semantic properties. The [`DialogTag`] trait defines the contract for
//! any tag type: it must provide `axis()` and `key()` methods.
//!
//! Predefined tag types include [`MoralTag`] (alignment), [`SkillTag`]
//! (skill checks), and [`SpeechActTag`] (communication style). For
//! arbitrary tagging, use [`CustomDialogTag`].

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
    /// Returns the tag's axis label (e.g., `"moral"`, `"skill"`).
    fn axis(&self) -> &'static str;
    /// Returns the tag's specific key (e.g., `"paragon"`, `"medic"`).
    fn key(&self) -> &'static str;
}

/// Predefined moral alignment tags.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MoralTag {
    /// Heroic, selfless behavior.
    Paragon,
    /// Ruthless, self-serving behavior.
    Renegade,
    /// Neutral behavior with no strong moral leaning.
    Neutral,
}

/// Predefined skill-check tags.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SkillTag {
    /// Medical knowledge or first aid.
    Medic,
    /// Stealth, lockpicking, or theft.
    Thief,
    /// Mechanical or technical expertise.
    Engineer,
    /// Combat or military training.
    Soldier,
    /// Cybersecurity or digital intrusion.
    Hacker,
    /// Diplomacy, negotiation, or persuasion.
    Diplomat,
}

/// Predefined speech act classification tags.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpeechActTag {
    /// Asking a question or requesting information.
    Ask,
    /// Deliberately providing false information.
    Lie,
    /// Using intimidation or threats.
    Threaten,
    /// Romantic or flirtatious approach.
    Flirt,
    /// Making a joke or lightening the mood.
    Joke,
    /// Providing comfort or emotional support.
    Comfort,
    /// Making an accusation.
    Accuse,
    /// Attempting to negotiate or trade.
    Bargain,
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

/// Sum type wrapping all predefined and custom tag types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DialogTagKind {
    /// Moral alignment tag (Paragon, Renegade, Neutral).
    Moral(MoralTag),
    /// Skill-check tag (Medic, Thief, etc.).
    Skill(SkillTag),
    /// Speech act tag (Ask, Lie, Threaten, etc.).
    SpeechAct(SpeechActTag),
    /// User-defined custom tag.
    Custom(CustomDialogTag),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moral_tag_variants() {
        assert_eq!(MoralTag::Paragon, MoralTag::Paragon);
        assert_ne!(MoralTag::Paragon, MoralTag::Renegade);
    }

    #[test]
    fn skill_tag_variants() {
        assert_eq!(SkillTag::Medic, SkillTag::Medic);
        assert_ne!(SkillTag::Medic, SkillTag::Hacker);
    }

    #[test]
    fn speech_act_variants() {
        assert_eq!(SpeechActTag::Ask, SpeechActTag::Ask);
        assert_ne!(SpeechActTag::Ask, SpeechActTag::Lie);
    }

    #[test]
    fn custom_dialog_tag_construction() {
        let tag = CustomDialogTag::new("faction", "rebel");
        assert_eq!(&*tag.axis, "faction");
        assert_eq!(&*tag.key, "rebel");
    }

    #[test]
    fn dialog_tag_kind_variants() {
        let moral = DialogTagKind::Moral(MoralTag::Paragon);
        let skill = DialogTagKind::Skill(SkillTag::Medic);
        let speech = DialogTagKind::SpeechAct(SpeechActTag::Ask);
        let custom = DialogTagKind::Custom(CustomDialogTag::new("x", "y"));

        assert!(matches!(moral, DialogTagKind::Moral(_)));
        assert!(matches!(skill, DialogTagKind::Skill(_)));
        assert!(matches!(speech, DialogTagKind::SpeechAct(_)));
        assert!(matches!(custom, DialogTagKind::Custom(_)));
    }
}
