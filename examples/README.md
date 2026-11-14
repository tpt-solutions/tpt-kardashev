# Examples

Cross-crate examples live here. Each one enables the feature flags it needs. The first is the ContactPlan flow from `tpt-kardashev-astrodynamics` to `tpt-kardashev-dtn` (Phase 2 exit).

Crate-level examples live in each crate's own `examples/` directory.

## Running the examples

- `contact-plan-flow`: `cd examples/contact-plan-flow && cargo run --release`. It is its own workspace, outside the library workspace, so the library checks do not build it. Run `cargo clippy --all-targets -- -D warnings` there for the same lint level.
