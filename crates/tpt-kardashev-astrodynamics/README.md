# tpt-kardashev-astrodynamics

Time scales, reference frames, orbital elements, propagation, Lambert solving and impulsive manoeuvres.

**Status:** early slice. Implemented: unit newtypes (`Kilometres`, `Seconds`, `Radians`), a small `Vec3`, Keplerian elements for elliptic two-body orbits with conversions to and from state vectors, epochs on the TT scale with TAI, UTC (through a caller-supplied leap-second table) and Julian dates, the perifocal and RTN frames, two-body propagation by universal variables (`propagate::propagate`, which also covers hyperbolic states), and numerical propagation: a `ForceModel` trait, central-body gravity (`force::CentralBody`), a J2 zonal perturbation (`force::J2Zonal`, with WGS 84 constants), and a fixed-step RK4 `integrate::Cowell` propagator, impulsive manoeuvres (`manoeuvre`: Hohmann, bi-elliptic, and plane change), Lambert's problem (`lambert::lambert`, zero-revolution transfers), a low-precision analytic ephemeris for the Sun and Moon (`ephemeris`), third-body gravity (`force::ThirdBody`), solar radiation pressure with a cylindrical Earth shadow (`force::SolarRadiationPressure`), atmospheric drag from an exponential atmosphere (`force::AtmosphericDrag`, `atmosphere`), finite burns (`force::Thrust`), and ground-station visibility (`visibility`: WGS 84 stations, elevation, and contact windows). Everything else in the v0.1 scope is not written yet (see the workspace `todo.md` §2.2).

## Features

- `std` (default): links the standard library. The element conversions need it, because `core` has no trigonometry.
- `alloc`: enables heap-allocating APIs. Core is `no_std`.

## Conventions

- Positions are in km, velocities in km/s, and the gravitational parameter μ in km³/s². Each function takes μ explicitly; the crate bundles no body constants.
- Angles are in radians, returned in `[0, 2π)`. The reference plane is the x–y plane, and `h = r × v` is the orbit normal.
- Circular and equatorial orbits use fixed conventions, so each state has one element set: circular orbits take ω = 0 and ν as the argument of latitude; equatorial orbits take Ω = 0 and ω as the longitude of periapsis; circular equatorial orbits take ν as the true longitude. Retrograde orbits measure angles in the sense of the orbit.

## Optional features

- `relativity`: declared per spec §4.2; no code behind it yet.

## What it does not model

- Parabolic and hyperbolic orbits. Their states are rejected with `Error::NotElliptic`.
- Equinoctial elements. The singular cases above use the fixed conventions instead.
- TDB, UT1 and Earth orientation. Leap seconds come from the caller's table, which the crate does not bundle; a UTC second of 60 (leap-second insertion) is rejected. Epochs are `f64` seconds past J2000, resolving about 0.5 µs.
- Visibility has no atmospheric refraction, no terrain masking, and no station motion. Its Earth rotation starts from an angle the caller gives at the first instant, so it does not use UT1 or sidereal time. Contact windows are found by sampling at a fixed step, so a pass shorter than the step can be missed. Reference frames beyond perifocal and RTN. LVLH is not written, because its axis conventions differ between users and need an explicit choice. Heliocentric ecliptic (needs a heliocentric origin) and Earth-fixed (needs UT1) are not written either. Force models beyond central gravity, J2, the Sun and Moon as third bodies, solar pressure, drag and thrust (higher zonal harmonics). A thrust has a constant acceleration: the spacecraft's mass is not tracked, so there is no mass flow or propellant use. Its edges are discontinuous, so a fixed step should be much shorter than the burn (see `force::Thrust`). The drag uses an exponential atmosphere with no solar-cycle dependence, and returns zero above 1000 km, which understates drag at the top of the table's range; the solar pressure has a sharp cylindrical shadow, with no penumbra and no albedo or infrared; J2 is given in an inertial frame with the spin axis as z, so the Earth's rotation is not applied, adaptive integration (the step is fixed), multi-revolution Lambert transfers, and ephemerides beyond the Sun and Moon. The series are low precision: the Sun to about 0.01° in longitude, the Moon to a few tenths of a degree. Neither is a bundled ephemeris. `spk` reads a caller's SPK kernel (type 2 only, J2000 frame, little-endian) and has been tested only on kernels built in the test suite: no real kernel is checked, and none is bundled. Its epochs are seconds past J2000 on the TDB scale, not converted from TT. It does not chain segments, so each body needs a segment relative to the centre it is asked for. The ephemeris is in the J2000 ecliptic, so the third-body force rotates it into the equatorial frame, where J2 and the spacecraft state are defined. Lambert transfers whose plane contains the z axis are rejected, since their sense cannot be read from the z component of the angular momentum. The manoeuvres are instantaneous and two-body; the plane change is a single burn, with no combined apsidal plane change beyond the law of cosines. The propagator does not report events such as periapsis passage. Force-model limits are stated in full at v0.1 (todo §2.2).
- No ephemeris data is bundled. Low-thrust approximations are post-v0.1. SGP4 stays behind the `sgp4` feature (spec §12.4).

## Checks

- Known values: a geostationary-radius circular equatorial orbit (a = r, e = 0, i = 0), and an inclined circular orbit (i = 0.5 rad, Ω = 0).
- Invariants: for an elliptic state, |h| = √(μ·a(1 − e²)) and the energy is −μ/(2a).
- Round trips: state → elements → state over a grid of 2160 element sets, including circular, equatorial, polar and retrograde cases; and elements → state → elements away from the singular cases, with the elements recovered.
- Rejection: hyperbolic and radial states, non-finite values, and non-positive μ.
- Time: J2000 is Julian date 2451545.0 TT; TAI is TT − 32.184 s; 2000-01-01 12:00 UTC is 64.184 s before the J2000 instant in TT; the TAI difference across the 2017 leap second is 2 s; UTC round trips through TT; and invalid calendar values are rejected. The leap-second fixture is a test table, not crate data.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
