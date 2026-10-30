# tpt-kardashev: todo

Foundation libraries for planetary-to-galactic scale. Ten independent Rust crates in one Cargo workspace.
Spec: `spec.txt`. Licence: MIT OR Apache-2.0. Owner: TPT Solutions.

## Decisions recorded (2026-10-30)

- Repo name: `tpt-kardashev`; crates are `tpt-kardashev-<thing>`.
- Toolchain: pinned Rust 1.90.0, MSRV 1.90.
- Publishing: git-only for now (tags `<crate>-v<x.y.z>`); crates.io is a later decision.
- Dependency licences: MIT-selectable only. Apache-2.0-only denied, GPL family denied, MPL-2.0 denied, `Unicode-3.0` denied. Dev-dependencies exempt.
- DTN reliability: v0.1 uses LTP-level reliability only; bundle-level reliability extension deferred.
- SGP4/SDP4 stays inside `tpt-kardashev-astrodynamics` behind a feature.
- Nuclear-data cross-section format: chosen by a research task at the start of the nuclear-plasma crate.
- Prior-art checks (tpt-rfc, tpt-aion, existing Lambert solver) are the first items in the relevant crate sections.

- [x] Read spec and resolve open questions (§12.1, .2, .3, .4, .5, .8, .9)
- [ ] Update `spec.txt` §12 and §9 to record the decisions above

---

## Phase 0: Scaffold

Exit: CI green on empty crates.

### Workspace and licensing
- [ ] Create root `Cargo.toml` workspace with `[workspace.package]` (edition, rust-version 1.90, authors/owner TPT Solutions, licence `MIT OR Apache-2.0`, repository)
- [ ] Add `rust-toolchain.toml` pinned to 1.90.0 (components: rustfmt, clippy; target thumbv7em-none-eabihf)
- [ ] Add `LICENSE-MIT` (copyright TPT Solutions)
- [ ] Add `LICENSE-APACHE` (Apache-2.0 text, copyright TPT Solutions)
- [ ] Add licence section to root `README.md` (MIT OR Apache-2.0, contribution clause for dual licensing)
- [ ] Add SPDX header template (`// SPDX-License-Identifier: MIT OR Apache-2.0`) and apply to all source files
- [ ] Add `rustfmt.toml`
- [ ] Add `clippy.toml`
- [ ] Add `[workspace.lints]` (`unsafe_code = "forbid"`, `clippy::all = "warn"`, no-unwrap lint for library code)
- [ ] Add `.gitignore`, `.editorconfig`
- [ ] Initialise git repository and make first commit

### Dependency licence policy (§4.5)
- [ ] Write `deny.toml` with explicit allow-list: MIT, MIT-0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unlicense, CC0-1.0
- [ ] Confirm dual/compound expressions with MIT as an option pass (e.g. `MIT OR Apache-2.0`)
- [ ] Confirm Apache-2.0-only (incl. `WITH LLVM-exception` without MIT alternative), GPL/LGPL/AGPL, MPL-2.0 and Unicode-3.0 are denied
- [ ] Confirm unknown/missing licence fields fail
- [ ] Exclude dev-dependencies (`exclude-dev = true` in `[graph]`; verify key against installed cargo-deny version)
- [ ] Add advisories and bans sections to `deny.toml`
- [ ] Test the policy with a deliberately Apache-only dummy dependency (must fail) and an `MIT OR Apache-2.0` one (must pass)
- [ ] Document the no-copying rule for third-party code (README or CONTRIBUTING)

### Crate skeletons
- [ ] Create ten crate directories under `crates/` named `tpt-kardashev-<thing>` (archive-codes, relativity, radhard, energy-ledger, dtn, astrodynamics, nuclear-plasma, causal-consensus, isru, reflect)
- [ ] Each crate: `Cargo.toml` inheriting workspace package/lints, independent version `0.0.0`
- [ ] Each crate: `#![no_std]` `lib.rs` with `alloc` and `std` features, `#![forbid(unsafe_code)]` (radhard exception documented)
- [ ] Each crate: `README.md` (one-line description, what it does not model), `CHANGELOG.md`, `examples/`
- [ ] Each crate: `docs/provenance/<crate>.md` stub (origin of ported code, licence of any third-party source)
- [ ] Add placeholder `examples/` directory at repo root for cross-crate examples
- [ ] Optional feature flags declared per §4.2 (relativity, dtn, fec, energy) with no mandatory internal deps
- [ ] Verify no sibling `path = "../tpt-xyz"` dependencies outside this workspace

