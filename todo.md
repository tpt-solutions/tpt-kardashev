# tpt-kardashev: todo

Foundation libraries for planetary-to-galactic scale. Ten independent Rust crates in one Cargo workspace.
Spec: `spec.txt`. Licence: MIT OR Apache-2.0. Owner: TPT Solutions.

**Status (2026-11-04):** Phase 0 scaffold written and checked locally (fmt, clippy, tests, no_std thumbv7em, builds-alone, docs, cargo-deny licences and advisories, licence-policy test). CI is written but not yet run on GitHub; the Phase 0 exit ("CI green") stays open until it is pushed and passes. Committed to master in stages (scaffold, relativity, archive-codes).

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
- [ ] Initialise git repository and make first commit

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
- [ ] Verify CI is green on empty crates (**Phase 0 exit**)

### Cross-cutting infrastructure
- [ ] Shared test conventions: recorded proptest seeds, tolerance helper pattern (per-crate, not shared crate)
- [ ] Decide benchmark setup (criterion as dev-dependency) and a regression-check convention
- [ ] Fuzz harness layout (`cargo-fuzz`, per-crate `fuzz/`) and a template

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
- [ ] Fuzz: decoders and frame parser never panic on arbitrary input — partial: targets in `fuzz/` ran 60 s each on nightly (frame ~6M runs, RS ~44k, soft Viterbi ~348k, LDPC ~1.7M) with no crash; longer campaigns and a kept corpus are still to do
- [x] Kani proofs for selected arithmetic (GF ops) — 5 GF(2^8) harnesses and 2 dual-basis harnesses, all verified with Kani 0.68 (run in WSL Ubuntu; Kani does not support Windows)
- [x] Benchmarks: BER curves vs theory as regression checks — `tests/ber.rs` (exhaustive-ML minimum-metric check; soft BER within the converged union bound at 3 dB) and `examples/ber_curve` (2–4 dB table, release mode). Criterion timing benchmarks are not part of this item.
- [x] `no_std` and no-alloc build for fixed-size paths; thumbv7em build
- [x] README states what is not modelled (no compliance claim; turbo codes deferred; RaptorQ not included)
- [x] Examples (RS + interleave round trip, archival frame)
- [x] **v0.1 exit:** RS(255,223)+interleaving with vectors (standard tables: Annex F, Annex G, §4.4.1; the standard has no codeword vectors); conv K=7 + Viterbi (ML-checked, BER-checked); CRC; framing; no_std/no-alloc

### 1.2 `tpt-kardashev-relativity`
- [x] Review `tpt-replicate/src/clock.rs` for the bounded-drift assumption; note in provenance — done: the bound is a configured limit (`DEFAULT_MAX_DRIFT_MS`), recorded in provenance
- [ ] Define `Scalar` trait (default `f64`), error enum, `Seconds`/`Metres` newtypes — partial: `f64` only; generic `Scalar` trait not defined
- [x] `Event`, `FourVector`, `Interval` (timelike/spacelike/lightlike)
- [x] `Lorentz` boost with `|v| < c` enforced; composition via rapidity
- [x] Time dilation, length contraction, relativistic Doppler
- [x] Inertial `Frame` and frame-to-frame transforms
- [x] Causal predicates: `precedes`, `light_cone_contains`, `causally_disconnected`
- [x] `CausalOrder` trait (neutral, consumed by other crates)
- [x] `proper_time(path)`
- [x] Weak-field gravitational time dilation (Schwarzschild approximation)
- [x] Proper time vs coordinate time for a moving, gravitating observer
- [x] Time-scale rate relations TT, TCG, TCB, TDB as functions (IAU constants documented) (constants to verify against the IAU texts)
- [x] `Metric` trait with Minkowski and Schwarzschild implementations
- [x] Documented float-tolerance policy
- [x] Property tests: interval invariance under boosts, composition law, `|v| ≥ c` rejected
- [x] Known-value regression: GPS ≈ +38 µs/day net offset
- [x] `no_std` build
- [x] README states what is not modelled (no geodesic integration, no strong-field)
- [x] Examples
- [x] **v0.1 exit:** events, intervals, boosts, proper time, causality, Doppler, weak-field redshift, tolerance policy (time-scale constants still to verify against the IAU texts)

