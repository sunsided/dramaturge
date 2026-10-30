//! Lossless serde DTOs for every public data type of `dramaturge-model`,
//! and for [`DialogWorldState`].
//!
//! `dramaturge-model` has no serde dependency (see ADR 0002). Each model
//! type `T` has a mirror `TDto` here with `Serialize`/`Deserialize`, a
//! `From<&T>` conversion into the DTO, and an [`IntoModel`] conversion back.
//! Conversions destructure the model types exhaustively, so adding a field
//! to a model type is a compile error here until the DTO follows.
//!
//! # Format
//!
//! - Enums are externally tagged with `snake_case` names:
//!   `{"fact_exists": {"scope": "world", "fact_id": "met_hero"}}`.
//! - Ids are their raw numbers. To store keys instead, wrap state in a
//!   [`SavedWorldState`](crate::save::SavedWorldState).
//! - `FactValue::Float` is a JSON number. Non-finite floats are not
//!   representable in JSON. With `serde_json`, enable its `float_roundtrip`
//!   feature for bit-exact floats; the default parser may be off by one ulp.
//!
//! # Id mapping
//!
//! [`IntoModel::into_model_with`] takes an [`IdMap`] that turns raw ids
//! back into model ids. [`Identity`] keeps them as they are;
//! [`KeyRemap`](crate::save::KeyRemap) re-resolves them by key against the
//! current content.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use dramaturge_model as model;
use dramaturge_model::{DialogueNodeId, DialogueOptionId, EntityId, MemoryId};
use dramaturge_runtime::DialogWorldState;

/// Maps raw ids from a DTO back to model ids.
pub trait IdMap {
    /// Maps a raw entity id.
    fn entity(&mut self, raw: u64) -> EntityId;
    /// Maps a raw dialogue node id.
    fn node(&mut self, raw: u64) -> DialogueNodeId;
    /// Maps a raw dialogue option id.
    fn option(&mut self, raw: u64) -> DialogueOptionId;
    /// Maps a raw memory id.
    fn memory(&mut self, raw: u64) -> MemoryId;
}

/// The [`IdMap`] that keeps every id unchanged.
#[derive(Debug, Clone, Copy, Default)]
pub struct Identity;

impl IdMap for Identity {
    fn entity(&mut self, raw: u64) -> EntityId {
        EntityId::new(raw)
    }
    fn node(&mut self, raw: u64) -> DialogueNodeId {
        DialogueNodeId::new(raw)
    }
    fn option(&mut self, raw: u64) -> DialogueOptionId {
        DialogueOptionId::new(raw)
    }
    fn memory(&mut self, raw: u64) -> MemoryId {
        MemoryId::new(raw)
    }
}

/// Conversion from a DTO back into its model type.
pub trait IntoModel {
    /// The model type.
    type Model;

    /// Converts into the model, mapping ids through `ids`.
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> Self::Model;

    /// Converts into the model, keeping ids unchanged.
    fn into_model(self) -> Self::Model
    where
        Self: Sized,
    {
        self.into_model_with(&mut Identity)
    }
}

impl<T: IntoModel> IntoModel for Vec<T> {
    type Model = Vec<T::Model>;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> Self::Model {
        self.into_iter().map(|d| d.into_model_with(ids)).collect()
    }
}

impl<T: IntoModel> IntoModel for Option<T> {
    type Model = Option<T::Model>;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> Self::Model {
        self.map(|d| d.into_model_with(ids))
    }
}

impl<T: IntoModel> IntoModel for Box<T> {
    type Model = Box<T::Model>;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> Self::Model {
        Box::new((*self).into_model_with(ids))
    }
}

fn dtos<'a, T: 'a, D: From<&'a T>>(items: impl IntoIterator<Item = &'a T>) -> Vec<D> {
    items.into_iter().map(D::from).collect()
}

fn arc(s: String) -> Arc<str> {
    Arc::from(s)
}

/// Implements `IntoModel` for DTOs without ids (plain field-by-field copies).
macro_rules! no_ids {
    ($dto:ty => $model:ty, |$d:ident| $body:expr) => {
        impl IntoModel for $dto {
            type Model = $model;
            fn into_model_with<M: IdMap + ?Sized>(self, _ids: &mut M) -> $model {
                let $d = self;
                $body
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Characters
// ---------------------------------------------------------------------------

/// DTO for [`model::CharacterTrait`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterTraitDto {
    /// Trait category.
    pub namespace: String,
    /// Trait value within the namespace.
    pub key: String,
}

impl From<&model::CharacterTrait> for CharacterTraitDto {
    fn from(t: &model::CharacterTrait) -> Self {
        let model::CharacterTrait { namespace, key } = t;
        Self {
            namespace: namespace.to_string(),
            key: key.to_string(),
        }
    }
}

no_ids!(CharacterTraitDto => model::CharacterTrait, |d| model::CharacterTrait::new(d.namespace, d.key));

/// DTO for [`model::Character`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterDto {
    /// Entity id.
    pub id: u64,
    /// Stable authoring key.
    pub key: String,
    /// Display name.
    pub display_name: String,
    /// Traits.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<CharacterTraitDto>,
}

impl From<&model::Character> for CharacterDto {
    fn from(c: &model::Character) -> Self {
        let model::Character {
            id,
            key,
            display_name,
            traits,
        } = c;
        Self {
            id: id.0.0,
            key: key.0.to_string(),
            display_name: display_name.to_string(),
            traits: dtos(traits),
        }
    }
}

impl IntoModel for CharacterDto {
    type Model = model::Character;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::Character {
        model::Character {
            id: model::CharacterId::new(ids.entity(self.id)),
            key: model::CharacterKey::new(self.key),
            display_name: arc(self.display_name),
            traits: self.traits.into_model_with(ids),
        }
    }
}

// ---------------------------------------------------------------------------
// Facts
// ---------------------------------------------------------------------------

/// DTO for [`model::FactValue`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactValueDto {
    /// Boolean flag.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// Text.
    Text(String),
    /// Entity reference (raw id).
    Entity(u64),
    /// Nested values.
    Set(Vec<FactValueDto>),
}

impl From<&model::FactValue> for FactValueDto {
    fn from(v: &model::FactValue) -> Self {
        use model::FactValue as V;
        match v {
            V::Bool(b) => Self::Bool(*b),
            V::Int(i) => Self::Int(*i),
            V::Float(f) => Self::Float(*f),
            V::Text(t) => Self::Text(t.clone()),
            V::Entity(e) => Self::Entity(e.0),
            V::Set(vs) => Self::Set(dtos(vs)),
        }
    }
}

impl IntoModel for FactValueDto {
    type Model = model::FactValue;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::FactValue {
        use model::FactValue as V;
        match self {
            Self::Bool(b) => V::Bool(b),
            Self::Int(i) => V::Int(i),
            Self::Float(f) => V::Float(f),
            Self::Text(t) => V::Text(t),
            Self::Entity(e) => V::Entity(ids.entity(e)),
            Self::Set(vs) => V::Set(vs.into_model_with(ids)),
        }
    }
}

/// DTO for [`model::FactScope`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactScopeDto {
    /// Objective truth.
    World,
    /// The player.
    Player,
    /// One character (raw entity id).
    Character(u64),
    /// A faction by key.
    Faction(String),
    /// The party.
    Party,
}