### Repo docs
- [ ] Root `README.md` (crate table, dependency graph, design principles, non-goals)
- [ ] `CLAUDE.md` (conventions, commands, rules: no unwrap, no panics, no sibling path deps, deterministic, no AI attribution in commits)
- [ ] Per-crate sections in this `todo.md` kept in sync as work completes
- [ ] Floating-point tolerance policy doc (shared template for physics crates)
- [ ] Define the admission rule for new crates in `CONTRIBUTING.md` (one concern, small trait API, no app code, builds alone)

### CI (`.github/workflows/`)
- [ ] `fmt` check
- [ ] `clippy -D warnings`
- [ ] Tests with default features and with all features
- [ ] `no_std` build, including thumbv7em-none-eabihf for crates claiming embedded use
- [ ] "Builds alone" job: `cargo build -p <crate> --no-default-features` for each crate
- [ ] `cargo deny check licenses` on full tree, all features and default features
- [ ] `cargo deny check advisories`
- [ ] Doc build with warnings denied
- [ ] SPDX header and dual-licence file enforcement check
- [ ] Scheduled Kani job
- [ ] Scheduled fuzz job
- [ ] Verify CI is green on empty crates (**Phase 0 exit**)

### Cross-cutting infrastructure
- [ ] Shared test conventions: recorded proptest seeds, tolerance helper pattern (per-crate, not shared crate)
- [ ] Decide benchmark setup (criterion as dev-dependency) and a regression-check convention
- [ ] Fuzz harness layout (`cargo-fuzz`, per-crate `fuzz/`) and a template

---

## Phase 1: Leaf crates

Exit: each crate at its v0.1 exit.

### 1.1 `tpt-kardashev-archive-codes`
- [ ] Investigate `tpt-aion` and `tpt-rfc` for existing archival coding/CRC (§12.6); record result in provenance
- [ ] Obtain current CCSDS 131.0-B revision; record parameters, polynomials, sync markers from the document (not memory)
- [ ] Define error type (`CodeError`, `#[non_exhaustive]`), `DecodeReport`, and traits (`BlockEncoder`, `BlockDecoder`, `SoftDecoder`, `ErasureCoder`)
- [ ] GF(2^8) arithmetic (in-crate, tables generated at const time)
- [ ] Reed-Solomon RS(255,223) encoder with const generics, no alloc
- [ ] Reed-Solomon decoder (syndromes, Berlekamp-Massey, Chien, Forney), erasure support
- [ ] Block interleaver / de-interleaver (depth configurable)
- [ ] CCSDS dual-basis (conventional/Berlekamp) representation handling
- [ ] CRC variants (CRC-16, CRC-32, others as needed) in-crate
- [ ] Attached sync markers (ASM) insert/detect
- [ ] Convolutional encoder, rate 1/2, K=7
- [ ] Viterbi decoder, hard decision
- [ ] Viterbi decoder, soft decision
- [ ] Self-describing archival framing (header with code parameters, versioned)
- [ ] LDPC block codes with iterative belief-propagation decoder (soft input)
- [ ] LT-style fountain/erasure codes encoder and decoder session
- [ ] Optional `serde` feature on data types
- [ ] Known-answer tests from the CCSDS standard
- [ ] Round-trip property tests (≤ t symbol errors always recovered)
- [ ] Fuzz: decoders and frame parser never panic on arbitrary input
- [ ] Kani proofs for selected arithmetic (GF ops)
- [ ] Benchmarks: BER curves vs theory as regression checks
- [ ] `no_std` and no-alloc build for fixed-size paths; thumbv7em build
- [ ] README states what is not modelled (no compliance claim; turbo codes deferred; RaptorQ not included)
- [ ] Examples (RS + interleave round trip, archival frame)
- [ ] **v0.1 exit:** RS(255,223)+interleaving with vectors; conv K=7 + Viterbi; CRC; framing; no_std/no-alloc

