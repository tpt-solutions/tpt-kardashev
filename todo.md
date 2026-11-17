# tpt-kardashev: todo

Foundation libraries for planetary-to-galactic scale. Ten independent Rust crates in one Cargo workspace.
Spec: `spec.txt`. Licence: MIT OR Apache-2.0. Owner: TPT Solutions.

**Status (2026-11-04):** Phase 0 scaffold written and checked locally (fmt, clippy, tests, no_std thumbv7em, builds-alone, docs, cargo-deny licences and advisories, licence-policy test). CI is written but not yet run on GitHub; the Phase 0 exit ("CI green") stays open until it is pushed and passes. Committed to master in stages (scaffold, relativity, archive-codes).

## Decisions recorded (2026-10-30)

## Decisions recorded (2026-11-16)

- Test conventions: seeds are written in each test (in-crate `SplitMix64`, no proptest dependency; the sibling `tpt-for-det-proptest` cannot be a dependency). Tolerances: a helper per test file, as the crates already do.
- Benchmarks: no criterion dependency. Regression checks are tests with stated tolerances, and examples are timed by hand in release mode. Timing is not gated in CI.
- Fuzzing: layout and corpus rules are in `docs/fuzzing.md`.
- Resource: not a separate type. Stock is an array indexed by species, as the planner already takes it.
- Scalar: `f64` only for v0.1 in every crate; a generic `Scalar` trait is post-v0.1 in each crate that deferred it.
- `tpt-substrate` licence: no action, since nothing depends on it; the question is re-opened only if it becomes a dependency.


- Repo name: `tpt-kardashev`; crates are `tpt-kardashev-<thing>`.
- Toolchain: pinned Rust 1.90.0, MSRV 1.90.
- Publishing: git-only for now (tags `<crate>-v<x.y.z>`); crates.io is a later decision.
- Dependency licences: MIT-selectable only. Apache-2.0-only denied, GPL family denied, MPL-2.0 denied, `Unicode-3.0` denied. Dev-dependencies exempt.
- DTN reliability: v0.1 uses LTP-level reliability only; bundle-level reliability extension deferred.
- SGP4/SDP4 stays inside `tpt-kardashev-astrodynamics` behind a feature.
- Nuclear-data cross-section format: chosen by a research task at the start of the nuclear-plasma crate.
- Prior-art checks (tpt-rfc, tpt-aion, existing Lambert solver) are the first items in the relevant crate sections.

- [x] Read spec and resolve open questions (§12.1, .2, .3, .4, .5, .8, .9)
- [x] Update `spec.txt` §12 and §9 to record the decisions above

---

## Phase 0: Scaffold

Exit: CI green on empty crates.

### Workspace and licensing
- [x] Create root `Cargo.toml` workspace with `[workspace.package]` (edition, rust-version 1.90, authors/owner TPT Solutions, licence `MIT OR Apache-2.0`, repository) (repository URL left out until confirmed)
- [x] Add `rust-toolchain.toml` pinned to 1.90.0 (components: rustfmt, clippy; target thumbv7em-none-eabihf)
- [x] Add `LICENSE-MIT` (copyright TPT Solutions)
- [x] Add `LICENSE-APACHE` (Apache-2.0 text, copyright TPT Solutions)
- [x] Add licence section to root `README.md` (MIT OR Apache-2.0, contribution clause for dual licensing)
- [x] Add SPDX header template (`// SPDX-License-Identifier: MIT OR Apache-2.0`) and apply to all source files
- [x] Add `rustfmt.toml`
- [x] Add `clippy.toml`
- [x] Add `[workspace.lints]` (`unsafe_code = "forbid"`, `clippy::all = "warn"`, no-unwrap lint for library code)
- [x] Add `.gitignore`, `.editorconfig` (`.gitignore` already existed; `.editorconfig` added)
- [x] Initialise git repository and make first commit

### Dependency licence policy (§4.5)
- [x] Write `deny.toml` with explicit allow-list: MIT, MIT-0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unlicense, CC0-1.0
- [x] Confirm dual/compound expressions with MIT as an option pass (e.g. `MIT OR Apache-2.0`)
- [x] Confirm Apache-2.0-only (incl. `WITH LLVM-exception` without MIT alternative), GPL/LGPL/AGPL, MPL-2.0 and Unicode-3.0 are denied
- [x] Confirm unknown/missing licence fields fail
- [x] Exclude dev-dependencies (`exclude-dev = true` in `[graph]`; verify key against installed cargo-deny version)
- [x] Add advisories and bans sections to `deny.toml`
- [x] Test the policy with a deliberately Apache-only dummy dependency (must fail) and an `MIT OR Apache-2.0` one (must pass)
- [x] Document the no-copying rule for third-party code (README or CONTRIBUTING)

### Crate skeletons
- [x] Create ten crate directories under `crates/` named `tpt-kardashev-<thing>` (archive-codes, relativity, radhard, energy-ledger, dtn, astrodynamics, nuclear-plasma, causal-consensus, isru, reflect) (scaffold: `publish = false`)
- [x] Each crate: `Cargo.toml` inheriting workspace package/lints, independent version `0.0.0`
- [x] Each crate: `#![no_std]` `lib.rs` with `alloc` and `std` features, `#![forbid(unsafe_code)]` (radhard exception documented)
- [x] Each crate: `README.md` (one-line description, what it does not model), `CHANGELOG.md`, `examples/`
- [x] Each crate: `docs/provenance/<crate>.md` stub (origin of ported code, licence of any third-party source)
- [x] Add placeholder `examples/` directory at repo root for cross-crate examples
- [x] Optional feature flags declared per §4.2 (relativity, energy) with no mandatory internal deps — `dtn` was removed from astrodynamics, as `fec` was from dtn, because a dependency on another crate breaks the rule. `relativity` in astrodynamics remains a placeholder pending a decision the same way
- [x] Verify no sibling `path = "../tpt-xyz"` dependencies outside this workspace

### Repo docs
- [x] Root `README.md` (crate table, dependency graph, design principles, non-goals)
- [x] `CLAUDE.md` (conventions, commands, rules: no unwrap, no panics, no sibling path deps, deterministic, no AI attribution in commits)
- [ ] Per-crate sections in this `todo.md` kept in sync as work completes
- [x] Floating-point tolerance policy doc (shared template for physics crates)
- [x] Define the admission rule for new crates in `CONTRIBUTING.md` (one concern, small trait API, no app code, builds alone)

### CI (`.github/workflows/`)
- [x] `fmt` check
- [x] `clippy -D warnings`
- [x] Tests with default features and with all features
- [x] `no_std` build, including thumbv7em-none-eabihf for crates claiming embedded use
- [x] "Builds alone" job: `cargo build -p <crate> --no-default-features` for each crate
- [x] `cargo deny check licenses` on full tree, all features and default features
- [x] `cargo deny check advisories`
- [x] Doc build with warnings denied
- [x] SPDX header and dual-licence file enforcement check
- [x] Scheduled Kani job (runs only once `#[kani::proof]` exists)
- [x] Scheduled fuzz job (runs only once a `fuzz/` target exists)
- [x] Verify CI is green on empty crates (**Phase 0 exit**) — GitHub Actions runs 43695134560 (b9197a4) and 43699093000 (d7590e5) succeeded on master (fmt, clippy, no-std, deny, docs, spdx-and-licences, builds-alone matrix, test matrix); per-step logs need a sign-in, so step results are taken from the job summary

### Cross-cutting infrastructure
- [x] Shared test conventions (decided 2026-11-16: seeds in each test, tolerance helper per test file): recorded proptest seeds, tolerance helper pattern (per-crate, not shared crate)
- [x] Benchmark setup decided 2026-11-16: no criterion; regression checks are tests with stated tolerances
- [x] Fuzz harness layout and template: `docs/fuzzing.md` (per-crate `fuzz/`, committed seed corpus, campaign lengths)