impl From<&model::FactScope> for FactScopeDto {
    fn from(s: &model::FactScope) -> Self {
        use model::FactScope as S;
        match s {
            S::World => Self::World,
            S::Player => Self::Player,
            S::Character(c) => Self::Character(c.0.0),
            S::Faction(f) => Self::Faction(f.to_string()),
            S::Party => Self::Party,
        }
    }
}

impl IntoModel for FactScopeDto {
    type Model = model::FactScope;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::FactScope {
        use model::FactScope as S;
        match self {
            Self::World => S::World,
            Self::Player => S::Player,
            Self::Character(c) => S::Character(model::CharacterId::new(ids.entity(c))),
            Self::Faction(f) => S::Faction(arc(f)),
            Self::Party => S::Party,
        }
    }
}

/// DTO for [`model::Fact`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactDto {
    /// Fact id.
    pub id: String,
    /// Value.
    pub value: FactValueDto,
}

impl From<&model::Fact> for FactDto {
    fn from(f: &model::Fact) -> Self {
        let model::Fact { id, value } = f;
        Self {
            id: id.0.to_string(),
            value: value.into(),
        }
    }
}

impl IntoModel for FactDto {
    type Model = model::Fact;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::Fact {
        model::Fact {
            id: model::FactId::new(self.id),
            value: self.value.into_model_with(ids),
        }
    }
}

/// DTO for [`model::ScopedFact`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopedFactDto {
    /// Scope.
    pub scope: FactScopeDto,
    /// Fact id.
    pub id: String,
    /// Value.
    pub value: FactValueDto,
}

impl From<&model::ScopedFact> for ScopedFactDto {
    fn from(sf: &model::ScopedFact) -> Self {
        let model::ScopedFact { scope, fact } = sf;
        Self {
            scope: scope.into(),
            id: fact.id.0.to_string(),
            value: (&fact.value).into(),
        }
    }
}

impl IntoModel for ScopedFactDto {
    type Model = model::ScopedFact;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::ScopedFact {
        model::ScopedFact {
            scope: self.scope.into_model_with(ids),
            fact: model::Fact {
                id: model::FactId::new(self.id),
                value: self.value.into_model_with(ids),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Alignment and relationships
// ---------------------------------------------------------------------------

/// DTO for [`model::AlignmentPole`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlignmentPoleDto {
    /// Pole id.
    pub id: String,
    /// Display name.
    pub display_name: String,
}

impl From<&model::AlignmentPole> for AlignmentPoleDto {
    fn from(p: &model::AlignmentPole) -> Self {
        let model::AlignmentPole { id, display_name } = p;
        Self {
            id: id.0.to_string(),
            display_name: display_name.to_string(),
        }
    }
}

no_ids!(AlignmentPoleDto => model::AlignmentPole, |d| model::AlignmentPole::new(d.id, d.display_name));

/// DTO for [`model::AlignmentAxis`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlignmentAxisDto {
    /// Axis id.
    pub id: String,
    /// Display name.
    pub display_name: String,
    /// Poles.
    pub poles: Vec<AlignmentPoleDto>,
}

impl From<&model::AlignmentAxis> for AlignmentAxisDto {
    fn from(a: &model::AlignmentAxis) -> Self {
        let model::AlignmentAxis {
            id,
            display_name,
            poles,
        } = a;
        Self {
            id: id.0.to_string(),
            display_name: display_name.to_string(),
            poles: dtos(poles),
        }
    }
}

no_ids!(AlignmentAxisDto => model::AlignmentAxis, |d| model::AlignmentAxis {
    id: model::AlignmentAxisId::new(d.id),
    display_name: arc(d.display_name),
    poles: d.poles.into_model(),
});

/// DTO for [`model::AlignmentScore`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlignmentScoreDto {
    /// Axis id.
    pub axis: String,
    /// Pole id.
    pub pole: String,
    /// Score.
    pub value: i32,
}

impl From<&model::AlignmentScore> for AlignmentScoreDto {
    fn from(s: &model::AlignmentScore) -> Self {
        let model::AlignmentScore { axis, pole, value } = s;
        Self {
            axis: axis.0.to_string(),
            pole: pole.0.to_string(),
            value: *value,
        }
    }
}

no_ids!(AlignmentScoreDto => model::AlignmentScore, |d| model::AlignmentScore::new(d.axis, d.pole, d.value));

/// DTO for [`model::RelationshipAxis`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipAxisDto {
    /// Axis id.
    pub id: String,
    /// Display name.
    pub display_name: String,
}

impl From<&model::RelationshipAxis> for RelationshipAxisDto {
    fn from(a: &model::RelationshipAxis) -> Self {
        let model::RelationshipAxis { id, display_name } = a;
        Self {
            id: id.0.to_string(),
            display_name: display_name.to_string(),
        }
    }
}

no_ids!(RelationshipAxisDto => model::RelationshipAxis, |d| model::RelationshipAxis::new(d.id, d.display_name));

/// DTO for [`model::RelationshipScore`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipScoreDto {
    /// Source entity (raw id).
    pub source: u64,
    /// Target entity (raw id).
    pub target: u64,
    /// Axis id.
    pub axis: String,
    /// Score.
    pub value: i32,
}

impl From<&model::RelationshipScore> for RelationshipScoreDto {
    fn from(s: &model::RelationshipScore) -> Self {
        let model::RelationshipScore {
            source,
            target,
            axis,
            value,
        } = s;
        Self {
            source: source.0,
            target: target.0,
            axis: axis.0.to_string(),
            value: *value,
        }
    }
}

impl IntoModel for RelationshipScoreDto {
    type Model = model::RelationshipScore;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::RelationshipScore {
        model::RelationshipScore::new(
            ids.entity(self.source),
            ids.entity(self.target),
            model::RelationshipAxisId::new(self.axis),
            self.value,
        )
    }
}