### 1.2 `tpt-kardashev-relativity`
- [ ] Review `tpt-replicate/src/clock.rs` for the bounded-drift assumption; note in provenance
- [ ] Define `Scalar` trait (default `f64`), error enum, `Seconds`/`Metres` newtypes
- [ ] `Event`, `FourVector`, `Interval` (timelike/spacelike/lightlike)
- [ ] `Lorentz` boost with `|v| < c` enforced; composition via rapidity
- [ ] Time dilation, length contraction, relativistic Doppler
- [ ] Inertial `Frame` and frame-to-frame transforms
- [ ] Causal predicates: `precedes`, `light_cone_contains`, `causally_disconnected`
- [ ] `CausalOrder` trait (neutral, consumed by other crates)
- [ ] `proper_time(path)`
- [ ] Weak-field gravitational time dilation (Schwarzschild approximation)
- [ ] Proper time vs coordinate time for a moving, gravitating observer
- [ ] Time-scale rate relations TT, TCG, TCB, TDB as functions (IAU constants documented)
- [ ] `Metric` trait with Minkowski and Schwarzschild implementations
- [ ] Documented float-tolerance policy
- [ ] Property tests: interval invariance under boosts, composition law, `|v| ≥ c` rejected
- [ ] Known-value regression: GPS ≈ +38 µs/day net offset
- [ ] `no_std` build
- [ ] README states what is not modelled (no geodesic integration, no strong-field)
- [ ] Examples
- [ ] **v0.1 exit:** events, intervals, boosts, proper time, causality, Doppler, weak-field redshift, tolerance policy

### 1.3 `tpt-kardashev-radhard`
- [ ] Review `tpt-equinox-tmr` and `tpt-for-redundancy`; record in provenance
- [ ] `Tmr<T>` with majority vote and scrub-on-mismatch
- [ ] N-modular variant
- [ ] Typestate wrappers (unvoted vs voted types)
- [ ] SECDED (72,64) encode/decode
- [ ] CRC-protected cells and checksummed buffers
- [ ] `Checkpoint<T>` with rollback and watchdog-friendly progress marker
- [ ] `FaultInjector` (seeded bit flips, burst errors)
- [ ] Volatile access behind a feature, `unsafe` documented (the one exception to forbid-unsafe)
- [ ] Kani proof: no single corrupted replica changes the voter result
- [ ] Kani proof: SECDED corrects every single-bit error and detects every double-bit error
- [ ] Exhaustive tests over small word sizes
- [ ] Injector-driven property tests
- [ ] `no_std` build, thumbv7em build
- [ ] README states what is not modelled
- [ ] Examples
- [ ] **v0.1 exit:** Tmr, SECDED(72,64), checksummed cells, checkpoint/rollback, injector, Kani proofs

### 1.4 `tpt-kardashev-energy-ledger`
- [ ] Review `tpt-equinox-power`, `tpt-energy`, `tpt-thermodynamics` (physics only, no code duplication)
- [ ] Newtypes `Joules`, `Watts`, `Kelvin`
- [ ] Physical constants (k_B, σ) with CODATA source and year recorded
- [ ] Double-entry `Ledger` with explicit `WasteHeat` account
- [ ] Declared sources and sinks; checked conservation invariant
- [ ] `landauer_limit(bits_erased, temperature)`
- [ ] Radiator capacity from Stefan-Boltzmann (`ε σ A T⁴`)
- [ ] `HeatBudget` with headroom report
- [ ] `EnergyOracle` trait
- [ ] Optional `units` feature mapping to `tpt-math-units`
- [ ] Optional `serde` feature
- [ ] Conservation property tests (random transfer sequences)
- [ ] Constants cross-checked against CODATA; hand-calculation tests for Landauer (≈2.9×10⁻²¹ J/bit at 300 K) and radiator
- [ ] `no_std` build
- [ ] README states what is not modelled
- [ ] Example: compute node with radiator
- [ ] **v0.1 exit:** ledger with checks, Landauer, radiator, oracle, example

### Phase 1 close
- [ ] All four crates at v0.1 exit
- [ ] Tag `<crate>-v0.1.0` for each (git-only)
- [ ] CI green for all four, including cargo-deny

---

## Phase 2: Networking, orbits, plasma

