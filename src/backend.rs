//! The solver-side trait an abductive driver consumes.
//!
//! Any SAT / SMT-shaped backend can implement [`AbductiveBackend`] to
//! participate in abductive search. The trait deliberately avoids
//! committing to a particular term representation, error type, or
//! verdict shape — the only constraint is that the backend can:
//!
//! - report whether a candidate set of assumptions makes the formula
//!   satisfiable
//! - clean up after itself between candidate trials (push / pop)
//!
//! Backends with richer surfaces (`Z3`-style assumption assumption
//! literals, MaxSAT cores, model extraction) are encouraged to expose
//! those via inherent methods; the trait stays minimal so the cost
//! of implementing it is small.

use crate::Hypothesis;

/// Coarse verdict returned by [`AbductiveBackend::check_with`].
///
/// Backends with finer-grained outcomes can box their own type in
/// [`AbductiveBackend::FullVerdict`] and convert at the trait
/// boundary; the driver only needs the [`Verdict`] coarsening to
/// decide whether to keep / prune a candidate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Formula is satisfiable under the supplied assumptions.
    Sat,
    /// Formula is unsatisfiable under the supplied assumptions.
    Unsat,
    /// Backend timed out, ran out of budget, or otherwise can't
    /// decide. Abductive search treats this conservatively
    /// (typically by skipping the candidate).
    Unknown,
}

/// The minimal interface an abductive driver needs from a solver.
///
/// `Self::Term` is the backend's native formula representation. The
/// driver passes [`Hypothesis<Self::Term>`] values verbatim so the
/// backend can interpret pattern terms with whatever matching
/// semantics it prefers (α-equivalence, unification, etc.).
pub trait AbductiveBackend {
    /// The term / atom representation native to this backend.
    type Term: Clone;

    /// Optional richer verdict; backends without one default this
    /// to `Verdict` and return it from `check_with` directly.
    type FullVerdict;

    /// Run the solver under the supplied assumptions and report
    /// both the coarse verdict and the backend-specific full
    /// verdict.
    ///
    /// The backend must restore its prior state before returning
    /// (push/pop or equivalent), so successive calls with
    /// different candidate sets are independent.
    fn check_with(
        &mut self,
        assumptions: &[Hypothesis<Self::Term>],
    ) -> (Verdict, Self::FullVerdict);

    /// Enumerate the atoms the backend regards as abducible. The
    /// driver iterates this list to seed candidate sets. Order is
    /// preserved.
    fn abducibles(&self) -> Vec<Hypothesis<Self::Term>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial mock backend that returns `Sat` iff every supplied
    /// hypothesis's pattern starts with `'p'`. Used here only to
    /// validate the trait surface compiles and tests can call
    /// `check_with` end-to-end.
    struct MockBackend {
        abducibles: Vec<Hypothesis<String>>,
    }

    impl AbductiveBackend for MockBackend {
        type Term = String;
        type FullVerdict = Verdict;

        fn check_with(
            &mut self,
            assumptions: &[Hypothesis<Self::Term>],
        ) -> (Verdict, Self::FullVerdict) {
            let v = if assumptions.iter().all(|h| h.pattern.starts_with('p')) {
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
    fn mock_backend_returns_sat_for_matching_prefix() {
        let mut b = MockBackend {
            abducibles: vec![
                Hypothesis::new("p1".into(), "src"),
                Hypothesis::new("p2".into(), "src"),
            ],
        };
        let (v, _) = b.check_with(&b.abducibles());
        assert_eq!(v, Verdict::Sat);
    }

    #[test]
    fn mock_backend_returns_unsat_on_mismatch() {
        let mut b = MockBackend {
            abducibles: vec![Hypothesis::new("q1".into(), "src")],
        };
        let (v, _) = b.check_with(&b.abducibles());
        assert_eq!(v, Verdict::Unsat);
    }
}
