# Provenance: tpt-kardashev-isru

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reviewed for reference (not copied)

- tpt-process (tpt-proc-mass-balance, tpt-proc-flowsheet), tpt-systems-optimisation, hiveforge-resources (todo §3.2)

## Third-party licences

None.

## Sources

- Atomic masses: the table of standard atomic weights of the Commission on Isotopic Abundances and Atomic Weights, https://ciaaw.org/atomic-weights.htm, read for this crate. Midpoints of the intervals are used for the elements that the table gives as intervals (H, C, N, O, Mg, S, Si); single values are used for the rest. The CIAAW's terms of use for the table were not checked and should be before release.

## Written from standard relations

- `simplex.rs`: the dense tableau simplex method for `maximise c·x` with `A·x <= b, x >= 0` and `b >= 0`, with Bland's rule. Written from the standard description; no reference code consulted. Tested on a textbook problem with a known optimum and an unbounded case.
- `plan.rs`: the planning formulation (run counts as variables; stock rows; run-limit rows; the target produced as the objective) is this crate's, from the mass balance. No reference code consulted.
- `material.rs`: the elemental mass balance (total mass and each element), written from the definition of conservation. No reference code consulted.
- `closure.rs`: the closure definition (in-situ net production over demand, capped at 1, the lowest ratio as the score) is this crate's, from the mass balance. No reference code consulted.
