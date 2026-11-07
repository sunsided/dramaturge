//! The text selection core: [`select`], [`select_named`], [`commit`] and
//! the [`Trace`].
//!
//! [`select`] chooses what a [`Pool`] shows for a [`Query`] (speaker, topic
//! and perceiver), without a change to the progress. [`commit`] records
//! what showed and hands each `learn` item to the host. Thus a preview (an
//! editor, an explorer) calls `select` only, with a copy of the host RNG,
//! and nothing changes.
//!
//! The host gives the core three things:
//!
//! - a [`Judge`]: the condition evaluation, with the host state,
//! - a [`ProgressStore`]: where the progress lives ([`TextProgress`]
//!   implements it, and so can a host store),
//! - a [`TextRng`]: the randomness for `Shuffle` and `Sticky`.
//!
//! # Progress keys and counts
//!
//! The progress key of a candidate is `{pool id}/{candidate key or
//! index}`. An inline pool in a variant ([`Entry::Pool`]) appends its
//! candidates to the key of the candidate that holds it. A named pool
//! ([`Entry::Named`]) starts again at its own id, so it has one progress
//! for all callers.
//!
//! `FirstTime` asks for the first resolution of a *count*:
//!
//! - The root pool of [`select`] is an inline pool. It counts as
//!   [`Count::Inline`], and its owner is its pool id. The count is per
//!   owner, not per caller: an inline pool has one place.
//! - The root pool of [`select_named`] and each [`Entry::Named`] pool count
//!   as [`Count::Named`], per pool and caller. The caller is the scope of
//!   "the first time" that the host chooses (loremaster: the room).
//! - An inline pool inside another pool has no count of its own. It uses
//!   the count of the nearest root or named pool around it.
//!
//! The kind of the count comes from the place of the pool (a root of
//! `select`, a root of `select_named`, an `Entry::Pool` or an
//! `Entry::Named`), not from a flag on the pool.
//!
//! # Example
//!
//! ```rust
//! use dramaturge_model::{
//!     Candidate, CharacterId, Condition, DialogText, DialogTypes, EntityId, Fact, FactId,
//!     FactScope, FragmentMode, Pick, Pool, Pools,
//! };
//! use dramaturge_runtime::text::{DefaultJudge, Query, commit, select, world_learner};
//! use dramaturge_runtime::{DefaultConditionEvaluator, DialogWorldState, SplitMix64, TextProgress};
//!
//! let player = CharacterId::new(EntityId::new(0));
//! let hot = Condition::FactExists { scope: FactScope::World, fact_id: FactId::new("stove.hot") };
//! let stove: Pool<DialogTypes> = Pool::new("stove", Pick::Best)
//!     .with_candidate(Candidate::leaf(DialogText::inline("The stove is cold.")))
//!     .with_candidate(Candidate::leaf(DialogText::inline("Too hot to touch.")).when(hot.clone()))
//!     .with_candidate(
//!         Candidate::leaf(DialogText::inline("The ash is fresh."))
//!             .when(Condition::All(vec![hot, Condition::SeenBy(player)]))
//!             .learn(Fact::bool("ash_is_fresh", true)),
//!     );
//!
//! let mut state = DialogWorldState::new();
//! state.set_fact(FactScope::World, Fact::bool("stove.hot", true));
//! let evaluator = DefaultConditionEvaluator::new();
//! let mut progress = TextProgress::new();
//! let mut rng = SplitMix64::new(1);
//! let query = Query::new().perceiver(player);
//!
//! let pools = Pools::new(); // no named pools
//! let judge = DefaultJudge::new(&evaluator, &state);
//! let selection = select(&stove, &pools, "kitchen", &query, &judge, &progress, &mut rng);
//! let shown: Vec<_> = selection.leaves().filter_map(DialogText::as_inline).collect();
//! assert_eq!(shown, ["The ash is fresh."]);
//!
//! // `select` changed nothing. `commit` records the progress and learns.
//! commit(&selection, &query, &mut progress, &mut world_learner(&mut state));
//! assert!(state.get_fact(&FactScope::World, &FactId::new("ash_is_fresh")).is_some());
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use dramaturge_model::{
    Candidate, CharacterId, Condition, DialogTypes, Entry, Fact, FactScope, FragmentMode, Pick,
    Pool, Pools, TextTypes,
};

use super::{FragmentProgress, TextProgress, TextRng};
use crate::evaluation::ConditionEvaluator;
use crate::state::NarrativeState;

