# tpt-kardashev-isru

In-situ resource planning: elemental mass-balance checks, linear-program planning over a horizon, and (not yet) closure scores.

**Status:** Phase 3.2, in progress. Implemented: elements with atomic masses, compositions and materials, the elemental mass-balance check of a process, processes checked when built, a dense simplex solver behind a solver trait, a planner that maximises the amount of a target species made within stocks and run limits, closure against a demand with its limiting imports, and a text format for materials and processes (`format`). Not yet: the resource type, the energy account (by decision, the caller's), the measured regolith composition, and a time horizon.

## Features

- `std` (default): links the standard library.
- `alloc`: enables the processes, the planner and the simplex solver. The element, composition and balance checks need no heap.
- `energy` is not a feature. An in-repo library crate may not depend on the energy-ledger crate, so the caller combines the two.

## Conventions

- Masses are in kg, atomic masses in g/mol. Atomic masses are the midpoints of the CIAAW intervals (https://ciaaw.org/atomic-weights.htm), so an element's mass is taken to the precision of that table.
- A process balances when the total mass and the mass of each element agree to a relative tolerance of 1e-6 (`plan::BALANCE_TOLERANCE`).
- A plan is a linear program over the runs of each process. A horizon here is the stock on hand and the run limits; there are no time-indexed stocks yet.

## What it does not model

- No time: a plan does not sequence runs, schedule power, or account for the time a process takes.
- No energy: the energy account and closure in energy terms are not written.
- No closure score yet, and no list of limiting imports.
- Compositions are elemental. A species' oxide or mineral form is not modelled, only its elements.
- The simplex is dense and suits small problems only; it has no sparse or revised form.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.

## Example

- `cargo run --example regolith --features alloc`: two reductions of an iron oxide standing in for regolith, planned against a demand for iron and oxygen, with the closure and the imports. The ore is an illustration, not a measured lunar composition.

## Loading format

- `format::read_problem` reads materials and processes from a versioned text file (see the module docs). A process is checked for balance when it is read; a malformed or unbalanced file is rejected with the line number.
