//! Solver-agnostic abductive-reasoning trait surface.
//!
//! This crate exposes the [`AbductiveBackend`] trait that any SAT /
//! SMT-shaped solver can implement to participate in abductive
//! reasoning, plus a generic [`Hypothesis`] / [`HypothesisSet`] data
//! model and a minimal [`search::abduce`] driver that runs over an
//! [`AbductiveBackend`].
//!
//! The crate is intentionally independent of any specific solver
//! implementation; an optional `oxiz-sat` feature provides a
//! ready-made adapter, but downstream solvers (adsmt's engine,
//! others) can also implement the trait directly.
//!
//! # Why this exists
//!
//! Abductive reasoning — "what minimal assumption would make this
//! goal derivable?" — is a meta-level pattern that sits on top of
//! standard SAT / SMT solving. The Aristotelian abduction loop
//! complements deduction the way induction does, but without the
//! statistical machinery induction needs. Several solvers expose
//! variations of this idea (Z3's "abduce", custom MaxSAT-as-
//! abduction patterns, etc.) but none publish a portable trait
//! surface. `oxiz-contrib-abduction` is an attempt at one.
//!
//! # License & governance
//!
//! Apache-2.0, matching `cool-japan/oxiz` upstream. The crate lives
//! under `newsniper-org/oxiz-contrib-abduction` but is offered to
//! cool-japan for promotion to a first-party `oxiz-abduction` crate
//! at any time — the trait shape and ownership are intentionally
//! kept thin so a lift-and-shift takes a directory move.

pub mod backend;
pub mod hypothesis;
pub mod minimize;
pub mod search;

#[cfg(feature = "oxiz-sat")]
pub mod oxiz_sat_adapter;

pub use backend::{AbductiveBackend, Verdict};
pub use hypothesis::{Hypothesis, HypothesisSet};
pub use minimize::{minimize_by_subsumption, MinimizePolicy};
pub use search::{abduce, AbductiveSolution};
