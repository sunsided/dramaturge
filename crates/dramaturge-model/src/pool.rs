//! Text pools: the data of the text selection core.
//!
//! A [`Pool`] is an ordered list of [`Candidate`]s and a [`Pick`] rule. A
//! candidate is one line that can show: it has an optional key, an optional
//! condition (`when`), optional tuple keys (speaker and topic), a
//! [`FragmentMode`], one or more variants and a `learn` list. A variant
//! ([`Entry`]) is a host text leaf, an inline pool (`one_of`), or the id of
//! a named pool in [`Pools`].
//!
//! The pool types are generic over the host types in [`TextTypes`]:
//! the text leaf, the condition, the learn item, the actor (speaker and
//! perceiver) and the topic. dramaturge does not look inside the leaf, the
//! condition or the learn item. [`DialogTypes`] are the types of plain
//! dramaturge: [`DialogText`] leaves, [`Condition`]s and [`Fact`]s.
//!
//! The selection itself (`select`, `commit`, the trace) is in the
//! `dramaturge-runtime` crate, module `text`.
//!
//! ```rust
//! use dramaturge_model::{
//!     Candidate, Condition, DialogText, DialogTypes, FactId, FactScope, Pick, Pool,
//! };
//!
//! let fresh = Condition::FactExists {
//!     scope: FactScope::World,
//!     fact_id: FactId::new("ash_is_fresh"),
//! };
//! let pool: Pool<DialogTypes> = Pool::new("keeper_remark", Pick::Best)
//!     .with_candidate(Candidate::leaf(DialogText::inline("Mind your step.")))
//!     .with_candidate(Candidate::leaf(DialogText::inline("Cold stove.")).when(fresh));
//! assert_eq!(pool.candidates[0].score(), 0);
//! assert_eq!(pool.candidates[1].score(), 1);
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::character::CharacterId;
use crate::condition::Condition;
use crate::dialogue::{DialogText, FragmentMode};
use crate::fact::Fact;

/// The number of tests in a condition. [`Pick::Best`] uses it as the score
/// of a candidate.
///
/// The rules:
///
/// - Each atomic test (a fact test, a perceiver test) counts 1.
/// - `a and b` counts `score(a) + score(b)`.
/// - `a or b` counts `min(score(a), score(b))`. Thus a candidate never wins
///   by an `or` branch that did not hold.
/// - `not a` counts `score(a)`.
/// - A constant (always or never) counts 0.
pub trait Specificity {
    /// Returns the number of tests, by the rules above.
    fn specificity(&self) -> u32;
}

impl<C: Specificity + ?Sized> Specificity for &C {
    fn specificity(&self) -> u32 {
        (**self).specificity()
    }
}

impl Specificity for Condition {
    fn specificity(&self) -> u32 {
        match self {
            Condition::Always | Condition::Never => 0,
            Condition::All(conds) => conds
                .iter()
                .map(Specificity::specificity)
                .fold(0, u32::saturating_add),
            Condition::Any(conds) => conds
                .iter()
                .map(Specificity::specificity)
                .min()
                .unwrap_or(0),
            Condition::Not(inner) => inner.specificity(),
            _ => 1,
        }
    }
}

/// The host types of a text pool.
///
/// dramaturge never looks inside `Leaf`, `Cond` or `Learn`. The core
/// compares `Actor` and `Topic` values with `==` for the tuple keys.
pub trait TextTypes {
    /// The host text leaf: what shows.
    type Leaf;
    /// The host condition (`when`). Its [`Specificity`] is the score.
    type Cond: Specificity;
    /// What a shown text teaches the perceiver (`learn`).
    type Learn;
    /// A speaker or a perceiver.
    type Actor: Clone + Eq;
    /// A topic or a thing.
    type Topic: Clone + Eq;
}

/// The [`TextTypes`] of plain dramaturge.
///
/// - `Leaf` is [`DialogText<T>`] (`T` is the host text type, default `()`).
/// - `Cond` is [`Condition`].
/// - `Learn` is [`Fact`]. The default learner of the runtime writes it in
///   the world scope.
/// - `Actor` is [`CharacterId`] and `Topic` is `Arc<str>`.
pub struct DialogTypes<T = ()>(PhantomData<fn() -> T>);

