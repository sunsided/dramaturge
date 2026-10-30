//! Resolving state-dependent [`DialogText`] into the pieces to display.
//!
//! [`resolve_text`] walks a [`DialogText::Fragments`] list, evaluates each
//! fragment's condition, picks a variant according to its
//! [`FragmentMode`], and records what it showed in a [`TextProgress`]. The
//! progress belongs in the save state, next to the
//! [`DialogWorldState`](crate::DialogWorldState).
//!
//! Randomness comes from a [`TextRng`]. [`SplitMix64`] is a small seeded
//! generator: the same seed and the same progress always give the same
//! text, so tests and explorers can replay a run.
//!
//! Localization is left to the host: [`ResolvedText::Localized`] carries the
//! key.
//!
//! ```rust
//! use dramaturge_model::{Condition, DialogText, FactId, FactScope, Fact, FragmentMode, TextFragment};
//! use dramaturge_runtime::{
//!     DefaultConditionEvaluator, DialogWorldState, SplitMix64, TextProgress, resolve_text,
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
//! assert_eq!(first.iter().map(|p| p.text.as_str()).collect::<Vec<_>>(), ["Mara looks up.", "\"Again?\""]);
//!
//! state.set_fact(FactScope::World, Fact::bool("trusted", true));
//! let second = resolve_text(&text, "mara.intro", &state, &evaluator, &mut progress, &mut rng);
//! assert_eq!(second.iter().map(|p| p.text.as_str()).collect::<Vec<_>>(), ["Mara looks up.", "She smiles.", "\"You.\""]);
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;

use dramaturge_model::{DialogText, FragmentMode, TextFragment};

use crate::evaluation::ConditionEvaluator;
use crate::state::NarrativeState;

/// Source of randomness for `Shuffle` and `Sticky` fragments.
pub trait TextRng {
    /// Returns the next 64 random bits.
    fn next_u64(&mut self) -> u64;

    /// Returns a value in `0..n`. `n` must be non-zero.
    fn below(&mut self, n: u32) -> u32 {
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

/// What [`resolve_text`] has shown so far. Part of the save state.
///
/// Fragment progress is keyed by `"{owner}/{fragment}"`, where `fragment` is
/// the fragment's [`key`](TextFragment::key) or its index; nested fragment
/// lists append further segments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TextProgress {
    fragments: BTreeMap<Arc<str>, FragmentProgress>,
    owners: BTreeMap<Arc<str>, u32>,
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

    /// Forgets all progress.
    pub fn clear(&mut self) {
        self.fragments.clear();
        self.owners.clear();
    }
}

/// Resolved text: inline text or a localization key for the host.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResolvedText {
    /// Text to show as is.
    Inline(String),
    /// A key the host looks up in its localization table.
    Localized(Arc<str>),
}

impl ResolvedText {
    /// Returns the inline text, or the localization key.
    pub fn as_str(&self) -> &str {
        match self {
            ResolvedText::Inline(s) => s,
            ResolvedText::Localized(k) => k,
        }
    }
}

/// One piece of resolved text and the fragment that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedPiece {
    /// The text.
    pub text: ResolvedText,
    /// Progress key of the fragment that produced it, or `None` for plain
    /// `Inline`/`LocalizedKey` text outside any fragment list.
    pub source: Option<Arc<str>>,
}

/// Resolves `text` against `state` and records progress.
///
/// `owner` identifies the text (typically the node or option key) and
/// prefixes all progress keys. Plain `Inline` and `LocalizedKey` text
/// resolves to one piece and records nothing.
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
pub fn resolve_text<S, E, R>(
    text: &DialogText,
    owner: &str,
    state: &S,
    evaluator: &E,
    progress: &mut TextProgress,
    rng: &mut R,
) -> Vec<ResolvedPiece>
where
    S: NarrativeState + ?Sized,
    E: ConditionEvaluator<S> + ?Sized,
    R: TextRng + ?Sized,
{
    let first_time = progress.resolutions(owner) == 0;
    let mut out = Vec::new();
    let mut resolver = Resolver {
        state,
        evaluator,
        progress,
        rng,
        first_time,
    };
    resolver.resolve(text, owner, None, &mut out);
    let count = resolver.progress.resolutions(owner).saturating_add(1);
    resolver.progress.set_resolutions(owner, count);
    out
}

struct Resolver<'a, S: ?Sized, E: ?Sized, R: ?Sized> {
    state: &'a S,
    evaluator: &'a E,
    progress: &'a mut TextProgress,
    rng: &'a mut R,
    first_time: bool,
}

impl<S, E, R> Resolver<'_, S, E, R>
where
    S: NarrativeState + ?Sized,
    E: ConditionEvaluator<S> + ?Sized,
    R: TextRng + ?Sized,
{
    fn resolve(
        &mut self,
        text: &DialogText,
        path: &str,
        source: Option<&Arc<str>>,
        out: &mut Vec<ResolvedPiece>,
    ) {
        match text {
            DialogText::Inline(s) => out.push(ResolvedPiece {
                text: ResolvedText::Inline(s.clone()),
                source: source.cloned(),
            }),
            DialogText::LocalizedKey(k) => out.push(ResolvedPiece {
                text: ResolvedText::Localized(k.clone()),
                source: source.cloned(),
            }),
            DialogText::Fragments(fragments) => {
                for (index, fragment) in fragments.iter().enumerate() {
                    self.fragment(fragment, path, index, out);
                }
            }
            // Text kinds added in later versions contribute nothing here.
            _ => {}
        }
    }

    fn fragment(
        &mut self,
        fragment: &TextFragment,
        path: &str,
        index: usize,
        out: &mut Vec<ResolvedPiece>,
    ) {
        let key: Arc<str> = match &fragment.key {
            Some(k) => format!("{path}/{k}").into(),
            None => format!("{path}/{index}").into(),
        };
        if fragment.variants.is_empty() {
            return;
        }
        if let Some(condition) = &fragment.condition
            && !self.evaluator.evaluate(condition, self.state)
        {
            return;
        }

        let mut p = self.progress.fragment(&key).unwrap_or_default();
        let len = fragment.variants.len() as u32;
        let pick = match fragment.mode {
            FragmentMode::Always => Some(0),
            FragmentMode::FirstTime => self.first_time.then_some(0),
            FragmentMode::Once => (p.shown == 0).then_some(0),
            FragmentMode::Cycle => Some(p.shown % len),
            FragmentMode::Shuffle => Some(match p.pick {
                Some(last) if len > 1 => {
                    // Draw from the other len - 1 variants.
                    let r = self.rng.below(len - 1);
                    if r >= last { r + 1 } else { r }
                }
                _ => self.rng.below(len),
            }),
            FragmentMode::Sticky => Some(match p.pick {
                Some(chosen) if chosen < len => chosen,
                _ => self.rng.below(len),
            }),
        };
        let Some(pick) = pick else {
            return;
        };

        p.shown = p.shown.saturating_add(1);
        if matches!(fragment.mode, FragmentMode::Shuffle | FragmentMode::Sticky) {
            p.pick = Some(pick);
        }
        self.progress.set_fragment(key.clone(), p);

        if fragment.replaces {
            out.clear();
        }
        self.resolve(&fragment.variants[pick as usize], &key, Some(&key), out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DefaultConditionEvaluator, DialogWorldState};
    use assert2::check;
    use dramaturge_model::{Condition, Fact, FactId, FactScope};

    fn texts(pieces: &[ResolvedPiece]) -> Vec<&str> {
        pieces.iter().map(|p| p.text.as_str()).collect()
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
                    .map(|p| p.text.as_str().to_string())
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
