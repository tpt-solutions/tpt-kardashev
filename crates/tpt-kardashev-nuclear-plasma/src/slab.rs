// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fixed-source Monte Carlo transport in a one-group, one-dimensional slab, and the infinite medium.
//!
//! Neutrons start at the slab's left face moving normally into it. Free flights are sampled from the
//! total cross section; at each collision the neutron is absorbed with probability `σa/σt`, and
//! otherwise scatters isotropically (the direction cosine is uniform on [-1, 1]).
//!
//! Three estimators are provided. The analog one follows each neutron to its end. The implicit-capture
//! one never absorbs a neutron outright: at each collision it removes the absorbed fraction of its
//! weight, and it plays Russian roulette when the weight falls below a threshold, which keeps the
//! estimates unbiased while reducing the variance of deep penetration. The splitting one does the same
//! and, when a neutron first enters a deeper zone of the slab, splits it into copies of equal share of
//! the weight at that face; the copies are followed in turn, and all of them count toward the source
//! neutron that made them. Every estimator is checked against analytic results or against each other
//! in the tests.
//!
//! Reaction rates are tallied per reaction at each collision: the absorption weight and the scattering
//! weight, which sum to the collision weight. The estimators are collision estimators, so the
//! rates are the expected number of each reaction per source neutron.

use crate::error::NeutronicsError;
use crate::rng::SplitMix64;
use std::vec;
use std::vec::Vec;

/// One-group cross sections, in 1/cm (or any length unit, used consistently with the thickness).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossSections {
    /// The total cross section `σt`.
    pub total: f64,
    /// The absorption cross section `σa`, with `0 <= σa <= σt`.
    pub absorption: f64,
}

impl CrossSections {
    /// Cross sections with `total > 0` and `0 <= absorption <= total`, all finite. Fails with
    /// [`NeutronicsError::BadCrossSection`] otherwise.
    pub fn new(total: f64, absorption: f64) -> Result<Self, NeutronicsError> {
        if !(total.is_finite() && total > 0.0) {
            return Err(NeutronicsError::BadCrossSection);
        }
        if !(absorption.is_finite() && (0.0..=total).contains(&absorption)) {
            return Err(NeutronicsError::BadCrossSection);
        }
        Ok(Self { total, absorption })
    }

    /// The scattering cross section `σs = σt − σa`.
    pub fn scattering(&self) -> f64 {
        self.total - self.absorption
    }
}

/// A slab of the given thickness and cross sections.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slab {
    /// The thickness, in the same length unit as the cross sections.
    pub thickness: f64,
    /// The cross sections of the material.
    pub xs: CrossSections,
}

/// The estimate of a fraction from its histories, with its standard error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    /// The mean score per source neutron.
    pub mean: f64,
    /// The standard error of the mean.
    pub std_error: f64,
}

/// The tallies of a slab run, each per source neutron.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlabTally {
    /// Weight leaving through the far face.
    pub transmitted: Estimate,
    /// Weight leaving through the near face.
    pub reflected: Estimate,
    /// Weight absorbed in the slab: the absorption reaction rate, per source neutron.
    pub absorbed: Estimate,
    /// Weight scattered in the slab: the scattering reaction rate, per source neutron. With
    /// `absorbed`, it sums to `collisions` history by history.
    pub scattered: Estimate,
    /// The track length in the slab (the flux integrated over the slab), per source neutron.
    pub track_length: Estimate,
    /// Collisions per source neutron (the reaction count for all reactions).
    pub collisions: Estimate,
}

/// The estimator to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estimator {
    /// Each neutron is followed to its end; a collision either absorbs it or scatters it.
    Analog,
    /// A neutron is never absorbed outright; its weight is reduced at each collision, and
    /// Russian roulette removes low-weight neutrons without bias.
    ImplicitCapture,
    /// As [`Estimator::ImplicitCapture`], with deep-penetration splitting. The slab is divided into
    /// `zones` equal slices. When a neutron first enters a slice deeper than any it has reached, it
    /// is replaced by `copies` neutrons at that face, each with `1/copies` of its weight and the same
    /// direction; the copies then travel on independently. Splitting raises the number of neutrons
    /// that reach the far zones. Whether that lowers the variance of a given tally depends on the
    /// problem, and it costs more histories to follow. With one zone or one copy there is no
    /// splitting, and the run is implicit capture draw for draw.
    Splitting {
        /// The number of equal slices of the slab. Must be at least 1.
        zones: usize,
        /// The number of neutrons made at each zone face. Must be at least 1.
        copies: usize,
    },
}

/// Weight below which Russian roulette is played, and the survival probability used in it.
const ROULETTE_WEIGHT: f64 = 0.25;
const ROULETTE_SURVIVAL: f64 = 0.5;

