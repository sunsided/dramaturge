//! The text selection core, and resolving state-dependent [`DialogText`]
//! into the pieces to display.
//!
//! # The core
//!
//! [`select`](fn@select) chooses what a text [`Pool`] shows for a [`Query`]: one
//! candidate (`Pick::First`, `Pick::Best`) or all eligible candidates
//! (`Pick::All`), by speaker, topic, perceiver and condition. It returns a
//! [`Selection`] with a [`Trace`] and changes nothing. [`commit`] records the
//! progress and hands each `learn` item to the host. See the [`select`
//! module](mod@select) for the rules, the progress keys and the counts.
//!
//! # Fragment text
//!
//! [`resolve_text`] walks a [`DialogText::Fragments`] list, evaluates each
//! fragment's condition, picks a variant according to its
//! [`FragmentMode`](dramaturge_model::FragmentMode), and records what it
//! showed in a [`TextProgress`]. The progress belongs in the save state,
//! next to the [`DialogWorldState`](crate::DialogWorldState). It runs on the
//! core: the fragment list is a `Pick::All` pool.
//!
//! Randomness comes from a [`TextRng`]. [`SplitMix64`] is a small seeded
//! generator: the same seed and the same progress always give the same
//! text, so tests and explorers can replay a run.
//!
//! Localization is left to the host: [`ResolvedText::Localized`] carries the
//! key. Host text ([`DialogText::Host`]) is returned as it is, as
//! [`ResolvedText::Host`]: fragment selection works for host text, but its
//! content belongs to the host.
//!
//! ```rust
//! use dramaturge_model::{Condition, DialogText, FactId, FactScope, Fact, FragmentMode, TextFragment};
//! use dramaturge_runtime::{
//!     DefaultConditionEvaluator, DialogWorldState, ResolvedPiece, SplitMix64, TextProgress,
//!     resolve_text,
//! };
//!
//! let text = DialogText::fragments(vec![
//!     TextFragment::always(DialogText::inline("Mara looks up.")),
//!     TextFragment::always(DialogText::inline("She smiles.")).when(Condition::FactExists {
//!         scope: FactScope::World,
//!         fact_id: FactId::new("trusted"),
//!     }),
//!     TextFragment::with_mode(
//!         FragmentMode::Cycle,
//!         vec![DialogText::inline("\"Again?\""), DialogText::inline("\"You.\"")],
//!     ),
//! ]);
//!
//! let mut state = DialogWorldState::new();
//! let evaluator = DefaultConditionEvaluator::new();
//! let mut progress = TextProgress::new();
//! let mut rng = SplitMix64::new(7);
//!
//! let first = resolve_text(&text, "mara.intro", &state, &evaluator, &mut progress, &mut rng);
//! let texts = |pieces: &[ResolvedPiece]| -> Vec<String> {
//!     pieces.iter().filter_map(|p| p.text.as_str()).map(str::to_string).collect()
//! };
//! assert_eq!(texts(&first), ["Mara looks up.", "\"Again?\""]);
//!
//! state.set_fact(FactScope::World, Fact::bool("trusted", true));
//! let second = resolve_text(&text, "mara.intro", &state, &evaluator, &mut progress, &mut rng);
//! assert_eq!(texts(&second), ["Mara looks up.", "She smiles.", "\"You.\""]);
//! ```

use std::collections::{BTreeMap, HashSet};
use std::marker::PhantomData;
use std::sync::Arc;

use dramaturge_model::{
    Candidate, Condition, DialogText, Entry, Pick, Pool, Pools, TextFragment, TextTypes,
};

use crate::evaluation::ConditionEvaluator;
use crate::state::NarrativeState;

pub mod select;

pub use select::{
    Count, CountKey, DefaultJudge, Judge, ProgressStore, Query, Selection, Shown, Step, Trace,
    TraceRow, Verdict, commit, select, select_named, world_learner,
};

/// Source of randomness for `Shuffle` and `Sticky` fragments.
pub trait TextRng {
    /// Returns the next 64 random bits.
    fn next_u64(&mut self) -> u64;

    /// Returns a value in `0..n`. `n` must be non-zero.
    fn below(&mut self, n: u32) -> u32 {
        debug_assert!(n > 0, "TextRng::below(0) has no valid result");
        // Multiply-shift: unbiased enough for picking text variants.
        (((self.next_u64() >> 32) * u64::from(n)) >> 32) as u32
    }
}