// ------------------------------------------------------------------ query

/// What a selection is for: the speaker, the topic and the perceiver.
///
/// A candidate with a speaker or topic key matches only a query with the
/// same value. A candidate without a key matches every query (`*`). The
/// perceiver is the actor who sees or hears the text; a [`Judge`] answers
/// "seen by" tests from it.
#[non_exhaustive]
pub struct Query<X: TextTypes> {
    /// The speaker (ask and show: the character asked).
    pub speaker: Option<X::Actor>,
    /// The topic or thing.
    pub topic: Option<X::Topic>,
    /// The actor who sees or hears the text.
    pub perceiver: Option<X::Actor>,
}

impl<X: TextTypes> Query<X> {
    /// Creates a query with no speaker, topic or perceiver.
    pub fn new() -> Self {
        Self {
            speaker: None,
            topic: None,
            perceiver: None,
        }
    }

    /// Sets the speaker.
    pub fn speaker(mut self, speaker: X::Actor) -> Self {
        self.speaker = Some(speaker);
        self
    }

    /// Sets the topic.
    pub fn topic(mut self, topic: X::Topic) -> Self {
        self.topic = Some(topic);
        self
    }

    /// Sets the perceiver.
    pub fn perceiver(mut self, perceiver: X::Actor) -> Self {
        self.perceiver = Some(perceiver);
        self
    }

    fn matches(&self, candidate: &Candidate<X>) -> bool {
        key_matches(&candidate.speaker, &self.speaker) && key_matches(&candidate.topic, &self.topic)
    }
}

/// A candidate key matches when it is not set (`*`) or equals the query.
fn key_matches<K: Eq>(want: &Option<K>, have: &Option<K>) -> bool {
    match want {
        None => true,
        Some(w) => have.as_ref() == Some(w),
    }
}

impl<X: TextTypes> Default for Query<X> {
    fn default() -> Self {
        Self::new()
    }
}

impl<X: TextTypes> Clone for Query<X> {
    fn clone(&self) -> Self {
        Self {
            speaker: self.speaker.clone(),
            topic: self.topic.clone(),
            perceiver: self.perceiver.clone(),
        }
    }
}

impl<X: TextTypes> fmt::Debug for Query<X>
where
    X::Actor: fmt::Debug,
    X::Topic: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Query")
            .field("speaker", &self.speaker)
            .field("topic", &self.topic)
            .field("perceiver", &self.perceiver)
            .finish()
    }
}

// ------------------------------------------------------------------ judge

/// The host's condition evaluation. It borrows the host state.
///
/// A closure `Fn(&X::Cond, &Query<X>) -> bool` is a judge.
pub trait Judge<X: TextTypes> {
    /// Returns `true` when `cond` holds for `query`.
    fn holds(&self, cond: &X::Cond, query: &Query<X>) -> bool;
}

impl<X: TextTypes, F: Fn(&X::Cond, &Query<X>) -> bool> Judge<X> for F {
    fn holds(&self, cond: &X::Cond, query: &Query<X>) -> bool {
        self(cond, query)
    }
}

/// The [`Judge`] of plain dramaturge ([`DialogTypes`]).
///
/// It walks `All`, `Any` and `Not` itself, answers [`Condition::SeenBy`]
/// from [`Query::perceiver`], and gives every other test to the
/// [`ConditionEvaluator`] with the state.
pub struct DefaultJudge<'a, E: ?Sized, S: ?Sized> {
    /// The evaluator for every test except `SeenBy`.
    pub evaluator: &'a E,
    /// The state.
    pub state: &'a S,
}

impl<'a, E: ?Sized, S: ?Sized> DefaultJudge<'a, E, S> {
    /// Creates a judge.
    pub fn new(evaluator: &'a E, state: &'a S) -> Self {
        Self { evaluator, state }
    }
}

impl<T, E, S> Judge<DialogTypes<T>> for DefaultJudge<'_, E, S>
where
    E: ConditionEvaluator<S> + ?Sized,
    S: NarrativeState + ?Sized,
{
    fn holds(&self, cond: &Condition, query: &Query<DialogTypes<T>>) -> bool {
        match cond {
            Condition::All(conds) => conds.iter().all(|c| self.holds(c, query)),
            Condition::Any(conds) => conds.iter().any(|c| self.holds(c, query)),
            Condition::Not(inner) => !self.holds(inner, query),
            Condition::SeenBy(who) => query.perceiver.as_ref() == Some(who),
            other => self.evaluator.evaluate(other, self.state),
        }
    }
}