### 1.3 `tpt-kardashev-radhard`
- [ ] Review `tpt-equinox-tmr` (done: read, recorded in provenance) and `tpt-for-redundancy` (not found on GitHub; stays open)
- [x] `Tmr<T>` with majority vote and scrub-on-mismatch
- [x] N-modular variant
- [x] Typestate wrappers (unvoted vs voted types)
- [x] SECDED (72,64) encode/decode
- [x] CRC-protected cells and checksummed buffers
- [x] `Checkpoint<T>` with rollback and watchdog-friendly progress marker
- [x] `FaultInjector` (seeded bit flips, burst errors)
- [x] Volatile access behind a feature, `unsafe` documented (the one exception to forbid-unsafe)
- [ ] Kani proof: no single corrupted replica changes the voter result — not run: Kani not installed; exhaustive voter test instead
- [ ] Kani proof: SECDED corrects every single-bit error and detects every double-bit error — not run: Kani not installed; exhaustive SECDED test instead
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
- [ ] Optional `units` feature mapping to `tpt-math-units`
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
- [ ] CI green for all four, including cargo-deny

---

## Phase 2: Networking, orbits, plasma

Exit: each crate at v0.1 exit; `ContactPlan` flow from astrodynamics to dtn demonstrated in an example.

### 2.1 `tpt-kardashev-dtn`
- [ ] Review `substrate-dtn` and `tpt-zenith/routing`; record in provenance — partial: `tpt-zenith/routing/cgr` read (earliest arrival, no SABR rules); `substrate-dtn` is not on GitHub under `tpt-solutions` or `PhillipC05` and stays open
- [x] Check `tpt-rfc` / `tpt-aion` for existing CBOR before writing one (§12.6) — done: `tpt-rfc` has a sibling CBOR codec (`tpt-cbor`) and `tpt-aion` has none; this crate keeps its own codec, since a dependency on another repo would break the no-inter-crate rule (provenance records this)
- [ ] Confirm current RFC 9171 (BPv7), RFC 5326 (LTP), CCSDS 734.1-B and SABR documents — partial: RFC 9171, RFC 8949 and RFC 5326 read; CCSDS 734.1-B not yet; SABR (CCSDS 734.3-B-1) read
- [x] Minimal in-crate CBOR codec (definite and indefinite lengths as required by BPv7)
- [x] Endpoint ids: `dtn:` and `ipn:`
- [x] Primary block, canonical blocks (payload, previous node, bundle age, hop count) — typed previous-node, bundle-age and hop-count blocks (`extension`), validated on encode and decode; the simulator enforces the hop limit from the Hop Count block
- [x] Bundle CRCs (CRC-16 X.25, CRC-32C)
- [x] `Bundle` encode/decode with strict validation
- [x] `BundleStore` trait; in-memory store
- [x] File-backed store
- [x] `ContactPlan` and `Contact` types (start, end, rate, one-way light time)
- [ ] Contact-graph router (CGR/SABR family), `Router` trait — partial: `Router` trait and `EarliestArrival` (earliest-arrival over the contact plan, with waiting and store-and-forward) are written, and the simulator holds bundles and routes them hop by hop with them (opt-in), re-routing when the contact plan changes during a run; SABR expiry, OWLT margin and per-contact volume (CCSDS 734.3-B-1 §2.4.1–2.4.3, §3.2.6.9) are applied in the simulator, with volume reserved when a bundle is routed; the rule that a bundle is not sent back to the node it came from (§3.2.5.2) is applied; critical bundles are sent one copy per neighbour (`Network::send_critical`, §2.4.5); not done: multiple copies beyond that, excluded neighbours from custody refusal (§2.4.4; deferred with the custody extension, see the reliability note), fragmentation (§2.4.4–2.4.5, §3.2.8.2), and backlog measured in volume
- [x] `ConvergenceLayer` trait (`convergence`)
- [ ] LTP convergence layer (segments, sessions, reliability via report/ack, timers on virtual clock) — partial: segment codec (`ltp::segment`), red-part sessions with report/ack and countdown timers on a virtual clock (`ltp::session`), and `convergence::LtpConvergence` carrying one encoded bundle per LTP block; green part, cancellation procedures and link-state scheduling not modelled
- [x] Reliability note: bundle-level custody/reliability extension deferred, documented as such (README)
- [x] Optional `fec` feature — dropped: the no-inter-crate-dependency rule (CLAUDE.md) forbids `tpt-kardashev-dtn` depending on `tpt-kardashev-archive-codes`, so the spec §4.2 row and the empty `fec` feature were removed. Link-layer FEC, if wanted later, needs a dtn-local trait and a decision recorded then
- [x] Deterministic simulator with virtual clock and contact-plan-driven links (`sim::Network`: a datagram moves only inside a contact window, at the contact's rate, plus one-way light time)
- [ ] Seeded fault injection (loss, delay, partition, duplication, reordering) — partial: seeded loss, duplication, and delay jitter (which reorders); link failures (`fail_link`) learned from cancelled sessions; `Network::partition` cuts both directions of a link for a time window (tested: a short partition delays delivery and does not mark the link failed); partitions that split a set of nodes are written (`Network::partition_sets`, which cuts every link between two groups in both directions, tested); reordering still comes only through jitter
- [ ] RFC 9171 appendix examples as byte-exact vectors — partial: RFC 9171 has no byte-level examples (its appendix B is CDDL). Used a bundle derived by hand from the CDDL, and the RFC 8949 Appendix A CBOR examples
- [ ] Round-trip tests and fuzzing of CBOR/bundle parser — partial: round-trip and bit-flip tests written; fuzz targets (`bundle_decode`, `cbor_read`, `ltp_segment`, `sdnv_decode`) ran 4 minutes each on nightly after the 60-second runs, with no crash; a kept corpus and longer campaigns are still to do
- [x] Routing tests against hand-computed contact-plan scenarios
- [x] Simulator tests for delivery under disruption — bundles delivered across a relay whose second link opens late (store-and-forward, transmissions checked against windows), under 25% loss with duplication and jitter (exactly one copy), and determinism for a fixed seed
- [x] Optional `serde` feature (`serde` feature: URI text for Eid, codes for CrcType, maps for the integer records; Contact, BundleId, Hop and Route serialise only; bundles, segments and sessions not covered)
- [x] `no_std` + `alloc` build (thumbv7em, with and without `alloc`)
- [x] README states what is not modelled (no compliance claim; no BPSec, no TCPCL in v0.1)
- [ ] **v0.1 exit:** BPv7 codec with vectors, stores, CGR router, LTP layer, simulator

### 2.2 `tpt-kardashev-astrodynamics`
- [x] Check `tpt-sci-astro` and `tpt-transportation` for an existing Lambert solver (§12.7); record in provenance — done: `tpt-transportation`, `tpt-zenith` and `tpt-science/crates/tpt-sci-astro` (under `tpt-solutions`) were read; none has a Lambert solver (`tpt-sci-astro` says so in its own example). `tpt-equinox/crates/tpt-equinox-orbital` is terrain insolation, not orbital mechanics
- [ ] Review `tpt-zenith/orbital-mechanics` and `tpt-equinox-orbital`; record in provenance
- [ ] Define `Scalar`, units newtypes (`Kilometres`, `Seconds`), error enum — partial: `Kilometres`, `Seconds`, `Radians` (finite-checked), `Error` (`#[non_exhaustive]`) and `Vec3` are written and tested; no `Scalar` type yet (plain `f64` in the API)
- [ ] Epochs and time scales (UTC, TAI, TT, TDB), Julian dates, leap-second handling (user-supplied table, no bundled data) — partial: TT epochs, TAI, UTC through a caller-supplied table, and Julian dates are written and tested; TDB not written (its periodic difference from TT needs a series not yet verified); UT1 not written
- [ ] Optional `relativity` feature for relativistic scale relations
- [ ] Reference frames: inertial (GCRF-like), Earth-fixed, perifocal, RTN/LVLH, heliocentric ecliptic, and transforms between them — partial: perifocal ↔ inertial (`Keplerian::perifocal_to_inertial`) and RTN (`frames::rtn_matrix`) are written and tested; LVLH is written with one documented convention (`frames::lvlh_matrix`: x along-track, y along −N, z nadir; tested); heliocentric ecliptic needs a heliocentric origin; Earth-fixed needs UT1; GCRF-like not separately defined yet
- [x] Element sets (Keplerian, equinoctial) and state-vector conversions — Keplerian ↔ state for elliptic orbits, with fixed conventions for circular and equatorial cases; equinoctial `(p, f, g, h, k, L)` ↔ Keplerian and ↔ state (`equinoctial::Equinoctial`), tested by closed-form values, a 144-case state round trip including the singular cases, and regularity across the equatorial plane
- [x] Two-body propagation (universal variables) — `propagate::propagate`, tested for periods, conservation, composition, reversal, agreement with an independent Kepler-equation solve, and hyperbolic states
- [x] `ForceModel` trait — `force::ForceModel` (acceleration at a time and state); a pair of models is a model (their sum)
- [ ] Forces: central body, zonal harmonics J2…Jn, third bodies, SRP with shadow model, atmospheric drag, thrust — partial: central body (`force::CentralBody`), J2 (`force::J2Zonal`, WGS 84 constants) and third-body gravity of the Sun and Moon (`force::ThirdBody`) written; solar radiation pressure with the Earth's umbra and penumbra (`force::SolarRadiationPressure`, the visible fraction of the Sun's disk from `force::lit_fraction`, checked against sampling of the disk; albedo and infrared not modelled) exponential-atmosphere drag (`force::AtmosphericDrag`, `atmosphere`) and finite burns (`force::Thrust`: constant acceleration, fixed or prograde, in a time window, no mass flow) also written; zonal harmonics of any degree are written (`force::ZonalHarmonics`, with the coefficients supplied by the caller; checked to degree 6 against the gradient of the geopotential) but their WGS 84 constants are not bundled, because the values could not be checked against a primary source; a solar-cycle-dependent atmosphere, and mass-dependent thrust are deferred past v0.1 (decided: mass in `StateVector` is a public API change, and the solar-cycle atmosphere needs a sourced density dataset)
- [x] Cowell combined numerical propagator — `integrate::Cowell` integrates any `ForceModel`, including summed models; tested against the analytic two-body solution
- [x] Integrator: fixed-step RK4 — `integrate::rk4_step`; error falls as the fourth power of the step (measured 16.4× per halving)
- [x] Integrator: adaptive high-order Runge-Kutta — `adaptive::AdaptiveCowell`, Dormand–Prince 5(4) with error control on position and velocity; tested against the analytic two-body solution over three orbits (5e-7 km at 1e-12 tolerances), tighter tolerances giving smaller errors, energy drift 4e-12 over five orbits, and backward propagation
- [ ] Integrator: symplectic for N-body — partial: `symplectic::Verlet` (velocity Verlet, second order) is written for accelerations that depend only on position, and tested on the two-body problem; it does not yet integrate several bodies in one state, so the N-body case is not covered
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
- [ ] Cross-check new force model against old secular formulas (J2, drag, etc.) — partial: J2 nodal regression of an integrated orbit matches the secular rate to 0.2% (`tests/j2.rs`); the perigee rotation matches the J2 secular rate to 0.5%, with the rate formula from `tpt-sci-astro` (`tests/j2.rs`), and that crate's `EARTH_J2` agrees with ours to 3×10⁻⁷ relative; drag is checked only against the textbook decay rate (`tests/drag.rs`), since no sibling repo has an old drag formula
- [ ] Fuzz: TLE and SPK parsers — partial: deterministic robustness tests (`tests/parser_robustness.rs`) feed 20 000 mutated and 20 000 random element-set pairs, truncations of a valid set, and 5 000 random byte strings (half with the DAF magic) to `parse_tle` and `read_segments`; none panics. `cargo-fuzz` targets are in `fuzz/` (`tle_parse`, `spk_read`). A 90-second nightly run of each found a panic in `parse_tle` (a multi-byte character sliced mid-character; fixed, with a regression test). After the fix, five-minute runs of each found nothing more. The SPK target reaches its file-record checks only with a valid kernel as a seed and a magic-word dictionary (`fuzz/dictionaries/spk.dict`); with them, coverage rose from 26 to 97 edges over 4.5 million runs. The seed is not kept in the repository; longer campaigns and a kept corpus are still to do
- [x] `no_std` + `alloc` build — builds with `--no-default-features --features alloc` on the host and on thumbv7em; the crate has no heap-allocating API yet, so this checks the feature only, and no alloc-dependent test exists
- [x] README states force-model limits and what is not modelled
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
- [x] Example: astrodynamics visibility windows → `ContactPlan` → dtn routing — `examples/contact-plan-flow`, a standalone package. It uses no feature flags: a library crate may not depend on another crate in this repo, but an example may, so the example depends on both crates directly. The astrodynamics `dtn` feature was removed for the same reason. The run computes four passes per station in six hours and delivers a routed bundle to station B
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
