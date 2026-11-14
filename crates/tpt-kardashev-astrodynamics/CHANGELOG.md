# Changelog: tpt-kardashev-astrodynamics

## Unreleased

- `Error` (`#[non_exhaustive]`): non-finite input, invalid μ, non-elliptic orbit, degenerate state.
- `Kilometres`, `Seconds`, `Radians`: finite-checked unit newtypes; `Vec3` with dot, cross, add, subtract and scale.
- `Keplerian` elements and `StateVector`: `from_state` and `to_state` for elliptic two-body orbits, with the singular-case conventions in the README. Tests in `tests/elements.rs`: known values, the angular-momentum and energy identities, round trips over a grid of 2160 element sets, and rejection of invalid input.
- The element conversions are behind the default `std` feature. The `no_std` build compiles the error, unit and vector types only.