---

## Phase 1: Leaf crates

Exit: each crate at its v0.1 exit.

### 1.1 `tpt-kardashev-archive-codes`
- [x] Investigate `tpt-aion` and `tpt-rfc` for existing archival coding/CRC (§12.6); record result in provenance — done: read from GitHub; neither has archival coding or CRC code (provenance records this)
- [x] Obtain current CCSDS 131.0-B revision; record parameters, polynomials, sync markers from the document (not memory) — CCSDS 131.0-B-5 (Sept 2023) fetched and read; sources in `docs/provenance/tpt-kardashev-archive-codes.md`
- [x] Define error type (`CodeError`, `#[non_exhaustive]`), `DecodeReport`, and traits (`BlockEncoder`, `BlockDecoder`, `SoftDecoder`, `ErasureCoder`)
- [x] GF(2^8) arithmetic (in-crate, tables generated at const time)
- [x] Reed-Solomon RS(255,223) encoder with const generics, no alloc (runtime parameters, not const generics)
- [x] Reed-Solomon decoder (syndromes, Berlekamp-Massey, Chien, Forney), erasure support (errors-and-erasures, 2t + e <= nroots)
- [x] Block interleaver / de-interleaver (depth configurable)
- [x] CCSDS dual-basis (conventional/Berlekamp) representation handling (`dual::DualBasisRs`, §4.3.9 and Annex F)
- [x] CRC variants (CRC-16, CRC-32, others as needed) in-crate
- [x] Attached sync markers (ASM) insert/detect (byte-aligned search only)
- [x] Convolutional encoder, rate 1/2, K=7
- [x] Viterbi decoder, hard decision
- [x] Viterbi decoder, soft decision (confidence byte per coded bit)
- [x] Self-describing archival framing (header with code parameters, versioned) (version 1; header CRC-16, payload CRC-32C)
- [x] LDPC block codes with iterative belief-propagation decoder (soft input) — normalised min-sum variant; sum-product, encoder and standard matrices not implemented
- [x] LT-style fountain/erasure codes encoder and decoder session (peeling; robust soliton needs `std`)
- [x] Optional `serde` feature on data types
- [x] Known-answer tests from the CCSDS standard — from its tables and definitions (Annex F matrix, Annex G generator, §9.2 ASM, §4.4.1 interleaving, §3.3 convolutional code). The standard has no codeword vectors.
- [x] Round-trip property tests (≤ t symbol errors always recovered)
- [ ] Fuzz: decoders and frame parser never panic on arbitrary input — partial: targets in `fuzz/` ran 60 s each on nightly (frame ~6M runs, RS ~44k, soft Viterbi ~348k, LDPC ~1.7M), then 4 minutes each (frame ~24M, RS ~175k, soft Viterbi ~837k, LDPC ~7M), with no crash; a kept corpus is still to do [2026-11-16: five minutes per target on nightly under WSL, twelve targets in all, no crash or artifact; minimised corpora committed in fuzz/seeds/; campaigns of an hour or more still to do]
- [x] Kani proofs for selected arithmetic (GF ops) — 5 GF(2^8) harnesses and 2 dual-basis harnesses, all verified with Kani 0.68 (run in WSL Ubuntu; Kani does not support Windows)
- [x] Benchmarks: BER curves vs theory as regression checks — `tests/ber.rs` (exhaustive-ML minimum-metric check; soft BER within the converged union bound at 3 dB) and `examples/ber_curve` (2–4 dB table, release mode). Criterion timing benchmarks are not part of this item.
- [x] `no_std` and no-alloc build for fixed-size paths; thumbv7em build
- [x] README states what is not modelled (no compliance claim; turbo codes deferred; RaptorQ not included)
- [x] Examples (RS + interleave round trip, archival frame)
- [x] **v0.1 exit:** RS(255,223)+interleaving with vectors (standard tables: Annex F, Annex G, §4.4.1; the standard has no codeword vectors); conv K=7 + Viterbi (ML-checked, BER-checked); CRC; framing; no_std/no-alloc

### 1.2 `tpt-kardashev-relativity`
- [x] Review `tpt-replicate/src/clock.rs` for the bounded-drift assumption; note in provenance — done: the bound is a configured limit (`DEFAULT_MAX_DRIFT_MS`), recorded in provenance
- [x] Scalar decided 2026-11-16: f64 only for v0.1, generic trait post-v0.1. Error enum and newtypes as written; was: partial: `f64` only; generic `Scalar` trait deferred past v0.1 (decided: no consumer needs a scalar other than `f64`, and a generic scalar would touch every type in the crate)
- [x] `Event`, `FourVector`, `Interval` (timelike/spacelike/lightlike)
- [x] `Lorentz` boost with `|v| < c` enforced; composition via rapidity
- [x] Time dilation, length contraction, relativistic Doppler
- [x] Inertial `Frame` and frame-to-frame transforms
- [x] Causal predicates: `precedes`, `light_cone_contains`, `causally_disconnected`
- [x] `CausalOrder` trait (neutral, consumed by other crates)
- [x] `proper_time(path)`
- [x] Weak-field gravitational time dilation (Schwarzschild approximation)
- [x] Proper time vs coordinate time for a moving, gravitating observer
- [x] Time-scale rate relations TT, TCG, TCB, TDB as functions (IAU constants documented) (constants checked 2026-11-16 against the IERS Conventions 2010 appendix (IAU 2006 B3, item 14) and the IAU 2000 supplement: L_G is the defining constant of B1.9, and L_B is the defining constant of B3 at 1.550519768e-8; the earlier "derived" note was wrong and is reverted)
- [x] `Metric` trait with Minkowski and Schwarzschild implementations
- [x] Documented float-tolerance policy
- [x] Property tests: interval invariance under boosts, composition law, `|v| ≥ c` rejected
- [x] Known-value regression: GPS ≈ +38 µs/day net offset
- [x] `no_std` build
- [x] README states what is not modelled (no geodesic integration, no strong-field)
- [x] Examples
- [x] **v0.1 exit:** events, intervals, boosts, proper time, causality, Doppler, weak-field redshift, tolerance policy (time-scale constants checked against the IAU 2000 explanatory supplement on 2026-11-16)

### 1.3 `tpt-kardashev-radhard`
- [x] Review `tpt-equinox-tmr` (done: read, recorded in provenance) and `tpt-for-redundancy` (in `tpt-solutions/tpt-formal`, `crates/tpt-for-redundancy`; read, recorded in provenance, no code taken)
- [x] `Tmr<T>` with majority vote and scrub-on-mismatch
- [x] N-modular variant
- [x] Typestate wrappers (unvoted vs voted types)
- [x] SECDED (72,64) encode/decode
- [x] CRC-protected cells and checksummed buffers
- [x] `Checkpoint<T>` with rollback and watchdog-friendly progress marker
- [x] `FaultInjector` (seeded bit flips, burst errors)
- [x] Volatile access behind a feature, `unsafe` documented (the one exception to forbid-unsafe)
- [x] Kani proof: no single corrupted replica changes the voter result — verified by Kani (`cargo kani`, WSL Ubuntu 24.04; `tmr::kani_proofs`); exhaustive voter test kept
- [x] Kani proof: SECDED corrects every single-bit error and detects every double-bit error (decided 2026-11-16: 64-bit proof not pursued; the 8-bit proof stands, with the exhaustive and randomised tests for the full width) — was partial: verified by Kani for 8-bit data (`secded::kani_proofs`, both properties over all 72 positions); the 64-bit proof is still open (the solver did not finish on symbolic 64-bit data), and the exhaustive SECDED tests cover the full width
- [x] Exhaustive tests over small word sizes
- [x] Injector-driven property tests
- [x] `no_std` build, thumbv7em build
- [x] README states what is not modelled
- [x] Examples
- [ ] **v0.1 exit:** Tmr, SECDED(72,64), checksummed cells, checkpoint/rollback, injector, Kani proofs

