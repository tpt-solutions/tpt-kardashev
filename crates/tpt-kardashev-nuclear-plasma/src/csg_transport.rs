// SPDX-License-Identifier: MIT OR Apache-2.0
//! Analog transport through the three-dimensional geometry of [`crate::csg`]: one energy group, one
//! material per zone, isotropic scattering in the laboratory frame, and one fate for each history. A
//! history ends absorbed, leaked into the void outside every zone, or unfinished when it reaches the
//! event cap, so the three counts always sum to the number of histories.
//!
//! A flight is sampled in the zone the neutron is in and cut at the next surface crossing. Because the
//! exponential flight is memoryless, a flight cut at a surface is resampled in the zone beyond it, which
//! is exact for this model. A neutron moves [`NUDGE`] past a surface so that [`Model::zone_of`] finds the
//! zone beyond it. Track length is counted only inside the zone the neutron is in, not over the nudge.
//!
//! Not modelled: weights, variance reduction, multigroup or anisotropic scattering, and a neutron that
//! grazes a surface within `NUDGE` of it, which is not resolved.

use std::vec::Vec;

use crate::csg::{Model, Point};
use crate::error::NeutronicsError;
use crate::rng::SplitMix64;
use crate::slab::CrossSections;

/// The distance a neutron moves past a surface, so that the zone beyond is found by point location.
pub const NUDGE: f64 = 1e-9;

/// The tally of a run.
#[derive(Clone, Debug, PartialEq)]
pub struct Tally {
    /// The number of histories.
    pub histories: usize,
    /// Histories that ended in absorption.
    pub absorbed: usize,
    /// Histories that left every zone.
    pub leaked: usize,
    /// Histories that reached the event cap before absorption or leakage.
    pub unfinished: usize,
    /// Collisions, over all histories.
    pub collisions: usize,
    /// Track length in each zone, summed over all histories, in the zone order of the model.
    pub track_length: Vec<f64>,
}

/// A direction uniform on the unit sphere.
fn isotropic(rng: &mut SplitMix64) -> Point {
    let mu = 2.0 * rng.next_f64() - 1.0;
    let phi = 2.0 * core::f64::consts::PI * rng.next_f64();
    let s = (1.0 - mu * mu).max(0.0).sqrt();
    [s * phi.cos(), s * phi.sin(), mu]
}

fn advance(p: Point, d: Point, distance: f64) -> Point {
    [
        p[0] + distance * d[0],
        p[1] + distance * d[1],
        p[2] + distance * d[2],
    ]
}

/// Transport `histories` neutrons from the point `source`, each history at most `max_events` collisions
/// and boundary crossings, seeded with `seed`.
///
/// `materials` has one entry per zone of `model`, in the zone order. Fails with
/// [`NeutronicsError::NoHistories`] for zero histories, and with [`NeutronicsError::BadGeometry`] for a
/// material list of the wrong length, a source that is not finite or not inside a zone, or zero
/// `max_events`.
pub fn run(
    model: &Model,
    materials: &[CrossSections],
    source: Point,
    histories: usize,
    max_events: usize,
    seed: u64,
) -> Result<Tally, NeutronicsError> {
    if histories == 0 {
        return Err(NeutronicsError::NoHistories);
    }
    if materials.len() != model.zones().len() || max_events == 0 {
        return Err(NeutronicsError::BadGeometry);
    }
    if !source.iter().all(|c| c.is_finite()) || model.zone_of(source).is_none() {
        return Err(NeutronicsError::BadGeometry);
    }
    let mut rng = SplitMix64::new(seed);
    let mut tally = Tally {
        histories,
        absorbed: 0,
        leaked: 0,
        unfinished: 0,
        collisions: 0,
        track_length: std::vec![0.0; materials.len()],
    };
    for _ in 0..histories {
        let mut p = source;
        let mut d = isotropic(&mut rng);
        let mut events = 0usize;
        loop {
            let Some(zone) = model.zone_of(p) else {
                tally.leaked += 1;
                break;
            };
            if events == max_events {
                tally.unfinished += 1;
                break;
            }
            events += 1;
            let material = materials[zone];
            let flight = -(1.0 - rng.next_f64()).ln() / material.total;
            let boundary = match model.distance_to_surface(p, d)? {
                Some((t, _)) => t,
                None => f64::INFINITY,
            };
            if flight < boundary {
                tally.track_length[zone] += flight;
                tally.collisions += 1;
                p = advance(p, d, flight);
                if rng.next_f64() < material.absorption / material.total {
                    tally.absorbed += 1;
                    break;
                }
                d = isotropic(&mut rng);
            } else {
                tally.track_length[zone] += boundary;
                p = advance(p, d, boundary + NUDGE);
            }
        }
    }
    Ok(tally)
}