// ---------------------------------------------------------------------------
// Tags and intentions
// ---------------------------------------------------------------------------

/// DTO for [`model::CustomDialogTag`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogTagDto {
    /// Axis.
    pub axis: String,
    /// Key within the axis.
    pub key: String,
    /// Display prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_prefix: Option<String>,
    /// Whether the player needs the trait.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub requires_trait: bool,
}

impl From<&model::CustomDialogTag> for DialogTagDto {
    fn from(t: &model::CustomDialogTag) -> Self {
        let model::CustomDialogTag {
            axis,
            key,
            display_prefix,
            requires_trait,
        } = t;
        Self {
            axis: axis.to_string(),
            key: key.to_string(),
            display_prefix: display_prefix.as_deref().map(str::to_string),
            requires_trait: *requires_trait,
        }
    }
}

no_ids!(DialogTagDto => model::CustomDialogTag, |d| model::CustomDialogTag {
    axis: arc(d.axis),
    key: arc(d.key),
    display_prefix: d.display_prefix.map(arc),
    requires_trait: d.requires_trait,
});

/// DTO for [`model::Intention`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum IntentionDto {
    Learn(String),
    Reveal(String),
    Hide(String),
    Persuade,
    Intimidate,
    Comfort,
    Deceive,
    Deflect,
    Escalate,
    Deescalate,
    Trade,
    Recruit,
    Romance,
    Exit,
    Custom(String),
}

impl From<&model::Intention> for IntentionDto {
    fn from(i: &model::Intention) -> Self {
        use model::Intention as I;
        match i {
            I::Learn(f) => Self::Learn(f.0.to_string()),
            I::Reveal(f) => Self::Reveal(f.0.to_string()),
            I::Hide(f) => Self::Hide(f.0.to_string()),
            I::Persuade => Self::Persuade,
            I::Intimidate => Self::Intimidate,
            I::Comfort => Self::Comfort,
            I::Deceive => Self::Deceive,
            I::Deflect => Self::Deflect,
            I::Escalate => Self::Escalate,
            I::Deescalate => Self::Deescalate,
            I::Trade => Self::Trade,
            I::Recruit => Self::Recruit,
            I::Romance => Self::Romance,
            I::Exit => Self::Exit,
            I::Custom(c) => Self::Custom(c.to_string()),
        }
    }
}

no_ids!(IntentionDto => model::Intention, |d| {
    use model::Intention as I;
    match d {
        IntentionDto::Learn(f) => I::Learn(model::FactId::new(f)),
        IntentionDto::Reveal(f) => I::Reveal(model::FactId::new(f)),
        IntentionDto::Hide(f) => I::Hide(model::FactId::new(f)),
        IntentionDto::Persuade => I::Persuade,
        IntentionDto::Intimidate => I::Intimidate,
        IntentionDto::Comfort => I::Comfort,
        IntentionDto::Deceive => I::Deceive,
        IntentionDto::Deflect => I::Deflect,
        IntentionDto::Escalate => I::Escalate,
        IntentionDto::Deescalate => I::Deescalate,
        IntentionDto::Trade => I::Trade,
        IntentionDto::Recruit => I::Recruit,
        IntentionDto::Romance => I::Romance,
        IntentionDto::Exit => I::Exit,
        IntentionDto::Custom(c) => I::Custom(arc(c)),
    }
});

/// DTO for [`model::DialogIntentions`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogIntentionsDto {
    /// Explicit intentions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explicit: Vec<IntentionDto>,
    /// Implicit intentions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implicit: Vec<IntentionDto>,
}

impl From<&model::DialogIntentions> for DialogIntentionsDto {
    fn from(i: &model::DialogIntentions) -> Self {
        let model::DialogIntentions { explicit, implicit } = i;
        Self {
            explicit: dtos(explicit),
            implicit: dtos(implicit),
        }
    }
}

no_ids!(DialogIntentionsDto => model::DialogIntentions, |d| model::DialogIntentions {
    explicit: d.explicit.into_model(),
    implicit: d.implicit.into_model(),
});

// ---------------------------------------------------------------------------
// Memories
// ---------------------------------------------------------------------------

/// DTO for [`model::Confidence`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ConfidenceDto {
    Rumor,
    Suspected,
    Believed,
    Known,
    Witnessed,
}

impl From<&model::Confidence> for ConfidenceDto {
    fn from(c: &model::Confidence) -> Self {
        use model::Confidence as C;
        match c {
            C::Rumor => Self::Rumor,
            C::Suspected => Self::Suspected,
            C::Believed => Self::Believed,
            C::Known => Self::Known,
            C::Witnessed => Self::Witnessed,
        }
    }
}

no_ids!(ConfidenceDto => model::Confidence, |d| {
    use model::Confidence as C;
    match d {
        ConfidenceDto::Rumor => C::Rumor,
        ConfidenceDto::Suspected => C::Suspected,
        ConfidenceDto::Believed => C::Believed,
        ConfidenceDto::Known => C::Known,
        ConfidenceDto::Witnessed => C::Witnessed,
    }
});

/// DTO for [`model::MemoryTag`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum MemoryTagDto {
    HelpedMe,
    BetrayedMe,
    LiedToMe,
    SavedLife,
    ThreatenedMe,
    ShowedMercy,
    KilledSomeone,
    RevealedSecret,
    Custom(String),
}

impl From<&model::MemoryTag> for MemoryTagDto {
    fn from(t: &model::MemoryTag) -> Self {
        use model::MemoryTag as T;
        match t {
            T::HelpedMe => Self::HelpedMe,
            T::BetrayedMe => Self::BetrayedMe,
            T::LiedToMe => Self::LiedToMe,
            T::SavedLife => Self::SavedLife,
            T::ThreatenedMe => Self::ThreatenedMe,
            T::ShowedMercy => Self::ShowedMercy,
            T::KilledSomeone => Self::KilledSomeone,
            T::RevealedSecret => Self::RevealedSecret,
            T::Custom(c) => Self::Custom(c.to_string()),
        }
    }
}

no_ids!(MemoryTagDto => model::MemoryTag, |d| {
    use model::MemoryTag as T;
    match d {
        MemoryTagDto::HelpedMe => T::HelpedMe,
        MemoryTagDto::BetrayedMe => T::BetrayedMe,
        MemoryTagDto::LiedToMe => T::LiedToMe,
        MemoryTagDto::SavedLife => T::SavedLife,
        MemoryTagDto::ThreatenedMe => T::ThreatenedMe,
        MemoryTagDto::ShowedMercy => T::ShowedMercy,
        MemoryTagDto::KilledSomeone => T::KilledSomeone,
        MemoryTagDto::RevealedSecret => T::RevealedSecret,
        MemoryTagDto::Custom(c) => T::Custom(arc(c)),
    }
});

