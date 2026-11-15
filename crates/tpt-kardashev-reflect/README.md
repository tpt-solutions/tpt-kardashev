# tpt-kardashev-reflect

Verified self-modification: linear-invariant containment certificates, a checker and an audited update gate.

**Status:** Phase 3.3, in progress. Implemented: exact rational arithmetic (always available), linear constraint systems, Farkas-style containment certificates, and the exact checker. Not yet: certificate generation, the update gate, receipts, the audit chain, the hash function, Kani proofs and fuzzing.

**Open research risk.** The checker is sound for the systems it supports, which are linear bounds only. Whether a policy that changes itself under a checked invariant stays safe in practice depends on the invariant being the right one; the checker proves containment, not that the invariant is wise.

## Features

- `std` (default): links the standard library.
- `alloc`: the constraint systems, certificates and checker. Without it, the crate has rationals and errors only, so it builds for `thumbv7em`.

## What is supported

- Linear bounds `coefficients · x <= bound` over exact rationals. No nonlinear constraints, no integer programming, no strict inequalities.
- Containment of one such system (the update) in another (the invariant), proved by a certificate of non-negative multipliers. The check is exact: no tolerance.
- Overflow is reported as an error, never wrapped or rounded.

## What it does not model

- It does not find certificates. A certificate is an input, and the checker only checks it. A generator would be a separate, untrusted helper.
- It does not gate updates, issue receipts, or keep an audit chain yet.
- It does not cover Byzantine or adversarial policy authors beyond the malformed-certificate tests.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