impl<T> TextTypes for DialogTypes<T> {
    type Leaf = DialogText<T>;
    type Cond = Condition;
    type Learn = Fact;
    type Actor = CharacterId;
    type Topic = Arc<str>;
}

impl<T> fmt::Debug for DialogTypes<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DialogTypes")
    }
}

/// How a [`Pool`] chooses among its eligible candidates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Pick {
    /// Every eligible candidate, in file order. This is a fragment list.
    All,
    /// The first eligible candidate in file order. The default.
    #[default]
    First,
    /// The eligible candidate with the highest [score](Candidate::score).
    /// A tie goes to file order.
    Best,
}

/// One variant of a [`Candidate`].
#[non_exhaustive]
pub enum Entry<X: TextTypes> {
    /// A host text leaf.
    Leaf(X::Leaf),
    /// An inline pool (`one_of`). Its resolutions count with the pool
    /// that holds it.
    Pool(Pool<X>),
    /// The id of a named pool in [`Pools`]. Its resolutions count on their
    /// own, per caller.
    Named(Arc<str>),
}

impl<X: TextTypes> Entry<X> {
    /// Creates a reference to a named pool.
    pub fn named(id: impl Into<Arc<str>>) -> Self {
        Entry::Named(id.into())
    }
}

/// One line that can show.
///
/// A candidate is eligible when all of these are true:
///
/// 1. Its tuple keys match the query. A missing key (`None`) matches all.
/// 2. Its mode allows it now (`Once`: not shown yet; `FirstTime`: the first
///    resolution).
/// 3. It has at least one variant.
/// 4. Its condition holds.
#[non_exhaustive]
pub struct Candidate<X: TextTypes> {
    /// The progress key. Without a key, the candidate's index is its key,
    /// so a reorder moves its progress to another candidate. A key must
    /// pass [`Candidate::is_valid_key`] and be unique in its pool.
    pub key: Option<Arc<str>>,
    /// The speaker key. `None` matches every speaker (`*`).
    pub speaker: Option<X::Actor>,
    /// The topic key. `None` matches every topic (`*`).
    pub topic: Option<X::Topic>,
    /// The condition.
    pub when: Option<X::Cond>,
    /// Which variant shows, and how often.
    pub mode: FragmentMode,
    /// The variants. One for `Always`, `FirstTime` and `Once`.
    pub variants: Vec<Entry<X>>,
    /// What the perceiver learns when the candidate shows.
    pub learn: Vec<X::Learn>,
}

impl<X: TextTypes> Candidate<X> {
    /// Creates a candidate with the mode `Always` and the given variants.
    pub fn new(variants: Vec<Entry<X>>) -> Self {
        Self {
            key: None,
            speaker: None,
            topic: None,
            when: None,
            mode: FragmentMode::Always,
            variants,
            learn: Vec::new(),
        }
    }

    /// Creates a candidate with one leaf.
    pub fn leaf(leaf: X::Leaf) -> Self {
        Self::new(vec![Entry::Leaf(leaf)])
    }

    /// Creates a candidate with one inline pool.
    pub fn pool(pool: Pool<X>) -> Self {
        Self::new(vec![Entry::Pool(pool)])
    }

    /// Creates a candidate that shows a named pool.
    pub fn named(id: impl Into<Arc<str>>) -> Self {
        Self::new(vec![Entry::named(id)])
    }

    /// Returns `true` when `key` can be a progress key: not empty, not
    /// digits only (the key of an unkeyed candidate) and without a `/` (the
    /// separator of inline pool paths). Other keys share progress with
    /// another candidate.
    pub fn is_valid_key(key: &str) -> bool {
        !key.is_empty() && !key.contains('/') && !key.bytes().all(|b| b.is_ascii_digit())
    }

