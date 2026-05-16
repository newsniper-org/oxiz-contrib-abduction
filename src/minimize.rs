//! Hypothesis-set minimization helpers.
//!
//! Given a list of candidate hypothesis sets that all discharge the
//! goal, [`minimize_by_subsumption`] removes any candidate that is a
//! superset (with respect to the user-supplied equality predicate)
//! of another candidate already in the list. The result is the
//! *minimal* / *Pareto-optimal* set of solutions.
//!
//! [`MinimizePolicy`] controls additional tie-breaking. Default is
//! [`MinimizePolicy::SmallestFirst`] which sorts the surviving
//! candidates by `len()` ascending.

use crate::Hypothesis;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum MinimizePolicy {
    /// Sort surviving candidates by length ascending.
    #[default]
    SmallestFirst,
    /// Leave the input order intact.
    PreserveOrder,
}

/// Drop any candidate set that contains another candidate set as a
/// (proper or equal) subset. Equality of [`Hypothesis<T>`] patterns
/// is decided by the supplied `eq` predicate; this keeps the call
/// site free of `Eq` bounds for arbitrary term types.
pub fn minimize_by_subsumption<T, F>(
    mut candidates: Vec<Vec<Hypothesis<T>>>,
    eq: F,
    policy: MinimizePolicy,
) -> Vec<Vec<Hypothesis<T>>>
where
    T: Clone,
    F: Fn(&T, &T) -> bool,
{
    let mut keep = vec![true; candidates.len()];
    for i in 0..candidates.len() {
        if !keep[i] {
            continue;
        }
        for j in 0..candidates.len() {
            if i == j || !keep[j] {
                continue;
            }
            // Drop j if it is a (non-strict) superset of i.
            if is_superset(&candidates[j], &candidates[i], &eq)
                && !(is_superset(&candidates[i], &candidates[j], &eq)
                    && candidates[i].len() == candidates[j].len()
                    && i < j)
            {
                keep[j] = false;
            }
        }
    }
    let mut out: Vec<Vec<Hypothesis<T>>> = candidates
        .drain(..)
        .zip(keep.iter())
        .filter_map(|(c, k)| if *k { Some(c) } else { None })
        .collect();
    match policy {
        MinimizePolicy::SmallestFirst => out.sort_by_key(|c| c.len()),
        MinimizePolicy::PreserveOrder => {}
    }
    out
}

fn is_superset<T, F>(big: &[Hypothesis<T>], small: &[Hypothesis<T>], eq: &F) -> bool
where
    F: Fn(&T, &T) -> bool,
{
    small.iter().all(|s| big.iter().any(|b| eq(&b.pattern, &s.pattern)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(p: &str) -> Hypothesis<&str> {
        Hypothesis::new(p, "test")
    }

    #[test]
    fn drops_superset_keeps_subset() {
        // {p, q} subsumes {p, q, r}; only {p, q} should survive.
        let cs = vec![vec![h("p"), h("q"), h("r")], vec![h("p"), h("q")]];
        let out = minimize_by_subsumption(cs, |a, b| a == b, MinimizePolicy::default());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 2);
    }

    #[test]
    fn keeps_pareto_incomparable_candidates() {
        // {p, q} and {p, r} share `p` but neither contains the other.
        let cs = vec![vec![h("p"), h("q")], vec![h("p"), h("r")]];
        let out = minimize_by_subsumption(cs, |a, b| a == b, MinimizePolicy::PreserveOrder);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn smallest_first_orders_by_length() {
        let cs = vec![vec![h("a"), h("b")], vec![h("c")]];
        let out = minimize_by_subsumption(cs, |a, b| a == b, MinimizePolicy::SmallestFirst);
        assert_eq!(out[0].len(), 1);
        assert_eq!(out[1].len(), 2);
    }

    #[test]
    fn equal_candidates_deduplicated() {
        let cs = vec![vec![h("p")], vec![h("p")]];
        let out = minimize_by_subsumption(cs, |a, b| a == b, MinimizePolicy::default());
        assert_eq!(out.len(), 1);
    }
}
