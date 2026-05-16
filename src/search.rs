//! Generic abductive-search driver.
//!
//! Given an [`AbductiveBackend`] reporting `Unsat` on the bare
//! formula, [`abduce`] enumerates subsets of the backend's abducibles
//! and reports the minimal subsets that flip the verdict to `Sat`.
//!
//! The driver is intentionally simple — it grows subsets in
//! increasing-size order and consults the backend on each. Production
//! systems with rich Z3-style `solve-with-assumptions` cores can
//! bypass this driver and implement smarter strategies directly
//! against the trait; this implementation is a portable baseline.

use crate::backend::{AbductiveBackend, Verdict};
use crate::hypothesis::Hypothesis;
use crate::minimize::{minimize_by_subsumption, MinimizePolicy};

/// A successful abductive search result: a minimal hypothesis set
/// whose adoption makes the formula satisfiable.
#[derive(Clone, Debug)]
pub struct AbductiveSolution<T> {
    /// The chosen hypotheses, in the backend's enumeration order.
    pub hypotheses: Vec<Hypothesis<T>>,
    /// Aggregated `explanation` field from the constituent
    /// hypotheses, joined by `" + "` when more than one carries an
    /// explanation. `None` if none did.
    pub explanation: Option<String>,
}

/// Drive an abductive search.
///
/// `max_size` caps the subset size considered; pass `usize::MAX`
/// to enumerate exhaustively. Beware that the search space is
/// `2^n` in the number of abducibles, so an explicit cap is
/// almost always desirable.
///
/// The result list is minimized by [`minimize_by_subsumption`]
/// before being returned, so any reported solution is Pareto-
/// optimal with respect to subset inclusion.
pub fn abduce<B, F>(
    backend: &mut B,
    max_size: usize,
    eq: F,
) -> Vec<AbductiveSolution<B::Term>>
where
    B: AbductiveBackend,
    F: Fn(&B::Term, &B::Term) -> bool,
{
    let abducibles = backend.abducibles();
    let n = abducibles.len();
    let limit = max_size.min(n);

    let mut hits: Vec<Vec<Hypothesis<B::Term>>> = Vec::new();
    for size in 0..=limit {
        for_each_subset_of_size(n, size, &mut |idxs| {
            let subset: Vec<_> = idxs.iter().map(|i| abducibles[*i].clone()).collect();
            let (verdict, _) = backend.check_with(&subset);
            if verdict == Verdict::Sat {
                hits.push(subset);
            }
        });
        // Early exit if smallest layer already yields hits — but
        // continue one more layer for completeness so Pareto-
        // incomparable larger sets at the boundary aren't missed
        // when they touch different abducibles. Keep it simple:
        // we always scan up to `limit` and minimize at the end.
    }

    let minimized = minimize_by_subsumption(hits, eq, MinimizePolicy::SmallestFirst);
    minimized
        .into_iter()
        .map(|hs| {
            let parts: Vec<&str> = hs
                .iter()
                .filter_map(|h| h.explanation.as_deref())
                .collect();
            let explanation = if parts.is_empty() {
                None
            } else {
                Some(parts.join(" + "))
            };
            AbductiveSolution {
                hypotheses: hs,
                explanation,
            }
        })
        .collect()
}

fn for_each_subset_of_size<F: FnMut(&[usize])>(n: usize, k: usize, f: &mut F) {
    let mut idx = (0..k).collect::<Vec<usize>>();
    if k == 0 {
        f(&idx);
        return;
    }
    if k > n {
        return;
    }
    loop {
        f(&idx);
        // increment in lexicographic order
        let mut i = k - 1;
        loop {
            if idx[i] < n - (k - i) {
                idx[i] += 1;
                for j in i + 1..k {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return;
            }
            i -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Backend whose formula is "the set of selected hypotheses
    /// contains `target`". This lets us test that the driver
    /// finds the minimal singleton `{target}` and not a superset.
    struct ContainsTarget {
        abducibles: Vec<Hypothesis<&'static str>>,
        target: &'static str,
    }

    impl AbductiveBackend for ContainsTarget {
        type Term = &'static str;
        type FullVerdict = Verdict;

        fn check_with(
            &mut self,
            assumptions: &[Hypothesis<Self::Term>],
        ) -> (Verdict, Self::FullVerdict) {
            let v = if assumptions.iter().any(|h| h.pattern == self.target) {
                Verdict::Sat
            } else {
                Verdict::Unsat
            };
            (v, v)
        }

        fn abducibles(&self) -> Vec<Hypothesis<Self::Term>> {
            self.abducibles.clone()
        }
    }

    #[test]
    fn finds_singleton_solution() {
        let mut b = ContainsTarget {
            abducibles: vec![
                Hypothesis::new("p", "src"),
                Hypothesis::new("q", "src"),
                Hypothesis::new("r", "src"),
            ],
            target: "q",
        };
        let solutions = abduce(&mut b, 3, |a, b| a == b);
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].hypotheses.len(), 1);
        assert_eq!(solutions[0].hypotheses[0].pattern, "q");
    }

    #[test]
    fn explanation_threads_through() {
        let mut b = ContainsTarget {
            abducibles: vec![
                Hypothesis::new("p", "src").with_explanation("from L42"),
                Hypothesis::new("q", "src"),
            ],
            target: "p",
        };
        let solutions = abduce(&mut b, 2, |a, b| a == b);
        assert_eq!(solutions[0].explanation.as_deref(), Some("from L42"));
    }

    #[test]
    fn empty_abducible_list_returns_no_solutions() {
        let mut b = ContainsTarget {
            abducibles: vec![],
            target: "p",
        };
        let solutions = abduce(&mut b, 5, |a, b| a == b);
        assert!(solutions.is_empty());
    }

    /// Backend that returns Sat for the empty assumption set,
    /// modelling "formula is already satisfiable" — the driver
    /// reports a single empty solution.
    struct AlwaysSat;
    impl AbductiveBackend for AlwaysSat {
        type Term = &'static str;
        type FullVerdict = Verdict;
        fn check_with(
            &mut self,
            _: &[Hypothesis<Self::Term>],
        ) -> (Verdict, Self::FullVerdict) {
            (Verdict::Sat, Verdict::Sat)
        }
        fn abducibles(&self) -> Vec<Hypothesis<Self::Term>> {
            vec![Hypothesis::new("p", "src")]
        }
    }

    #[test]
    fn empty_set_solution_when_formula_already_sat() {
        let mut b = AlwaysSat;
        let solutions = abduce(&mut b, 2, |a, b| a == b);
        assert_eq!(solutions.len(), 1);
        assert!(solutions[0].hypotheses.is_empty());
    }
}
