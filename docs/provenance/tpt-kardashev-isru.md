# Provenance: tpt-kardashev-isru

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reviewed for reference (not copied)

- tpt-process (tpt-proc-mass-balance, tpt-proc-flowsheet), tpt-systems-optimisation, hiveforge-resources (todo §3.2). Read on 2026-11-18 at the level of READMEs, module docs and public API; function bodies were not read. Nothing copied.
  - `tpt-solutions/tpt-process`, `crates/simulation/tpt-proc-mass-balance`: overall and component balances, and a weighted least-squares reconciliation of one node. Not used here: this crate's closure is a ratio over the planner's own run counts.
  - `tpt-solutions/tpt-process`, `crates/simulation/tpt-proc-flowsheet`: a sequential-modular flowsheet solver with tear streams and Wegstein acceleration (RFC 0003). It solves a recycle-loop problem this crate does not have. The planner here is an LP, not a flowsheet.
  - `tpt-solutions/tpt-systems-optimisation`, `crates/tpt-opt-milp/src/lp.rs`: a two-phase primal simplex that uses Bland's rule, as the relaxation for a branch-and-bound MILP solver. It is a larger solver than `simplex.rs` here (sparse columns, duals and reduced costs). `simplex.rs` in this crate is independently written, and the two have the same textbook method.
  - `tpt-solutions/tpt-hiveforge`, `hiveforge-resources/src/lib.rs`: battery, feedstock and tool-wear budgets for a robot swarm, with charging stations and an exhaustion hook. No linear programming, no regolith chemistry. Nothing shared with this crate.

- Regolith composition (`examples/regolith.rs`): NASA/TM-20240011783, Revision A, "Lunar Regolith Simulant User's Guide" (Oct 2024), Table 6, the regional average oxide chemistry from the Lunar Sample Compendium (https://ntrs.nasa.gov/citations/20240011783, read for this crate). The values are the Apollo regional averages, with their totals 99.84 (highlands), 98.81 (low-Ti mare) and 99.23 (high-Ti mare) wt%. The same values appear in arXiv:2609.06496, Table 1, which cites this guide and the Lunar Sourcebook. Note: the low-Ti mare and highlands SiO2 are both 45.62 wt% in the source; this is not corrected here. The oxide-to-element conversion is this crate's example code, from the atomic masses above.

## Third-party licences

None.

## Sources

- Atomic masses: the table of standard atomic weights of the Commission on Isotopic Abundances and Atomic Weights, https://ciaaw.org/atomic-weights.htm, read for this crate. Midpoints of the intervals are used for the elements that the table gives as intervals (H, C, N, O, Mg, S, Si); single values are used for the rest. The CIAAW's terms of use for the table were not checked and should be before release.

## Written from standard relations

- `simplex.rs`: the dense tableau simplex method for `maximise c·x` with `A·x <= b, x >= 0` and `b >= 0`, with Bland's rule. Written from the standard description; no reference code consulted. Tested on a textbook problem with a known optimum and an unbounded case.
- `plan.rs`: the planning formulation (run counts as variables; stock rows; run-limit rows; the target produced as the objective) is this crate's, from the mass balance. No reference code consulted.
- `material.rs`: the elemental mass balance (total mass and each element), written from the definition of conservation. No reference code consulted.
- `closure.rs`: the closure definition (in-situ net production over demand, capped at 1, the lowest ratio as the score) is this crate's, from the mass balance. No reference code consulted.
- `format.rs`: the loading format is this crate's design, documented in the module; no third-party format copied.
