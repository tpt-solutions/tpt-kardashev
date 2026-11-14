# tpt-kardashev-astrodynamics

Time scales, reference frames, orbital elements, propagation, Lambert solving and impulsive manoeuvres.

**Status:** early slice. Implemented: unit newtypes (`Kilometres`, `Seconds`, `Radians`), a small `Vec3`, Keplerian elements for elliptic two-body orbits with conversions to and from state vectors, epochs on the TT scale with TAI, UTC (through a caller-supplied leap-second table) and Julian dates, and the perifocal and RTN frames. Everything else in the v0.1 scope is not written yet (see the workspace `todo.md` §2.2).

## Features

- `std` (default): links the standard library. The element conversions need it, because `core` has no trigonometry.
- `alloc`: enables heap-allocating APIs. Core is `no_std`.

## Conventions

- Positions are in km, velocities in km/s, and the gravitational parameter μ in km³/s². Each function takes μ explicitly; the crate bundles no body constants.
- Angles are in radians, returned in `[0, 2π)`. The reference plane is the x–y plane, and `h = r × v` is the orbit normal.
- Circular and equatorial orbits use fixed conventions, so each state has one element set: circular orbits take ω = 0 and ν as the argument of latitude; equatorial orbits take Ω = 0 and ω as the longitude of periapsis; circular equatorial orbits take ν as the true longitude. Retrograde orbits measure angles in the sense of the orbit.

## Optional features

- `relativity`: declared per spec §4.2; no code behind it yet.
- `dtn`: declared per spec §4.2; no code behind it yet.

## What it does not model

- Parabolic and hyperbolic orbits. Their states are rejected with `Error::NotElliptic`.
- Equinoctial elements. The singular cases above use the fixed conventions instead.
- TDB, UT1 and Earth orientation. Leap seconds come from the caller's table, which the crate does not bundle; a UTC second of 60 (leap-second insertion) is rejected. Epochs are `f64` seconds past J2000, resolving about 0.5 µs.
- Reference frames beyond perifocal and RTN. LVLH is not written, because its axis conventions differ between users and need an explicit choice. Heliocentric ecliptic (needs a heliocentric origin) and Earth-fixed (needs UT1) are not written either. Propagation, force models, Lambert solving, manoeuvres and ephemerides. Force-model limits are stated in full at v0.1 (todo §2.2).
- No ephemeris data is bundled. Low-thrust approximations are post-v0.1. SGP4 stays behind the `sgp4` feature (spec §12.4).

## Checks

- Known values: a geostationary-radius circular equatorial orbit (a = r, e = 0, i = 0), and an inclined circular orbit (i = 0.5 rad, Ω = 0).
- Invariants: for an elliptic state, |h| = √(μ·a(1 − e²)) and the energy is −μ/(2a).
- Round trips: state → elements → state over a grid of 2160 element sets, including circular, equatorial, polar and retrograde cases; and elements → state → elements away from the singular cases, with the elements recovered.
- Rejection: hyperbolic and radial states, non-finite values, and non-positive μ.
- Time: J2000 is Julian date 2451545.0 TT; TAI is TT − 32.184 s; 2000-01-01 12:00 UTC is 64.184 s before the J2000 instant in TT; the TAI difference across the 2017 leap second is 2 s; UTC round trips through TT; and invalid calendar values are rejected. The leap-second fixture is a test table, not crate data.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
