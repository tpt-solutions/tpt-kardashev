# Provenance: tpt-kardashev-relativity

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reference list (not yet reviewed)

These older repos are listed in the todo but have not been opened yet. Review is an open todo item.

- tpt-replicate/src/clock.rs: bounded-drift assumption (todo §1.2)

## Constants and sources

- Speed of light: exact, 299 792 458 m/s (SI definition).
- Time-scale rate constants in `src/timescale.rs`, checked 2026-11-16 against the IAU 2000 explanatory supplement (arXiv:astro-ph/0303376). `L_G` is the defining constant of IAU Resolution B1.9 (2000). `L_C` is the Irwin and Fukushima (1999) estimate. `L_B` is derived from them (1 - L_B = (1 - L_C)(1 - L_G)); an earlier version attributed it to IAU 2006 B3, which was wrong, and the value is now the full derived 1.55051976772e-8.
- GPS check values (GM, radii) are standard published figures, recorded in the test and example.

## Third-party licences

None.

## Sibling review (todo §1.2: bounded-drift assumption)

- `tpt-solutions/tpt-replicate`, `src/clock.rs`, read, not copied. Its hybrid logical clock assumes that the skew between clocks is bounded, and enforces the bound by rejecting a remote timestamp more than `max_drift_ms` ahead of local physical time (default 60 000 ms, `DEFAULT_MAX_DRIFT_MS`). The bound is a configured parameter, not derived from a clock model. The ordering itself does not depend on the bound; only acceptance does. A physical clock model that bounds skew (for example, the relativistic rate relations in this crate) would give a principled value for the parameter, but the sibling crate does not derive one.
