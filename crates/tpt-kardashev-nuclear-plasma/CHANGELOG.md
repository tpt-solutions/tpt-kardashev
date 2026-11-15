# Changelog: tpt-kardashev-nuclear-plasma

## Unreleased

- Phase 0 scaffold.
- `rng::SplitMix64`: a seeded 64-bit generator, reproducible from its seed.
- `slab::{run_slab, run_infinite, CrossSections, Slab, Estimator, SlabTally, Estimate}` (feature `std`): one-group fixed-source Monte Carlo in a slab, with analog and implicit-capture estimators, and the infinite-medium track length. Tests in `tests/neutronics.rs`: the uncollided transmission law and track length for a pure absorber, the infinite-medium track length 1/σa, history-by-history weight conservation, agreement of the two estimators, reproducibility, the square-root error law, the generator's uniformity, and rejection of invalid input.
- `error::NeutronicsError` (`#[non_exhaustive]`).
- `equilibrium::{Grid, Solovev, Solution, solve_fixed_boundary, max_interior_error}` (feature `alloc`): fixed-boundary solve of `Δ*ψ = κR²` on a rectangular (R, Z) grid, by central differences and SOR, with the Solov'ev function as the exact benchmark. Tests in `tests/equilibrium.rs`: the discrete operator's residual on the exact function falls as h²; the solver's interior error falls by about 4 when the grid is halved; boundary values are kept exactly; running out of sweeps is reported; and bad grids and tolerances are rejected.
- `error::NeutronicsError`: `BadGrid`, `BadTolerance`, `NotConverged` added.
- `surface::{contour, safety_factor, bilinear, gradient_magnitude}` (feature `alloc`): flux surfaces from a sampled flux function, found along rays from a given magnetic axis, and the safety factor `q = (F/2π)∮dl/(R|∇ψ|)` for a constant toroidal flux function. Tests in `tests/surface.rs`: bilinear interpolation, the gradient of the circular flux function, the contour as the circle about the axis, q against the closed form on three surfaces (within 2%) and monotone in radius, and the error paths.
- `error::NeutronicsError::NoSurface`.
- `xs::Multigroup` and `xs::read_multigroup` (features `std` and `alloc`): multigroup total and absorption cross sections in this crate's text format, version 1 (documented in the module). `Multigroup::to_text` writes the same format. Tests in `tests/xs.rs`. The format was chosen over ENDF-6 and ACE so that no third-party format is needed for the first version.
- `fuzz/`: the `xs_read` target. A run of 2.5 minutes found no crash.
