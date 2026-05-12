use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

pub trait DialogTag: Debug + Clone + PartialEq + Eq + Hash {
    fn axis(&self) -> &'static str;
    fn key(&self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MoralTag {
    Paragon,
    Renegade,
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SkillTag {
    Medic,
    Thief,
    Engineer,
    Soldier,
    Hacker,
    Diplomat,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpeechActTag {
    Ask,
    Lie,
    Threaten,
    Flirt,
    Joke,
    Comfort,
    Accuse,
    Bargain,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomDialogTag {
    pub axis: Arc<str>,
    pub key: Arc<str>,
}

impl CustomDialogTag {
    pub fn new(axis: impl Into<Arc<str>>, key: impl Into<Arc<str>>) -> Self {
        Self {
            axis: axis.into(),
            key: key.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DialogTagKind {
    Moral(MoralTag),
    Skill(SkillTag),
    SpeechAct(SpeechActTag),
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
