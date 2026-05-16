//! `oxiz_sat::Solver`-backed [`AbductiveBackend`] adapter.
//!
//! Behind the `oxiz-sat` feature. Wraps an `oxiz_sat::Solver` plus a
//! list of `Lit`-shaped abducibles and exposes them through the
//! trait. The adapter uses `solve_with_assumptions` for each
//! candidate set, so the underlying solver state isn't disturbed
//! between calls.
//!
//! This is the simplest possible adapter — it doesn't try to extract
//! cores, learn from prior failed candidates, or otherwise exploit
//! oxiz-sat's richer surface. Smarter strategies are deliberately
//! left as inherent methods on a wrapping type so the trait stays
//! small.

use oxiz_sat::{Lit, Solver as OxSolver, SolverResult};

use crate::backend::{AbductiveBackend, Verdict};
use crate::hypothesis::Hypothesis;

pub struct OxizSatBackend {
    solver: OxSolver,
    abducibles: Vec<Hypothesis<Lit>>,
}

impl OxizSatBackend {
    pub fn new(solver: OxSolver, abducibles: Vec<Hypothesis<Lit>>) -> Self {
        Self { solver, abducibles }
    }

    pub fn solver(&self) -> &OxSolver {
        &self.solver
    }

    pub fn solver_mut(&mut self) -> &mut OxSolver {
        &mut self.solver
    }
}

impl AbductiveBackend for OxizSatBackend {
    type Term = Lit;
    /// `(verdict, unsat-core if any)` from
    /// `oxiz_sat::Solver::solve_with_assumptions`. Downstream
    /// strategies that want to prune candidates by inspecting
    /// the core can read the second element directly.
    type FullVerdict = (SolverResult, Option<Vec<Lit>>);

    fn check_with(
        &mut self,
        assumptions: &[Hypothesis<Self::Term>],
    ) -> (Verdict, Self::FullVerdict) {
        let lits: Vec<Lit> = assumptions.iter().map(|h| h.pattern).collect();
        let full = self.solver.solve_with_assumptions(&lits);
        let coarse = match full.0 {
            SolverResult::Sat => Verdict::Sat,
            SolverResult::Unsat => Verdict::Unsat,
            _ => Verdict::Unknown,
        };
        (coarse, full)
    }

    fn abducibles(&self) -> Vec<Hypothesis<Self::Term>> {
        self.abducibles.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Adapter mechanics — confirm `check_with(assumptions)` runs
    /// `solve_with_assumptions` and routes the verdict through the
    /// coarsening.
    #[test]
    fn check_with_returns_sat_on_consistent_assumption() {
        // (x1 ∨ x2) ∧ ¬x1 — satisfiable; assuming x2 stays sat.
        let mut solver = OxSolver::new();
        let x1 = solver.new_var();
        let x2 = solver.new_var();
        let _ = solver.add_clause(vec![Lit::pos(x1), Lit::pos(x2)]);
        let _ = solver.add_clause(vec![Lit::neg(x1)]);

        let abducibles = vec![Hypothesis::new(Lit::pos(x2), "abducible-x2")];
        let mut backend = OxizSatBackend::new(solver, abducibles.clone());
        let (verdict, _) = backend.check_with(&abducibles);
        assert_eq!(verdict, Verdict::Sat);
    }

    #[test]
    fn check_with_returns_unsat_on_contradictory_assumption() {
        // (x1 ∨ x2) ∧ ¬x1 ∧ ¬x2 — unsat under {¬x2} because x1 must
        // be false (from ¬x1) and x2 false (from assumption); the
        // clause (x1 ∨ x2) has no satisfier.
        let mut solver = OxSolver::new();
        let x1 = solver.new_var();
        let x2 = solver.new_var();
        let _ = solver.add_clause(vec![Lit::pos(x1), Lit::pos(x2)]);
        let _ = solver.add_clause(vec![Lit::neg(x1)]);

        let abducibles = vec![Hypothesis::new(Lit::neg(x2), "abducible-not-x2")];
        let mut backend = OxizSatBackend::new(solver, abducibles.clone());
        let (verdict, _) = backend.check_with(&abducibles);
        assert_eq!(verdict, Verdict::Unsat);
    }

    #[test]
    fn abducibles_returns_configured_list() {
        let solver = OxSolver::new();
        let abducibles = vec![
            Hypothesis::new(Lit::pos(oxiz_sat::Var::new(0)), "a"),
            Hypothesis::new(Lit::neg(oxiz_sat::Var::new(1)), "b"),
        ];
        let backend = OxizSatBackend::new(solver, abducibles.clone());
        let recovered = backend.abducibles();
        assert_eq!(recovered.len(), 2);
        assert_eq!(recovered[0].source, "a");
        assert_eq!(recovered[1].source, "b");
    }
}