Exit: each crate at v0.1 exit; `ContactPlan` flow from astrodynamics to dtn demonstrated in an example.

### 2.1 `tpt-kardashev-dtn`
- [ ] Review `substrate-dtn` and `tpt-zenith/routing`; record in provenance
- [ ] Check `tpt-rfc` / `tpt-aion` for existing CBOR before writing one (§12.6)
- [ ] Confirm current RFC 9171 (BPv7), RFC 5326 (LTP), CCSDS 734.1-B and SABR documents
- [ ] Minimal in-crate CBOR codec (definite and indefinite lengths as required by BPv7)
- [ ] Endpoint ids: `dtn:` and `ipn:`
- [ ] Primary block, canonical blocks (payload, previous node, bundle age, hop count)
- [ ] Bundle CRCs (CRC-16 X.25, CRC-32C)
- [ ] `Bundle` encode/decode with strict validation
- [ ] `BundleStore` trait; in-memory store
- [ ] File-backed store
- [ ] `ContactPlan` and `Contact` types (start, end, rate, one-way light time)
- [ ] Contact-graph router (CGR/SABR family), `Router` trait
- [ ] `ConvergenceLayer` trait
- [ ] LTP convergence layer (segments, sessions, reliability via report/ack, timers on virtual clock)
- [ ] Reliability note: bundle-level custody/reliability extension deferred, documented as such
- [ ] Optional `fec` feature wiring to `tpt-kardashev-archive-codes`
- [ ] Deterministic simulator with virtual clock and contact-plan-driven links
- [ ] Seeded fault injection (loss, delay, partition, duplication, reordering)
- [ ] RFC 9171 appendix examples as byte-exact vectors
- [ ] Round-trip tests and fuzzing of CBOR/bundle parser
- [ ] Routing tests against hand-computed contact-plan scenarios
- [ ] Simulator tests for delivery under disruption
- [ ] Optional `serde` feature
- [ ] `no_std` + `alloc` build
- [ ] README states what is not modelled (no compliance claim; no BPSec, no TCPCL in v0.1)
- [ ] **v0.1 exit:** BPv7 codec with vectors, stores, CGR router, LTP layer, simulator

### 2.2 `tpt-kardashev-astrodynamics`
- [ ] Check `tpt-sci-astro` and `tpt-transportation` for an existing Lambert solver (§12.7); record in provenance
- [ ] Review `tpt-zenith/orbital-mechanics` and `tpt-equinox-orbital`; record in provenance
- [ ] Define `Scalar`, units newtypes (`Kilometres`, `Seconds`), error enum
- [ ] Epochs and time scales (UTC, TAI, TT, TDB), Julian dates, leap-second handling (user-supplied table, no bundled data)
- [ ] Optional `relativity` feature for relativistic scale relations
- [ ] Reference frames: inertial (GCRF-like), Earth-fixed, perifocal, RTN/LVLH, heliocentric ecliptic, and transforms between them
- [ ] Element sets (Keplerian, equinoctial) and state-vector conversions
- [ ] Two-body propagation (universal variables)
- [ ] `ForceModel` trait
- [ ] Forces: central body, zonal harmonics J2…Jn, third bodies, SRP with shadow model, atmospheric drag, thrust
- [ ] Cowell combined numerical propagator
- [ ] Integrator: fixed-step RK4
- [ ] Integrator: adaptive high-order Runge-Kutta
- [ ] Integrator: symplectic for N-body
- [ ] Lambert solver
- [ ] Impulsive manoeuvres: Hohmann, bi-elliptic, plane change
- [ ] `Ephemeris` trait with low-precision analytic provider
- [ ] SPK (Chebyshev) reader behind a feature (no ephemeris data bundled)
- [ ] `sgp4` feature: SGP4/SDP4 and TLE parsing (port from zenith)
- [ ] Visibility and contact-window computation
- [ ] Optional `dtn` feature: emit `ContactPlan` from visibility windows
- [ ] SGP4 reference vector tests
- [ ] Textbook Lambert and Kepler cases
- [ ] Energy and angular-momentum drift checks (symplectic, adaptive) over long arcs
- [ ] Frame round-trip tests
- [ ] Cross-check new force model against old secular formulas (J2, drag, etc.)
- [ ] Fuzz: TLE and SPK parsers
- [ ] `no_std` + `alloc` build
- [ ] README states force-model limits and what is not modelled
- [ ] Low-thrust (Edelbaum-type) approximations (post-v0.1)
- [ ] **v0.1 exit:** time scales, frames, elements, two-body + Cowell (J2/third-body/SRP/drag), RK4 + adaptive, Lambert, impulsive manoeuvres, analytic ephemeris, SGP4 feature