/// A small, fast, seedable generator (SplitMix64).
///
/// Deterministic across platforms. Store its state (via [`state`](Self::state))
/// in a save to continue the same sequence after loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Creates a generator from a seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns the internal state, which is also a valid seed.
    pub fn state(&self) -> u64 {
        self.state
    }
}

impl TextRng for SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Progress of one fragment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FragmentProgress {
    /// How many times the fragment contributed text.
    pub shown: u32,
    /// The variant shown last (`Shuffle`) or the chosen variant (`Sticky`).
    pub pick: Option<u32>,
}

/// What [`resolve_text`] and [`commit`] have shown so far. Part of the save
/// state.
///
/// Fragment progress is keyed by `"{owner}/{fragment}"`, where `fragment` is
/// the fragment's [`key`](TextFragment::key) or its index; nested fragment
/// lists append further segments. For a pool, the first segment is the pool
/// id and the next is the candidate key or index.
///
/// Resolutions are counted in two separate maps (see [`Count`]): per owner
/// for inline texts ([`resolutions`](Self::resolutions)), and per pool and
/// caller for named pools ([`named_resolutions`](Self::named_resolutions)).
/// A named pool never reads an owner count, even when its id is the same
/// string as an owner.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TextProgress {
    fragments: BTreeMap<Arc<str>, FragmentProgress>,
    owners: BTreeMap<Arc<str>, u32>,
    /// Resolutions of named pools: pool id, then caller.
    named: BTreeMap<Arc<str>, BTreeMap<Arc<str>, u32>>,
}

impl TextProgress {
    /// Creates empty progress.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the progress of a fragment by its progress key.
    pub fn fragment(&self, key: &str) -> Option<FragmentProgress> {
        self.fragments.get(key).copied()
    }

    /// Sets the progress of a fragment by its progress key.
    pub fn set_fragment(&mut self, key: impl Into<Arc<str>>, progress: FragmentProgress) {
        self.fragments.insert(key.into(), progress);
    }

    /// Iterates over all fragment progress entries in key order.
    pub fn fragments(&self) -> impl Iterator<Item = (&str, FragmentProgress)> {
        self.fragments.iter().map(|(k, p)| (&**k, *p))
    }

    /// Returns how often the text of `owner` was resolved.
    pub fn resolutions(&self, owner: &str) -> u32 {
        self.owners.get(owner).copied().unwrap_or(0)
    }

    /// Sets how often the text of `owner` was resolved.
    pub fn set_resolutions(&mut self, owner: impl Into<Arc<str>>, count: u32) {
        self.owners.insert(owner.into(), count);
    }

    /// Iterates over all owners and their resolution counts in key order.
    pub fn owners(&self) -> impl Iterator<Item = (&str, u32)> {
        self.owners.iter().map(|(k, c)| (&**k, *c))
    }

    /// Returns how often the named pool `pool` was resolved for `caller`.
    pub fn named_resolutions(&self, pool: &str, caller: &str) -> u32 {
        self.named
            .get(pool)
            .and_then(|callers| callers.get(caller))
            .copied()
            .unwrap_or(0)
    }

    /// Sets how often the named pool `pool` was resolved for `caller`.
    pub fn set_named_resolutions(
        &mut self,
        pool: impl Into<Arc<str>>,
        caller: impl Into<Arc<str>>,
        count: u32,
    ) {
        self.named
            .entry(pool.into())
            .or_default()
            .insert(caller.into(), count);
    }

    /// Iterates over all named pools, callers and their resolution counts,
    /// in key order.
    pub fn named(&self) -> impl Iterator<Item = (&str, &str, u32)> {
        self.named.iter().flat_map(|(pool, callers)| {
            callers
                .iter()
                .map(move |(caller, count)| (&**pool, &**caller, *count))
        })
    }

    /// Forgets all progress.
    pub fn clear(&mut self) {
        self.fragments.clear();
        self.owners.clear();
        self.named.clear();
    }
}

/// Resolved text: inline text, a localization key or host text.
///
/// `T` is the host text type (see [`DialogText::Host`]), default `()`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResolvedText<T = ()> {
    /// Text to show as is.
    Inline(String),
    /// A key the host looks up in its localization table.
    Localized(Arc<str>),
    /// Host text, as it is in the graph. The host renders it.
    Host(T),
    /// The id of a named text pool ([`DialogText::Pool`]). The host
    /// selects the pool with [`select_named`].
    Pool(Arc<str>),
}