#[derive(Default, Clone)]
struct Accumulator {
    sum: f64,
    sum_sq: f64,
}

impl Accumulator {
    fn add(&mut self, value: f64) {
        self.sum += value;
        self.sum_sq += value * value;
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

/// A particle in flight: its position in the slab, its direction cosine and its weight.
#[derive(Clone, Copy, Debug)]
struct Particle {
    x: f64,
    mu: f64,
    weight: f64,
    /// The deepest zone the particle's line of descent has entered.
    zone: usize,
}

/// The scores of one source neutron, summed over every particle its history gives rise to.
#[derive(Default, Clone, Copy)]
struct Scores {
    transmitted: f64,
    reflected: f64,
    absorbed: f64,
    scattered: f64,
    track: f64,
    collisions: f64,
}

/// The accumulators behind a [`SlabTally`].
#[derive(Default)]
struct Totals {
    transmitted: Accumulator,
    reflected: Accumulator,
    absorbed: Accumulator,
    scattered: Accumulator,
    track: Accumulator,
    collisions: Accumulator,
}

impl Totals {
    fn add(&mut self, s: &Scores) {
        self.transmitted.add(s.transmitted);
        self.reflected.add(s.reflected);
        self.absorbed.add(s.absorbed);
        self.scattered.add(s.scattered);
        self.track.add(s.track);
        self.collisions.add(s.collisions);
    }

    fn estimates(&self, n: usize) -> SlabTally {
        SlabTally {
            transmitted: self.transmitted.estimate(n),
            reflected: self.reflected.estimate(n),
            absorbed: self.absorbed.estimate(n),
            scattered: self.scattered.estimate(n),
            track_length: self.track.estimate(n),
            collisions: self.collisions.estimate(n),
        }
    }
}

fn sample_flight(rng: &mut SplitMix64, total: f64) -> f64 {
    // -ln(1 - ξ) / σt with ξ in [0, 1): the argument is in (0, 1], so the logarithm is finite.
    -(1.0 - rng.next_f64()).ln() / total
}

/// Runs `histories` source neutrons through the slab with `estimator`, seeded with `seed`. Fails with
/// [`NeutronicsError::NoHistories`] for zero histories, [`NeutronicsError::BadThickness`] for a
/// thickness that is not positive and finite, and [`NeutronicsError::BadSplitting`] for a splitting
/// estimator with zero zones or zero copies.
pub fn run_slab(
    slab: &Slab,
    estimator: Estimator,
    histories: usize,
    seed: u64,
) -> Result<SlabTally, NeutronicsError> {
    if histories == 0 {
        return Err(NeutronicsError::NoHistories);
    }
    if !(slab.thickness.is_finite() && slab.thickness > 0.0) {
        return Err(NeutronicsError::BadThickness);
    }
    let (zones, copies) = match estimator {
        Estimator::Splitting { zones, copies } => {
            if zones == 0 || copies == 0 {
                return Err(NeutronicsError::BadSplitting);
            }
            (zones, copies)
        }
        Estimator::Analog | Estimator::ImplicitCapture => (1, 1),
    };
    let split = zones > 1 && copies > 1;
    let zone_width = slab.thickness / zones as f64;
    let mut rng = SplitMix64::new(seed);
    let total = slab.xs.total;
    let p_absorb = slab.xs.absorption / total;
    let mut totals = Totals::default();
    // Copies made by splitting wait here until the particle that made them has ended.
    let mut stack: Vec<Particle> = Vec::new();

    for _ in 0..histories {
        let mut scores = Scores::default();
        stack.clear();
        stack.push(Particle {
            x: 0.0,
            mu: 1.0,
            weight: 1.0,
            zone: 0,
        });
        while let Some(mut p) = stack.pop() {
            loop {
                let flight = sample_flight(&mut rng, total);
                if split && p.mu > 0.0 && p.zone + 1 < zones {
                    let edge = (p.zone + 1) as f64 * zone_width;
                    let to_edge = (edge - p.x) / p.mu;
                    if to_edge < flight {
                        // The face of the next zone comes before a collision. The flight is
                        // exponential and so has no memory: resampling it at the face is exact.
                        scores.track += p.weight * to_edge;
                        p.x = edge;
                        p.zone += 1;
                        let share = p.weight / copies as f64;
                        for _ in 1..copies {
                            stack.push(Particle {
                                x: edge,
                                mu: p.mu,
                                weight: share,
                                zone: p.zone,
                            });
                        }
                        p.weight = share;
                        continue;
                    }
                }
                let to_boundary = if p.mu > 0.0 {
                    (slab.thickness - p.x) / p.mu
                } else {
                    -p.x / p.mu
                };
                if flight >= to_boundary {
                    // Leaves the slab through the face it is heading for.
                    scores.track += p.weight * to_boundary;
                    if p.mu > 0.0 {
                        scores.transmitted += p.weight;
                    } else {
                        scores.reflected += p.weight;
                    }
                    break;
                }
                scores.track += p.weight * flight;
                p.x += p.mu * flight;
                scores.collisions += p.weight;
                match estimator {
                    Estimator::Analog => {
                        if rng.next_f64() < p_absorb {
                            scores.absorbed += p.weight;
                            break;
                        }
                        scores.scattered += p.weight;
                        p.mu = 2.0 * rng.next_f64() - 1.0;
                    }
                    Estimator::ImplicitCapture | Estimator::Splitting { .. } => {
                        scores.absorbed += p.weight * p_absorb;
                        scores.scattered += p.weight * (1.0 - p_absorb);
                        p.weight *= 1.0 - p_absorb;
                        p.mu = 2.0 * rng.next_f64() - 1.0;
                        if p.weight < ROULETTE_WEIGHT {
                            if rng.next_f64() < ROULETTE_SURVIVAL {
                                p.weight /= ROULETTE_SURVIVAL;
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
        totals.add(&scores);
    }

    Ok(totals.estimates(histories))
}

/// The track length per source neutron in an infinite medium of the given cross sections: every
/// neutron is eventually absorbed, and the expected track length is `1/σa`. Fails as [`run_slab`].
pub fn run_infinite(
    xs: CrossSections,
    histories: usize,
    seed: u64,
) -> Result<Estimate, NeutronicsError> {
    if histories == 0 {
        return Err(NeutronicsError::NoHistories);
    }
    if xs.absorption <= 0.0 {
        // With no absorption there is no end to a history: the track length is unbounded.
        return Err(NeutronicsError::BadCrossSection);
    }
    let mut rng = SplitMix64::new(seed);
    let p_absorb = xs.absorption / xs.total;
    let mut track = Accumulator::default();
    for _ in 0..histories {
        let mut length = 0.0_f64;
        loop {
            length += sample_flight(&mut rng, xs.total);
            if rng.next_f64() < p_absorb {
                break;
            }
            // Scattering in an infinite medium changes direction, not the distance already counted.
        }
        track.add(length);
    }
    Ok(track.estimate(histories))
}

/// Track length per source neutron in each of `bins` equal slices of the slab, from the analog
/// estimator: the spatial flux tally. Each bin's estimate has its standard error. The bins sum to the
/// slab's total track length. Fails as [`run_slab`], and with [`NeutronicsError::NoHistories`] for
/// zero bins.
pub fn run_slab_binned(
    slab: &Slab,
    histories: usize,
    seed: u64,
    bins: usize,
) -> Result<Vec<Estimate>, NeutronicsError> {
    if histories == 0 || bins == 0 {
        return Err(NeutronicsError::NoHistories);
    }
    if !(slab.thickness.is_finite() && slab.thickness > 0.0) {
        return Err(NeutronicsError::BadThickness);
    }
    let mut rng = SplitMix64::new(seed);
    let total = slab.xs.total;
    let p_absorb = slab.xs.absorption / total;
    let width = slab.thickness / bins as f64;
    let mut sums = vec![Accumulator::default(); bins];
    let mut history_scores = vec![0.0_f64; bins];
    for _ in 0..histories {
        let mut x = 0.0_f64;
        let mut mu = 1.0_f64;
        for score in history_scores.iter_mut() {
            *score = 0.0;
        }
        loop {
            let flight = sample_flight(&mut rng, total);
            let to_boundary = if mu > 0.0 {
                (slab.thickness - x) / mu
            } else {
                -x / mu
            };
            let length = flight.min(to_boundary);
            // The segment covers x-extent |mu| * length, from x to x + mu * length.
            let (x0, x1) = (x, x + mu * length);
            let (lo, hi) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
            for (bin, score) in history_scores.iter_mut().enumerate() {
                let (b_lo, b_hi) = (bin as f64 * width, (bin + 1) as f64 * width);
                let overlap = (hi.min(b_hi) - lo.max(b_lo)).max(0.0);
                *score += overlap / mu.abs();
            }
            if flight >= to_boundary {
                break;
            }
            x = x1;
            if rng.next_f64() < p_absorb {
                break;
            }
            mu = 2.0 * rng.next_f64() - 1.0;
        }
        for (acc, score) in sums.iter_mut().zip(history_scores.iter()) {
            acc.add(*score);
        }
    }
    Ok(sums.iter().map(|acc| acc.estimate(histories)).collect())
}