### 2.3 `tpt-kardashev-nuclear-plasma`
- [ ] Review `tpt-ignis` (`tpt-physics-math`, `tpt-aether`); record in provenance; note overlap on equilibrium/MHD
- [ ] Research: choose first cross-section format (ENDF-6, ACE, OpenMC multigroup, simple multigroup text); check licences of any reference parsers; record decision
- [ ] Decide feature layout (`equilibrium`, `neutronics`, `mhd`) and the sub-crate split trigger (size)
- [ ] `Scalar` trait, error enum, grid types
- [ ] **equilibrium:** (R, Z) grid and fixed-boundary Grad-Shafranov solver
- [ ] equilibrium: flux-surface extraction
- [ ] equilibrium: safety-factor profile
- [ ] equilibrium: Solov'ev analytic benchmark
- [ ] equilibrium: convergence-order tests
- [ ] **neutronics:** constructive solid geometry
- [ ] neutronics: multigroup cross-section types and the chosen format reader
- [ ] neutronics: seeded RNG and fixed-source Monte Carlo transport
- [ ] neutronics: tallies (flux, reaction rates)
- [ ] neutronics: implicit capture, Russian roulette, splitting
- [ ] neutronics: slab-attenuation (exponential law) validation
- [ ] neutronics: infinite-medium validation
- [ ] neutronics: statistical tests with fixed seeds
- [ ] Optional features for `tpt-math-prob-monte-carlo`, `tpt-math-pde-operators`, `tpt-math-linalg-*` (licence-checked)
- [ ] Fuzz: cross-section file reader
- [ ] `no_std` + `alloc` build for the core where feasible
- [ ] README states what is not modelled (no bundled data)
- [ ] Later (post-v0.1): eigenvalue/criticality problems, free-boundary equilibrium, MHD 1D then 2D
- [ ] **v0.1 exit:** fixed-boundary G-S with Solov'ev, fixed-source MC with slab validation, tallies, one documented cross-section format

### Phase 2 close
- [ ] Example: astrodynamics visibility windows → `ContactPlan` → dtn routing (uses feature flags)
- [ ] All three crates at v0.1 exit
- [ ] Tag `<crate>-v0.1.0` for each
- [ ] CI green, including cargo-deny with all features

---

## Phase 3: Agreement, resources, self-modification

Exit: each crate at v0.1 exit.

### 3.1 `tpt-kardashev-causal-consensus`
- [ ] Review `tpt-replicate`, `hiveforge-consensus`, `tpt-scheduler`, `tpt-fabric`; record in provenance
- [ ] Hybrid logical clock and vector clock
- [ ] Causally ordered operation log
- [ ] Optional `relativity` feature: `CausalOrder` as ordering source
- [ ] Domain layer: Raft-style leader election and log replication within a domain
- [ ] `Transport` trait (pluggable)
- [ ] Federation layer: signed, hash-chained epoch summaries
- [ ] Optional `tpt-crypto` feature for signatures/hashes (licence-checked)
- [ ] `ConsistencyLevel`: Commutative, Causal, DomainLinearizable, GlobalAgreed
- [ ] API reporting latency lower bound per level
- [ ] `Domain`, `Federation`, `Proposal<Op>` types
- [ ] Deterministic simulator with partitions, delay, duplication, reordering
- [ ] Invariant checks: no conflicting `GlobalAgreed` commits; causal order never violated
- [ ] Linearizability checker for small domain-layer histories
- [ ] Randomised partition/delay simulation tests
- [ ] Fuzz: message decoders
- [ ] `no_std` + `alloc` build
- [ ] README states honest limits (CAP under partition); Byzantine tolerance deferred
- [ ] Later: Byzantine tolerance behind a feature
- [ ] **v0.1 exit:** causal layer, domain consensus, federation epochs, consistency API with bounds, simulator

