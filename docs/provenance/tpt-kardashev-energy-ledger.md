# Provenance: tpt-kardashev-energy-ledger

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reference list (not yet reviewed)

These repos are listed in the todo but have not been opened yet. Review is an open todo item.

- tpt-equinox-power, tpt-energy, tpt-thermodynamics: physics only, no code duplication (todo §1.4)

## Constants and sources

- k_B, h, c: exact 2019 SI values.
- sigma: CODATA 2018 value 5.670374419e-8, cross-checked in a test against the value derived from the exact constants.

## Third-party licences

Runtime and optional dependencies: `serde` (optional), `tpt-math-units` (optional, `units` feature; see below). Dev-dependency: `serde_json`. All are MIT-selectable.

## Sibling review (todo §1.4: physics only)

- `tpt-solutions/tpt-thermodynamics`, read, not copied: the Boltzmann constant in its electrolyte and SAFT crates is 1.380649×10⁻²³ J/K, the same as `physics::BOLTZMANN`, which is exact by the SI definition. Its other constants are for other models.
- `tpt-solutions/tpt-energy`: the grid and storage crates. No Landauer or radiator physics.
- `tpt-solutions/tpt-equinox`, crates `tpt-equinox-power` (breaker topology and exclusion, a safety rule for a power bus, no energy physics) and `tpt-equinox-thermal` (radiator deployment control; its cooling coefficient is an arbitrary per-deployment constant, not the Stefan–Boltzmann law). Neither duplicates the ledger's radiator model.
- The Stefan–Boltzmann constant in this crate (5.670374419×10⁻⁸ W·m⁻²·K⁻⁴, CODATA 2018) is not in any sibling repo, so it is not cross-checked here.

## Third-party dependency: `units` feature

- `tpt-math-units` 0.1.0 (crates.io, MIT OR Apache-2.0; published by the TPT Solutions owner, source `github.com/tpt-solutions/tpt-math`). Optional, behind `units`, `default-features = false`. Used only as a typed wrapper around `uom`; no code is copied from it.
- Its transitive dependencies `uom` 0.38.0 and `num-traits` 0.2.19 are MIT OR Apache-2.0, and pass `cargo deny check licenses` with all features.
- Its declared `rust-version` is 1.84, below the workspace MSRV of 1.90.
