# Trait consolidation and the path to 1.0

Recorded 2026-11-16. Covers the public traits of every crate, the cross-crate policy, and the criteria
for 1.0. The todo items for the consolidation pass and the 1.0 decision refer to this file.

## Inventory

| Crate | Trait | Methods | Implementations in repo | Consumer |
| --- | --- | --- | --- | --- |
| archive-codes | `BlockEncoder` | `encode` | 2 (RS, conv) | examples, tests |
| archive-codes | `BlockDecoder` | `decode` | 2 | tests |
| archive-codes | `SoftDecoder` | `decode_soft` | 1 (Viterbi) | tests |
| archive-codes | `ErasureCoder` | `decode_erasures` | 2 (LT, RS) | tests |
| astrodynamics | `Ephemeris` | `geocentric_position` | 5 (incl. test doubles) | `ThirdBody`, `SolarRadiationPressure` |
| astrodynamics | `ForceModel` | `acceleration` | 13 | integrators, `Cowell`, `AdaptiveCowell` |
| causal-consensus | `Transport<M>` | `send`, `deliver` | 1 (`SimNetwork`) | domain and global layers |
| dtn | `ConvergenceLayer` | 7 methods | 1 (`LtpConvergence`) | `sim::Network` |
| dtn | `Router` | `route` | 1 (`EarliestArrival`) | `sim::Network` |
| dtn | `BundleStore` | 5 + 1 default method | 2 (memory, file) | store tests |
| energy-ledger | `EnergyOracle` | `cost`, `remaining_budget` | 1 (example) | the caller (by design) |
| isru | `LpSolver` | `maximise` | 6 (incl. test doubles) | `plan_maximum_with` |
| relativity | `CausalOrder` | `precedes` | 1 (`Event`) | none in this repo |
| relativity | `Metric` | `g` | 2 (Minkowski, Schwarzschild) | geodesic-free uses only |

## Findings

1. **No shared trait crate.** The admission rule (CONTRIBUTING.md) and CLAUDE.md forbid one crate depending
   on another in this repo, so a shared "core traits" crate would have to be depended on by all ten.
   Every trait stays in the crate that owns its concern. Two traits that want to be the same concept
   (`CausalOrder` in relativity and the ordering used by causal-consensus) stay separate.
2. **`CausalOrder` has no consumer.** `causal-consensus` would consume it under its optional `relativity`
   feature, which is blocked by the same rule. It stays, documented as a hook for a future adapter, and
   the causal-consensus `relativity` todo item stays open.
3. **`EnergyOracle` is the caller's extension point.** The energy account is decided to be the caller's
   (isru and energy-ledger). The trait is kept, with the example as its only implementation, and this is
   said in its module docs.
4. **Test doubles inflate counts.** `Ephemeris` and `LpSolver` have test implementations; they are not
   evidence of a second real implementation, and no trait is removed on that count.
5. **Errors.** Every crate has one `#[non_exhaustive]` error enum with a `Display` impl. The traits return
   those enums. No change is needed for 1.0.
6. **Stability.** Traits are unstable until 1.0. Their method sets may change in a 0.x release, and the
   changelog records each change.

## Decisions (2026-11-16)

- Keep every trait listed above in its crate. No trait is removed in this pass.
- No shared trait crate. Cross-crate use is by adapters outside both crates, or by a written format, as
  reflect/norma already decided.
- `CausalOrder` and `EnergyOracle` stay, with their status stated in their docs.
- Crates stay at `0.0.0`, `publish = false`, as already decided. Versions move to `0.1.0` only at the
  crate's v0.1 exit, and the Phase 1–3 tags are still a publishing-related decision for the owner.

## Path to 1.0

1.0 is not yet a goal. The criteria, all of which must hold first:

1. Every crate in Phases 1–3 has met its v0.1 exit (todo.md). Phase 3 and Phase 2 exits are not all met yet.
2. Open verification items that a 1.0 would be judged on are closed or explicitly accepted: the Kani
   coverage (see the reflect and radhard entries), fuzz campaigns of an hour or more per target, and the
   blocked physical constants (WGS 84 zonal coefficients, UT1 supply).
3. The trait list above has been reviewed again after the v0.1 exits, with any change recorded here.
4. The owner has decided on publishing (todo, Phase 4). Until then, 1.0 means a tagged, documented
   workspace release, not a crates.io release.

The "cross-check against old implementations" item stays open. The old implementations are in sibling
repositories that cannot be built inside this workspace, so the cross-checks are recorded per crate in the
provenance files (for example the SGP4 printed states and the TDB reference output), and the rest is not
claimed.