    /// Sets the progress key.
    pub fn keyed(mut self, key: impl Into<Arc<str>>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Sets the speaker key.
    pub fn speaker(mut self, speaker: X::Actor) -> Self {
        self.speaker = Some(speaker);
        self
    }

    /// Sets the topic key.
    pub fn topic(mut self, topic: X::Topic) -> Self {
        self.topic = Some(topic);
        self
    }

    /// Sets the condition.
    pub fn when(mut self, when: X::Cond) -> Self {
        self.when = Some(when);
        self
    }

    /// Sets the mode.
    pub fn mode(mut self, mode: FragmentMode) -> Self {
        self.mode = mode;
        self
    }

    /// Adds a learn item.
    pub fn learn(mut self, item: X::Learn) -> Self {
        self.learn.push(item);
        self
    }

    /// Returns the score for [`Pick::Best`]: the [`Specificity`] of the
    /// condition (0 without one), plus 1 for each tuple key that is set.
    ///
    /// The score comes from the fields, so a change to a field changes the
    /// score too.
    pub fn score(&self) -> u32 {
        self.when
            .as_ref()
            .map_or(0, Specificity::specificity)
            .saturating_add(u32::from(self.speaker.is_some()))
            .saturating_add(u32::from(self.topic.is_some()))
    }
}

/// An ordered list of candidates and a pick rule.
///
/// A pool in [`Pools`] is a named pool: rules and dialogue call it by id.
/// A pool in an [`Entry::Pool`] is an inline pool. The pool type is the
/// same; the place (named or inline) decides where its resolutions count.
#[non_exhaustive]
pub struct Pool<X: TextTypes> {
    /// The id. For an inline pool at the root of a selection, it is also
    /// the owner of its resolutions and the first segment of the progress
    /// keys.
    pub id: Arc<str>,
    /// The pick rule.
    pub pick: Pick,
    /// The candidates, in file order.
    pub candidates: Vec<Candidate<X>>,
}

impl<X: TextTypes> Pool<X> {
    /// Returns `true` when `id` can name a pool in [`Pools`]: not empty and
    /// without a `/`. The candidates of a named pool have the progress paths
    /// `<id>/<key>`, and with a `/` in the id that path could equal one of
    /// an inline pool in another pool (pool `a/x`, key `k` against pool `a`,
    /// candidate `x`, inline key `k`). The id of a root inline pool (an
    /// owner, see `Count::Inline`) is the host's own namespace and may hold
    /// `/`.
    pub fn is_valid_id(id: &str) -> bool {
        !id.is_empty() && !id.contains('/')
    }

    /// Creates an empty pool.
    pub fn new(id: impl Into<Arc<str>>, pick: Pick) -> Self {
        Self {
            id: id.into(),
            pick,
            candidates: Vec::new(),
        }
    }

    /// Adds a candidate.
    pub fn with_candidate(mut self, candidate: Candidate<X>) -> Self {
        self.candidates.push(candidate);
        self
    }

    /// Returns the first candidate key, here or in an inline pool, that
    /// would share progress with another candidate: a key that fails
    /// [`Candidate::is_valid_key`], or a key that two candidates of one
    /// progress path have. The inline pools of one candidate share its
    /// path, so their candidates must have distinct keys too.
    pub fn bad_key(&self) -> Option<&str> {
        bad_key(&mut self.candidates.iter())
    }
}

/// Returns the first bad key among `candidates`, which share one progress
/// path: a pool, or every inline pool of one candidate.
fn bad_key<'a, X: TextTypes + 'a>(
    candidates: &mut dyn Iterator<Item = &'a Candidate<X>>,
) -> Option<&'a str> {
    let mut seen = std::collections::BTreeSet::new();
    for c in candidates {
        if let Some(key) = c.key.as_deref()
            && (!Candidate::<X>::is_valid_key(key) || !seen.insert(key))
        {
            return Some(key);
        }
        let mut inner = c
            .variants
            .iter()
            .filter_map(|v| match v {
                Entry::Pool(inner) => Some(inner.candidates.iter()),
                _ => None,
            })
            .flatten();
        if let Some(key) = bad_key(&mut inner) {
            return Some(key);
        }
    }
    None
}

/// The named pools, by id.
#[non_exhaustive]
pub struct Pools<X: TextTypes> {
    pools: BTreeMap<Arc<str>, Pool<X>>,
}

impl<X: TextTypes> Default for Pools<X> {
    fn default() -> Self {
        Self {
            pools: BTreeMap::new(),
        }
    }
}

impl<X: TextTypes> Pools<X> {
    /// Creates an empty set of pools.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a pool under its id. Returns the pool that had the same id.
    pub fn insert(&mut self, pool: Pool<X>) -> Option<Pool<X>> {
        self.pools.insert(pool.id.clone(), pool)
    }

