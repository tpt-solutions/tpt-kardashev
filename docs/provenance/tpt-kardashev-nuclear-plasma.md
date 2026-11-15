# Provenance: tpt-kardashev-nuclear-plasma

Origin of code and designs in this crate. Reference code is read, not linked (spec §3.8).

## Ported code

None yet.

## Reviewed for reference (not copied)

- `tpt-solutions/tpt-ignis` (todo §2.3), read from a shallow clone, not copied. Its `tpt-physics-math` has a Miller-parametrised shaped equilibrium (boundary R(θ), Z(θ) with elongation and triangularity, toroidal and poloidal fields, safety-factor and poloidal-beta profiles, flux-surface cross-sections). It is a reduced description, not a grid solve of the Grad-Shafranov equation. This crate's solver is a grid solve, so the overlap is in the flux-surface and q-profile definitions only. Its neutronics: none found in tpt-physics-math (no Monte Carlo, no cross-section code). `tpt-aether` (apps/tpt-aether) is a digital twin and reactor topology graph: it maps a 14.1 MeV neutron source onto component positions by inverse-square flux, with no Monte Carlo, equilibrium or MHD. Overlap with this crate is therefore none beyond the flux-surface definitions above. The repository is Apache-2.0.

## Third-party licences

None.