/// Returns a learner for [`commit`] that sets each learned [`Fact`] in the
/// world scope of `state`.
///
/// A scope per perceiver (`FactScope::Character`) waits for actors with
/// their own knowledge. The learner ignores the perceiver.
pub fn world_learner<S: NarrativeState + ?Sized>(
    state: &mut S,
) -> impl FnMut(&Fact, Option<&CharacterId>) + '_ {
    move |fact, _perceiver| state.set_fact(FactScope::World, fact.clone())
}

// --------------------------------------------------------------- progress

/// What a resolution counts against. See the [module](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Count<'a> {
    /// An inline pool at the root of [`select`]. The owner is its pool id.
    Inline {
        /// The owner (the pool id).
        owner: &'a str,
    },
    /// A named pool, per caller. The caller is a scope that the host
    /// chooses.
    Named {
        /// The pool id.
        pool: &'a str,
        /// The caller.
        caller: &'a str,
    },
}

/// The owned form of [`Count`]. The caller is [`Selection::caller`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CountKey {
    /// An inline pool, by owner.
    Inline(Arc<str>),
    /// A named pool, by id.
    Named(Arc<str>),
}

impl CountKey {
    /// Returns the [`Count`] of this key for `caller`.
    pub fn count<'a>(&'a self, caller: &'a str) -> Count<'a> {
        match self {
            CountKey::Inline(owner) => Count::Inline { owner },
            CountKey::Named(pool) => Count::Named { pool, caller },
        }
    }
}

/// Where the progress lives. [`TextProgress`] implements it; so can a host
/// store.
pub trait ProgressStore {
    /// Returns the progress of a candidate. `None`: it has not shown yet.
    fn fragment_progress(&self, key: &str) -> Option<FragmentProgress>;

    /// Sets the progress of a candidate.
    fn set_fragment_progress(&mut self, key: &str, progress: FragmentProgress);

    /// Returns `true` on the first resolution under `count`. A host can
    /// answer from its own data (loremaster: the first visit of the room).
    fn first_time(&self, count: Count<'_>) -> bool;

    /// Counts one resolution under `count`.
    fn count_resolution(&mut self, count: Count<'_>);
}

impl ProgressStore for TextProgress {
    fn fragment_progress(&self, key: &str) -> Option<FragmentProgress> {
        self.fragment(key)
    }

    fn set_fragment_progress(&mut self, key: &str, progress: FragmentProgress) {
        self.set_fragment(key, progress);
    }

    fn first_time(&self, count: Count<'_>) -> bool {
        match count {
            Count::Inline { owner } => self.resolutions(owner) == 0,
            Count::Named { pool, caller } => self.named_resolutions(pool, caller) == 0,
        }
    }

    fn count_resolution(&mut self, count: Count<'_>) {
        match count {
            Count::Inline { owner } => {
                let n = self.resolutions(owner).saturating_add(1);
                self.set_resolutions(owner, n);
            }
            Count::Named { pool, caller } => {
                let n = self.named_resolutions(pool, caller).saturating_add(1);
                self.set_named_resolutions(pool, caller, n);
            }
        }
    }
}

// ------------------------------------------------------------------ trace

/// Why a candidate showed or did not show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Verdict {
    /// The candidate won the pick (`First`, `Best`).
    Won,
    /// The candidate showed in a `Pick::All` pool.
    Shown,
    /// The candidate was eligible, but another candidate won.
    Lost,
    /// A speaker or topic key does not match the query.
    TupleMismatch,
    /// The condition does not hold.
    ConditionFalse,
    /// The mode does not allow it now (`Once` shown, `FirstTime` not the
    /// first resolution).
    Exhausted,
    /// The candidate has no variants.
    Empty,
    /// The candidate won, but its variant names a pool that is not in the
    /// [`Pools`].
    MissingPool,
    /// The candidate won, but its variant names a named pool that is
    /// already open in this selection (a loop of named pools).
    Recursive,
}

impl Verdict {
    /// Returns `true` for [`Won`](Self::Won) and [`Shown`](Self::Shown).
    pub fn showed(self) -> bool {
        matches!(self, Verdict::Won | Verdict::Shown)
    }