### 1.4 `tpt-kardashev-energy-ledger`
- [x] Review `tpt-equinox-power`, `tpt-energy`, `tpt-thermodynamics` (physics only, no code duplication)
- [x] Newtypes `Joules`, `Watts`, `Kelvin`
- [x] Physical constants (k_B, σ) with CODATA source and year recorded
- [x] Double-entry `Ledger` with explicit `WasteHeat` account
- [x] Declared sources and sinks; checked conservation invariant
- [x] `landauer_limit(bits_erased, temperature)`
- [x] Radiator capacity from Stefan-Boltzmann (`ε σ A T⁴`)
- [x] `HeatBudget` with headroom report
- [x] `EnergyOracle` trait
- [ ] Optional `units` feature mapping to `tpt-math-units` — blocked (2026-11-15): `tpt-solutions/tpt-math-units` is 404 under the owner used so far, so there is nothing to map to yet
- [x] Optional `serde` feature
- [x] Conservation property tests (random transfer sequences)
- [x] Constants cross-checked against CODATA; hand-calculation tests for Landauer (≈2.9×10⁻²¹ J/bit at 300 K) and radiator
- [x] `no_std` build
- [x] README states what is not modelled
- [x] Example: compute node with radiator
- [x] **v0.1 exit:** ledger with checks, Landauer, radiator, oracle, example

### Phase 1 close
- [ ] All four crates at v0.1 exit
- [ ] Tag `<crate>-v0.1.0` for each (git-only)
- [x] CI green for all four, including cargo-deny — runs 43695134560 (b9197a4) and 43699093000 (d7590e5) cover the workspace, deny job included

---

## Phase 2: Networking, orbits, plasma

Exit: each crate at v0.1 exit; `ContactPlan` flow from astrodynamics to dtn demonstrated in an example.

