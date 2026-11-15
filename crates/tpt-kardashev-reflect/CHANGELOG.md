# Changelog: tpt-kardashev-reflect

## Unreleased

- Phase 0 scaffold.
- `rational::Rational`: exact rationals on checked `i128`, reduced, with a positive denominator. Sums, products and comparisons report `Overflow` rather than wrapping. Tests in `tests/rational.rs`.
- `invariant::{Constraints, Row, Invariant, Update, Certificate}` (feature `alloc`): linear bounds over exact rationals, and Farkas-style certificates of containment. Tests in `tests/containment.rs`.
- `verify::verify` (feature `alloc`): the exact containment checker. Each invariant row needs non-negative multipliers over the update's rows that reproduce its coefficients and give a bound no larger than its own. `ReflectError` names each way a certificate can fail.
- `ReflectError` (`#[non_exhaustive]`).