/// DTO for [`model::MemoryQuery`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryQueryDto {
    /// Required tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<MemoryTagDto>,
    /// Minimum confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_confidence: Option<ConfidenceDto>,
    /// Minimum emotional weight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_emotional_weight: Option<i32>,
    /// Required subject (raw entity id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<u64>,
    /// Any of these facts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<String>,
}

impl From<&model::MemoryQuery> for MemoryQueryDto {
    fn from(q: &model::MemoryQuery) -> Self {
        let model::MemoryQuery {
            tag,
            min_confidence,
            min_emotional_weight,
            subject,
            facts,
        } = q;
        Self {
            tag: tag.as_ref().map(Into::into),
            min_confidence: min_confidence.as_ref().map(Into::into),
            min_emotional_weight: *min_emotional_weight,
            subject: subject.map(|s| s.0),
            facts: facts.iter().map(|f| f.0.to_string()).collect(),
        }
    }
}

impl IntoModel for MemoryQueryDto {
    type Model = model::MemoryQuery;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::MemoryQuery {
        model::MemoryQuery {
            tag: self.tag.into_model_with(ids),
            min_confidence: self.min_confidence.into_model_with(ids),
            min_emotional_weight: self.min_emotional_weight,
            subject: self.subject.map(|s| ids.entity(s)),
            facts: self.facts.into_iter().map(model::FactId::new).collect(),
        }
    }
}

/// DTO for [`model::Memory`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryDto {
    /// Memory id (raw).
    pub id: u64,
    /// Main subject (raw entity id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<u64>,
    /// Related entities (raw ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub about: Vec<u64>,
    /// Referenced facts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<String>,
    /// Tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<MemoryTagDto>,
    /// Emotional weight.
    #[serde(default)]
    pub emotional_weight: i32,
    /// Confidence.
    pub confidence: ConfidenceDto,
}

impl From<&model::Memory> for MemoryDto {
    fn from(m: &model::Memory) -> Self {
        let model::Memory {
            id,
            subject,
            about,
            facts,
            tags,
            emotional_weight,
            confidence,
        } = m;
        Self {
            id: id.0,
            subject: subject.map(|s| s.0),
            about: about.iter().map(|e| e.0).collect(),
            facts: facts.iter().map(|f| f.0.to_string()).collect(),
            tags: dtos(tags),
            emotional_weight: *emotional_weight,
            confidence: confidence.into(),
        }
    }
}

impl IntoModel for MemoryDto {
    type Model = model::Memory;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::Memory {
        model::Memory {
            id: ids.memory(self.id),
            subject: self.subject.map(|s| ids.entity(s)),
            about: self.about.into_iter().map(|e| ids.entity(e)).collect(),
            facts: self.facts.into_iter().map(model::FactId::new).collect(),
            tags: self.tags.into_model_with(ids),
            emotional_weight: self.emotional_weight,
            confidence: self.confidence.into_model_with(ids),
        }
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// DTO for [`model::EventKind`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum EventKindDto {
    DialogOptionChosen {
        node_id: u64,
        option_id: u64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<DialogTagDto>,
        #[serde(default)]
        intentions: DialogIntentionsDto,
    },
    FactLearned {
        scope: FactScopeDto,
        fact_id: String,
    },
    FactRevealed {
        from: u64,
        to: u64,
        fact_id: String,
    },
    RelationshipChanged {
        a: u64,
        b: u64,
        axis: String,
        delta: i32,
    },
    AlignmentChanged {
        axis: String,
        pole: String,
        delta: i32,
    },
    Custom {
        kind: String,
    },
}

impl From<&model::EventKind> for EventKindDto {
    fn from(k: &model::EventKind) -> Self {
        use model::EventKind as K;
        match k {
            K::DialogOptionChosen {
                node_id,
                option_id,
                tags,
                intentions,
            } => Self::DialogOptionChosen {
                node_id: node_id.0,
                option_id: option_id.0,
                tags: dtos(tags),
                intentions: intentions.into(),
            },
            K::FactLearned { scope, fact_id } => Self::FactLearned {
                scope: scope.into(),
                fact_id: fact_id.0.to_string(),
            },
            K::FactRevealed { from, to, fact_id } => Self::FactRevealed {
                from: from.0,
                to: to.0,
                fact_id: fact_id.0.to_string(),
            },
            K::RelationshipChanged { a, b, axis, delta } => Self::RelationshipChanged {
                a: a.0,
                b: b.0,
                axis: axis.0.to_string(),
                delta: *delta,
            },
            K::AlignmentChanged { axis, pole, delta } => Self::AlignmentChanged {
                axis: axis.0.to_string(),
                pole: pole.0.to_string(),
                delta: *delta,
            },
            K::Custom { kind } => Self::Custom {
                kind: kind.to_string(),
            },
        }
    }
}

impl IntoModel for EventKindDto {
    type Model = model::EventKind;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::EventKind {
        use model::EventKind as K;
        match self {
            Self::DialogOptionChosen {
                node_id,
                option_id,
                tags,
                intentions,
            } => K::DialogOptionChosen {
                node_id: ids.node(node_id),
                option_id: ids.option(option_id),
                tags: tags.into_model_with(ids),
                intentions: intentions.into_model_with(ids),
            },
            Self::FactLearned { scope, fact_id } => K::FactLearned {
                scope: scope.into_model_with(ids),
                fact_id: model::FactId::new(fact_id),
            },
            Self::FactRevealed { from, to, fact_id } => K::FactRevealed {
                from: ids.entity(from),
                to: ids.entity(to),
                fact_id: model::FactId::new(fact_id),
            },
            Self::RelationshipChanged { a, b, axis, delta } => K::RelationshipChanged {
                a: ids.entity(a),
                b: ids.entity(b),
                axis: model::RelationshipAxisId::new(axis),
                delta,
            },
            Self::AlignmentChanged { axis, pole, delta } => K::AlignmentChanged {
                axis: model::AlignmentAxisId::new(axis),
                pole: model::AlignmentPoleId::new(pole),
                delta,
            },
            Self::Custom { kind } => K::Custom { kind: arc(kind) },
        }
    }
}

/// DTO for [`model::Event`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDto {
    /// Event id.
    pub id: u64,
    /// What happened.
    pub kind: EventKindDto,
    /// Actor (raw entity id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<u64>,
    /// Target (raw entity id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<u64>,
}