### 2.1 `tpt-kardashev-dtn`
- [x] Review `substrate-dtn` and `tpt-zenith/routing`; record in provenance — `tpt-zenith/routing/cgr` read (earliest arrival, no SABR rules); `substrate-dtn` read inside `tpt-solutions/tpt-substrate` (see `docs/migration.md`)
- [x] Check `tpt-rfc` / `tpt-aion` for existing CBOR before writing one (§12.6) — done: `tpt-rfc` has a sibling CBOR codec (`tpt-cbor`) and `tpt-aion` has none; this crate keeps its own codec, since a dependency on another repo would break the no-inter-crate rule (provenance records this)
- [x] Confirm current RFC 9171 (BPv7), RFC 5326 (LTP), CCSDS 734.1-B and SABR documents — RFC 9171, RFC 8949 and RFC 5326 read; CCSDS 734.1-B-1 (LTP profile, May 2015, from public.ccsds.org/Pubs/734x1b1.pdf) read, and its §3.5 limits are now checked by `ltp::session::Config::check_ccsds_profile` and `check_ccsds_session_number` (tests in `tests/ltp_ccsds_profile.rs`); SABR (CCSDS 734.3-B-1) read
- [x] Minimal in-crate CBOR codec (definite and indefinite lengths as required by BPv7)
- [x] Endpoint ids: `dtn:` and `ipn:`
- [x] Primary block, canonical blocks (payload, previous node, bundle age, hop count) — typed previous-node, bundle-age and hop-count blocks (`extension`), validated on encode and decode; the simulator enforces the hop limit from the Hop Count block
- [x] Bundle CRCs (CRC-16 X.25, CRC-32C)
- [x] `Bundle` encode/decode with strict validation
- [x] `BundleStore` trait; in-memory store
- [x] File-backed store
- [x] `ContactPlan` and `Contact` types (start, end, rate, one-way light time)
- [ ] Contact-graph router (CGR/SABR family), `Router` trait — partial: `Router` trait and `EarliestArrival` (earliest-arrival over the contact plan, with waiting and store-and-forward) are written, and the simulator holds bundles and routes them hop by hop with them (opt-in), re-routing when the contact plan changes during a run; SABR expiry, OWLT margin and per-contact volume (CCSDS 734.3-B-1 §2.4.1–2.4.3, §3.2.6.9) are applied in the simulator, with volume reserved when a bundle is routed; the rule that a bundle is not sent back to the node it came from (§3.2.5.2) is applied; critical bundles are sent one copy per neighbour (`Network::send_critical`, §2.4.5); not done: multiple copies beyond that, excluded neighbours from custody refusal (§2.4.4; deferred with the custody extension, see the reliability note), fragmentation (§2.4.4–2.4.5, §3.2.8.2), and backlog measured in volume [2026-11-16, superseding the "not done" list in this item for these two: multiple copies by first-hop order (`Network::send_copies`, tests in `tests/copies.rs`; `send_critical` is the all-neighbours case); fragmentation and reassembly per RFC 9171 §5.8–5.9 (`fragment::fragment`, `fragment::reassemble`, `fragment::reassemble_encoded`, `Network::send_fragmented`; tests in `tests/fragment.rs` and `tests/fragment_sim.rs`). 2026-11-17: `Network::send_fitting` fragments to the free volume of the sender's first hops (CCSDS 734.3-B-1 §2.4.4), and the hop counter is now kept per fragment (regression test `the_hop_limit_counts_each_fragment_on_its_own`). Its end-to-end test (`a_bundle_too_big_...`) is ignored: two concurrent LTP sessions on one contact stall in the simulator, a pre-existing defect that plain bundles also show (two 15 kB bundles through one 60 s window at 1 kB/s, mtu 1400: the second never completes; see the next item). Still open: backlog measured in volume]
- [x] `ConvergenceLayer` trait (`convergence`)
- [ ] LTP convergence layer (segments, sessions, reliability via report/ack, timers on virtual clock) — partial: segment codec (`ltp::segment`), red-part sessions with report/ack and countdown timers on a virtual clock (`ltp::session`), and `convergence::LtpConvergence` carrying one encoded bundle per LTP block; green part (best effort, gap reporting, never retransmitted) and cancellation (repeated until acknowledged, from application, limit or peer) added; link-state scheduling not modelled
- [ ] Reliability note: bundle-level custody/reliability extension deferred, documented as such (README). Design only (2026-11-18): `docs/dtn-custody-design.md`; the spec source, CCSDS 734.2-B-1, is not yet read
- [x] Optional `fec` feature — dropped: the no-inter-crate-dependency rule (CLAUDE.md) forbids `tpt-kardashev-dtn` depending on `tpt-kardashev-archive-codes`, so the spec §4.2 row and the empty `fec` feature were removed. Link-layer FEC, if wanted later, needs a dtn-local trait and a decision recorded then
- [x] Deterministic simulator with virtual clock and contact-plan-driven links (`sim::Network`: a datagram moves only inside a contact window, at the contact's rate, plus one-way light time)
- [ ] Seeded fault injection (loss, delay, partition, duplication, reordering) — partial: seeded loss, duplication, and delay jitter (which reorders); link failures (`fail_link`) learned from cancelled sessions; `Network::partition` cuts both directions of a link for a time window (tested: a short partition delays delivery and does not mark the link failed); partitions that split a set of nodes are written (`Network::partition_sets`, which cuts every link between two groups in both directions, tested); reordering still comes only through jitter
- [ ] RFC 9171 appendix examples as byte-exact vectors — partial: RFC 9171 has no byte-level examples (its appendix B is CDDL). Used a bundle derived by hand from the CDDL, and the RFC 8949 Appendix A CBOR examples
- [ ] Round-trip tests and fuzzing of CBOR/bundle parser — partial: round-trip and bit-flip tests written; fuzz targets (`bundle_decode`, `cbor_read`, `ltp_segment`, `sdnv_decode`) ran 4 minutes each on nightly after the 60-second runs, with no crash; a kept corpus and longer campaigns are still to do [2026-11-16: five minutes per target on nightly under WSL, twelve targets in all, no crash or artifact; minimised corpora committed in fuzz/seeds/; campaigns of an hour or more still to do]
- [x] Routing tests against hand-computed contact-plan scenarios
- [x] Simulator tests for delivery under disruption — bundles delivered across a relay whose second link opens late (store-and-forward, transmissions checked against windows), under 25% loss with duplication and jitter (exactly one copy), and determinism for a fixed seed
- [x] Optional `serde` feature (`serde` feature: URI text for Eid, codes for CrcType, maps for the integer records; Contact, BundleId, Hop and Route serialise only; bundles, segments and sessions not covered)
- [x] `no_std` + `alloc` build (thumbv7em, with and without `alloc`)
- [x] README states what is not modelled (no compliance claim; no BPSec, no TCPCL in v0.1)
- [ ] **v0.1 exit:** BPv7 codec with vectors, stores, CGR router, LTP layer, simulator

### 2.2 `tpt-kardashev-astrodynamics`
- [x] Check `tpt-sci-astro` and `tpt-transportation` for an existing Lambert solver (§12.7); record in provenance — done: `tpt-transportation`, `tpt-zenith` and `tpt-science/crates/tpt-sci-astro` (under `tpt-solutions`) were read; none has a Lambert solver (`tpt-sci-astro` says so in its own example). `tpt-equinox/crates/tpt-equinox-orbital` is terrain insolation, not orbital mechanics
- [ ] Review `tpt-zenith/orbital-mechanics` and `tpt-equinox-orbital`; record in provenance
- [x] Scalar decided 2026-11-16: f64 only for v0.1, generic trait post-v0.1. Units newtypes and error enum as written; was: partial: `Kilometres`, `Seconds`, `Radians` (finite-checked), `Error` (`#[non_exhaustive]`) and `Vec3` are written and tested; no `Scalar` type yet (plain `f64` in the API)
- [x] Epochs and time scales (UTC, TAI, TT, TDB), Julian dates, leap-second handling (user-supplied table, no bundled data) — TT epochs, TAI, UTC through a caller-supplied table, and Julian dates are written and tested. TDB: the periodic part of TDB−TT is written as `tdb::tdb_minus_tt_seconds` (the Fairhead and Bretagnon series in the FB2001 form, 1697 terms from the IERS data file `fb2001.dat`); it matches the IERS reference output `fb2001.out` to under 1 ns over more than 200 epochs (`tests/tdb.rs`). The constant offset TDB0 is not in this function. UT1: decided 2026-11-16 as caller-supplied, like the leap-second table: no earth-orientation data is bundled, and the Earth-fixed frame takes the UT1 angle from the caller.
- [ ] Optional `relativity` feature for relativistic scale relations
- [ ] Reference frames: inertial (GCRF-like), Earth-fixed, perifocal, RTN/LVLH, heliocentric ecliptic, and transforms between them — partial: perifocal ↔ inertial (`Keplerian::perifocal_to_inertial`) and RTN (`frames::rtn_matrix`) are written and tested; LVLH is written with one documented convention (`frames::lvlh_matrix`: x along-track, y along −N, z nadir; tested); heliocentric ecliptic needs a heliocentric origin; Earth-fixed needs UT1; GCRF-like not separately defined yet
- [x] Element sets (Keplerian, equinoctial) and state-vector conversions — Keplerian ↔ state for elliptic orbits, with fixed conventions for circular and equatorial cases; equinoctial `(p, f, g, h, k, L)` ↔ Keplerian and ↔ state (`equinoctial::Equinoctial`), tested by closed-form values, a 144-case state round trip including the singular cases, and regularity across the equatorial plane
- [x] Two-body propagation (universal variables) — `propagate::propagate`, tested for periods, conservation, composition, reversal, agreement with an independent Kepler-equation solve, and hyperbolic states
- [x] `ForceModel` trait — `force::ForceModel` (acceleration at a time and state); a pair of models is a model (their sum)
- [ ] Forces: central body, zonal harmonics J2…Jn, third bodies, SRP with shadow model, atmospheric drag, thrust — partial: central body (`force::CentralBody`), J2 (`force::J2Zonal`, WGS 84 constants) and third-body gravity of the Sun and Moon (`force::ThirdBody`) written; solar radiation pressure with the Earth's umbra and penumbra (`force::SolarRadiationPressure`, the visible fraction of the Sun's disk from `force::lit_fraction`, checked against sampling of the disk; albedo and infrared not modelled) exponential-atmosphere drag (`force::AtmosphericDrag`, `atmosphere`) and finite burns (`force::Thrust`: constant acceleration, fixed or prograde, in a time window, no mass flow) also written; zonal harmonics of any degree are written (`force::ZonalHarmonics`, with the coefficients supplied by the caller; checked to degree 6 against the gradient of the geopotential) but their WGS 84 reference-ellipsoid constants J2 to J10 are now bundled (2026-11-17, from the EGM96 report, `WGS84_ZONAL_J2_TO_J10`; the EGM96 field coefficients are not bundled); a solar-cycle-dependent atmosphere, and mass-dependent thrust are deferred past v0.1 (decided: mass in `StateVector` is a public API change, and the solar-cycle atmosphere needs a sourced density dataset)
- [x] Cowell combined numerical propagator — `integrate::Cowell` integrates any `ForceModel`, including summed models; tested against the analytic two-body solution
- [x] Integrator: fixed-step RK4 — `integrate::rk4_step`; error falls as the fourth power of the step (measured 16.4× per halving)
- [x] Integrator: adaptive high-order Runge-Kutta — `adaptive::AdaptiveCowell`, Dormand–Prince 5(4) with error control on position and velocity; tested against the analytic two-body solution over three orbits (5e-7 km at 1e-12 tolerances), tighter tolerances giving smaller errors, energy drift 4e-12 over five orbits, and backward propagation
- [x] Integrator: symplectic for N-body — `nbody::NBodyVerlet` (velocity Verlet over N mutually attracting bodies, needs `std` and `alloc`): equilateral three-body solution, bounded energy and conserved momentum over ten periods on a stable binary, agreement with `symplectic::Verlet` for a test particle (`tests/nbody.rs`). Post-Newtonian and softened forces are not modelled
- [x] Lambert solver — `lambert::lambert` (zero-revolution, universal variables, bisection on z); tested on exact circular arcs and by round trip through the propagator for prograde and retrograde orbits; multi-revolution transfers not written
- [x] Impulsive manoeuvres: Hohmann, bi-elliptic, plane change — `manoeuvre::hohmann`, `manoeuvre::bi_elliptic`, `manoeuvre::plane_change`; tested by propagating the transfer orbits to the next apsis, not by re-using the formulas
- [x] `Ephemeris` trait with low-precision analytic provider — `ephemeris::Ephemeris` and `ephemeris::LowPrecision`, which gives the Sun (longitude to about 0.01°, distance to about 10⁻⁴ AU) and the Moon (a few tenths of a degree in longitude); the planets are not written (deferred past v0.1: they need coefficient series from a verified source)
- [x] SPK (Chebyshev) reader behind a feature (no ephemeris data bundled) — `spk`: DAF/SPK type 2, little-endian, J2000 frame, gated by `std` rather than a separate feature. Tested on kernels built in the NAIF layout; not checked against a real kernel, since none is bundled
- [ ] `sgp4` feature: SGP4/SDP4 and TLE parsing — partial: near-earth SGP4 (period under 225 min) and two-line element parsing are written in `sgp4` (behind `std`, not a separate feature), from the equations of AIAA 2006-6753 rather than ported from zenith (zenith is not in the workspace); SDP4 (deep-space terms, orbits of 225 min or more) written in `sdp4` and used by `sgp4`; both from the equations of AIAA 2006-6753
- [x] Visibility and contact-window computation — `visibility`: WGS 84 ground stations, Earth rotation from a caller-supplied start angle (no UT1), elevation, and `contact_windows` with edges bisected to a millisecond; tested on exact station geometry and on a 500 km equatorial orbit (15 passes a day, 237–474 s each, edges on the 10° limit)
- [x] Optional `dtn` feature: emit `ContactPlan` from visibility windows — dropped as a feature: it would be a dependency on another crate. `visibility::contact_windows` gives the windows, and the caller builds the `ContactPlan` with `dtn`'s `Contact::new`
- [ ] SGP4 reference vector tests — partial: six near-earth cases of the published verification set (WGS-72) match the paper's printed states to 1 mm and 10⁻⁸ km/s: 00005, 06251, 28057, 28350, 22312 and 29238 (normal, low-eccentricity, and the low-perigee drag branches); the sub-orbital case 28872 is refused at initialization, and the trap times of 28350, 22312 and the decay of 29141 match the paper; deep-space cases 28129, 26975, 08195, 28626, 09998, 04632 and 20413 match the paper's printed states (1e-5 km; 08195 at 3e-4 km, with a residual that grows with time and is not yet explained); deep-space cases 25954, 24208, 21897, 22674, 09880, 23333, 28623, 16925, 26900, 23177, 23599, 14128, 11801 and 88888 match the printed states to 3e-5 km (most to about 5e-9 km; 09880, 21897, 16925, 23333 and 23599 reach 2e-6 to 1.7e-5 km, not yet explained). Every verification case in the set with printed outputs is now tested
- [x] Textbook Lambert and Kepler cases — Lambert on exact circular quarter and three-quarter arcs; Lambert against the time of flight from Kepler's equation for a quarter turn of an eccentric ellipse (`tests/lambert.rs`), with the propagator round trips as before
- [x] Energy and angular-momentum drift checks (symplectic, adaptive) over long arcs — adaptive: energy 4e-12 over five orbits (`tests/adaptive.rs`) and angular-momentum vector 1.7e-11 over twenty orbits (`tests/angular_momentum.rs`); symplectic: energy error bounded over a thousand orbits at two step sizes, with the peak per orbit unchanged to 2e-8 relative between the first and last hundred orbits, angular momentum 1.5e-14 over a hundred orbits (`tests/symplectic.rs`)
- [x] Frame round-trip tests — ecliptic ↔ equatorial round trip (inverse written in the test) and length preservation; RTN components mapped back to the inertial state over a 192-case element grid; RTN normal equals the perifocal W axis (`tests/frames.rs`)
- [ ] Cross-check new force model against old secular formulas (J2, drag, etc.) — partial: J2 nodal regression of an integrated orbit matches the secular rate to 0.2% (`tests/j2.rs`); the perigee rotation matches the J2 secular rate to 0.5%, with the rate formula from `tpt-sci-astro` (`tests/j2.rs`), and that crate's `EARTH_J2` agrees with our old, tide-free value to 3×10⁻⁷ relative (2026-11-17: both are the tide-free J2; the WGS 84 zonal-tide value differs by 3×10⁻⁶, and the crate now uses it); drag is checked only against the textbook decay rate (`tests/drag.rs`), since no sibling repo has an old drag formula
- [x] Simulator: two concurrent LTP sessions on one contact stalled (found 2026-11-17). RESOLVED the same day by exponential backoff of the LTP retransmission timeout, per session, from the configured timeout up to 4 times it (`max_rto_factor (default DEFAULT_MAX_RTO_FACTOR)`; `Sender::backoff_shift`, applied in `LtpConvergence::poll_outbound`). The cause was a constant timeout under queueing: every checkpoint was resent before its report came back, Karn's rule discarded the samples, and the retransmissions queued behind the other session. Trade-off, which is the owner's to accept: a silent link is now declared failed after about 75 s, not about 18 s, and four routing tests were re-timed to match (the worked-before test was renamed `..._within_the_backoff_budget`). Tests: `tests/concurrent.rs` (two bundles, one contact), `tests/routing_sim.rs`, and the fitting test in `tests/fragment_sim.rs`, now on a 10 kB/s link, since at 1 kB/s LTP's checkpoint round trips take longer than the window.
- [ ] Fuzz: TLE and SPK parsers — partial: deterministic robustness tests (`tests/parser_robustness.rs`) feed 20 000 mutated and 20 000 random element-set pairs, truncations of a valid set, and 5 000 random byte strings (half with the DAF magic) to `parse_tle` and `read_segments`; none panics. `cargo-fuzz` targets are in `fuzz/` (`tle_parse`, `spk_read`). A 90-second nightly run of each found a panic in `parse_tle` (a multi-byte character sliced mid-character; fixed, with a regression test). After the fix, five-minute runs of each found nothing more. The SPK target reaches its file-record checks only with a valid kernel as a seed and a magic-word dictionary (`fuzz/dictionaries/spk.dict`); with them, coverage rose from 26 to 97 edges over 4.5 million runs. The seed is not kept in the repository; longer campaigns and a kept corpus are still to do [2026-11-16: five minutes per target on nightly under WSL, twelve targets in all, no crash or artifact; minimised corpora committed in fuzz/seeds/; campaigns of an hour or more still to do]
- [x] `no_std` + `alloc` build — builds with `--no-default-features --features alloc` on the host and on thumbv7em; the crate has no heap-allocating API yet, so this checks the feature only, and no alloc-dependent test exists
- [x] README states force-model limits and what is not modelled
- [ ] Low-thrust (Edelbaum-type) approximations (post-v0.1)
- [ ] **v0.1 exit:** time scales, frames, elements, two-body + Cowell (J2/third-body/SRP/drag), RK4 + adaptive, Lambert, impulsive manoeuvres, analytic ephemeris, SGP4 feature

### 2.3 `tpt-kardashev-nuclear-plasma`
- [x] Review `tpt-ignis` (`tpt-physics-math`, `tpt-aether`); record in provenance; note overlap on equilibrium/MHD
- [x] Research: choose first cross-section format (ENDF-6, ACE, OpenMC multigroup, simple multigroup text); check licences of any reference parsers; record decision — decided: a crate-specific multigroup text format, version 1 (`xs`), not ENDF-6 or ACE: no third-party format or licence is needed for the first version; ENDF-6 and ACE remain possible later readers
- [x] Decide feature layout (`equilibrium`, `neutronics`, `mhd`) and the sub-crate split trigger (size) — decided: no per-module features. `std` gates the Monte Carlo, the slab and the cross-section reader needs `std` and `alloc`, and the equilibrium needs `alloc`; a sub-crate split is considered only when a module passes about 5000 lines or needs a dependency the others must not have
- [x] Scalar decided 2026-11-16: f64 only for v0.1, generic trait post-v0.1. Error enum and grid types as written; was: partial: error enum and grid types are written; the generic `Scalar` trait is deferred past v0.1, as for relativity (f64 is the only scalar in use)
- [x] **equilibrium:** (R, Z) grid and fixed-boundary Grad-Shafranov solver (2026-11-16: general source `Δ*ψ = S(R, ψ)` written as `solve_with_source`, with the residual check `max_residual`; the profiles p'(ψ) and FF'(ψ) are the caller's, none bundled; the lagged source's convergence is checked by residual, not proven; tests in `tests/equilibrium_source.rs`). Earlier state: `equilibrium::Grid` and `solve_fixed_boundary` solve `Δ*ψ = κR²` (pressure-driven source only) by SOR with Dirichlet boundary values; the toroidal-field function F(ψ) is not included, so the general equation is not solved
- [x] equilibrium: flux-surface extraction — `surface::contour`: rays from a given magnetic axis, bisected on the bilinear interpolant; assumes star-shaped surfaces; tested on the circular flux function (`tests/surface.rs`)
- [x] equilibrium: safety-factor profile — `surface::safety_factor`, q = (F/2π)∮dl/(R|∇ψ|) for a constant F, checked against the closed form on the circular flux function to 2% on three surfaces; a profile over many surfaces is a loop over `contour`
- [x] equilibrium: Solov'ev analytic benchmark — ψ = A R⁴ + B R² Z² (exact with κ = 8A + 2B), used as the boundary and as the reference (`tests/equilibrium.rs`)
- [x] equilibrium: convergence-order tests — the discrete operator's residual on the exact function and the solver's interior error both fall by a factor of about 4 when the grid is halved (second order)
- [ ] **neutronics:** constructive solid geometry — partial: one-dimensional layered geometry (`layers::Layers`, regions in a row with the source at the left face, analog transport across boundaries, tested against the exponential through two regions, invariance under cutting a region, and agreement with the slab run); three-dimensional CSG (intersections, unions, complements of surfaces) is not written
- [x] neutronics: multigroup cross-section types and the chosen format reader — `xs::Multigroup` and `xs::read_multigroup` (text format version 1), with a writer; tests in `tests/xs.rs` cover valid files, comments, round trips and malformed files with their line numbers
- [x] neutronics: seeded RNG and fixed-source Monte Carlo transport — `rng::SplitMix64` and `slab::run_slab` (tests in `tests/neutronics.rs`)
- [x] neutronics: tallies (flux, reaction rates) — transmission, reflection, absorption, track length and collisions, each with its standard error; spatial flux bins (`slab::run_slab_binned`, checked against the exponential in each bin and summing to the total track length); reaction-rate tallies per reaction at each collision: absorption and scattering, summing to the collisions history by history (`SlabTally::scattered`, also in `layers`; tests in `tests/reactions.rs`). Track-length reaction rates (σ times track length) are not written
- [x] neutronics: implicit capture, Russian roulette, splitting — implicit capture and Russian roulette at one weight threshold (unbiased, checked against analog); zone splitting (`Estimator::Splitting { zones, copies }`: a neutron entering a deeper zone is split at that face; checked against analog for transmission, reflection, absorption and track length, including a deep-penetration slab; one zone or one copy is implicit capture draw for draw). Splitting at every collision was tried and rejected: with three copies per collision the test run did not finish in a thick slab (the population grows at each collision, as the branching rate exceeds one; not bounded in the code)
- [x] neutronics: slab-attenuation (exponential law) validation — the pure-absorber transmission exp(-σt T) and track length, checked at four standard errors with fixed seeds
- [x] neutronics: infinite-medium validation — track length 1/σa per source neutron in an infinite medium with scattering, checked at four standard errors
- [x] neutronics: statistical tests with fixed seeds — fixed seeds, the square-root error law, the reproducibility and the weight conservation (partly; the wider suite is still to do)
- [ ] Optional features for `tpt-math-prob-monte-carlo`, `tpt-math-pde-operators`, `tpt-math-linalg-*` (licence-checked)
- [ ] Fuzz: cross-section file reader — partial: `fuzz/` target `xs_read` (any accepted file must write back to an equal set) ran 2.5 minutes on nightly with no crash over 8.6 million inputs; a kept corpus and longer runs are still to do [2026-11-16: five minutes per target on nightly under WSL, twelve targets in all, no crash or artifact; minimised corpora committed in fuzz/seeds/; campaigns of an hour or more still to do]
- [x] `no_std` + `alloc` build for the core where feasible — `tpt-kardashev-nuclear-plasma` builds with no features and with `alloc` on thumbv7em-none-eabihf (and with `alloc` on the host)
- [x] README states what is not modelled (no bundled data) — `tpt-kardashev-nuclear-plasma` README "What it does not model"
- [ ] Later (post-v0.1): eigenvalue/criticality problems, free-boundary equilibrium, MHD 1D then 2D
- [x] **v0.1 exit (2026-11-16):** fixed-boundary G-S with Solov'ev, fixed-source MC with slab validation, tallies, one documented cross-section format

### Phase 2 close
- [x] Example: astrodynamics visibility windows → `ContactPlan` → dtn routing — `examples/contact-plan-flow`, a standalone package. It uses no feature flags: a library crate may not depend on another crate in this repo, but an example may, so the example depends on both crates directly. The astrodynamics `dtn` feature was removed for the same reason. The run computes four passes per station in six hours and delivers a routed bundle to station B
- [ ] All three crates at v0.1 exit
- [ ] Tag `<crate>-v0.1.0` for each
- [ ] CI green, including cargo-deny with all features

---

## Phase 3: Agreement, resources, self-modification

Exit: each crate at v0.1 exit.

### 3.1 `tpt-kardashev-causal-consensus`
- [ ] Review `tpt-replicate`, `hiveforge-consensus`, `tpt-scheduler`, `tpt-fabric`; record in provenance — partial: read tpt-replicate clock, tpt-fabric clock, tpt-scheduler; hiveforge-consensus is not on GitHub under tpt-solutions (404), so it stays open
- [x] Hybrid logical clock and vector clock — `clock::HybridClock` (strictly increasing per node, causal across receives, drift bound) and `vector::VectorClock<N>` (fixed size, inline), tested in `tests/clocks.rs`
- [x] Causally ordered operation log — `causal::CausalBroadcast` (feature `alloc`): causal broadcast with vector-clock dependencies; a message is delivered only after its causes, duplicates are dropped, early messages wait in a buffer. Tested with a seeded simulation (4 nodes, random sends, reordering and duplicates: every node delivers every message once, in causal order) and a reply-before-cause case
- [ ] Optional `relativity` feature: `CausalOrder` as ordering source
- [x] Domain layer: Raft-style leader election and log replication within a domain — `domain::RaftNode`: leader election with seeded timeouts, heartbeats, log replication with conflict repair, and commitment by majority in the leader's term. Tested on a five-node simulated cluster (`tests/domain.rs`): one leader, commitment in order on every node, a refused command naming the leader, an isolated leader that cannot commit while the majority elects and commits, and invariants checked at every step under loss, duplication and partitions. Membership change and log compaction are not written
- [x] `Transport` trait (pluggable) — `transport::Transport<M>` takes time from the caller, with `send` and `deliver`; `transport::SimNetwork` is the simulated implementation (tests in `tests/transport.rs`)
- [x] Federation layer: signed, hash-chained epoch summaries — hash-chained summaries (`federation`) and Ed25519 signatures of each summary's hash (`signed`, feature `crypto`, RFC 8032 vector checked, tampering, wrong key and moved signatures rejected); a single signer per domain, with no key distribution or rotation (not in v0.1)
- [x] Optional `tpt-crypto` feature for signatures/hashes (licence-checked) — feature `crypto` in causal-consensus and reflect, with `ed25519-compact` 2.6.1 (optional, MIT, checked by `cargo deny check licenses`); hashes stay in-crate SHA-256
- [x] `ConsistencyLevel`: Commutative, Causal, DomainLinearizable, GlobalAgreed — `consistency::ConsistencyLevel`, with `needs_cross_domain_round_trip` and `needs_domain_round_trip` as the spec table states them (tests in `tests/consistency.rs`)
- [x] API reporting latency lower bound per level — `ConsistencyLevel::latency_lower_bound_ms(Delays)`: zero for Commutative and Causal, two intra-domain one-way delays for DomainLinearizable, two cross-domain one-way delays for GlobalAgreed (a round trip is two one-way delays; the spec wording gives one, a weaker valid bound). Saturating; tested
- [x] `Domain`, `Federation`, `Proposal<Op>` types — `global::Domain<Op>`, `global::Proposal<Op>`, `global::Proposer` and `global::Federation<Op>` (in-process accept round across domains, sealing and chain verification, `tests/federation_group.rs`); the networked round is in `tests/global.rs`; the domain-level `RaftNode` is separate
- [x] Deterministic simulator with partitions, delay, duplication, — `transport::SimNetwork` gives seeded delay, jitter (which reorders), loss, duplication, timed partitions of a pair and of two node sets (`partition_sets`), and timed crashes (`crash`, which drops a node's traffic); crashed nodes are not ticked in the harness (`tests/domain_random.rs`, `tests/global_random.rs`), and the invariant checks below run after every step
- [x] Invariant checks: no conflicting `GlobalAgreed` commits; causal order never violated — conflicting commits are impossible by the quorum argument (each domain accepts one operation per key), and the simulation checks no key commits two values over 40 seeds with jitter, with non-vacuity checks; causal order is checked on every delivered log (`CausalBroadcast::is_causally_ordered`) and on the domain logs at every step
- [x] Linearizability checker for small domain-layer histories — `linearizable::is_linearizable`, a depth-first search with memoised failures over a single register (at most 16 operations; tests in `tests/linearizable.rs`: sequential, stale, concurrent, order-disagreeing and never-written histories, and limits); end to end, `tests/domain_linearizable.rs` proposes writes and reads as log entries of a replicated register under partitions and crashes (300 seeds), checks that the committed history is linearizable and that nodes agree on committed entries; a weakened quorum is not caught by this test, but by the election-safety check in `tests/domain_random.rs`
- [ ] Randomised partition/delay simulation tests — partial: the domain layer has seeded loss, duplication and partitions with per-step invariant checks (`tests/domain.rs`); GlobalAgreed now has 60 seeded schedules with jitter, loss, duplication and a random partition window and up to two crash windows (a crashed domain keeps its state), checking after every step that no key commits two values and that each commit is backed by a majority of domains that hold it (`tests/global_random.rs`, a mutation that drops the conflict rule is caught); the domain layer now has 40 seeded schedules with loss, duplication, pair partitions and crash windows, checking the Raft invariants after every step (`tests/domain_random.rs`; weakening the quorum makes it fail); crashes that lose state are not modelled
- [ ] Fuzz: message decoders — not applicable as stated (2026-11-15): causal-consensus has no byte-level decoder; messages are typed values passed through `Transport<M>`. Opens when a wire format exists
- [x] `no_std` + `alloc` build — builds with no features on thumbv7em (the clock, vector, consistency and error modules), and with `alloc` on the host; the `alloc` modules are not built for thumbv7em
- [x] README states honest limits (CAP under partition); Byzantine tolerance deferred — `README.md` gives the choice per level with its test, and states the crash model, the clock bound, the history scope and the signature scope; the causal layer is tested across a cut (`tests/causal_partition.rs`)
- [ ] Later: Byzantine tolerance behind a feature
- [ ] **v0.1 exit:** causal layer, domain consensus, federation epochs, consistency API with bounds, simulator

### 3.2 `tpt-kardashev-isru`
- [ ] Review `tpt-process` (`tpt-proc-mass-balance`, `tpt-proc-flowsheet`), `tpt-systems-optimisation`, `hiveforge-resources`; record in provenance
- [x] Types: `Element`, `Material` (mass fractions), `Resource` (decided 2026-11-16: not a separate type), `Process` — `Resource` is not written (stock is a plain array indexed by species); `Element` (14 elements with atomic masses), `Composition` (fixed array of mass fractions, no heap), `Material`, `Process` (`plan::Process`, checked at construction)
- [x] Elemental mass-balance validation (reject unbalanced processes at load time, with tolerance) — `material::check_balance` checks total mass and each element to a relative tolerance; `Process::new` rejects unbalanced processes (tests in `tests/planning.rs`)
- [x] Atomic-mass data source for elements (documented, licence recorded) — https://ciaaw.org/atomic-weights.htm, read for this crate: midpoints of the intervals, or the single value; the CIAAW table's terms of use were not checked
- [x] Process/resource loading format (document it) — `format`: version 1 text format (materials with element fractions, processes with run limits and `in`/`out` lines, an `end` line), read by `read_problem` and written by `problem_to_text`; tests in `tests/format.rs` cover round trips, malformed and unbalanced input with line numbers, truncation, and the every-prefix property. Resources are not a separate type: stock is a plain array indexed by species
- [x] LP formulation over a time horizon — `plan::plan_maximum`: run counts as variables, stocks and run limits as bounds, the target produced as the objective; a horizon is the stock and run limits (no time-indexed stocks yet)
- [x] Built-in simplex solver for small problems — `simplex::maximise`: dense tableau, Bland's rule, slack start (b >= 0), tested on a textbook LP and an unbounded case
- [x] Solver adapter trait for external solvers — `simplex::LpSolver` trait; `plan_maximum_with` takes any solver, and `Simplex` is the built-in one; a counting adapter is tested (`tests/closure.rs`)
- [x] Closure scores (bill of materials) and list of limiting imports — `closure::closure`: per demanded species, in-situ net production over demand capped at 1; the score is the lowest ratio; `limiting` and `imports_kg` name the shortfalls. Energy closure is not in it (see the energy item)
- [ ] Optional `energy` feature: accounts via `tpt-kardashev-energy-ledger` — decided: no feature. A library crate in this repo may not depend on the ledger crate; the caller combines the planner's output with the ledger. The empty feature is removed
- [x] Conservation tests (total mass and each element through any plan) — the planner's plans conserve every element (`tests/planning.rs`)
- [x] Closed-form toy systems with known optima — hematite reduction in exact proportions, and hydrogen reduction against its closed form
- [x] Closure monotonicity test (adding a process never lowers closure) — `tests/closure_horizon.rs`: the closure score never falls as stock or run limits grow over a horizon, matching its closed form, and adding a magnetite process never lowers it and raises it for some demands; the planner-optimum check remains in `tests/planning.rs`
- [x] Worked lunar-regolith example — `examples/regolith.rs` (run with `--features alloc`) uses the Apollo regional averages (highlands, low-Ti mare, high-Ti mare) from NASA/TM-20240011783 Table 6, the primary source, checked against the same table in arXiv 2609.06496 (headers and oxide order confirmed). Oxides are converted to element fractions and normalised to 100 wt%. Hydrogen reduction of FeO with water electrolysis, planned for oxygen, with closure and imports. Highlands, checked by hand: iron 51.38 kg, oxygen 14.72 kg, score 0.0368. Mass only; energy not modelled, and only FeO is reduced, so the closure is a lower bound
- [x] `no_std` + `alloc` build — builds with no features and with `alloc` on thumbv7em-none-eabihf; the planner needs `alloc`, so it is not in the no-alloc build
- [x] README states what is not modelled — `tpt-kardashev-isru` README "What it does not model"
- [x] **v0.1 exit:** types, balance validation, LP planner, closure score, lunar-regolith example

### 3.3 `tpt-kardashev-reflect`
- [x] Review `tpt-formal`, `tpt-writ`, `tpt-norma` (queued); note interoperability with norma — the 20 `tpt-formal` crates at README level, `tpt-writ` and `tpt-norma` at README and architecture level; the norma note (overlap with the linear-invariant layer, certificate formats, no code dependency) is in `docs/provenance/tpt-kardashev-reflect.md`
- [x] `Invariant` type: linear inequality systems over policy parameters — `invariant::Constraints` (rows `coefficients · x <= bound` over exact rationals, `Invariant` and `Update` aliases), with exact `contains`
- [x] `Update` and `Certificate` types — `invariant::Certificate`: one non-negative multiplier vector per invariant row, over the update's rows
- [x] Exact or rational arithmetic choice for the checker (in-crate, no float surprises); document — `rational::Rational` on checked `i128`: every operation reduces or reports `Overflow`; the checker compares exactly, with no tolerance (tests in `tests/rational.rs`)
- [x] `Verifier`: Farkas-style containment certificate checker — `verify::verify`: the checker; the soundness argument is in its module docs, and the property test in `tests/containment.rs` checks it on 2000 seeded systems
- [x] Certificate generation helper (not trusted) for tests/examples — `certify::certify`: a phase-one simplex over exact rationals (Bland's rule) for the multipliers; its output is checked by `verify`, and tests show it finds a certificate in every constructed containment and that its output always passes the checker
- [x] `UpdateGate` issuing hash-chained `Receipt` or rejection with reason — `gate::UpdateGate`: the only path to a new policy; every submission is recorded, accepted or refused with its reason, in a hash-chained log (`gate::Receipt`, `Outcome`); tests in `tests/gate.rs`
- [x] Audit-log verification — `gate::verify_audit`: positions, links and receipt hashes; it cannot re-check the updates, which the log keeps only as digests (tested for rewriting, reordering, removal and broken links)
- [x] Rule: checker not modifiable through the gate; external attestation path for replacement (design and docs) — `attest::AttestedGate`: the invariant is replaced only under an Ed25519 signature by a fixed authority over the old digest, the new digest and the log position, and only if the current policy is still contained in the replacement; refusals are recorded (`tests/attest.rs`). Choosing and rotating the authority key is outside the crate
- [x] Hash function choice (in-crate or `tpt-crypto` feature, licence-checked) — in-crate SHA-256, a copy of the causal-consensus module (no crate may depend on another); merging the copies is a phase 4 consolidation item
- [x] Kani proofs for checker arithmetic core (decided 2026-11-16): kept the two harnesses that finished, `new_is_in_lowest_terms_and_keeps_the_value` (SUCCESSFUL, 1522 s) and `checked_cmp_agrees_with_the_exact_order` (SUCCESSFUL, 2060 s), both bounded to |x| < 2^8. The checked add and checked multiply harnesses were removed: each ran past two hours in CBMC with the solver still working, so they have no verdict. Their exact checks are now exhaustive over all fractions with numerators and denominators in [-6, 6] (`tests/rational_exhaustive.rs`).


- [x] Soundness property tests with random polytopes — random two-parameter systems, certificates built by combination with and without slack, and every accepted certificate checked on a grid of points (`tests/containment.rs`); polytopes here are the general systems, not only bounded ones
- [x] Adversarial tests: malformed and truncated certificates — the text format (`text`) rejects malformed lines with their numbers, requires a terminating `end` line and a declared row or multiplier count, and every prefix of a valid constraint or certificate file is either rejected or reads to the same value (`tests/text.rs`); the property found two truncation bugs, both fixed: a file cut at a row boundary read as shorter, and a number cut inside a fraction read as a different value
- [ ] Fuzz: certificate parser — partial: `fuzz/` target `text_read` (any accepted file writes back to an equal value) ran 2.5 minutes with no crash over 3.8 million inputs; a kept corpus and longer runs are still to do [2026-11-16: five minutes per target on nightly under WSL, twelve targets in all, no crash or artifact; minimised corpora committed in fuzz/seeds/; campaigns of an hour or more still to do]
- [x] `no_std` + `alloc` build (`tpt-kardashev-reflect`: no features and `alloc`, on thumbv7em-none-eabihf)
- [x] README states open research risk and that only linear bounds are supported — `tpt-kardashev-reflect` README (status and "What it does not model")
- [ ] Later: optional adapters to `tpt-formal` and `tpt-solver`
- [ ] **v0.1 exit:** linear invariants, containment certificates + checker, gate with receipts, audit-chain verification

### Phase 3 close
- [ ] All three crates at v0.1 exit
- [ ] Tag `<crate>-v0.1.0` for each
- [x] CI green — runs 43695134560 (b9197a4) and 43699093000 (d7590e5) cover this crate set

---

## Phase 4: Consolidation

Exit: per-repo migration plans (notes, not code changes).

- [x] Migration notes: `tpt-zenith` (orbital-mechanics, rf-pipeline fec, routing) → astrodynamics, archive-codes, dtn — written in `docs/migration.md`; `rf-pipeline` fec and framing are not read yet and are marked so
- [x] Migration notes: `substrate` (substrate-dtn) → dtn — `substrate-dtn` is inside `tpt-solutions/tpt-substrate` (its own repo is 404); read and written up in `docs/migration.md`: nothing of the wire format migrates, and the flood fallback and `HashMap` ordering do not, with the concepts that do map listed
- [x] Migration notes: `tpt-equinox` (tmr, orbital) → radhard, astrodynamics — in `docs/migration.md`
- [x] Migration notes: `tpt-sci-astro` → astrodynamics — in `docs/migration.md`
- [x] Migration notes: `tpt-transportation` (space) → astrodynamics — in `docs/migration.md`
- [x] Migration notes: `tpt-formal` (tpt-for-redundancy, proof AST) → radhard, reflect — in `docs/migration.md`; the other tpt-formal crates are listed as not read
- [ ] Decide ignis adoption of nuclear-plasma (coordinate when ignis reaches physics phase) — deferred: ignis's equilibrium is a reduced model (see the provenance review); the decision waits for ignis to reach its physics phase
- [ ] Cross-check tests of new crates against old implementations; record divergences (2026-11-16: the old implementations are in sibling repositories that cannot be built in this workspace; the checks that exist are recorded per crate in the provenance files, and the rest is not claimed)
- [x] Public trait consolidation pass and the path to 1.0 (decided 2026-11-16; see `docs/consolidation.md`: no shared trait crate, every trait stays in its crate, criteria for 1.0 listed). Earlier note: deferred until the v0.1 crates are complete; the traits so far (Transport, LpSolver, ForceModel, ConvergenceLayer, EnergyOracle) are listed for that pass
- [x] Decide on crates.io publishing and optional umbrella crate `tpt-kardashev` — decided: no publishing (every crate keeps `publish = false`) and no umbrella crate for now; the user has not asked for publication, and an umbrella adds a dependency graph to maintain before the crates are at v0.1
- [x] Decided 2026-11-16: no action, nothing depends on it. Licence inconsistency (MIT only vs dual) if it is to become a dependency — finding (2026-11-15): `tpt-solutions/tpt-substrate` has one LICENSE file, MIT (GitHub licence key `mit`), and no Apache text; the decision is the owner's, and it is not needed while nothing depends on it
- [ ] Final spec update and retrospective

---

## Ongoing

- [ ] Keep every crate's README "what this does not model" section current
- [ ] Keep `docs/provenance/` current whenever code is ported
- [ ] Re-verify standards parameters against current documents when work on a crate starts
- [ ] Re-run `cargo deny` whenever a dependency is added or bumped
- [ ] No Apache-only or GPL-family code pasted into library source
