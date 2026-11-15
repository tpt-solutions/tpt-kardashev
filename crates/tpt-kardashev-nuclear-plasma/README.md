# tpt-kardashev-nuclear-plasma

Fixed-boundary Grad-Shafranov equilibrium and fixed-source Monte Carlo neutronics with a cross-section reader.

**Status:** Phase 2.3, in progress. Implemented: a seeded generator (always), and with the `std` feature a one-group fixed-source Monte Carlo for a slab and for an infinite medium, with analog and implicit-capture estimators, validated against analytic results. Not yet: the equilibrium solver, the cross-section reader and multigroup types, constructive solid geometry, splitting, spatial flux bins, and the rest of the statistical suite.

## Features

- `std` (default): links the standard library. The Monte Carlo needs `ln`.
- The generator and the error type need neither `std` nor `alloc`.

## Validation

- A pure absorber transmits exp(-σt T) of the neutrons, and its track length is (1 - e^{-σt T})/σt. Checked at four standard errors with fixed seeds.
- An infinite medium has a track length of 1/σa per source neutron, whatever the scattering. Checked the same way.
- The analog estimator conserves weight history by history: transmission, reflection and absorption sum to one.
- The implicit-capture and analog estimators agree on a scattering slab, within their combined standard errors.
- The standard error falls as the square root of the number of histories.

## What it does not model

- No data: no cross-section library is bundled, and the cross-section reader is not written. The format decision is open.
- One group, one dimension, normal incidence at the left face. No multigroup transport, no curved geometry, no anisotropic scattering.
- No eigenvalue or criticality calculation. The runs are fixed-source only.
- Russian roulette is played at one fixed weight threshold; splitting is not implemented.
- The equilibrium part is not written. The reviewed sibling (`tpt-ignis`) has a reduced Miller-parametrised equilibrium, not a grid solve, and this crate's solver will be a grid solve.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