impl From<&model::Event> for EventDto {
    fn from(e: &model::Event) -> Self {
        let model::Event {
            id,
            kind,
            actor,
            target,
        } = e;
        Self {
            id: id.0,
            kind: kind.into(),
            actor: actor.map(|a| a.0),
            target: target.map(|t| t.0),
        }
    }
}

impl IntoModel for EventDto {
    type Model = model::Event;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::Event {
        model::Event {
            id: model::EventId::new(self.id),
            kind: self.kind.into_model_with(ids),
            actor: self.actor.map(|a| ids.entity(a)),
            target: self.target.map(|t| ids.entity(t)),
        }
    }
}

// ---------------------------------------------------------------------------
// Conditions and outcomes
// ---------------------------------------------------------------------------

/// DTO for [`model::Condition`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ConditionDto {
    Always,
    Never,
    All(Vec<ConditionDto>),
    Any(Vec<ConditionDto>),
    Not(Box<ConditionDto>),
    FactEquals {
        scope: FactScopeDto,
        fact_id: String,
        value: FactValueDto,
    },
    FactExists {
        scope: FactScopeDto,
        fact_id: String,
    },
    FactAtLeast {
        scope: FactScopeDto,
        fact_id: String,
        value: i64,
    },
    FactAtMost {
        scope: FactScopeDto,
        fact_id: String,
        value: i64,
    },
    AlignmentAtLeast {
        axis: String,
        pole: String,
        value: i32,
    },
    AlignmentAtMost {
        axis: String,
        pole: String,
        value: i32,
    },
    RelationshipAtLeast {
        source: u64,
        target: u64,
        axis: String,
        value: i32,
    },
    RelationshipAtMost {
        source: u64,
        target: u64,
        axis: String,
        value: i32,
    },
    HasMemory {
        owner: u64,
        query: MemoryQueryDto,
    },
    HasTagAvailable {
        tag: DialogTagDto,
    },
    Custom {
        key: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<FactValueDto>,
    },
}

impl From<&model::Condition> for ConditionDto {
    fn from(c: &model::Condition) -> Self {
        use model::Condition as C;
        match c {
            C::Always => Self::Always,
            C::Never => Self::Never,
            C::All(cs) => Self::All(dtos(cs)),
            C::Any(cs) => Self::Any(dtos(cs)),
            C::Not(c) => Self::Not(Box::new((&**c).into())),
            C::FactEquals {
                scope,
                fact_id,
                value,
            } => Self::FactEquals {
                scope: scope.into(),
                fact_id: fact_id.0.to_string(),
                value: value.into(),
            },
            C::FactExists { scope, fact_id } => Self::FactExists {
                scope: scope.into(),
                fact_id: fact_id.0.to_string(),
            },
            C::FactAtLeast {
                scope,
                fact_id,
                value,
            } => Self::FactAtLeast {
                scope: scope.into(),
                fact_id: fact_id.0.to_string(),
                value: *value,
            },
            C::FactAtMost {
                scope,
                fact_id,
                value,
            } => Self::FactAtMost {
                scope: scope.into(),
                fact_id: fact_id.0.to_string(),
                value: *value,
            },
            C::AlignmentAtLeast { axis, pole, value } => Self::AlignmentAtLeast {
                axis: axis.0.to_string(),
                pole: pole.0.to_string(),
                value: *value,
            },
            C::AlignmentAtMost { axis, pole, value } => Self::AlignmentAtMost {
                axis: axis.0.to_string(),
                pole: pole.0.to_string(),
                value: *value,
            },
            C::RelationshipAtLeast {
                source,
                target,
                axis,
                value,
            } => Self::RelationshipAtLeast {
                source: source.0,
                target: target.0,
                axis: axis.0.to_string(),
                value: *value,
            },
            C::RelationshipAtMost {
                source,
                target,
                axis,
                value,
            } => Self::RelationshipAtMost {
                source: source.0,
                target: target.0,
                axis: axis.0.to_string(),
                value: *value,
            },
            C::HasMemory { owner, query } => Self::HasMemory {
                owner: owner.0,
                query: query.into(),
            },
            C::HasTagAvailable { tag } => Self::HasTagAvailable { tag: tag.into() },
            C::Custom { key, args } => Self::Custom {
                key: key.clone(),
                args: dtos(args),
            },
        }
    }
}

impl IntoModel for ConditionDto {
    type Model = model::Condition;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::Condition {
        use model::Condition as C;
        match self {
            Self::Always => C::Always,
            Self::Never => C::Never,
            Self::All(cs) => C::All(cs.into_model_with(ids)),
            Self::Any(cs) => C::Any(cs.into_model_with(ids)),
            Self::Not(c) => C::Not(c.into_model_with(ids)),
            Self::FactEquals {
                scope,
                fact_id,
                value,
            } => C::FactEquals {
                scope: scope.into_model_with(ids),
                fact_id: model::FactId::new(fact_id),
                value: value.into_model_with(ids),
            },
            Self::FactExists { scope, fact_id } => C::FactExists {
                scope: scope.into_model_with(ids),
                fact_id: model::FactId::new(fact_id),
            },
            Self::FactAtLeast {
                scope,
                fact_id,
                value,
            } => C::FactAtLeast {
                scope: scope.into_model_with(ids),
                fact_id: model::FactId::new(fact_id),
                value,
            },
            Self::FactAtMost {
                scope,
                fact_id,
                value,
            } => C::FactAtMost {
                scope: scope.into_model_with(ids),
                fact_id: model::FactId::new(fact_id),
                value,
            },
            Self::AlignmentAtLeast { axis, pole, value } => C::AlignmentAtLeast {
                axis: model::AlignmentAxisId::new(axis),
                pole: model::AlignmentPoleId::new(pole),
                value,
            },
            Self::AlignmentAtMost { axis, pole, value } => C::AlignmentAtMost {
                axis: model::AlignmentAxisId::new(axis),
                pole: model::AlignmentPoleId::new(pole),
                value,
            },
            Self::RelationshipAtLeast {
                source,
                target,
                axis,
                value,
            } => C::RelationshipAtLeast {
                source: ids.entity(source),
                target: ids.entity(target),
                axis: model::RelationshipAxisId::new(axis),
                value,
            },
            Self::RelationshipAtMost {
                source,
                target,
                axis,
                value,
            } => C::RelationshipAtMost {
                source: ids.entity(source),
                target: ids.entity(target),
                axis: model::RelationshipAxisId::new(axis),
                value,
            },
            Self::HasMemory { owner, query } => C::HasMemory {
                owner: ids.entity(owner),
                query: query.into_model_with(ids),
            },
            Self::HasTagAvailable { tag } => C::HasTagAvailable {
                tag: tag.into_model_with(ids),
            },
            Self::Custom { key, args } => C::Custom {
                key,
                args: args.into_model_with(ids),
            },
        }
    }
}