impl<T> ResolvedText<T> {
    /// Returns the inline text or the localization key, or `None` for host
    /// text and pool references.
    ///
    /// Host text has no string form here. Use [`as_host`](Self::as_host)
    /// for it. The `None` makes a renderer that only knows strings see the
    /// host text, so it does not drop it in silence.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ResolvedText::Inline(s) => Some(s),
            ResolvedText::Localized(k) => Some(k),
            ResolvedText::Host(_) | ResolvedText::Pool(_) => None,
        }
    }

    /// Returns the pool id of a pool reference, or `None`.
    pub fn as_pool(&self) -> Option<&str> {
        match self {
            ResolvedText::Pool(id) => Some(id),
            _ => None,
        }
    }

    /// Returns the host text, or `None` for inline and localized text.
    pub fn as_host(&self) -> Option<&T> {
        match self {
            ResolvedText::Host(text) => Some(text),
            _ => None,
        }
    }
}

/// One piece of resolved text and the fragment that produced it.
///
/// `T` is the host text type (see [`DialogText::Host`]), default `()`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedPiece<T = ()> {
    /// The text.
    pub text: ResolvedText<T>,
    /// Progress key of the fragment that produced it, or `None` for plain
    /// `Inline`/`LocalizedKey` text outside any fragment list.
    pub source: Option<Arc<str>>,
}

/// Resolves `text` against `state` and records progress.
///
/// `owner` identifies the text (typically the node or option key) and
/// prefixes all progress keys. Plain `Inline`, `LocalizedKey` and `Host`
/// text resolves to one piece and records nothing but the resolution of
/// `owner`. Host text is cloned into a [`ResolvedText::Host`] piece;
/// dramaturge does not look into it. A pool reference
/// ([`DialogText::Pool`]) resolves to a [`ResolvedText::Pool`] piece: this
/// function has no pools, so the host selects the pool with
/// [`select_named`].
///
/// Every call clones the host text it selects, as it clones inline text.
/// For large host text (for example compiled markup), use a type that is
/// cheap to clone, such as `Arc<Markup>`.
///
/// Rules for a fragment list, in order:
///
/// - A fragment whose condition does not hold is skipped and its progress
///   does not change.
/// - `Always` shows its first variant; `FirstTime` only on the owner's first
///   resolution; `Once` the first time it is eligible.
/// - `Cycle` shows variant `shown % len`; `Shuffle` a random variant other
///   than the previous one; `Sticky` a random variant chosen once.
/// - A shown fragment with `replaces` discards the pieces before it.
/// - Variants may themselves be fragment lists; they are resolved recursively.
///
/// The result is deterministic for a given `progress`, `state` and `rng` state.
///
/// `resolve_text` runs on the [text selection core](mod@select): the fragment
/// list is a [`Pick::All`] pool with the id `owner`, which [`select`](fn@select)
/// resolves and [`commit`] records. `replaces` is the only rule of its own.
pub fn resolve_text<T, S, E, R>(
    text: &DialogText<T>,
    owner: &str,
    state: &S,
    evaluator: &E,
    progress: &mut TextProgress,
    rng: &mut R,
) -> Vec<ResolvedPiece<T>>
where
    T: Clone,
    S: NarrativeState + ?Sized,
    E: ConditionEvaluator<S> + ?Sized,
    R: TextRng + ?Sized,
{
    let DialogText::Fragments(fragments) = text else {
        progress.count_resolution(Count::Inline { owner });
        return piece(text, None).into_iter().collect();
    };
    let mut replacing = HashSet::new();
    let pool = fragment_pool(owner, fragments, owner, &mut replacing);
    let judge = |cond: &&Condition, _: &Query<Fragments<'_, T>>| evaluator.evaluate(cond, state);
    let query = Query::new();
    let pools = Pools::new();
    let selection = select(&pool, &pools, owner, &query, &judge, &*progress, rng);
    commit(&selection, &query, progress, &mut |_, _| {});

    // A shown fragment with `replaces` discards the leaves before it.
    let start = selection
        .steps
        .iter()
        .filter(|step| replacing.contains(&*step.progress_key))
        .map(|step| step.leaves.start)
        .max()
        .unwrap_or(0);
    selection.shown[start..]
        .iter()
        .filter_map(|shown| piece(shown.leaf, Some(&shown.progress_key)))
        .collect()
}

/// The [`TextTypes`] of [`resolve_text`]: borrowed fragment text and
/// conditions.
struct Fragments<'a, T>(PhantomData<&'a T>);

