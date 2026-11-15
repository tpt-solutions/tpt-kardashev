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
- `attest::AttestedGate` and `attest::replacement_message` (feature `crypto`): the only way to replace the invariant. The replacement must carry an Ed25519 signature by a fixed authority over the old invariant's digest, the new digest and the log position, and the current policy must still be contained in the replacement. Refusals are recorded in the log. Tests in `tests/attest.rs`.
- Feature `crypto` (optional dependency `ed25519-compact`).
- `certify::certify` (feature `alloc`): a certificate generator, not part of the trusted base. A phase-one simplex over exact rationals (Bland's rule) finds the multipliers, so the output needs no rounding; `verify` decides whether it is accepted. Tests in `tests/certify.rs`: a narrower interval is certified and accepted, a wider one gets `NoCertificate`, every constructed containment is certified, and every returned certificate passes the checker.
- `ReflectError::NoCertificate`.
- `text` (feature `alloc`): a text format for constraint systems and certificates, version 1, with readers and writers. Every file declares its counts and ends with an `end` line, so a truncated file is rejected rather than read as a different one. Tests in `tests/text.rs`: exact round trips, a hand-written file, malformed lines with their numbers, truncated certificates, and the every-prefix property for both formats. The property found two truncation bugs, both fixed before this entry.
- `fuzz/`: the `text_read` target; a 2.5 minute run found no crash.