    fn label(self) -> &'static str {
        match self {
            Verdict::Won => "won",
            Verdict::Shown => "shown",
            Verdict::Lost => "lost",
            Verdict::TupleMismatch => "tuple mismatch",
            Verdict::ConditionFalse => "condition false",
            Verdict::Exhausted => "exhausted",
            Verdict::Empty => "empty",
            Verdict::MissingPool => "missing pool",
            Verdict::Recursive => "recursive pool",
        }
    }
}

/// Why each candidate of a pool won or lost.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Trace {
    /// The pool id.
    pub pool: Arc<str>,
    /// The pick rule.
    pub pick: Pick,
    /// `true` when the pool is not in the [`Pools`]. Then `rows` is empty.
    pub missing: bool,
    /// One row for each candidate, in file order.
    pub rows: Vec<TraceRow>,
}

/// One candidate in a [`Trace`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TraceRow {
    /// The index of the candidate in its pool.
    pub index: usize,
    /// The key of the candidate.
    pub key: Option<Arc<str>>,
    /// The progress key of the candidate.
    pub progress_key: Arc<str>,
    /// The score (see [`Candidate::score`]).
    pub score: u32,
    /// Why the candidate showed or did not show.
    pub verdict: Verdict,
    /// For a candidate that showed: the chosen variant and the number of
    /// variants.
    pub variant: Option<(u32, u32)>,
    /// The trace of the pool in the chosen variant, if it is a pool.
    pub nested: Option<Box<Trace>>,
}

impl Trace {
    fn not_found(pool: &str) -> Self {
        Self {
            pool: pool.into(),
            pick: Pick::default(),
            missing: true,
            rows: Vec::new(),
        }
    }

    /// Returns the row of the candidate that won or showed first.
    pub fn winner(&self) -> Option<&TraceRow> {
        self.rows.iter().find(|r| r.verdict.showed())
    }

    fn write(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let pick = match self.pick {
            Pick::All => "all",
            Pick::First => "first",
            Pick::Best => "best",
            _ => "?",
        };
        if self.missing {
            return writeln!(f, "{:indent$}{}  (missing)", "", self.pool);
        }
        writeln!(f, "{:indent$}{}  (pick: {pick})", "", self.pool)?;
        for row in &self.rows {
            let mark = if row.verdict.showed() { '*' } else { '-' };
            let name = match &row.key {
                Some(k) => k.to_string(),
                None => format!("#{}", row.index),
            };
            write!(
                f,
                "{:indent$}  {mark}  {name}  score {}  {}",
                "",
                row.score,
                row.verdict.label()
            )?;
            if let Some((chosen, count)) = row.variant
                && count > 1
            {
                write!(f, ", variant {} of {count}", chosen + 1)?;
            }
            writeln!(f)?;
            if let Some(nested) = &row.nested {
                nested.write(f, indent + 4)?;
            }
        }
        Ok(())
    }
}

/// Writes the trace as the editor shows it:
///
/// ```text
/// keeper_remark  (pick: best)
///   -  #0  score 0  lost
///   -  #1  score 1  lost
///   *  #2  score 2  won, variant 1 of 2
/// ```
impl fmt::Display for Trace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f, 0)
    }
}

// -------------------------------------------------------------- selection

/// One candidate that shows, in the order of the walk (a candidate before
/// the candidates of its nested pools).
#[non_exhaustive]
pub struct Step<'p, X: TextTypes> {
    /// The progress key of the candidate.
    pub progress_key: Arc<str>,
    /// The candidate.
    pub candidate: &'p Candidate<X>,
    /// The chosen variant.
    pub variant: u32,
    /// The progress of the candidate after [`commit`].
    pub progress: FragmentProgress,
    /// The leaves in [`Selection::shown`] that come from this candidate,
    /// its nested pools included.
    pub leaves: Range<usize>,
}

impl<'p, X: TextTypes> Step<'p, X> {
    /// Returns what the perceiver learns from this candidate.
    pub fn learn(&self) -> &'p [X::Learn] {
        &self.candidate.learn
    }
}

/// One leaf that shows.
#[non_exhaustive]
pub struct Shown<'p, X: TextTypes> {
    /// The progress key of the candidate that holds the leaf.
    pub progress_key: Arc<str>,
    /// The index of that candidate in [`Selection::steps`].
    pub step: usize,
    /// The leaf.
    pub leaf: &'p X::Leaf,
}