impl<'a, T> TextTypes for Fragments<'a, T> {
    type Leaf = &'a DialogText<T>;
    type Cond = &'a Condition;
    type Learn = ();
    type Actor = ();
    type Topic = ();
}

/// Builds the pool of a fragment list. `path` is the progress key of the
/// list; the progress keys of fragments with `replaces` go into `replacing`.
fn fragment_pool<'a, T>(
    id: &str,
    fragments: &'a [TextFragment<T>],
    path: &str,
    replacing: &mut HashSet<String>,
) -> Pool<Fragments<'a, T>> {
    let mut pool = Pool::new(id, Pick::All);
    for (index, fragment) in fragments.iter().enumerate() {
        let key = match &fragment.key {
            Some(k) => format!("{path}/{k}"),
            None => format!("{path}/{index}"),
        };
        let variants = fragment
            .variants
            .iter()
            .map(|variant| match variant {
                DialogText::Fragments(inner) => {
                    Entry::Pool(fragment_pool(&key, inner, &key, replacing))
                }
                other => Entry::Leaf(other),
            })
            .collect();
        let mut candidate = Candidate::new(variants).mode(fragment.mode);
        candidate.key = fragment.key.clone();
        candidate.when = fragment.condition.as_ref();
        pool.candidates.push(candidate);
        if fragment.replaces {
            replacing.insert(key);
        }
    }
    pool
}

