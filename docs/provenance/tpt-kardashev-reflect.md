# Provenance: tpt-kardashev-reflect

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reviewed for reference (not copied)

- tpt-formal, tpt-writ, tpt-norma: reviewed; see "Sibling review (todo §3.3)" at the end of this file. README and architecture level for all of them, with source read only for `tpt-for-redundancy` and `tpt-for-proof-ast` (see `docs/migration.md`). No code taken.

## Third-party licences

None.

## Written from standard relations

- `verify.rs` and `invariant.rs`: the Farkas lemma for linear inequalities, in the form used for containment: a system `A x <= b` implies `c·x <= d` when some non-negative multiplier vector gives `λ^T A = c^T` and `λ^T b <= d`. The soundness argument is in the module docs. Written from that standard form; no reference code consulted.
- `rational.rs`: exact rational arithmetic on checked `i128`, with the usual reduction, written here. No reference code consulted.
- `gate.rs`: the gate and the receipt chain are this crate's design: a fixed invariant, the policy changed only on a verified certificate, each receipt naming the hash of the one before with a domain-separation prefix. No reference code consulted.
- `hash.rs`: a copy of the causal-consensus crate's SHA-256 module (FIPS 180-4), by the same author. Kept as a copy because no crate may depend on another; see the phase 4 consolidation item.
- `attest.rs`: the attestation path is this crate's design: a fixed authority signs a message with a domain tag, the old invariant digest, the new digest and the log position. The signature is `ed25519-compact` 2.6.1 (MIT). No reference code consulted.

## Sibling review (migration notes)

- `tpt-solutions/tpt-formal`, `crates/tpt-for-proof-ast` (MIT OR Apache-2.0), read: first-order term, formula and proof-tree structure, with soundness left to downstream tools. It has no linear-arithmetic certificates, so it does not replace the containment checker here. No code taken.
- `certify.rs`: the phase-one simplex (artificial variables, Bland's rule, rational pivots) written from the standard method. The generator is not trusted: `verify` decides acceptance. No reference code consulted.
- `text.rs`: the file format is this crate's design, documented in the module; no third-party format copied.

## Sibling review (todo §3.3)

Depth: README and architecture for `tpt-writ` and `tpt-norma`; the README opening for each of the 20 `tpt-formal` crates; source only for `tpt-for-redundancy` and `tpt-for-proof-ast`. No code taken from any of them.

- `tpt-solutions/tpt-norma` (MIT OR Apache-2.0, a Rust runtime policy engine for AI-agent actions). Overlap with reflect: `tpt-norma-linear` (exact linear constraints with a contradiction check), `tpt-norma-policy` (a text DSL of N-dimensional boxes and linear rules), `tpt-norma-proof` (a certificate format pinned to source digests) and `tpt-norma-solver` (certifies decisions through an external SMT solver). The questions differ. Reflect asks whether one linear invariant system implies another, and checks a Farkas certificate for it. Norma's contradiction check asks whether one policy is consistent with itself. Interoperability: a norma policy of boxes and linear rules is a reflect `Constraints` system (linear, so nothing is lost), and reflect's text certificate could serve as the proof format that a norma certificate binds to, if both sides agree on the digest binding. The crates must not depend on each other (CLAUDE.md), so any bridge is an adapter outside both, or a written format specification. Norma's README says its `.telos` ideal layer is written but not verified (see its THREAT_MODEL.md), so reflect should not treat norma's proofs as checked.
- `tpt-solutions/tpt-writ` (MIT OR Apache-2.0, a proof-gated guard for MCP tool calls). Nothing is shared with reflect's invariant layer. Its `WritGrant` is a capability derived from a proof manifest, a different object from reflect's `Receipt`. Its interop point is MCP, which lies outside this workspace.
- `tpt-solutions/tpt-formal` (20 crates). Candidate adapters for reflect's "optional adapters to tpt-formal and tpt-solver" item: `tpt-for-sat` (a CDCL SAT solver) and `tpt-for-smt-lite` (an SMT-LIB2 bridge). Relevant elsewhere: `tpt-for-det-proptest` (seeded property testing, for the shared test-convention item), `tpt-for-deterministic-sim` (overlaps the simulators in dtn and causal-consensus), `tpt-for-verified-ode` (contract-guarded fixed-step integrators, for astrodynamics). `tpt-for-witness` and `tpt-for-refinement` express the same idea as radhard's typestate wrappers (a value carrying its checked property in its type), and are not used here.
