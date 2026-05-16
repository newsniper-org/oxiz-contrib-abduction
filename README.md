# `oxiz-contrib-abduction`

A solver-agnostic abductive-reasoning trait surface for the
[OxiZ](https://github.com/cool-japan/oxiz) ecosystem.

This crate publishes one trait — `AbductiveBackend` — that any SAT /
SMT-shaped solver can implement to participate in abductive
reasoning, plus a tiny portable search driver that finds minimal
hypothesis sets discharging a goal.

```rust
use oxiz_contrib_abduction::{abduce, AbductiveBackend, Hypothesis, Verdict};
```

## Status

- 0.1.x: trait surface stable for the use cases adsmt drives. API
  may evolve before a 1.0 promotion.
- Apache-2.0, matching `cool-japan/oxiz` upstream.

## Governance

The crate currently lives under
[`newsniper-org/oxiz-contrib-abduction`](https://github.com/newsniper-org/oxiz-contrib-abduction)
as a community contribution to the OxiZ ecosystem. It is offered to
`cool-japan/oxiz` for promotion to a first-party `oxiz-abduction`
crate at any time — the trait shape and ownership are intentionally
kept thin so a lift-and-shift takes a directory move.

See `LICENSE` for the Apache-2.0 text.

## Features

- `oxiz-sat` (optional): provides an `OxizSatBackend` adapter that
  wraps `oxiz_sat::Solver` and implements `AbductiveBackend`. Off by
  default so the trait surface stays usable in environments where
  pulling in `oxiz-sat` is undesirable.

## Modules

- `backend` — the `AbductiveBackend` trait and the coarse `Verdict`.
- `hypothesis` — `Hypothesis<T>` / `HypothesisSet<T>` data model.
- `minimize` — subsumption-based candidate-set minimization.
- `search` — the portable `abduce` driver.
- `oxiz_sat_adapter` (feature `oxiz-sat`) — the OxiZ adapter.
