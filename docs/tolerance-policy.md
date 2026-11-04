# Floating-point tolerance policy

Shared template for the physics crates (relativity, astrodynamics, energy-ledger, nuclear-plasma, isru). Each crate may tighten these values but must not loosen them without a note in its README.

## Rules

1. **No bitwise float equality** in tests, except for exact known values such as `0.0`, `1.0` and integer-valued results.
2. **Relative tolerance by default:** `|a - b| <= rel * max(|a|, |b|)`.
3. **Absolute tolerance only near zero:** `|a - b| <= abs` when the expected value is below `abs`.
4. **Named constants per crate**, documented next to the test that uses them. No magic numbers in assertions.
5. **Every tolerance states its basis**: a known-answer reference, a published constant's precision, or an analytic error bound.
6. **Conservation checks** (energy, mass, probability, invariants) use the accumulated bound, not the per-step bound.

## Default values

| Quantity | Relative | Absolute | Basis |
|---|---|---|---|
| Closed-form scalar identities | 1e-12 | 1e-15 | f64 round-off for a few operations |
| Iterative solvers vs known answers | 1e-9 | 1e-12 | solver convergence target |
| Integrator drift over a long arc | per crate | per crate | stated with the integrator's order |
| Published physical constants vs computed values | 1e-6 | n/a | precision of the cited source |

## Lightlike classification

Interval classification in `tpt-kardashev-relativity` treats `s^2` as lightlike when `|s^2| <= 1e-12 * ((c dt)^2 + |dx|^2)`. Identical events are reported as lightlike. Each classification is documented in `event.rs`.

## Helper pattern

Each crate defines its own helper, for example:

```rust
#[cfg(test)]
fn assert_close(actual: f64, expected: f64, rel: f64, abs: f64) {
    let diff = (actual - expected).abs();
    let bound = abs.max(rel * expected.abs());
    assert!(diff <= bound, "actual {actual}, expected {expected}, diff {diff} > bound {bound}");
}
```

The helper is copied per crate, not shared, so that no crate depends on another (spec §4.4 and todo, cross-cutting infrastructure).
