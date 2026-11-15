# Changelog: tpt-kardashev-isru

## Unreleased

- `closure`: monotonicity over a horizon. Tests in `tests/closure_horizon.rs`: the score is non-decreasing as hematite stock grows and matches min(1, iron/demand); non-decreasing as the run limit grows, with stock capping the runs; adding a magnetite process never lowers the score and raises it for some demands.
- `element::Element`: the 14 elements of lunar regolith and the processes that make them, with standard atomic masses (midpoints of the CIAAW intervals, read from https://ciaaw.org/atomic-weights.htm for this crate).
- `material::Composition` and `material::Material`: mass fractions in a fixed array (no heap), validated to sum to 1 and to be in range. `material::check_balance`: the elemental mass balance of a process to a relative tolerance.
- `plan::Process` and `plan::plan_maximum` (feature `alloc`): a process is checked for elemental balance when it is built. The planner chooses run counts to maximise the amount of a target species, within stocks and run limits, as a linear program. Tests in `tests/planning.rs`: a textbook LP through the solver, an unbounded case, balanced and unbalanced processes, hematite reduction in exact proportions, hydrogen reduction against its closed form, element conservation through a plan, monotonicity when a route is added, and rejection of bad input.
- `simplex::maximise` (feature `alloc`): a dense simplex solver with Bland's rule, for `maximise c·x` subject to `A·x <= b, x >= 0` with `b >= 0`.
- `error::IsruError` (`#[non_exhaustive]`).
- Not written yet: closure scores, the energy account, the resource type, a solver adapter trait, the loading format, and the worked regolith example.
- `closure::closure` and `closure::Closure` (feature `alloc`): closure of a plan against a demand. Per demanded species, the in-situ net production over the demand, capped at 1; the score is the lowest ratio, and `limiting` and `imports_kg` name the shortfalls. Tests in `tests/closure.rs`.
- `simplex::LpSolver` trait and `simplex::Simplex`; `plan::plan_maximum_with` takes any solver. A counting adapter is tested.
- The empty `energy` feature is removed: the energy account is for the caller to combine with the energy ledger, since a library crate may not depend on it.
- `examples/regolith.rs` (`cargo run --example regolith --features alloc`): an illustrative plan, with hematite standing in for an iron-bearing regolith mineral, against a demand for iron and oxygen. Not a measured lunar composition.
- `format` (feature `alloc`): the loading format for materials and processes, version 1, with `read_problem`, `problem_to_text` and `Problem`. A process is checked for balance when read, and an unbalanced one is rejected at its line. Tests in `tests/format.rs`.
- `IsruError::Malformed { line }`.
- `Material` and `Process` names are owned strings now (so that a file can name them); `Material` needs `alloc`.
