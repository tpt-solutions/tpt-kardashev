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

## Sibling review (todo §1.2: bounded-drift assumption)

- `tpt-solutions/tpt-replicate`, `src/clock.rs`, read, not copied. Its hybrid logical clock assumes that the skew between clocks is bounded, and enforces the bound by rejecting a remote timestamp more than `max_drift_ms` ahead of local physical time (default 60 000 ms, `DEFAULT_MAX_DRIFT_MS`). The bound is a configured parameter, not derived from a clock model. The ordering itself does not depend on the bound; only acceptance does. A physical clock model that bounds skew (for example, the relativistic rate relations in this crate) would give a principled value for the parameter, but the sibling crate does not derive one.
