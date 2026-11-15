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
