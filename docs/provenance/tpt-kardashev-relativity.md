# Provenance: tpt-kardashev-relativity

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reference list (not yet reviewed)

These older repos are listed in the todo but have not been opened yet. Review is an open todo item.

- tpt-replicate/src/clock.rs: bounded-drift assumption (todo §1.2)

## Constants and sources

- Speed of light: exact, 299 792 458 m/s (SI definition).
- Time-scale rate constants in `src/timescale.rs`, checked 2026-11-16 against the IERS Conventions 2010 appendix (IAU 2006 Resolution B3, item 14) and the IAU 2000 explanatory supplement (arXiv:astro-ph/0303376). `L_G` is the defining constant of IAU Resolution B1.9 (2000). `L_B` is the defining constant of IAU Resolution B3 (2006), 1.550519768e-8; the 2003 supplement gives the estimate 1.55051976772e-8 that it rounds. `L_C` is the Irwin and Fukushima (1999) estimate, an input and not a resolution value. An earlier revision of this file, and of the todo, called `L_B` derived and changed its value to the estimate; that was wrong, and the defined value is restored. The epoch offset TDB0 = -6.55e-5 s and T0 are not modelled.
- GPS check values (GM, radii) are standard published figures, recorded in the test and example.

## Third-party licences

None.

## Sibling review (todo §1.2: bounded-drift assumption)

- `tpt-solutions/tpt-replicate`, `src/clock.rs`, read, not copied. Its hybrid logical clock assumes that the skew between clocks is bounded, and enforces the bound by rejecting a remote timestamp more than `max_drift_ms` ahead of local physical time (default 60 000 ms, `DEFAULT_MAX_DRIFT_MS`). The bound is a configured parameter, not derived from a clock model. The ordering itself does not depend on the bound; only acceptance does. A physical clock model that bounds skew (for example, the relativistic rate relations in this crate) would give a principled value for the parameter, but the sibling crate does not derive one.