    /// Adds a pool and returns the set.
    pub fn with(mut self, pool: Pool<X>) -> Self {
        self.insert(pool);
        self
    }

    /// Returns the pool with the given id.
    pub fn get(&self, id: &str) -> Option<&Pool<X>> {
        self.pools.get(id)
    }

    /// Removes the pool with the given id and returns it.
    pub fn remove(&mut self, id: &str) -> Option<Pool<X>> {
        self.pools.remove(id)
    }

    /// Iterates over the pools in id order.
    pub fn iter(&self) -> impl Iterator<Item = &Pool<X>> {
        self.pools.values()
    }

    /// Returns the number of pools.
    pub fn len(&self) -> usize {
        self.pools.len()
    }

    /// Returns `true` when there are no pools.
    pub fn is_empty(&self) -> bool {
        self.pools.is_empty()
    }
}

// ------------------------------------------------------------ trait impls
//
// `derive` would ask for `X: Clone` (and so on), not for the associated
// types. These impls ask for what they need.

impl<X: TextTypes> Clone for Entry<X>
where
    X::Leaf: Clone,
    X::Cond: Clone,
    X::Learn: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Entry::Leaf(l) => Entry::Leaf(l.clone()),
            Entry::Pool(p) => Entry::Pool(p.clone()),
            Entry::Named(id) => Entry::Named(id.clone()),
        }
    }
}

impl<X: TextTypes> Clone for Candidate<X>
where
    X::Leaf: Clone,
    X::Cond: Clone,
    X::Learn: Clone,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            speaker: self.speaker.clone(),
            topic: self.topic.clone(),
            when: self.when.clone(),
            mode: self.mode,
            variants: self.variants.clone(),
            learn: self.learn.clone(),
        }
    }
}

impl<X: TextTypes> Clone for Pool<X>
where
    X::Leaf: Clone,
    X::Cond: Clone,
    X::Learn: Clone,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            pick: self.pick,
            candidates: self.candidates.clone(),
        }
    }
}

impl<X: TextTypes> Clone for Pools<X>
where
    X::Leaf: Clone,
    X::Cond: Clone,
    X::Learn: Clone,
{
    fn clone(&self) -> Self {
        Self {
            pools: self.pools.clone(),
        }
    }
}

impl<X: TextTypes> PartialEq for Entry<X>
where
    X::Leaf: PartialEq,
    X::Cond: PartialEq,
    X::Learn: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Entry::Leaf(a), Entry::Leaf(b)) => a == b,
            (Entry::Pool(a), Entry::Pool(b)) => a == b,
            (Entry::Named(a), Entry::Named(b)) => a == b,
            _ => false,
        }
    }
}

impl<X: TextTypes> PartialEq for Candidate<X>
where
    X::Leaf: PartialEq,
    X::Cond: PartialEq,
    X::Learn: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.speaker == other.speaker
            && self.topic == other.topic
            && self.when == other.when
            && self.mode == other.mode
            && self.variants == other.variants
            && self.learn == other.learn
    }
}

impl<X: TextTypes> PartialEq for Pool<X>
where
    X::Leaf: PartialEq,
    X::Cond: PartialEq,
    X::Learn: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.pick == other.pick && self.candidates == other.candidates
    }
}

impl<X: TextTypes> PartialEq for Pools<X>
where
    X::Leaf: PartialEq,
    X::Cond: PartialEq,
    X::Learn: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.pools == other.pools
    }
}

impl<X: TextTypes> fmt::Debug for Entry<X>
where
    X::Leaf: fmt::Debug,
    X::Cond: fmt::Debug,
    X::Learn: fmt::Debug,
    X::Actor: fmt::Debug,
    X::Topic: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::Leaf(l) => f.debug_tuple("Leaf").field(l).finish(),
            Entry::Pool(p) => f.debug_tuple("Pool").field(p).finish(),
            Entry::Named(id) => f.debug_tuple("Named").field(id).finish(),
        }
    }
}

