// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bit-error-rate simulation for the K=7, rate-1/2 convolutional code over BPSK with AWGN, and the
//! union bound it is compared against.
//!
//! Everything is deterministic: a fixed seed drives the bits and the noise.
//!
//! Channel model: bit 0 is sent as +1 and bit 1 as -1 (BPSK). Each coded bit gets independent
//! Gaussian noise of variance `1 / gamma`, with `gamma = Eb/N0` (linear). For rate 1/2 with unit
//! amplitude per coded bit, `sigma^2 = N0 / 2 = 1 / gamma`.
//!
//! Framing: each frame is `frame_bits` random information bits followed by 6 zero tail bits, so
//! the encoder starts and ends in the zero state. Errors are counted only in the interior of the
//! frame, `EDGE` bits from each end. The decoder picks its own end state, which lets errors near
//! the end of the data go uncorrected, and the union bound (for an infinitely long trellis) does
//! not describe those positions.
//!
//! The union bound uses the information-weight spectrum `B_d` of the code. `B_d` is computed here
//! from the generator polynomials, not taken from a table.

// Each test or example uses a subset of these helpers. `expect` is allowed because these are
// test helpers, not library code. The index loop is the clearer form for the trellis update.
#![allow(dead_code, clippy::expect_used, clippy::needless_range_loop)]

use tpt_kardashev_archive_codes::conv;

/// Generator polynomials 171 and 133 (octal), with the newest input in bit 6.
const G1: u8 = 0o171;
const G2: u8 = 0o133;

/// Zero tail bits that drive the encoder back to the zero state (K - 1).
pub const TAIL: usize = 6;
/// Bits excluded at each end of a frame when counting errors.
pub const EDGE: usize = 64;

/// Highest distance for which the weight spectrum is computed. At 3 dB the union sum decays by
/// only about 7% per two distances, so the tail has to be carried this far for the bound to converge.
pub const SPECTRUM_MAX_D: usize = 120;

/// SplitMix64: small, deterministic, good enough for simulation noise.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in the open interval (0, 1].
    fn unit_open(&mut self) -> f64 {
        ((self.next_u64() >> 11) + 1) as f64 / (1u64 << 53) as f64
    }

    /// Standard normal deviate (Box-Muller; one of the pair is used).
    pub fn gaussian(&mut self) -> f64 {
        let u1 = self.unit_open();
        let u2 = self.unit_open();
        (-2.0 * u1.ln()).sqrt() * (core::f64::consts::TAU * u2).cos()
    }

    pub fn bit(&mut self) -> u8 {
        (self.next_u64() >> 63) as u8
    }
}

/// Gaussian tail probability `Q(x) = P(N(0,1) > x)`, by Simpson's rule on `[x, x + 12]`.
/// The integrand beyond `x + 12` is below 1e-30.
pub fn q_function(x: f64) -> f64 {
    let steps = 4000usize;
    let a = x;
    let b = x + 12.0;
    let h = (b - a) / steps as f64;
    let phi = |t: f64| (-0.5 * t * t).exp() / (2.0 * core::f64::consts::PI).sqrt();
    let mut sum = phi(a) + phi(b);
    for i in 1..steps {
        let t = a + h * i as f64;
        sum += if i % 2 == 1 {
            4.0 * phi(t)
        } else {
            2.0 * phi(t)
        };
    }
    sum * h / 3.0
}

fn parity(v: u8) -> u8 {
    v.count_ones() as u8 & 1
}

/// Information-weight spectrum `B_d` for `d = 0..=SPECTRUM_MAX_D`: the total input weight of all
/// error events of output weight `d`. Error events leave and re-enter the zero state.
///
/// Computed by dynamic programming over the state diagram, in `f64` because the counts grow
/// exponentially with `d` (they pass 2^64 near d = 60). Below 2^53 every value is exact. The G2
/// inverter does not change error event weights, because it cancels in the difference of two
/// codewords.
pub fn weight_spectrum() -> [f64; SPECTRUM_MAX_D + 1] {
    const STATES: usize = 64;
    // Long enough that every event of weight <= SPECTRUM_MAX_D has completed.
    const MAX_STEPS: usize = 400;
    let mut spectrum = [0f64; SPECTRUM_MAX_D + 1];
    // acc[state][distance] = (number of partial paths, sum of their input weights).
    let mut acc = vec![vec![(0f64, 0f64); SPECTRUM_MAX_D + 1]; STATES];
    // First step: diverge from the zero state with input 1.
    let first = 1u8 << 6;
    let d0 = usize::from(parity(first & G1) + parity(first & G2));
    acc[(first >> 1) as usize][d0] = (1.0, 1.0);

    for _ in 0..MAX_STEPS {
        let mut next = vec![vec![(0f64, 0f64); SPECTRUM_MAX_D + 1]; STATES];
        for state in 1..STATES {
            for d in 0..=SPECTRUM_MAX_D {
                let (count, wsum) = acc[state][d];
                if count == 0.0 {
                    continue;
                }
                for u in 0u8..=1 {
                    let r = (u << 6) | state as u8;
                    let out = usize::from(parity(r & G1) + parity(r & G2));
                    let nd = d + out;
                    if nd > SPECTRUM_MAX_D {
                        continue;
                    }
                    let nw = wsum + f64::from(u) * count;
                    let ns = usize::from(r >> 1);
                    if ns == 0 {
                        // The path has merged with the zero state: a complete error event.
                        spectrum[nd] += nw;
                    } else {
                        let entry = &mut next[ns][nd];
                        entry.0 += count;
                        entry.1 += nw;
                    }
                }
            }
        }
        acc = next;
    }
    spectrum
}