/// The result of [`select`]: what shows, what [`commit`] records, and why.
#[non_exhaustive]
pub struct Selection<'p, X: TextTypes> {
    /// The caller: the scope of `FirstTime` for named pools.
    pub caller: Arc<str>,
    /// What `commit` counts, once each: the inline root pool, and each
    /// named pool that contributed at least one shown leaf.
    pub counts: Vec<CountKey>,
    /// The candidates that show, in walk order.
    pub steps: Vec<Step<'p, X>>,
    /// The leaves that show, in order. Nested pools are flattened.
    pub shown: Vec<Shown<'p, X>>,
    /// Why each candidate won or lost.
    pub trace: Trace,
}

impl<'p, X: TextTypes> Selection<'p, X> {
    /// Iterates over the leaves that show, in order.
    pub fn leaves(&self) -> impl Iterator<Item = &'p X::Leaf> + '_ {
        self.shown.iter().map(|s| s.leaf)
    }

    /// Returns `true` when no leaf shows.
    pub fn is_empty(&self) -> bool {
        self.shown.is_empty()
    }
}

impl<X: TextTypes> Clone for Step<'_, X> {
    fn clone(&self) -> Self {
        Self {
            progress_key: self.progress_key.clone(),
            candidate: self.candidate,
            variant: self.variant,
            progress: self.progress,
            leaves: self.leaves.clone(),
        }
    }
}

impl<X: TextTypes> Clone for Shown<'_, X> {
    fn clone(&self) -> Self {
        Self {
            progress_key: self.progress_key.clone(),
            step: self.step,
            leaf: self.leaf,
        }
    }
}

impl<X: TextTypes> Clone for Selection<'_, X> {
    fn clone(&self) -> Self {
        Self {
            caller: self.caller.clone(),
            counts: self.counts.clone(),
            steps: self.steps.clone(),
            shown: self.shown.clone(),
            trace: self.trace.clone(),
        }
    }
}

impl<X: TextTypes> fmt::Debug for Step<'_, X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Step")
            .field("progress_key", &self.progress_key)
            .field("variant", &self.variant)
            .field("progress", &self.progress)
            .field("leaves", &self.leaves)
            .finish_non_exhaustive()
    }
}

impl<X: TextTypes> fmt::Debug for Shown<'_, X>
where
    X::Leaf: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shown")
            .field("progress_key", &self.progress_key)
            .field("step", &self.step)
            .field("leaf", self.leaf)
            .finish()
    }
}

impl<X: TextTypes> fmt::Debug for Selection<'_, X>
where
    X::Leaf: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Selection")
            .field("caller", &self.caller)
            .field("counts", &self.counts)
            .field("steps", &self.steps)
            .field("shown", &self.shown)
            .field("trace", &self.trace)
            .finish()
    }
}

// ----------------------------------------------------------------- select

/// Chooses what an inline pool shows, without a change to the progress.
///
/// The root pool counts as [`Count::Inline`] with its pool id as owner
/// (see the [module](self)). `pools` holds the named pools that an
/// [`Entry::Named`] variant can name. `caller` is the scope of `FirstTime`
/// for those named pools; [`Selection::caller`] keeps it for [`commit`].
///
/// `Shuffle` and `Sticky` draw from `rng`, and only for candidates that
/// show. The same pool, query, state, progress and RNG state give the same
/// selection and the same trace. A preview passes a copy of the host RNG.
pub fn select<'p, X, J, P, R>(
    pool: &'p Pool<X>,
    pools: &'p Pools<X>,
    caller: impl Into<Arc<str>>,
    query: &Query<X>,
    judge: &J,
    progress: &P,
    rng: &mut R,
) -> Selection<'p, X>
where
    X: TextTypes,
    J: Judge<X> + ?Sized,
    P: ProgressStore + ?Sized,
    R: TextRng + ?Sized,
{
    let caller = caller.into();
    let first = progress.first_time(Count::Inline { owner: &pool.id });
    let mut walk = Walk::new(pools, &caller, query, judge, progress, rng);
    let trace = walk.pool(pool, &pool.id, first);
    let mut counts = vec![CountKey::Inline(pool.id.clone())];
    counts.append(&mut walk.counts);
    let (steps, shown) = (walk.steps, walk.shown);
    Selection {
        caller,
        counts,
        steps,
        shown,
        trace,
    }
}

