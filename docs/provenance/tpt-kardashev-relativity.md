# Provenance: tpt-kardashev-relativity

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reference list (not yet reviewed)

These older repos are listed in the todo but have not been opened yet. Review is an open todo item.

- tpt-replicate/src/clock.rs: bounded-drift assumption (todo §1.2)

## Constants and sources

- Speed of light: exact, 299 792 458 m/s (SI definition).
- Time-scale rate constants `L_G`, `L_C`, `L_B` in `src/timescale.rs`: transcribed from IAU 1991 B1.9 and IAU 2006 B3. Not yet verified against the IAU texts.
- GPS check values (GM, radii) are standard published figures, recorded in the test and example.

## Third-party licences

None.
