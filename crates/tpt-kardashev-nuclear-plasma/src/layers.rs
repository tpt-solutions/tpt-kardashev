// SPDX-License-Identifier: MIT OR Apache-2.0
//! A one-dimensional layered geometry: regions of different material in a row, with the neutron source
//! at the left face. This is the one-dimensional case of constructive solid geometry; the three-dimensional
//! case, for point location and ray crossings, is in [`crate::csg`]. Transport through the three-dimensional
//! geometry is not written.
//!
//! A neutron that reaches a boundary between regions crosses into the next region with its direction and
//! weight unchanged. Free flights are sampled in the region the neutron is in; because the exponential
//! flight is memoryless, a flight cut short by a boundary is resampled in the next region, which is exact.

use std::vec::Vec;

use crate::error::NeutronicsError;
use crate::rng::SplitMix64;
use crate::slab::{CrossSections, Estimate, SlabTally};

/// One region: a thickness and the cross sections of its material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    /// The thickness, positive and finite.
    pub thickness: f64,
    /// The material's cross sections.
    pub xs: CrossSections,
}

/// A row of regions, from the source face to the far face.
#[derive(Clone, Debug, PartialEq)]
pub struct Layers {
    regions: Vec<Region>,
    edges: Vec<f64>,
}

impl Layers {
    /// A row of regions. Fails with [`NeutronicsError::NoGroups`] for an empty list and with
    /// [`NeutronicsError::BadThickness`] for a thickness that is not positive and finite.
    pub fn new(regions: Vec<Region>) -> Result<Self, NeutronicsError> {
        if regions.is_empty() {
            return Err(NeutronicsError::NoGroups);
        }
        let mut edges = Vec::with_capacity(regions.len() + 1);
        edges.push(0.0);
        for region in &regions {
            if !(region.thickness.is_finite() && region.thickness > 0.0) {
                return Err(NeutronicsError::BadThickness);
            }
            let last = *edges.last().unwrap_or(&0.0);
            edges.push(last + region.thickness);
        }
        Ok(Self { regions, edges })
    }

    /// The regions, from the source face.
    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    /// The total thickness.
    pub fn thickness(&self) -> f64 {
        *self.edges.last().unwrap_or(&0.0)
    }
}

#[derive(Default, Clone)]
struct Acc {
    sum: f64,
    sum_sq: f64,
}

impl Acc {
    fn add(&mut self, v: f64) {
        self.sum += v;
        self.sum_sq += v * v;
    }

    fn estimate(&self, n: usize) -> Estimate {
        let nf = n as f64;
        let mean = self.sum / nf;
        let variance = (self.sum_sq / nf - mean * mean).max(0.0);
        Estimate {
            mean,
            std_error: (variance / nf).sqrt(),
        }
    }
}

/// Runs `histories` source neutrons through the layers, with the analog estimator, seeded with `seed`.
/// The tallies are those of [`SlabTally`], for the whole row. Fails with [`NeutronicsError::NoHistories`]
/// for zero histories.
pub fn run_layers(
    layers: &Layers,
    histories: usize,
    seed: u64,
) -> Result<SlabTally, NeutronicsError> {
    if histories == 0 {
        return Err(NeutronicsError::NoHistories);
    }
    let mut rng = SplitMix64::new(seed);
    let (mut t_acc, mut r_acc, mut a_acc, mut s_acc, mut track_acc, mut coll_acc) = (
        Acc::default(),
        Acc::default(),
        Acc::default(),
        Acc::default(),
        Acc::default(),
        Acc::default(),
    );
    let last = layers.regions.len() - 1;
    for _ in 0..histories {
        let mut region = 0usize;
        let mut x = 0.0_f64;
        let mut mu = 1.0_f64;
        let (mut t_score, mut r_score, mut a_score, mut track_score, mut coll_score) =
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
        loop {
            let xs = layers.regions[region].xs;
            let flight = -(1.0 - rng.next_f64()).ln() / xs.total;
            let (left, right) = (layers.edges[region], layers.edges[region + 1]);
            let to_edge = if mu > 0.0 {
                (right - x) / mu
            } else {
                (x - left) / -mu
            };
            if flight < to_edge {
                // A collision inside this region.
                track_score += flight;
                x += mu * flight;
                coll_score += 1.0;
                if rng.next_f64() < xs.absorption / xs.total {
                    a_score += 1.0;
                    break;
                }
                mu = 2.0 * rng.next_f64() - 1.0;
                continue;
            }
            // Reaches a boundary: the flight is resampled in whatever region is beyond it.
            track_score += to_edge;
            if mu > 0.0 {
                if region == last {
                    t_score += 1.0;
                    break;
                }
                region += 1;
                x = layers.edges[region];
            } else {
                if region == 0 {
                    r_score += 1.0;
                    break;
                }
                region -= 1;
                x = layers.edges[region + 1];
            }
        }
        // Each analog collision either absorbs the neutron or scatters it, so the scattering weight
        // is the collisions less the absorptions.
        s_acc.add(coll_score - a_score);
        t_acc.add(t_score);
        r_acc.add(r_score);
        a_acc.add(a_score);
        track_acc.add(track_score);
        coll_acc.add(coll_score);
    }

    Ok(SlabTally {
        transmitted: t_acc.estimate(histories),
        reflected: r_acc.estimate(histories),
        absorbed: a_acc.estimate(histories),
        scattered: s_acc.estimate(histories),
        track_length: track_acc.estimate(histories),
        collisions: coll_acc.estimate(histories),
    })
}