/// Chooses what the named pool `id` in `pools` shows, without a change to
/// the progress.
///
/// The pool counts as [`Count::Named`] for `caller`, as an
/// [`Entry::Named`] variant does. An unknown id gives an empty selection
/// whose trace is [`missing`](Trace::missing). Otherwise the same rules as
/// for [`select`] apply.
pub fn select_named<'p, X, J, P, R>(
    id: &str,
    pools: &'p Pools<X>,
    caller: impl Into<Arc<str>>,
    query: &Query<X>,
    judge: &J,
    progress: &P,
    rng: &mut R,
) -> Selection<'p, X>
where
    X: TextTypes,
    J: Judge<X> + ?Sized,
    P: ProgressStore + ?Sized,
    R: TextRng + ?Sized,
{
    let caller = caller.into();
    let mut walk = Walk::new(pools, &caller, query, judge, progress, rng);
    let trace = match walk.named(id) {
        Ok(trace) => trace,
        Err(_) => Trace::not_found(id),
    };
    let (counts, steps, shown) = (walk.counts, walk.steps, walk.shown);
    Selection {
        caller,
        counts,
        steps,
        shown,
        trace,
    }
}

/// Records what `selection` showed, then hands each `learn` item to
/// `learner` with the perceiver of `query`.
///
/// For each [`Step`], it sets the progress (shown count, last or sticky
/// variant). For each entry of [`Selection::counts`], it counts one
/// resolution for [`Selection::caller`]. Then it calls `learner` for the
/// `learn` items of the steps, in order.
pub fn commit<X, P, L>(
    selection: &Selection<'_, X>,
    query: &Query<X>,
    progress: &mut P,
    learner: &mut L,
) where
    X: TextTypes,
    P: ProgressStore + ?Sized,
    L: FnMut(&X::Learn, Option<&X::Actor>) + ?Sized,
{
    for step in &selection.steps {
        progress.set_fragment_progress(&step.progress_key, step.progress);
    }
    for count in &selection.counts {
        progress.count_resolution(count.count(&selection.caller));
    }
    for step in &selection.steps {
        for item in step.learn() {
            learner(item, query.perceiver.as_ref());
        }
    }
}

struct Walk<'p, 'r, X: TextTypes, J: ?Sized, P: ?Sized, R: ?Sized> {
    pools: &'p Pools<X>,
    caller: &'r str,
    query: &'r Query<X>,
    judge: &'r J,
    progress: &'r P,
    rng: &'r mut R,
    /// Progress written in this walk, so that a candidate shown twice (a
    /// named pool named twice) steps twice.
    overlay: BTreeMap<Arc<str>, FragmentProgress>,
    /// The named pools that are open, to stop a loop.
    open: Vec<Arc<str>>,
    counts: Vec<CountKey>,
    steps: Vec<Step<'p, X>>,
    shown: Vec<Shown<'p, X>>,
}

