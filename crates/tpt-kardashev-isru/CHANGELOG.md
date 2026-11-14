# Changelog: tpt-kardashev-isru

## Unreleased

- `element::Element`: the 14 elements of lunar regolith and the processes that make them, with standard atomic masses (midpoints of the CIAAW intervals, read from https://ciaaw.org/atomic-weights.htm for this crate).
- `material::Composition` and `material::Material`: mass fractions in a fixed array (no heap), validated to sum to 1 and to be in range. `material::check_balance`: the elemental mass balance of a process to a relative tolerance.
- `plan::Process` and `plan::plan_maximum` (feature `alloc`): a process is checked for elemental balance when it is built. The planner chooses run counts to maximise the amount of a target species, within stocks and run limits, as a linear program. Tests in `tests/planning.rs`: a textbook LP through the solver, an unbounded case, balanced and unbalanced processes, hematite reduction in exact proportions, hydrogen reduction against its closed form, element conservation through a plan, monotonicity when a route is added, and rejection of bad input.
- `simplex::maximise` (feature `alloc`): a dense simplex solver with Bland's rule, for `maximise c·x` subject to `A·x <= b, x >= 0` with `b >= 0`.
- `error::IsruError` (`#[non_exhaustive]`).
- Not written yet: closure scores, the energy account, the resource type, a solver adapter trait, the loading format, and the worked regolith example.
