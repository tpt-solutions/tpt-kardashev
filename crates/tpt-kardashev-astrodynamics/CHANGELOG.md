# Changelog: tpt-kardashev-astrodynamics

## Unreleased

- `Error` (`#[non_exhaustive]`): non-finite input, invalid μ, non-elliptic orbit, degenerate state.
- `Kilometres`, `Seconds`, `Radians`: finite-checked unit newtypes; `Vec3` with dot, cross, add, subtract and scale.
- `Keplerian` elements and `StateVector`: `from_state` and `to_state` for elliptic two-body orbits, with the singular-case conventions in the README. Tests in `tests/elements.rs`: known values, the angular-momentum and energy identities, round trips over a grid of 2160 element sets, and rejection of invalid input.
- The element conversions are behind the default `std` feature. The `no_std` build compiles the error, unit and vector types only.
- `time`: `Epoch` on the TT scale (seconds past J2000), with TAI, Julian dates, and conversion to and from `UtcTime` through a caller-supplied `LeapSeconds` table (`LeapEntry` rows). Calendar arithmetic is integer-only, so the module is `no_std`. Tests in `tests/time.rs`. TDB is not written.
- `Error`: `InvalidCalendar`, `NoLeapSecondEntry`, `InvalidLeapSeconds`.
- `Mat3` (`no_std`): 3×3 matrix with multiply, transpose, determinant and vector product.
- `Keplerian::perifocal_to_inertial`: the rotation Rz(Ω)·Rx(i)·Rz(ω). `to_state` uses it.
- `frames::rtn_matrix`: the radial, transverse and normal axes of a state. Tests in `tests/frames.rs`: orthonormality and handedness, closed-form perifocal components, RTN components of position and velocity, circular velocity along T, and rejection of degenerate and non-finite states. LVLH, the heliocentric ecliptic and the Earth-fixed frame are not written.