/// DTO for [`model::Outcome`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum OutcomeDto {
    SetFact {
        scope: FactScopeDto,
        fact: FactDto,
    },
    RemoveFact {
        scope: FactScopeDto,
        fact_id: String,
    },
    AddAlignment {
        axis: String,
        pole: String,
        delta: i32,
    },
    AddRelationship {
        source: u64,
        target: u64,
        axis: String,
        delta: i32,
    },
    AddMemory {
        owner: u64,
        memory: MemoryDto,
    },
    RemoveMemory {
        owner: u64,
        memory_id: u64,
    },
    EmitEvent {
        kind: EventKindDto,
    },
    StartQuest {
        quest_id: String,
    },
    AdvanceQuest {
        quest_id: String,
        stage: String,
    },
    Custom {
        key: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<FactValueDto>,
    },
}

impl From<&model::Outcome> for OutcomeDto {
    fn from(o: &model::Outcome) -> Self {
        use model::Outcome as O;
        match o {
            O::SetFact { scope, fact } => Self::SetFact {
                scope: scope.into(),
                fact: fact.into(),
            },
            O::RemoveFact { scope, fact_id } => Self::RemoveFact {
                scope: scope.into(),
                fact_id: fact_id.0.to_string(),
            },
            O::AddAlignment { axis, pole, delta } => Self::AddAlignment {
                axis: axis.0.to_string(),
                pole: pole.0.to_string(),
                delta: *delta,
            },
            O::AddRelationship {
                source,
                target,
                axis,
                delta,
            } => Self::AddRelationship {
                source: source.0,
                target: target.0,
                axis: axis.0.to_string(),
                delta: *delta,
            },
            O::AddMemory { owner, memory } => Self::AddMemory {
                owner: owner.0,
                memory: memory.into(),
            },
            O::RemoveMemory { owner, memory_id } => Self::RemoveMemory {
                owner: owner.0,
                memory_id: memory_id.0,
            },
            O::EmitEvent { kind } => Self::EmitEvent { kind: kind.into() },
            O::StartQuest { quest_id } => Self::StartQuest {
                quest_id: quest_id.clone(),
            },
            O::AdvanceQuest { quest_id, stage } => Self::AdvanceQuest {
                quest_id: quest_id.clone(),
                stage: stage.clone(),
            },
            O::Custom { key, args } => Self::Custom {
                key: key.clone(),
                args: dtos(args),
            },
        }
    }
}

impl IntoModel for OutcomeDto {
    type Model = model::Outcome;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::Outcome {
        use model::Outcome as O;
        match self {
            Self::SetFact { scope, fact } => O::SetFact {
                scope: scope.into_model_with(ids),
                fact: fact.into_model_with(ids),
            },
            Self::RemoveFact { scope, fact_id } => O::RemoveFact {
                scope: scope.into_model_with(ids),
                fact_id: model::FactId::new(fact_id),
            },
            Self::AddAlignment { axis, pole, delta } => O::AddAlignment {
                axis: model::AlignmentAxisId::new(axis),
                pole: model::AlignmentPoleId::new(pole),
                delta,
            },
            Self::AddRelationship {
                source,
                target,
                axis,
                delta,
            } => O::AddRelationship {
                source: ids.entity(source),
                target: ids.entity(target),
                axis: model::RelationshipAxisId::new(axis),
                delta,
            },
            Self::AddMemory { owner, memory } => O::AddMemory {
                owner: ids.entity(owner),
                memory: memory.into_model_with(ids),
            },
            Self::RemoveMemory { owner, memory_id } => O::RemoveMemory {
                owner: ids.entity(owner),
                memory_id: ids.memory(memory_id),
            },
            Self::EmitEvent { kind } => O::EmitEvent {
                kind: kind.into_model_with(ids),
            },
            Self::StartQuest { quest_id } => O::StartQuest { quest_id },
            Self::AdvanceQuest { quest_id, stage } => O::AdvanceQuest { quest_id, stage },
            Self::Custom { key, args } => O::Custom {
                key,
                args: args.into_model_with(ids),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Dialogue graphs
// ---------------------------------------------------------------------------

/// DTO for [`model::DialogText`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogTextDto {
    /// Inline text.
    Inline(String),
    /// Localization key.
    LocalizedKey(String),
    /// Fragment list.
    Fragments(Vec<TextFragmentDto>),
}

impl From<&model::DialogText> for DialogTextDto {
    fn from(t: &model::DialogText) -> Self {
        use model::DialogText as T;
        match t {
            T::Inline(s) => Self::Inline(s.clone()),
            T::LocalizedKey(k) => Self::LocalizedKey(k.to_string()),
            T::Fragments(fs) => Self::Fragments(dtos(fs)),
        }
    }
}

impl IntoModel for DialogTextDto {
    type Model = model::DialogText;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::DialogText {
        use model::DialogText as T;
        match self {
            Self::Inline(s) => T::Inline(s),
            Self::LocalizedKey(k) => T::LocalizedKey(arc(k)),
            Self::Fragments(fs) => T::Fragments(fs.into_model_with(ids)),
        }
    }
}

/// DTO for [`model::FragmentMode`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum FragmentModeDto {
    #[default]
    Always,
    FirstTime,
    Once,
    Cycle,
    Shuffle,
    Sticky,
}

impl From<&model::FragmentMode> for FragmentModeDto {
    fn from(m: &model::FragmentMode) -> Self {
        use model::FragmentMode as F;
        match m {
            F::Always => Self::Always,
            F::FirstTime => Self::FirstTime,
            F::Once => Self::Once,
            F::Cycle => Self::Cycle,
            F::Shuffle => Self::Shuffle,
            F::Sticky => Self::Sticky,
        }
    }
}

no_ids!(FragmentModeDto => model::FragmentMode, |d| {
    use model::FragmentMode as F;
    match d {
        FragmentModeDto::Always => F::Always,
        FragmentModeDto::FirstTime => F::FirstTime,
        FragmentModeDto::Once => F::Once,
        FragmentModeDto::Cycle => F::Cycle,
        FragmentModeDto::Shuffle => F::Shuffle,
        FragmentModeDto::Sticky => F::Sticky,
    }
});

/// DTO for [`model::TextFragment`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextFragmentDto {
    /// Stable progress key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Variants.
    pub variants: Vec<DialogTextDto>,
    /// Condition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<ConditionDto>,
    /// Mode.
    #[serde(default)]
    pub mode: FragmentModeDto,
    /// Replace earlier text.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replaces: bool,
}

impl From<&model::TextFragment> for TextFragmentDto {
    fn from(f: &model::TextFragment) -> Self {
        let model::TextFragment {
            key,
            variants,
            condition,
            mode,
            replaces,
        } = f;
        Self {
            key: key.as_deref().map(str::to_string),
            variants: dtos(variants),
            condition: condition.as_ref().map(Into::into),
            mode: mode.into(),
            replaces: *replaces,
        }
    }
}

impl IntoModel for TextFragmentDto {
    type Model = model::TextFragment;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::TextFragment {
        model::TextFragment {
            key: self.key.map(arc),
            variants: self.variants.into_model_with(ids),
            condition: self.condition.into_model_with(ids),
            mode: self.mode.into_model_with(ids),
            replaces: self.replaces,
        }
    }
}

/// DTO for [`model::DialogTarget`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogTargetDto {
    /// Another node (raw id).
    Node(u64),
    /// End of conversation.
    End,
    /// Return to caller.
    Return,
    /// Host-defined target.
    Custom(String),
}