### 3.2 `tpt-kardashev-isru`
- [ ] Review `tpt-process` (`tpt-proc-mass-balance`, `tpt-proc-flowsheet`), `tpt-systems-optimisation`, `hiveforge-resources`; record in provenance
- [ ] Types: `Element`, `Material` (mass fractions), `Resource`, `Process`
- [ ] Elemental mass-balance validation (reject unbalanced processes at load time, with tolerance)
- [ ] Atomic-mass data source for elements (documented, licence recorded)
- [ ] Process/resource loading format (document it)
- [ ] LP formulation over a time horizon
- [ ] Built-in simplex solver for small problems
- [ ] Solver adapter trait for external solvers
- [ ] Closure scores (bill of materials and energy) and list of limiting imports
- [ ] Optional `energy` feature: accounts via `tpt-kardashev-energy-ledger`
- [ ] Conservation tests (total mass and each element through any plan)
- [ ] Closed-form toy systems with known optima
- [ ] Closure monotonicity test (adding a process never lowers closure)
- [ ] Worked lunar-regolith example
- [ ] `no_std` + `alloc` build
- [ ] README states what is not modelled
- [ ] **v0.1 exit:** types, balance validation, LP planner, closure score, lunar-regolith example

### 3.3 `tpt-kardashev-reflect`
- [ ] Review `tpt-formal`, `tpt-writ`, `tpt-norma` (queued); note interoperability with norma
- [ ] `Invariant` type: linear inequality systems over policy parameters
- [ ] `Update` and `Certificate` types
- [ ] Exact or rational arithmetic choice for the checker (in-crate, no float surprises); document
- [ ] `Verifier`: Farkas-style containment certificate checker
- [ ] Certificate generation helper (not trusted) for tests/examples
- [ ] `UpdateGate` issuing hash-chained `Receipt` or rejection with reason
- [ ] Audit-log verification
- [ ] Rule: checker not modifiable through the gate; external attestation path for replacement (design and docs)
- [ ] Hash function choice (in-crate or `tpt-crypto` feature, licence-checked)
- [ ] Kani proofs for checker arithmetic core
- [ ] Soundness property tests with random polytopes
- [ ] Adversarial tests: malformed and truncated certificates
- [ ] Fuzz: certificate parser
- [ ] `no_std` + `alloc` build
- [ ] README states open research risk and that only linear bounds are supported
- [ ] Later: optional adapters to `tpt-formal` and `tpt-solver`
- [ ] **v0.1 exit:** linear invariants, containment certificates + checker, gate with receipts, audit-chain verification

### Phase 3 close
- [ ] All three crates at v0.1 exit
- [ ] Tag `<crate>-v0.1.0` for each
- [ ] CI green

---

## Phase 4: Consolidation

Exit: per-repo migration plans (notes, not code changes).

- [ ] Migration notes: `tpt-zenith` (orbital-mechanics, rf-pipeline fec, routing) → astrodynamics, archive-codes, dtn
- [ ] Migration notes: `substrate` (substrate-dtn) → dtn
- [ ] Migration notes: `tpt-equinox` (tmr, orbital) → radhard, astrodynamics
- [ ] Migration notes: `tpt-sci-astro` → astrodynamics
- [ ] Migration notes: `tpt-transportation` (space) → astrodynamics
- [ ] Migration notes: `tpt-formal` (tpt-for-redundancy, proof AST) → radhard, reflect
- [ ] Decide ignis adoption of nuclear-plasma (coordinate when ignis reaches physics phase)
- [ ] Cross-check tests of new crates against old implementations; record divergences
- [ ] Public trait consolidation pass; decide path to 1.0
- [ ] Decide on crates.io publishing and optional umbrella crate `tpt-kardashev`
- [ ] Decide `tpt-substrate` licence inconsistency (MIT only vs dual) if it is to become a dependency
- [ ] Final spec update and retrospective

---

## Ongoing

- [ ] Keep every crate's README "what this does not model" section current
- [ ] Keep `docs/provenance/` current whenever code is ported
- [ ] Re-verify standards parameters against current documents when work on a crate starts
- [ ] Re-run `cargo deny` whenever a dependency is added or bumped
- [ ] No Apache-only or GPL-family code pasted into library source
