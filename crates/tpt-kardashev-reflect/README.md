# tpt-kardashev-reflect

Verified self-modification: linear-invariant containment certificates, a checker and an audited update gate.

**Status:** Phase 3.3, in progress. Implemented: exact rational arithmetic (always available), linear constraint systems, Farkas-style containment certificates, the exact checker, the update gate with a hash-chained, auditable log, and (feature `crypto`) an attestation path for replacing the invariant. Not yet: certificate generation, Kani proofs, fuzzing.

**Open research risk.** The checker is sound for the systems it supports, which are linear bounds only. Whether a policy that changes itself under a checked invariant stays safe in practice depends on the invariant being the right one; the checker proves containment, not that the invariant is wise.

## Features

- `std` (default): links the standard library.
- `alloc`: constraint systems, certificates, the checker, the gate and the audit. Without it the crate has rationals, errors and the hash, so it builds for `thumbv7em`.

## The rule

- The gate's invariant is fixed when it is built. No method changes it, and the tests check that refused and accepted submissions leave it as it was.
- The policy changes only through `UpdateGate::submit`, and only when the certificate verifies.
- Replacing the invariant is possible only through `attest::AttestedGate` (feature `crypto`), under an Ed25519 signature by a fixed authority. Choosing and rotating that authority is outside the crate.

## What is supported

- Linear bounds `coefficients · x <= bound` over exact rationals. No nonlinear constraints, no integer programming, no strict inequalities.
- Containment of one such system (the update) in another (the invariant), proved by a certificate of non-negative multipliers. The check is exact: no tolerance. Overflow is an error, never a wrapped or rounded value.
- An audit log of every submission, hash-chained with SHA-256. The audit checks the chain. It cannot re-check the updates, since the log keeps only their digests.

## What it does not model

- It does not find certificates. A certificate is an input; the checker only checks it. A generator would be a separate, untrusted helper.
- It does not handle Byzantine or adversarial policy authors beyond the malformed-certificate tests.
- It does not authenticate the log: the hash chain detects edits to a copy of the head, not a forged log.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