impl From<&model::DialogTarget> for DialogTargetDto {
    fn from(t: &model::DialogTarget) -> Self {
        use model::DialogTarget as T;
        match t {
            T::Node(n) => Self::Node(n.0),
            T::End => Self::End,
            T::Return => Self::Return,
            T::Custom(c) => Self::Custom(c.to_string()),
        }
    }
}

impl IntoModel for DialogTargetDto {
    type Model = model::DialogTarget;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::DialogTarget {
        use model::DialogTarget as T;
        match self {
            Self::Node(n) => T::Node(ids.node(n)),
            Self::End => T::End,
            Self::Return => T::Return,
            Self::Custom(c) => T::Custom(arc(c)),
        }
    }
}

/// DTO for [`model::NavigationHint`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum NavigationHintDto {
    Continue,
    Leave,
    GoTo(String),
    End,
    Return,
    Custom(String),
}

impl From<&model::NavigationHint> for NavigationHintDto {
    fn from(h: &model::NavigationHint) -> Self {
        use model::NavigationHint as H;
        match h {
            H::Continue => Self::Continue,
            H::Leave => Self::Leave,
            H::GoTo(t) => Self::GoTo(t.clone()),
            H::End => Self::End,
            H::Return => Self::Return,
            H::Custom(c) => Self::Custom(c.clone()),
        }
    }
}

no_ids!(NavigationHintDto => model::NavigationHint, |d| {
    use model::NavigationHint as H;
    match d {
        NavigationHintDto::Continue => H::Continue,
        NavigationHintDto::Leave => H::Leave,
        NavigationHintDto::GoTo(t) => H::GoTo(t),
        NavigationHintDto::End => H::End,
        NavigationHintDto::Return => H::Return,
        NavigationHintDto::Custom(c) => H::Custom(c),
    }
});

/// DTO for [`model::DialogueOption`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueOptionDto {
    /// Option id (raw).
    pub id: u64,
    /// Option text.
    pub text: DialogTextDto,
    /// Tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<DialogTagDto>,
    /// Intentions.
    #[serde(default)]
    pub intentions: DialogIntentionsDto,
    /// Availability conditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<ConditionDto>,
    /// Outcomes when chosen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcomes: Vec<OutcomeDto>,
    /// Where the option leads.
    pub target: DialogTargetDto,
    /// Parent node (raw id).
    pub source_node_id: u64,
    /// Navigation hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation_hint: Option<NavigationHintDto>,
}

impl From<&model::DialogueOption> for DialogueOptionDto {
    fn from(o: &model::DialogueOption) -> Self {
        let model::DialogueOption {
            id,
            text,
            tags,
            intentions,
            conditions,
            outcomes,
            target,
            source_node_id,
            navigation_hint,
        } = o;
        Self {
            id: id.0,
            text: text.into(),
            tags: dtos(tags),
            intentions: intentions.into(),
            conditions: dtos(conditions),
            outcomes: dtos(outcomes),
            target: target.into(),
            source_node_id: source_node_id.0,
            navigation_hint: navigation_hint.as_ref().map(Into::into),
        }
    }
}

impl IntoModel for DialogueOptionDto {
    type Model = model::DialogueOption;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::DialogueOption {
        model::DialogueOption {
            id: ids.option(self.id),
            text: self.text.into_model_with(ids),
            tags: self.tags.into_model_with(ids),
            intentions: self.intentions.into_model_with(ids),
            conditions: self.conditions.into_model_with(ids),
            outcomes: self.outcomes.into_model_with(ids),
            target: self.target.into_model_with(ids),
            source_node_id: ids.node(self.source_node_id),
            navigation_hint: self.navigation_hint.into_model_with(ids),
        }
    }
}

/// DTO for [`model::DialogueNode`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueNodeDto {
    /// Node id (raw).
    pub id: u64,
    /// Speaker (raw entity id).
    pub speaker: u64,
    /// Spoken text.
    pub text: DialogTextDto,
    /// Entry conditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<ConditionDto>,
    /// Outcomes on entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcomes_on_enter: Vec<OutcomeDto>,
    /// Options.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<DialogueOptionDto>,
}

impl From<&model::DialogueNode> for DialogueNodeDto {
    fn from(n: &model::DialogueNode) -> Self {
        let model::DialogueNode {
            id,
            speaker,
            text,
            conditions,
            outcomes_on_enter,
            options,
        } = n;
        Self {
            id: id.0,
            speaker: speaker.0.0,
            text: text.into(),
            conditions: dtos(conditions),
            outcomes_on_enter: dtos(outcomes_on_enter),
            options: dtos(options),
        }
    }
}

impl IntoModel for DialogueNodeDto {
    type Model = model::DialogueNode;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::DialogueNode {
        model::DialogueNode {
            id: ids.node(self.id),
            speaker: model::CharacterId::new(ids.entity(self.speaker)),
            text: self.text.into_model_with(ids),
            conditions: self.conditions.into_model_with(ids),
            outcomes_on_enter: self.outcomes_on_enter.into_model_with(ids),
            options: self.options.into_model_with(ids),
        }
    }
}

/// DTO for [`model::DialogueGraph`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueGraphDto {
    /// Nodes.
    pub nodes: Vec<DialogueNodeDto>,
    /// Entry node (raw id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_node: Option<u64>,
}

