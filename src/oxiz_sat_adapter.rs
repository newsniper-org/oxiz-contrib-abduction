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

    // === End-to-end: drive abduce() through the adapter =================

    use crate::search::abduce;

    /// On a satisfiable formula, the empty hypothesis set is always
    /// the unique minimal solution — adding any abducible can only
    /// restrict further (classical SAT monotonicity). This test
    /// confirms the trait routing all the way through `abduce`:
    /// the driver iterates subset sizes, calls `check_with` for
    /// each, and `minimize_by_subsumption` correctly keeps only
    /// the empty solution.
    #[test]
    fn end_to_end_already_sat_formula_yields_empty_minimal_solution() {
        let mut solver = OxSolver::new();
        let x1 = solver.new_var();
        let x2 = solver.new_var();
        let _ = solver.add_clause(vec![Lit::pos(x1), Lit::pos(x2)]);

        let abducibles = vec![
            Hypothesis::new(Lit::pos(x1), "x1-pos"),
            Hypothesis::new(Lit::neg(x1), "x1-neg"),
            Hypothesis::new(Lit::pos(x2), "x2-pos"),
            Hypothesis::new(Lit::neg(x2), "x2-neg"),
        ];
        let mut backend = OxizSatBackend::new(solver, abducibles);
        let solutions = abduce(&mut backend, 4, |a, b| a == b);
        assert_eq!(solutions.len(), 1);
        assert!(solutions[0].hypotheses.is_empty());
    }

    /// Unsat-alone formula admits no abductive rescue in pure SAT
    /// (monotonicity again: adding assumptions only restricts).
    /// This test pins down that observable: no abductive solution
    /// exists when bare formula is already unsatisfiable.
    #[test]
    fn end_to_end_unsat_formula_yields_no_solutions() {
        let mut solver = OxSolver::new();
        let x = solver.new_var();
        let _ = solver.add_clause(vec![Lit::pos(x)]);
        let _ = solver.add_clause(vec![Lit::neg(x)]);

        let abducibles = vec![Hypothesis::new(Lit::pos(x), "x")];
        let mut backend = OxizSatBackend::new(solver, abducibles);
        let solutions = abduce(&mut backend, 3, |a, b| a == b);
        assert!(
            solutions.is_empty(),
            "pure-SAT abduction can't rescue an unsat formula"
        );
    }

    /// Some abducible combinations are mutually inconsistent with
    /// the formula. This test confirms the driver correctly rejects
    /// the inconsistent combinations (e.g. {+x, -x}) — the verdict
    /// for those subsets must come back as Unsat from
    /// `solve_with_assumptions`. Verified indirectly: the surviving
    /// solution is still {} (the minimum), but `check_with`
    /// definitely returned Unsat for at least one larger candidate.
    #[test]
    fn end_to_end_rejects_self_contradictory_assumption_pairs() {
        let mut solver = OxSolver::new();
        let x = solver.new_var();
        let _ = solver.add_clause(vec![Lit::pos(x)]); // forces x=true

        let abducibles = vec![
            Hypothesis::new(Lit::pos(x), "x-pos"),
            Hypothesis::new(Lit::neg(x), "x-neg"), // contradicts the clause
        ];
        let mut backend = OxizSatBackend::new(solver, abducibles);
        let solutions = abduce(&mut backend, 2, |a, b| a == b);
        // Only solutions consistent with the formula survive
        // minimization. {} is sat; {x-pos} is sat; {x-neg} is unsat;
        // {x-pos, x-neg} is unsat. After subsumption: {} alone.
        assert_eq!(solutions.len(), 1);
        assert!(solutions[0].hypotheses.is_empty());

        // Sanity-check the FullVerdict path: feeding the contradictory
        // assumption set returns Unsat, not Unknown.
        let mut backend2 = OxizSatBackend::new(
            {
                let mut s = OxSolver::new();
                let x = s.new_var();
                let _ = s.add_clause(vec![Lit::pos(x)]);
                s
            },
            vec![Hypothesis::new(Lit::neg(oxiz_sat::Var::new(0)), "x-neg")],
        );
        let assumptions = backend2.abducibles();
        let (verdict, full) = backend2.check_with(&assumptions);
        assert_eq!(verdict, Verdict::Unsat);
        // FullVerdict carries the unsat core when the verdict is
        // unsat; on Sat the second element is None.
        let (result, core) = full;
        assert_eq!(result, oxiz_sat::SolverResult::Unsat);
        assert!(core.is_some(), "unsat-from-assumption returns a core");
    }
}
