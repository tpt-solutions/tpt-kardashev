# tpt-kardashev-nuclear-plasma

Fixed-boundary Grad-Shafranov equilibrium and fixed-source Monte Carlo neutronics with a cross-section reader.

**Status:** Phase 2.3, in progress. Implemented: a seeded generator (always); with `alloc`, a fixed-boundary Grad-Shafranov solve for a pressure-driven source on a rectangular grid, checked against the Solov'ev benchmark; with `std`, a one-group fixed-source Monte Carlo for a slab and an infinite medium with analog and implicit-capture estimators. Also flux surfaces about a given axis and the safety factor for a constant toroidal flux function. Not yet: the toroidal-field function F(ψ), constructive solid geometry in three dimensions, reaction-rate tallies by track length, and the rest of the statistical suite.

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

- No data: no cross-section library is bundled. The reader reads this crate's own multigroup text format (version 1), and ENDF-6 and ACE are not read.
- One group, one dimension, normal incidence at the left face, in one slab or a row of layers (`layers`). No multigroup transport, no three-dimensional geometry, no anisotropic scattering.
- No eigenvalue or criticality calculation. The runs are fixed-source only.
- Russian roulette is played at one fixed weight threshold. Splitting is by fixed zones of the slab (`Estimator::Splitting`), not by an importance function, so the zone count is the caller's choice and no optimal one is found.
- The equilibrium part is partial: the toroidal-field function is not solved, and the safety factor assumes a constant F. The reviewed sibling (`tpt-ignis`) has a reduced Miller-parametrised equilibrium, not a grid solve, and this crate's solver will be a grid solve.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.

## Equilibrium

- `equilibrium::solve_fixed_boundary` solves `Δ*ψ = κR²` on a rectangular (R, Z) grid, with the flux given on the boundary. The discretisation is second order, and the solve is SOR. The Solov'ev function `ψ = A R⁴ + B R² Z²`, exact with `κ = 8A + 2B`, is the benchmark. The toroidal-field function is not included, so this is not the full Grad-Shafranov equation.
