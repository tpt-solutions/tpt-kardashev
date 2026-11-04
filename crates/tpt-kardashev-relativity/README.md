# tpt-kardashev-relativity

Special and weak-field relativity: intervals, Lorentz boosts, proper time, causal predicates, weak-field clock rates and TT/TCG/TCB/TDB rate relations.

**Status:** v0.1 candidate. The API is pre-1.0 and may change. Not published (`publish = false`).

## What it provides

- `Event`, `FourVector`, `interval` and `IntervalKind` (timelike, spacelike, lightlike), signature (+, -, -, -).
- `Lorentz`: boosts from a velocity, boosts from a rapidity, composition and application to four-vectors.
- `Frame`: inertial frames and coordinate transforms to and from a parent frame.
- Time dilation, length contraction, relativistic Doppler, and `proper_time` along a piecewise path.
- `CausalOrder` (neutral trait), `causally_disconnected`, `light_cone_contains`.
- Weak-field and Schwarzschild static clock rates, and gravitational redshift.
- `Metric` trait with `Minkowski` and `Schwarzschild`.
- `timescale`: rate relations between TT, TCG, TCB and TDB, as interval functions.

Numeric tolerances follow `docs/tolerance-policy.md`. Lightlike classification uses a relative tolerance of 1e-12 on `(c dt)^2 + |dx|^2`.

## Units and conventions

SI throughout: seconds, metres, metres per second, and gravitational parameters `GM` in m^3/s^2. Coordinates for `Metric::g` are `[ct, q1, q2, q3]`.

## What it does not model

- Geodesic integration. The `Metric` trait gives the metric only.
- Strong-field gravity beyond the Schwarzschild static-observer formulas.
- Earth rotation, orbital eccentricity, and any other terms beyond the first-order clock rate in the GPS example.
- The periodic terms of TDB (about +/- 1.7 ms) and any epoch offsets. Only secular rates are included.
- Calendar and epoch handling, which belongs to `tpt-kardashev-astrodynamics`.
- The generic `Scalar` trait from the spec. The v0.1 API is `f64` only.

## Dependencies

None at runtime. Elementary functions (`sqrt`, `ln`, `exp`, `sin`) are implemented in-crate so the core stays `no_std`.

## Examples

```sh
cargo run -p tpt-kardashev-relativity --example gps_clock
```

Prints the net GPS clock offset from gravity and motion, about +38.5 microseconds per day against the published ~38 us/day.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
