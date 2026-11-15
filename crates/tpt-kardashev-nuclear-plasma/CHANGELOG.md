# Changelog: tpt-kardashev-nuclear-plasma

## Unreleased

- Phase 0 scaffold.
- `rng::SplitMix64`: a seeded 64-bit generator, reproducible from its seed.
- `slab::{run_slab, run_infinite, CrossSections, Slab, Estimator, SlabTally, Estimate}` (feature `std`): one-group fixed-source Monte Carlo in a slab, with analog and implicit-capture estimators, and the infinite-medium track length. Tests in `tests/neutronics.rs`: the uncollided transmission law and track length for a pure absorber, the infinite-medium track length 1/σa, history-by-history weight conservation, agreement of the two estimators, reproducibility, the square-root error law, the generator's uniformity, and rejection of invalid input.
- `error::NeutronicsError` (`#[non_exhaustive]`).
