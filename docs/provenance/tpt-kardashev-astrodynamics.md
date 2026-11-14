# Provenance: tpt-kardashev-astrodynamics

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None.

## Written from standard relations

- `elements.rs` (Keplerian ↔ state vector): written from the standard two-body relations (the perifocal position and velocity, the rotation Rz(Ω)·Rx(i)·Rz(ω), and the usual state-to-element formulas for h, the node vector, the eccentricity vector and the energy). The singular-case conventions are this crate's own, chosen so that each state has one element set. No reference implementation was consulted for this code; the tests check it against identities and round trips, not against another implementation.

- `matrix.rs` and `frames.rs` (perifocal rotation and RTN): the rotation is the product Rz(Ω)·Rx(i)·Rz(ω), and RTN is the triad R = r̂, N = ĥ, T = N × R, both from standard definitions. No reference code was consulted.

- `propagate.rs` (two-body propagation): the universal-variable formulation, meaning the f and g coefficients, the universal Kepler equation and the Stumpff functions. Written from the standard relations; no reference code was consulted. The tests check it against conservation laws, composition, reversal and an independent Kepler-equation solve.

- `time.rs` (epochs, UTC and Julian dates): the constants are the definitions (TT − TAI = 32.184 s; J2000 is 2000-01-01 12:00 TT, JD 2451545.0). The calendar conversions use the civil-date algorithm of Howard Hinnant (integer days from 1970-01-01), written here from the published description; no code was copied. The leap-second fixture in `tests/time.rs` holds the IERS TAI − UTC values as recalled; it is test data, and should be checked against IERS Bulletin C before the test values are relied on. The crate bundles no leap-second data.

## Reviewed for reference (not copied)

- Not yet. tpt-sci-astro (Lambert solver check, spec §12.7), tpt-transportation, tpt-zenith/orbital-mechanics and tpt-equinox-orbital are not in the local workspace, so they have not been reviewed (todo §2.2 remains open for them).

## Third-party licences

None.