/// The piece of a text leaf.
fn piece<T: Clone>(text: &DialogText<T>, source: Option<&Arc<str>>) -> Option<ResolvedPiece<T>> {
    let text = match text {
        DialogText::Inline(s) => ResolvedText::Inline(s.clone()),
        DialogText::LocalizedKey(k) => ResolvedText::Localized(k.clone()),
        DialogText::Host(t) => ResolvedText::Host(t.clone()),
        DialogText::Pool(id) => ResolvedText::Pool(id.clone()),
        // `DialogText` is non-exhaustive; text kinds this resolver does
        // not know produce no pieces.
        _ => return None,
    };
    Some(ResolvedPiece {
        text,
        source: source.cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DefaultConditionEvaluator, DialogWorldState};
    use assert2::check;
    use dramaturge_model::{Condition, Fact, FactId, FactScope, FragmentMode};

    fn texts(pieces: &[ResolvedPiece]) -> Vec<&str> {
        pieces
            .iter()
            .map(|p| p.text.as_str().expect("plain text"))
            .collect()
    }

    fn run(text: &DialogText, n: usize, seed: u64) -> Vec<Vec<String>> {
        let state = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator::new();
        let mut progress = TextProgress::new();
        let mut rng = SplitMix64::new(seed);
        (0..n)
            .map(|_| {
                resolve_text(text, "owner", &state, &evaluator, &mut progress, &mut rng)
                    .into_iter()
                    .map(|p| p.text.as_str().expect("plain text").to_string())
                    .collect()
            })
            .collect()
    }

    fn inline(s: &str) -> DialogText {
        DialogText::inline(s)
    }

    #[test]
    fn plain_text_resolves_to_one_piece() {
        let out = run(&inline("Hello"), 1, 0);
        check!(out == vec![vec!["Hello".to_string()]]);
    }

    #[test]
    fn first_time_and_once() {
        let text = DialogText::fragments(vec![
            TextFragment::with_mode(FragmentMode::FirstTime, vec![inline("first")]),
            TextFragment::with_mode(FragmentMode::Once, vec![inline("once")]),
            TextFragment::always(inline("always")),
        ]);
        let out = run(&text, 3, 0);
        check!(out[0] == ["first", "once", "always"]);
        check!(out[1] == ["always"]);
        check!(out[2] == ["always"]);
    }

    #[test]
    fn once_waits_for_its_condition() {
        let flag = Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new("flag"),
        };
        let text = DialogText::fragments(vec![
            TextFragment::with_mode(FragmentMode::FirstTime, vec![inline("first")])
                .when(flag.clone()),
            TextFragment::with_mode(FragmentMode::Once, vec![inline("once")]).when(flag),
        ]);
        let mut state = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator::new();
        let mut progress = TextProgress::new();
        let mut rng = SplitMix64::new(0);
        let out = resolve_text(&text, "o", &state, &evaluator, &mut progress, &mut rng);
        check!(out.is_empty());
        state.set_fact(FactScope::World, Fact::bool("flag", true));
        let out = resolve_text(&text, "o", &state, &evaluator, &mut progress, &mut rng);
        // FirstTime missed its chance; Once had not been shown yet.
        check!(texts(&out) == ["once"]);
    }

    #[test]
    fn cycle_wraps() {
        let text = DialogText::fragments(vec![TextFragment::with_mode(
            FragmentMode::Cycle,
            vec![inline("a"), inline("b"), inline("c")],
        )]);
        let out: Vec<String> = run(&text, 4, 0).into_iter().flatten().collect();
        check!(out == ["a", "b", "c", "a"]);
    }

    #[test]
    fn shuffle_never_repeats_immediately_and_is_deterministic() {
        let text = DialogText::fragments(vec![TextFragment::with_mode(
            FragmentMode::Shuffle,
            vec![inline("a"), inline("b"), inline("c")],
        )]);
        let a: Vec<String> = run(&text, 50, 42).into_iter().flatten().collect();
        let b: Vec<String> = run(&text, 50, 42).into_iter().flatten().collect();
        check!(a == b);
        check!(a.windows(2).all(|w| w[0] != w[1]));
        check!(["a", "b", "c"].iter().all(|v| a.iter().any(|x| x == v)));
    }

    #[test]
    fn sticky_keeps_its_pick() {
        let text = DialogText::fragments(vec![TextFragment::with_mode(
            FragmentMode::Sticky,
            vec![inline("a"), inline("b"), inline("c")],
        )]);
        let out: Vec<String> = run(&text, 10, 9).into_iter().flatten().collect();
        check!(out.iter().all(|x| *x == out[0]));
    }

    #[test]
    fn replaces_discards_earlier_pieces() {
        let text = DialogText::fragments(vec![
            TextFragment::always(inline("a")),
            TextFragment::always(inline("b")).replacing(),
            TextFragment::always(inline("c")),
        ]);
        check!(run(&text, 1, 0)[0] == ["b", "c"]);
    }

    #[test]
    fn host_text_resolves_as_it_is() {
        #[derive(Debug, Clone, PartialEq)]
        struct Markup(&'static str);

        let state = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator::new();
        let mut progress = TextProgress::new();
        let mut rng = SplitMix64::new(0);

        let plain = DialogText::host(Markup("*Hi.*"));
        let out = resolve_text(&plain, "n", &state, &evaluator, &mut progress, &mut rng);
        check!(out.len() == 1);
        check!(out[0].text == ResolvedText::Host(Markup("*Hi.*")));
        check!(out[0].text.as_host() == Some(&Markup("*Hi.*")));
        check!(out[0].text.as_str().is_none());
        check!(out[0].source.is_none());

        // Fragment selection works for host text, too.
        let text = DialogText::Fragments(vec![
            TextFragment::always(DialogText::Inline("Mara: ".into())),
            TextFragment::with_mode(
                FragmentMode::Cycle,
                vec![DialogText::host(Markup("a")), DialogText::host(Markup("b"))],
            )
            .keyed("line"),
        ]);
        let first = resolve_text(&text, "n", &state, &evaluator, &mut progress, &mut rng);
        let second = resolve_text(&text, "n", &state, &evaluator, &mut progress, &mut rng);
        check!(first[0].text == ResolvedText::Inline("Mara: ".into()));
        check!(first[1].text.as_host() == Some(&Markup("a")));
        check!(second[1].text.as_host() == Some(&Markup("b")));
        check!(first[1].source.as_deref() == Some("n/line"));
    }

    #[test]
    fn nested_fragments_and_sources() {
        let inner = DialogText::fragments(vec![
            TextFragment::with_mode(FragmentMode::Cycle, vec![inline("x"), inline("y")])
                .keyed("inner"),
        ]);
        let text = DialogText::fragments(vec![TextFragment::always(inner).keyed("outer")]);
        let state = DialogWorldState::new();
        let evaluator = DefaultConditionEvaluator::new();
        let mut progress = TextProgress::new();
        let mut rng = SplitMix64::new(0);
        let a = resolve_text(&text, "n", &state, &evaluator, &mut progress, &mut rng);
        let b = resolve_text(&text, "n", &state, &evaluator, &mut progress, &mut rng);
        check!(texts(&a) == ["x"]);
        check!(texts(&b) == ["y"]);
        check!(a[0].source.as_deref() == Some("n/outer/inner"));
        check!(progress.fragment("n/outer").map(|p| p.shown) == Some(2));
        check!(progress.resolutions("n") == 2);
    }
}
