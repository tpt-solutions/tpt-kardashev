# Changelog: tpt-kardashev-reflect

## Unreleased

- Phase 0 scaffold.
- `rational::Rational`: exact rationals on checked `i128`, reduced, with a positive denominator. Sums, products and comparisons report `Overflow` rather than wrapping. Tests in `tests/rational.rs`.
- `invariant::{Constraints, Row, Invariant, Update, Certificate}` (feature `alloc`): linear bounds over exact rationals, and Farkas-style certificates of containment. Tests in `tests/containment.rs`.
- `verify::verify` (feature `alloc`): the exact containment checker. Each invariant row needs non-negative multipliers over the update's rows that reproduce its coefficients and give a bound no larger than its own. `ReflectError` names each way a certificate can fail.
- `ReflectError` (`#[non_exhaustive]`).
- `gate::UpdateGate` and `gate::Receipt` (feature `alloc`): the only way the policy changes. A submission is checked against the fixed invariant with its certificate; an accepted update becomes the policy, and every submission is recorded in a hash-chained log with its outcome. The invariant cannot be changed through the gate. Tests in `tests/gate.rs`.
- `gate::verify_audit`, `gate::AuditError`: checks positions, links and receipt hashes of a log. Tests cover rewriting, reordering, removal and broken links.
- `hash`: SHA-256 (FIPS 180-4), a copy of the causal-consensus module, so that no crate depends on another. Consolidation is a phase 4 item.
