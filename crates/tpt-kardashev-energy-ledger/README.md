# tpt-kardashev-energy-ledger

Double-entry energy accounting with explicit waste heat, Landauer bounds, Stefan-Boltzmann radiator capacity and an `EnergyOracle` hook for schedulers.

**Status:** v0.1 candidate. The API is pre-1.0. Not published (`publish = false`).

## What it provides

- `Joules`, `Watts`, `Kelvin` newtypes (SI).
- `Ledger<N>`: a fixed-capacity double-entry ledger with `Source`, `Store`, `Sink` and `WasteHeat` accounts. Flow rules are enforced, balances never go negative, and `check_conservation` compares the total with the declared total.
- `landauer_limit(bits, T)`: `bits * k_B * T * ln 2` as a lower-bound reporting aid.
- `radiator_capacity(emissivity, area, T)`: `emissivity * sigma * area * T^4`.
- `HeatBudget`: radiator capacity, current load and headroom.
- `EnergyOracle` trait, with `ErasureOracle` as the Landauer-based implementation.

## Constants

`k_B`, `h` and `c` are the exact 2019 SI values. `sigma` is 5.670374419e-8 W m^-2 K^-4 (CODATA 2018). A test derives `sigma` from the exact constants and checks it to 1e-8.

## What it does not model

- Supply systems, power electronics, batteries or conversion losses. Those stay in `tpt-equinox-power` and `tpt-energy`.
- Thermal transients, radiator view factors, or temperature-dependent emissivity.
- Device-level energy cost of computation. Landauer is a lower bound, not a model of any device.
- The optional `units` and `serde` features from the todo. They are not implemented.

## Examples

```sh
cargo run -p tpt-kardashev-energy-ledger --example compute_node
```

Runs ten one-second cycles of a compute node with a 10 m^2 radiator and checks the ledger's conservation and the heat headroom.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
