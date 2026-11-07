//! Checks on text pools: ties, unkeyed progress and unknown pools.

use dramaturge_model::{Candidate, Entry, FragmentMode, Pick, Pool, Pools, TextTypes};

use crate::{GraphValidator, ValidationIssue, ValidationIssueKind, ValidationSeverity};

impl GraphValidator {
    /// Checks every pool in `pools` and the pools inside them.
    ///
    /// See the [crate documentation](crate#text-pools) for the checks.
    /// Each issue has a [`path`](ValidationIssue::path): the pool id and
    /// the candidate key or index (`keeper_remark/2`), with more segments
    /// for inline pools.
    pub fn validate_pools<X: TextTypes>(&self, pools: &Pools<X>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        for pool in pools.iter() {
            self.check_pool(pool, &pool.id, pools, &mut issues);
        }
        issues.retain(|issue| self.options().reports(issue.kind));
        issues
    }

    /// Checks one pool (for example an inline description pool) and the
    /// pools inside it. `pools` holds the named pools that it can name.
    pub fn validate_pool<X: TextTypes>(
        &self,
        pool: &Pool<X>,
        pools: &Pools<X>,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        self.check_pool(pool, &pool.id, pools, &mut issues);
        issues.retain(|issue| self.options().reports(issue.kind));
        issues
    }

    fn check_pool<X: TextTypes>(
        &self,
        pool: &Pool<X>,
        path: &str,
        pools: &Pools<X>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let name = |index: usize, c: &Candidate<X>| match &c.key {
            Some(k) => format!("{path}/{k}"),
            None => format!("{path}/{index}"),
        };

        if pool.pick == Pick::Best {
            for (i, a) in pool.candidates.iter().enumerate() {
                for (j, b) in pool.candidates.iter().enumerate().skip(i + 1) {
                    if a.score() == b.score() && can_meet(a, b) {
                        issues.push(
                            ValidationIssue::new(
                                ValidationIssueKind::PickTie,
                                ValidationSeverity::Warning,
                                format!(
                                    "Candidates {} and {} of the best pool `{}` have the same \
                                     score {}; when both hold, {} always wins",
                                    name(i, a),
                                    name(j, b),
                                    pool.id,
                                    a.score(),
                                    name(i, a),
                                ),
                            )
                            .with_path(name(j, b)),
                        );
                    }
                }
            }
        }

        for (index, c) in pool.candidates.iter().enumerate() {
            let here = name(index, c);
            let tracked = matches!(
                c.mode,
                FragmentMode::Once
                    | FragmentMode::Cycle
                    | FragmentMode::Shuffle
                    | FragmentMode::Sticky
            );
            if tracked && c.key.is_none() {
                issues.push(
                    ValidationIssue::new(
                        ValidationIssueKind::UnkeyedProgress,
                        ValidationSeverity::Warning,
                        format!(
                            "Candidate {here} has no key; a candidate inserted before it \
                             moves its progress"
                        ),
                    )
                    .with_path(here.clone()),
                );
            }
            for variant in &c.variants {
                match variant {
                    Entry::Pool(inner) => self.check_pool(inner, &here, pools, issues),
                    Entry::Named(id) if pools.get(id).is_none() => issues.push(
                        ValidationIssue::new(
                            ValidationIssueKind::UnknownPool,
                            ValidationSeverity::Error,
                            format!("Candidate {here} names the pool `{id}`, which does not exist"),
                        )
                        .with_path(here.clone()),
                    ),
                    _ => {}
                }
            }
        }
    }
}

/// Returns `true` when one query can match both candidates: each tuple key
/// is unset on one side or equal on both.
fn can_meet<X: TextTypes>(a: &Candidate<X>, b: &Candidate<X>) -> bool {
    fn meet<K: Eq>(a: &Option<K>, b: &Option<K>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        }
    }
    meet(&a.speaker, &b.speaker) && meet(&a.topic, &b.topic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValidationOptions;
    use assert2::check;
    use dramaturge_model::{
        CharacterId, Condition, DialogText, DialogTypes, EntityId, FactId, FactScope,
    };

    type X = DialogTypes;

    fn is(name: &str) -> Condition {
        Condition::FactExists {
            scope: FactScope::World,
            fact_id: FactId::new(name),
        }
    }

    fn leaf(s: &str) -> Candidate<X> {
        Candidate::leaf(DialogText::inline(s))
    }

    fn kinds(issues: &[ValidationIssue]) -> Vec<ValidationIssueKind> {
        issues.iter().map(|i| i.kind).collect()
    }

    #[test]
    fn best_pool_reports_a_tie() {
        let pools = Pools::new().with(
            Pool::new("remark", Pick::Best)
                .with_candidate(leaf("a").when(is("x")))
                .with_candidate(leaf("b").when(is("y")))
                .with_candidate(leaf("c")),
        );
        let issues = GraphValidator::new().validate_pools(&pools);
        check!(kinds(&issues) == [ValidationIssueKind::PickTie]);
        check!(issues[0].path.as_deref() == Some("remark/1"));
        check!(issues[0].severity == ValidationSeverity::Warning);
    }

    #[test]
    fn different_tuple_keys_are_no_tie() {
        let keeper = CharacterId::new(EntityId::new(1));
        let mara = CharacterId::new(EntityId::new(2));
        let pools = Pools::new().with(
            Pool::new("replies", Pick::Best)
                .with_candidate(leaf("a").speaker(keeper))
                .with_candidate(leaf("b").speaker(mara))
                .with_candidate(leaf("c")),
        );
        check!(GraphValidator::new().validate_pools(&pools).is_empty());

        // `first` and `all` pools have no ties.
        let pools = Pools::new().with(
            Pool::new("list", Pick::First)
                .with_candidate(leaf("a"))
                .with_candidate(leaf("b")),
        );
        check!(GraphValidator::new().validate_pools(&pools).is_empty());
    }

    #[test]
    fn unkeyed_progress_and_unknown_pools() {
        let pool: Pool<X> = Pool::new("room", Pick::All)
            .with_candidate(leaf("once").mode(FragmentMode::Once))
            .with_candidate(leaf("keyed").mode(FragmentMode::Shuffle).keyed("k"))
            .with_candidate(leaf("first").mode(FragmentMode::FirstTime))
            .with_candidate(Candidate::pool(
                Pool::new("inner", Pick::First)
                    .with_candidate(leaf("cycle").mode(FragmentMode::Cycle)),
            ))
            .with_candidate(Candidate::named("nowhere"));
        let issues = GraphValidator::new().validate_pool(&pool, &Pools::new());
        check!(
            kinds(&issues)
                == [
                    ValidationIssueKind::UnkeyedProgress,
                    ValidationIssueKind::UnkeyedProgress,
                    ValidationIssueKind::UnknownPool,
                ]
        );
        let paths: Vec<_> = issues.iter().filter_map(|i| i.path.as_deref()).collect();
        check!(paths == ["room/0", "room/3/0", "room/4"]);

        let options = ValidationOptions::new().skip(ValidationIssueKind::UnkeyedProgress);
        let issues = GraphValidator::with_options(options).validate_pool(&pool, &Pools::new());
        check!(kinds(&issues) == [ValidationIssueKind::UnknownPool]);
    }
}
