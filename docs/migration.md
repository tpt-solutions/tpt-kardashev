# Migration notes: sibling repositories into tpt-kardashev

Notes, not code changes (todo, Phase 4). For each sibling repository: what it holds, where it goes in
this workspace, what has been read, and how it differs from the implementation here. Nothing is copied:
sibling code is read for reference, and each crate here is written from its own source. Licences are the
ones stated in each repository. Where a repository could not be read, the note says so.

Repositories were read from public GitHub under `tpt-solutions` with shallow clones in a scratch
directory, outside this workspace.

## tpt-zenith → astrodynamics, archive-codes, dtn

- `orbital-mechanics` (SGP4/SDP4, Go and Rust parts, MIT OR Apache-2.0). Read: the deep-space code
  (`deep_space.rs`: dpper, dspace, the resonance coefficients and the resonance integrator). Target:
  `tpt-kardashev-astrodynamics` (`sgp4`, `sdp4`). Differences found: zenith's `dsinit` root constants are
  written as square roots of the reference values (about 48 times this crate's d2201); zenith's low-
  inclination `dpper` applies the `atan2` correction unconditionally and omits the `dls` term. This crate
  follows the reference on all three. zenith's verification test was not run (running that code was
  refused in this session), so its status is unknown.
- `routing` (Go, MIT OR Apache-2.0). Read: `routing/cgr` (earliest arrival over a contact plan, with
  volume and duration checks). Target: `tpt-kardashev-dtn` (`route`). This crate's router adds the SABR
  rules that the zenith search does not have: expiry, owlt margin and volume reservation.
- `rf-pipeline` (`src/fec.rs`, `src/frame.rs`). Not read in detail. Target: `tpt-kardashev-archive-codes`
  (FEC, framing). To be read before any change; the archive codes crate is written from CCSDS 131.0-B-5.
- `bundle`, `cbor`, `contactplan`, `node`, `sim` (Go). Not read in detail. Target: `tpt-kardashev-dtn`.

## substrate (substrate-dtn) → dtn

- Not readable. `tpt-solutions/substrate-dtn` and `PhillipC05/substrate-dtn` both return 404 without
  credentials, and no other public repository of that name was found. No migration note can be written
  until the repository is accessible or its owner is confirmed.

## tpt-equinox (tmr, orbital) → radhard, astrodynamics

- `tpt-equinox-tmr` (MIT OR Apache-2.0). Read: `vote3` and `TmrCell`. Target: `tpt-kardashev-radhard`
  (`tmr`). Difference: `vote3` returns copy `a` as the value when all three copies differ, with a flag;
  radhard's `Nmr::vote` returns `NoMajority`. radhard keeps its behaviour. The `tpt-equinox-tmr` `kani.rs`
  is a proof harness and was not read in detail.
- `tpt-equinox-orbital` (MIT OR Apache-2.0). Read. It is lunar-terrain insolation (a `no_std` heightmap
  and shadow model). It has no orbital mechanics, so nothing migrates to astrodynamics. Its `lit_fraction`
  is a terrain test and is not the penumbra model in astrodynamics.
- `tpt-equinox-power` (breaker topology and exclusion, MIT OR Apache-2.0). Read. A power-bus safety rule,
  not energy physics. Target: none in this workspace; the energy ledger does not need it.
- `tpt-equinox-thermal` (radiator deployment). Read. Its cooling coefficient is arbitrary; it does not use
  the Stefan–Boltzmann law. Target: none; energy-ledger's radiator is the Stefan–Boltzmann model.
- `tpt-equinox-mpc`, `tpt-equinox` (umbrella). Not read in detail.

## tpt-sci-astro → astrodynamics

- `tpt-science/crates/tpt-sci-astro` (MIT OR Apache-2.0). Read: its public API (two-body elements, state
  conversion, two-body propagation, first-order J2 secular rates) and its `lambert_transfer` example. Its
  example states that the crate has no Lambert solver. Target: `tpt-kardashev-astrodynamics`.
- Constants: `EARTH_J2 = 1.08262668e-3` agrees with this crate's `WGS84_J2` to 3×10⁻⁷ relative (truncation).
  The perigee-rate formula is the same as this crate's J2 test.
- Not migrated: nothing; this crate already has the same physics, with more coverage.

## tpt-transportation (space) → astrodynamics

- `crates/space` (`orbital.rs`, MIT OR Apache-2.0). Read: the Keplerian elements and the Hohmann transfer.
  The Hohmann formula is the same as this crate's. Its test tolerance is loose (±7% of 4.04 km/s, where its
  own comment gives 3.85 km/s for the coplanar transfer). Target: `tpt-kardashev-astrodynamics`
  (`manoeuvre`). `aerospace/src/space.rs` and `thermal.rs` are not read in detail; the thermal code has no
  target in the current plan.

## tpt-formal (tpt-for-redundancy, proof AST) → radhard, reflect

- `tpt-for-redundancy` (`no_std`, MIT OR Apache-2.0). Read. Its `majority_vote` returns `None` without a
  strict majority, as radhard's `NoMajority` does; `Tmr`, `Redundant<T, N>` and `Parity` overlap with
  radhard's TMR, N-modular and checksum types. Target: `tpt-kardashev-radhard`. No code moves; the
  behaviour agrees, and the difference is in naming.
- `tpt-for-proof-ast` (MIT OR Apache-2.0). Read: its module header. It models first-order terms, formulas
  and proof trees as structure only, and says soundness checking is left to downstream tools. Target:
  `tpt-kardashev-reflect`, if a proof representation is ever needed. It has no linear-arithmetic
  certificates, so the containment checker in reflect is not replaced by it.
- Other `tpt-formal` crates (`tpt-for-sat`, `tpt-for-smt-lite`, `tpt-for-model-check`, `tpt-for-vcgen`,
  and the rest) were listed but not read. Relevant later to reflect's Kani and certificate-generation items.

## Repositories not reviewed

- `tpt-writ`, `tpt-norma` (queued for reflect, todo §3.3): reachable on GitHub, not yet read. The note for
  norma's interoperability is open.
- `tpt-for-deterministic-sim`, `tpt-for-det-proptest` (in `tpt-formal`): relevant to the simulators in
  dtn and causal-consensus; not yet read.

## Divergences recorded

The divergences found are kept in each crate's provenance file, and none is a bug in this workspace:

- radhard, TMR tie: `Nmr::vote` returns no value on a three-way disagreement.
- astrodynamics, SDP4: the zenith `dsinit` constants and `dpper` branch differ from the reference, which
  this crate follows.
- dtn, routing: the zenith search has no SABR rules.
