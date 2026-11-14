# Provenance: tpt-kardashev-astrodynamics

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None.

## Written from standard relations

- `elements.rs` (Keplerian ↔ state vector): written from the standard two-body relations (the perifocal position and velocity, the rotation Rz(Ω)·Rx(i)·Rz(ω), and the usual state-to-element formulas for h, the node vector, the eccentricity vector and the energy). The singular-case conventions are this crate's own, chosen so that each state has one element set. No reference implementation was consulted for this code; the tests check it against identities and round trips, not against another implementation.

## Reviewed for reference (not copied)

- Not yet. tpt-sci-astro (Lambert solver check, spec §12.7), tpt-transportation, tpt-zenith/orbital-mechanics and tpt-equinox-orbital are not in the local workspace, so they have not been reviewed (todo §2.2 remains open for them).

## Third-party licences

None.
