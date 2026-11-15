# Changelog: tpt-kardashev-nuclear-plasma

## Unreleased

- Phase 0 scaffold.
- `rng::SplitMix64`: a seeded 64-bit generator, reproducible from its seed.
- `slab::{run_slab, run_infinite, CrossSections, Slab, Estimator, SlabTally, Estimate}` (feature `std`): one-group fixed-source Monte Carlo in a slab, with analog and implicit-capture estimators, and the infinite-medium track length. Tests in `tests/neutronics.rs`: the uncollided transmission law and track length for a pure absorber, the infinite-medium track length 1/σa, history-by-history weight conservation, agreement of the two estimators, reproducibility, the square-root error law, the generator's uniformity, and rejection of invalid input.
- `error::NeutronicsError` (`#[non_exhaustive]`).
- `equilibrium::{Grid, Solovev, Solution, solve_fixed_boundary, max_interior_error}` (feature `alloc`): fixed-boundary solve of `Δ*ψ = κR²` on a rectangular (R, Z) grid, by central differences and SOR, with the Solov'ev function as the exact benchmark. Tests in `tests/equilibrium.rs`: the discrete operator's residual on the exact function falls as h²; the solver's interior error falls by about 4 when the grid is halved; boundary values are kept exactly; running out of sweeps is reported; and bad grids and tolerances are rejected.
- `error::NeutronicsError`: `BadGrid`, `BadTolerance`, `NotConverged` added.
