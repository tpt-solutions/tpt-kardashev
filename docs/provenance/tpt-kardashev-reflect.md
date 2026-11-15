# Provenance: tpt-kardashev-reflect

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reviewed for reference (not copied)

- tpt-formal, tpt-writ, tpt-norma (queued): interoperability with norma (todo §3.3)

## Third-party licences

None.

## Written from standard relations

- `verify.rs` and `invariant.rs`: the Farkas lemma for linear inequalities, in the form used for containment: a system `A x <= b` implies `c·x <= d` when some non-negative multiplier vector gives `λ^T A = c^T` and `λ^T b <= d`. The soundness argument is in the module docs. Written from that standard form; no reference code consulted.
- `rational.rs`: exact rational arithmetic on checked `i128`, with the usual reduction, written here. No reference code consulted.
- `gate.rs`: the gate and the receipt chain are this crate's design: a fixed invariant, the policy changed only on a verified certificate, each receipt naming the hash of the one before with a domain-separation prefix. No reference code consulted.
- `hash.rs`: a copy of the causal-consensus crate's SHA-256 module (FIPS 180-4), by the same author. Kept as a copy because no crate may depend on another; see the phase 4 consolidation item.