impl From<&model::DialogueGraph> for DialogueGraphDto {
    fn from(g: &model::DialogueGraph) -> Self {
        let model::DialogueGraph { nodes, entry_node } = g;
        Self {
            nodes: dtos(nodes),
            entry_node: entry_node.map(|n| n.0),
        }
    }
}

impl IntoModel for DialogueGraphDto {
    type Model = model::DialogueGraph;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> model::DialogueGraph {
        model::DialogueGraph {
            nodes: self.nodes.into_model_with(ids),
            entry_node: self.entry_node.map(|n| ids.node(n)),
        }
    }
}

// ---------------------------------------------------------------------------
// Symbol tables
// ---------------------------------------------------------------------------

/// DTO for [`model::SymbolTable`]: the keys of each namespace in id order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolTableDto {
    /// Entity keys; the index is the id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<String>,
    /// Node keys; the index is the id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<String>,
    /// Option keys; the index is the id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Memory keys; the index is the id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub memories: Vec<String>,
}

impl From<&model::SymbolTable> for SymbolTableDto {
    fn from(t: &model::SymbolTable) -> Self {
        let model::SymbolTable {
            entities,
            nodes,
            options,
            memories,
        } = t;
        fn keys<Id: model::SymbolId>(i: &model::Interner<Id>) -> Vec<String> {
            i.iter().map(|(_, k)| k.to_string()).collect()
        }
        Self {
            entities: keys(entities),
            nodes: keys(nodes),
            options: keys(options),
            memories: keys(memories),
        }
    }
}

no_ids!(SymbolTableDto => model::SymbolTable, |d| {
    let mut t = model::SymbolTable::new();
    for k in &d.entities {
        t.intern_entity(k);
    }
    for k in &d.nodes {
        t.intern_node(k);
    }
    for k in &d.options {
        t.intern_option(k);
    }
    for k in &d.memories {
        t.intern_memory(k);
    }
    t
});

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

/// A memory together with its owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedMemoryDto {
    /// Owner (raw entity id).
    pub owner: u64,
    /// The memory.
    #[serde(flatten)]
    pub memory: MemoryDto,
}

/// DTO for [`DialogWorldState`].
///
/// Lists are in the state's deterministic iteration order, so the same
/// state always serializes to the same bytes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldStateDto {
    /// Facts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<ScopedFactDto>,
    /// Alignment scores.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alignments: Vec<AlignmentScoreDto>,
    /// Relationship scores.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<RelationshipScoreDto>,
    /// Memories with their owners.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub memories: Vec<OwnedMemoryDto>,
    /// Available tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_tags: Vec<DialogTagDto>,
    /// Event log.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<EventDto>,
    /// Next event id to allocate; `None` once the id space is exhausted.
    pub next_event_id: Option<u64>,
}

impl From<&DialogWorldState> for WorldStateDto {
    fn from(s: &DialogWorldState) -> Self {
        Self {
            facts: s
                .facts()
                .map(|(scope, id, value)| ScopedFactDto {
                    scope: scope.into(),
                    id: id.0.to_string(),
                    value: value.into(),
                })
                .collect(),
            alignments: s
                .alignments()
                .map(|(axis, pole, value)| AlignmentScoreDto {
                    axis: axis.0.to_string(),
                    pole: pole.0.to_string(),
                    value,
                })
                .collect(),
            relationships: s
                .relationships()
                .map(|(source, target, axis, value)| RelationshipScoreDto {
                    source: source.0,
                    target: target.0,
                    axis: axis.0.to_string(),
                    value,
                })
                .collect(),
            memories: s
                .memories()
                .map(|(owner, memory)| OwnedMemoryDto {
                    owner: owner.0,
                    memory: memory.into(),
                })
                .collect(),
            available_tags: dtos(s.available_tags()),
            events: dtos(s.events()),
            next_event_id: s.next_event_id(),
        }
    }
}

impl IntoModel for WorldStateDto {
    type Model = DialogWorldState;
    fn into_model_with<M: IdMap + ?Sized>(self, ids: &mut M) -> DialogWorldState {
        let mut s = DialogWorldState::new();
        for f in self.facts {
            let sf = f.into_model_with(ids);
            s.set_fact(sf.scope, sf.fact);
        }
        for a in self.alignments {
            let a = a.into_model_with(ids);
            s.add_alignment(a.axis, a.pole, a.value);
        }
        for r in self.relationships {
            let r = r.into_model_with(ids);
            s.add_relationship(r.source, r.target, r.axis, r.value);
        }
        for m in self.memories {
            let owner = ids.entity(m.owner);
            s.add_memory(owner, m.memory.into_model_with(ids));
        }
        for t in self.available_tags {
            s.add_available_tag(t.into_model_with(ids));
        }
        for e in self.events {
            s.add_event(e.into_model_with(ids));
        }
        s.set_next_event_id(self.next_event_id);
        s
    }
}

/// Progress of one fragment, by progress key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentProgressDto {
    /// Progress key (`owner/fragment/...`).
    pub key: String,
    /// Times shown.
    pub shown: u32,
    /// Last or sticky pick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pick: Option<u32>,
}

/// How often one owner's text was resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerResolutionsDto {
    /// Owner key.
    pub owner: String,
    /// Resolution count.
    pub resolutions: u32,
}

/// DTO for [`TextProgress`](dramaturge_runtime::TextProgress).
///
/// Progress keys are author keys (owner and fragment keys), not ids, so
/// text progress stays valid across content changes without remapping.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextProgressDto {
    /// Fragment progress.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fragments: Vec<FragmentProgressDto>,
    /// Owner resolution counts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<OwnerResolutionsDto>,
}

impl From<&dramaturge_runtime::TextProgress> for TextProgressDto {
    fn from(p: &dramaturge_runtime::TextProgress) -> Self {
        Self {
            fragments: p
                .fragments()
                .map(|(key, f)| FragmentProgressDto {
                    key: key.to_string(),
                    shown: f.shown,
                    pick: f.pick,
                })
                .collect(),
            owners: p
                .owners()
                .map(|(owner, resolutions)| OwnerResolutionsDto {
                    owner: owner.to_string(),
                    resolutions,
                })
                .collect(),
        }
    }
}

no_ids!(TextProgressDto => dramaturge_runtime::TextProgress, |d| {
    let mut p = dramaturge_runtime::TextProgress::new();
    for f in d.fragments {
        p.set_fragment(
            f.key,
            dramaturge_runtime::FragmentProgress {
                shown: f.shown,
                pick: f.pick,
            },
        );
    }
    for o in d.owners {
        p.set_resolutions(o.owner, o.resolutions);
    }
    p
});
