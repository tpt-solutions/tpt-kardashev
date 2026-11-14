# Changelog: tpt-kardashev-astrodynamics

## Unreleased

- `Error` (`#[non_exhaustive]`): non-finite input, invalid μ, non-elliptic orbit, degenerate state.
- `Kilometres`, `Seconds`, `Radians`: finite-checked unit newtypes; `Vec3` with dot, cross, add, subtract and scale.
- `Keplerian` elements and `StateVector`: `from_state` and `to_state` for elliptic two-body orbits, with the singular-case conventions in the README. Tests in `tests/elements.rs`: known values, the angular-momentum and energy identities, round trips over a grid of 2160 element sets, and rejection of invalid input.
- The element conversions are behind the default `std` feature. The `no_std` build compiles the error, unit and vector types only.
- `time`: `Epoch` on the TT scale (seconds past J2000), with TAI, Julian dates, and conversion to and from `UtcTime` through a caller-supplied `LeapSeconds` table (`LeapEntry` rows). Calendar arithmetic is integer-only, so the module is `no_std`. Tests in `tests/time.rs`. TDB is not written.
- `Error`: `InvalidCalendar`, `NoLeapSecondEntry`, `InvalidLeapSeconds`.