impl<X: TextTypes> fmt::Debug for Candidate<X>
where
    X::Leaf: fmt::Debug,
    X::Cond: fmt::Debug,
    X::Learn: fmt::Debug,
    X::Actor: fmt::Debug,
    X::Topic: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Candidate")
            .field("key", &self.key)
            .field("speaker", &self.speaker)
            .field("topic", &self.topic)
            .field("when", &self.when)
            .field("mode", &self.mode)
            .field("variants", &self.variants)
            .field("learn", &self.learn)
            .finish()
    }
}

impl<X: TextTypes> fmt::Debug for Pool<X>
where
    X::Leaf: fmt::Debug,
    X::Cond: fmt::Debug,
    X::Learn: fmt::Debug,
    X::Actor: fmt::Debug,
    X::Topic: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pool")
            .field("id", &self.id)
            .field("pick", &self.pick)
            .field("candidates", &self.candidates)
            .finish()
    }
}

impl<X: TextTypes> fmt::Debug for Pools<X>
where
    X::Leaf: fmt::Debug,
    X::Cond: fmt::Debug,
    X::Learn: fmt::Debug,
    X::Actor: fmt::Debug,
    X::Topic: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.pools.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntityId, FactId, FactScope};
    use assert2::check;

    fn fact(name: &str) -> Condition {
        Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new(name),
        }
    }

    #[test]
    fn or_scores_smaller_side() {
        // a or (b and c)
        let c = Condition::Any(vec![fact("a"), Condition::All(vec![fact("b"), fact("c")])]);
        check!(c.specificity() == 1);
    }

    #[test]
    fn specificity_rules() {
        check!(Condition::Always.specificity() == 0);
        check!(Condition::All(vec![]).specificity() == 0);
        check!(Condition::Any(vec![]).specificity() == 0);
        check!(Condition::negate(fact("a")).specificity() == 1);
        let seen = Condition::SeenBy(CharacterId::new(EntityId::new(1)));
        check!(Condition::All(vec![fact("a"), fact("b"), seen]).specificity() == 3);
    }

    #[test]
    fn score_counts_tuple_keys() {
        let c: Candidate<DialogTypes> = Candidate::leaf(DialogText::inline("x"))
            .speaker(CharacterId::new(EntityId::new(1)))
            .topic("stove".into())
            .when(fact("hot"));
        check!(c.score() == 3);
    }

    #[test]
    fn pool_ids() {
        type P = Pool<crate::DialogTypes>;
        assert!(P::is_valid_id("keeper_replies"));
        assert!(P::is_valid_id("room:shore"));
        assert!(!P::is_valid_id(""));
        assert!(!P::is_valid_id("a/x"));
    }

    #[test]
    fn bad_keys() {
        type X = crate::DialogTypes;
        let leaf = || Candidate::<X>::leaf(DialogText::inline("a"));
        let pool = |keys: &[&str]| {
            keys.iter().fold(Pool::<X>::new("p", Pick::All), |p, k| {
                p.with_candidate(leaf().keyed(*k))
            })
        };
        check!(pool(&["a", "b1", "01x"]).bad_key().is_none());
        check!(pool(&["0"]).bad_key() == Some("0"));
        check!(pool(&[""]).bad_key() == Some(""));
        check!(pool(&["a/0"]).bad_key() == Some("a/0"));
        check!(pool(&["a", "a"]).bad_key() == Some("a"));
        let nested = Pool::<X>::new("p", Pick::All)
            .with_candidate(Candidate::pool(pool(&["x", "x"])).keyed("a"));
        check!(nested.bad_key() == Some("x"));
        // Sibling inline pools of one candidate share its progress path.
        let siblings = Pool::<X>::new("p", Pick::All).with_candidate(
            Candidate::new(vec![Entry::Pool(pool(&["x"])), Entry::Pool(pool(&["x"]))]).keyed("a"),
        );
        check!(siblings.bad_key() == Some("x"));
    }

    #[test]
    fn pools_by_id() {
        let mut pools: Pools<DialogTypes> = Pools::new();
        check!(pools.insert(Pool::new("a", Pick::First)).is_none());
        check!(pools.insert(Pool::new("a", Pick::Best)).is_some());
        check!(pools.len() == 1);
        check!(pools.get("a").map(|p| p.pick) == Some(Pick::Best));
        check!(Pick::default() == Pick::First);
    }
}