/// Soft union bound on the information-bit error rate, using only distances up to `max_d`.
pub fn union_bound_soft_upto(gamma: f64, spectrum: &[f64], max_d: usize) -> f64 {
    spectrum
        .iter()
        .enumerate()
        .take(max_d + 1)
        .filter(|(_, b)| **b > 0.0)
        .map(|(d, &b)| b * q_function((d as f64 * gamma).sqrt()))
        .sum()
}

/// Soft union bound using the full spectrum this module computes.
pub fn union_bound_soft(gamma: f64, spectrum: &[f64]) -> f64 {
    union_bound_soft_upto(gamma, spectrum, spectrum.len() - 1)
}

fn binomial(n: usize, k: usize) -> f64 {
    let mut r = 1.0;
    for i in 0..k {
        r = r * (n - i) as f64 / (i + 1) as f64;
    }
    r
}

/// Probability that a hard-decision error event of length `d` wins, with bit-flip probability `p`.
fn pairwise_hard(d: usize, p: f64) -> f64 {
    let mut total = 0.0;
    for k in 0..=d {
        let term = binomial(d, k) * p.powi(k as i32) * (1.0 - p).powi((d - k) as i32);
        if 2 * k > d {
            total += term;
        } else if 2 * k == d {
            total += 0.5 * term;
        }
    }
    total
}

/// Hard-decision union bound on the information-bit error rate.
pub fn union_bound_hard(gamma: f64, spectrum: &[f64]) -> f64 {
    // Hard decisions on rate-1/2 BPSK: the crossover probability is Q(sqrt(2 R gamma)) = Q(sqrt(gamma)).
    let p = q_function(gamma.sqrt());
    spectrum
        .iter()
        .enumerate()
        .filter(|(_, b)| **b > 0.0)
        .map(|(d, &b)| b * pairwise_hard(d, p))
        .sum()
}

/// Outcome of one simulated operating point. Error counts cover interior bits only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub ebn0_db: f64,
    pub bits: u64,
    pub soft_errors: u64,
    pub hard_errors: u64,
}

impl Point {
    pub fn soft_ber(&self) -> f64 {
        self.soft_errors as f64 / self.bits as f64
    }

    pub fn hard_ber(&self) -> f64 {
        self.hard_errors as f64 / self.bits as f64
    }
}

/// Soft confidence byte for a received BPSK sample `y`: bit 0 (near +1) maps to a low byte.
pub fn soft_byte(y: f64) -> u8 {
    // 128 +- 32 * y, clamped: |y| = 4 saturates.
    (128.0 - 32.0 * y).round().clamp(0.0, 255.0) as u8
}

/// Simulates up to `frames` terminated frames of `frame_bits` information bits at `ebn0_db`,
/// stopping early once `max_errors` interior soft-decision errors have occurred.
pub fn simulate(
    ebn0_db: f64,
    frame_bits: usize,
    frames: usize,
    max_errors: u64,
    seed: u64,
) -> Point {
    let gamma = 10f64.powf(ebn0_db / 10.0);
    let sigma = (1.0 / gamma).sqrt();
    let mut rng = Rng::new(seed);
    let mut point = Point {
        ebn0_db,
        bits: 0,
        soft_errors: 0,
        hard_errors: 0,
    };
    let total_bits = frame_bits + TAIL;
    let interior = EDGE..frame_bits - EDGE;
    let mut scratch = vec![0u64; total_bits];
    let mut decisions = vec![0u64; total_bits];
    for _ in 0..frames {
        let mut info: Vec<u8> = (0..frame_bits).map(|_| rng.bit()).collect();
        info.extend(std::iter::repeat_n(0u8, TAIL));
        let mut coded = vec![0u8; 2 * total_bits];
        conv::encode(&info, &mut coded).expect("encode");

        let mut soft = vec![0u8; coded.len()];
        let mut hard = vec![0u8; coded.len()];
        for (i, &c) in coded.iter().enumerate() {
            let x = if c == 0 { 1.0 } else { -1.0 };
            let y = x + sigma * rng.gaussian();
            soft[i] = soft_byte(y);
            hard[i] = u8::from(soft[i] >= 128);
        }

        let mut out_soft = vec![0u8; total_bits];
        conv::viterbi_decode_soft(&soft, &mut out_soft, &mut scratch).expect("soft decode");
        let mut out_hard = vec![0u8; total_bits];
        conv::viterbi_decode(&hard, &mut out_hard, &mut decisions).expect("hard decode");

        point.bits += interior.len() as u64;
        point.soft_errors += out_soft[interior.clone()]
            .iter()
            .zip(&info[interior.clone()])
            .filter(|(a, b)| a != b)
            .count() as u64;
        point.hard_errors += out_hard[interior.clone()]
            .iter()
            .zip(&info[interior.clone()])
            .filter(|(a, b)| a != b)
            .count() as u64;
        if point.soft_errors >= max_errors {
            break;
        }
    }
    point
}
