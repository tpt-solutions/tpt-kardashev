# Changelog: tpt-kardashev-energy-ledger

## Unreleased

- `Joules`, `Watts`, `Kelvin` newtypes.
- Fixed-capacity double-entry `Ledger<N>` with flow rules and conservation check.
- Landauer limit and Stefan-Boltzmann radiator capacity; CODATA sigma derived from exact constants in a test.
- `HeatBudget` and the `EnergyOracle` trait with `ErasureOracle`.
- Seeded conservation property tests; `compute_node` example.
