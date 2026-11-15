# Provenance: tpt-kardashev-nuclear-plasma

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reviewed for reference (not copied)

- `tpt-solutions/tpt-ignis` (todo §2.3), read from a shallow clone, not copied. Its `tpt-physics-math` has a Miller-parametrised shaped equilibrium (boundary R(θ), Z(θ) with elongation and triangularity, toroidal and poloidal fields, safety-factor and poloidal-beta profiles, flux-surface cross-sections). It is a reduced description, not a grid solve of the Grad-Shafranov equation. This crate's solver is a grid solve, so the overlap is in the flux-surface and q-profile definitions only. Its neutronics: none found in tpt-physics-math (no Monte Carlo, no cross-section code). `tpt-aether` (apps/tpt-aether) is a digital twin and reactor topology graph: it maps a 14.1 MeV neutron source onto component positions by inverse-square flux, with no Monte Carlo, equilibrium or MHD. Overlap with this crate is therefore none beyond the flux-surface definitions above. The repository is Apache-2.0.

## Third-party licences

None.

## Written from standard relations

- `equilibrium.rs`: the Grad-Shafranov operator `Δ* = R ∂_R(R⁻¹ ∂_R) + ∂_Z²` and its central-difference discretisation, and the Solov'ev function `ψ = A R⁴ + B R² Z²` with `Δ*ψ = (8A + 2B) R²`, written from the standard forms; the successive over-relaxation is the textbook method. No reference code consulted. The sibling `tpt-ignis` equilibrium (Miller parametrisation, reviewed above) is a different, reduced model and was not used.
- `surface.rs`: ray-bisection contour finding, bilinear interpolation, and the safety factor q = (F/2π)∮dl/(R|∇ψ|), written from the standard definitions (B_φ = F/R, B_p = |∇ψ|/R, poloidal flux per radian). No reference code consulted. The closed form used in the tests, q = F/(2c√(R0² − r²)) for ψ = c((R − R0)² + Z²), is derived from the same definitions.
- `xs.rs`: the text format (version 1) is this crate's own design; it is described in the module documentation and not copied from ENDF-6, ACE or OpenMC. No reference code consulted.
- `layers.rs`: the one-dimensional layered geometry and its analog transport across boundaries, written from the same definitions as the slab (memoryless exponential flights, resampled in the next region). No reference code consulted.