impl<'p, 'r, X, J, P, R> Walk<'p, 'r, X, J, P, R>
where
    X: TextTypes,
    J: Judge<X> + ?Sized,
    P: ProgressStore + ?Sized,
    R: TextRng + ?Sized,
{
    fn new(
        pools: &'p Pools<X>,
        caller: &'r str,
        query: &'r Query<X>,
        judge: &'r J,
        progress: &'r P,
        rng: &'r mut R,
    ) -> Self {
        Self {
            pools,
            caller,
            query,
            judge,
            progress,
            rng,
            overlay: BTreeMap::new(),
            open: Vec::new(),
            counts: Vec::new(),
            steps: Vec::new(),
            shown: Vec::new(),
        }
    }

    fn progress_of(&self, key: &str) -> FragmentProgress {
        self.overlay
            .get(key)
            .copied()
            .or_else(|| self.progress.fragment_progress(key))
            .unwrap_or_default()
    }

    /// Resolves the named pool `id`. Counts it when it showed a leaf.
    fn named(&mut self, id: &str) -> Result<Trace, Verdict> {
        if self.open.iter().any(|open| &**open == id) {
            return Err(Verdict::Recursive);
        }
        let pools = self.pools;
        let Some(pool) = pools.get(id) else {
            return Err(Verdict::MissingPool);
        };
        let first = self.progress.first_time(Count::Named {
            pool: id,
            caller: self.caller,
        });
        let before = self.shown.len();
        self.open.push(pool.id.clone());
        let trace = self.pool(pool, &pool.id, first);
        self.open.pop();
        let key = CountKey::Named(pool.id.clone());
        if self.shown.len() > before && !self.counts.contains(&key) {
            self.counts.push(key);
        }
        Ok(trace)
    }

    fn pool(&mut self, pool: &'p Pool<X>, path: &str, first: bool) -> Trace {
        let mut rows = Vec::with_capacity(pool.candidates.len());
        let mut eligible = Vec::new();
        for (index, c) in pool.candidates.iter().enumerate() {
            let progress_key: Arc<str> = match &c.key {
                Some(k) => format!("{path}/{k}").into(),
                None => format!("{path}/{index}").into(),
            };
            let p = self.progress_of(&progress_key);
            let verdict = if !self.query.matches(c) {
                Verdict::TupleMismatch
            } else if !mode_allows(c.mode, p, first) {
                Verdict::Exhausted
            } else if c.variants.is_empty() {
                Verdict::Empty
            } else if let Some(when) = &c.when
                && !self.judge.holds(when, self.query)
            {
                Verdict::ConditionFalse
            } else {
                eligible.push(index);
                Verdict::Lost
            };
            rows.push(TraceRow {
                index,
                key: c.key.clone(),
                progress_key,
                score: c.score(),
                verdict,
                variant: None,
                nested: None,
            });
        }

        let chosen: Vec<usize> = match pool.pick {
            Pick::All => eligible,
            // The first of the highest score: file order breaks a tie.
            Pick::Best => eligible
                .into_iter()
                .reduce(|a, b| if rows[b].score > rows[a].score { b } else { a })
                .into_iter()
                .collect(),
            // `First`, and pick rules this core does not know.
            _ => eligible.into_iter().take(1).collect(),
        };
        let shown_verdict = if pool.pick == Pick::All {
            Verdict::Shown
        } else {
            Verdict::Won
        };

        for index in chosen {
            let c = &pool.candidates[index];
            let row = &mut rows[index];
            row.verdict = shown_verdict;
            let key = row.progress_key.clone();
            let mut p = self.progress_of(&key);
            let len = c.variants.len() as u32;
            let variant = variant(c.mode, p, len, self.rng);
            p.shown = p.shown.saturating_add(1);
            if matches!(c.mode, FragmentMode::Shuffle | FragmentMode::Sticky) {
                p.pick = Some(variant);
            }
            self.overlay.insert(key.clone(), p);
            row.variant = Some((variant, len));

            let step = self.steps.len();
            let start = self.shown.len();
            self.steps.push(Step {
                progress_key: key.clone(),
                candidate: c,
                variant,
                progress: p,
                leaves: start..start,
            });
            match &c.variants[variant as usize] {
                Entry::Leaf(leaf) => self.shown.push(Shown {
                    progress_key: key,
                    step,
                    leaf,
                }),
                Entry::Pool(inner) => {
                    let trace = self.pool(inner, &key, first);
                    rows[index].nested = Some(Box::new(trace));
                }
                Entry::Named(id) => match self.named(id) {
                    Ok(trace) => rows[index].nested = Some(Box::new(trace)),
                    Err(verdict) => {
                        rows[index].verdict = verdict;
                        rows[index].nested = Some(Box::new(Trace::not_found(id)));
                    }
                },
                // `Entry` is non-exhaustive; entries this core does not
                // know show nothing.
                _ => {}
            }
            self.steps[step].leaves.end = self.shown.len();
        }

        Trace {
            pool: pool.id.clone(),
            pick: pool.pick,
            missing: false,
            rows,
        }
    }
}

fn mode_allows(mode: FragmentMode, p: FragmentProgress, first: bool) -> bool {
    match mode {
        FragmentMode::FirstTime => first,
        FragmentMode::Once => p.shown == 0,
        _ => true,
    }
}

/// The variant that a mode shows, as `resolve_text` chooses it.
fn variant<R: TextRng + ?Sized>(
    mode: FragmentMode,
    p: FragmentProgress,
    len: u32,
    rng: &mut R,
) -> u32 {
    match mode {
        FragmentMode::Cycle => p.shown % len,
        FragmentMode::Shuffle => match p.pick {
            Some(last) if len > 1 => {
                // Draw from the other len - 1 variants.
                let r = rng.below(len - 1);
                if r >= last { r + 1 } else { r }
            }
            _ => rng.below(len),
        },
        FragmentMode::Sticky => match p.pick {
            Some(chosen) if chosen < len => chosen,
            _ => rng.below(len),
        },
        // `Always`, `FirstTime`, `Once`, and modes this core does not know.
        _ => 0,
    }
}
