// SPDX-License-Identifier: MIT OR Apache-2.0
//! LT (Luby transform) fountain codes: rateless erasure coding over blocks.
//!
//! The source is `k` blocks of `block_len` bytes. Encoded symbol `id` is the XOR of a set of
//! source blocks. The set is a pseudo-random function of `id` alone, so the sender and receiver
//! derive identical neighbour sets without transmitting them. The degree of each symbol is drawn
//! from a [`DegreeDistribution`], and the receiver recovers sources by peeling degree-one symbols.
//!
//! Any number of symbols can be generated. Decoding succeeds once enough symbols arrive, in any
//! order and with any losses; the required count is a little over `k` in typical cases.
//!
//! The decoder keeps every symbol that has not yet been reduced to degree one, so its memory use
//! is not bounded by a constant. This module needs the `alloc` feature.
//!
//! Not modelled: RaptorQ precoding (excluded, see spec §11), any standard's degree distribution
//! bit-for-bit, and any particular wire format for `id` or the payload.

use crate::error::CodeError;
use alloc::vec::Vec;

/// Mixed into the symbol id before seeding, so that id 0 does not give a degenerate stream.
const SEED_TAG: u64 = 0x5EED_1A7C_0DE5_0001;
/// Probability mass scale for the integer cumulative table (used by `robust_soliton`).
#[cfg(feature = "std")]
const WEIGHT_SCALE: f64 = 4_294_967_296.0; // 2^32

/// Small deterministic generator (SplitMix64). Not cryptographic.
#[derive(Clone, Copy, Debug)]
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Degree distribution for symbol degrees `1..=len`, stored as an integer cumulative table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DegreeDistribution {
    /// `cumulative[i]` is the total weight of degrees `1..=i+1`.
    cumulative: Vec<u64>,
}

impl DegreeDistribution {
    /// Builds a distribution from non-negative weights: `weights[i]` is the weight of degree
    /// `i + 1`. At least one weight must be non-zero, and the total must fit in `u64`.
    pub fn from_weights(weights: &[u64]) -> Result<Self, CodeError> {
        if weights.is_empty() {
            return Err(CodeError::InvalidParameter { name: "weights" });
        }
        let mut cumulative = Vec::with_capacity(weights.len());
        let mut total: u64 = 0;
        for &w in weights {
            total = total
                .checked_add(w)
                .ok_or(CodeError::InvalidParameter { name: "weights" })?;
            cumulative.push(total);
        }
        if total == 0 {
            return Err(CodeError::InvalidParameter { name: "weights" });
        }
        Ok(Self { cumulative })
    }

    /// Robust soliton distribution for `k` source blocks (Luby 2002), with parameters `c > 0` and
    /// `0 < delta < 1`. Uses `ln` and `sqrt`, so it needs the `std` feature.
    #[cfg(feature = "std")]
    pub fn robust_soliton(k: usize, c: f64, delta: f64) -> Result<Self, CodeError> {
        if k == 0 {
            return Err(CodeError::InvalidParameter { name: "k" });
        }
        if !(c > 0.0 && c.is_finite()) {
            return Err(CodeError::InvalidParameter { name: "c" });
        }
        if !(delta > 0.0 && delta < 1.0) {
            return Err(CodeError::InvalidParameter { name: "delta" });
        }
        let kf = k as f64;
        let r = c * (kf / delta).ln() * kf.sqrt();
        // Pivot degree K/R, rounded down and kept within 1..=k.
        let pivot = ((kf / r) as usize).clamp(1, k);

        let mut mu = Vec::with_capacity(k);
        for d in 1..=k {
            let df = d as f64;
            let rho = if d == 1 {
                1.0 / kf
            } else {
                1.0 / (df * (df - 1.0))
            };
            let tau = if d < pivot {
                r / (df * kf)
            } else if d == pivot {
                let ratio = r / delta;
                if ratio > 1.0 {
                    r * ratio.ln() / kf
                } else {
                    0.0
                }
            } else {
                0.0
            };
            mu.push(rho + tau);
        }
        let sum: f64 = mu.iter().sum();
        let weights: Vec<u64> = mu
            .iter()
            .map(|&m| {
                if m > 0.0 {
                    // At least 1, so every positive-mass degree stays reachable.
                    ((m / sum) * WEIGHT_SCALE) as u64 + 1
                } else {
                    0
                }
            })
            .collect();
        Self::from_weights(&weights)
    }

    /// Largest degree with non-zero weight.
    pub fn max_degree(&self) -> usize {
        let total = self.total();
        // Highest index whose cumulative weight is still below the total.
        self.cumulative.partition_point(|&c| c < total) + 1
    }

    fn total(&self) -> u64 {
        self.cumulative.last().copied().unwrap_or(0)
    }

    fn sample(&self, rng: &mut SplitMix64) -> usize {
        let u = rng.next_u64() % self.total();
        self.cumulative.partition_point(|&c| c <= u) + 1
    }
}

/// Source-block indices of encoded symbol `id`: distinct, each below `k`.
fn neighbours(dist: &DegreeDistribution, k: usize, id: u32) -> Vec<usize> {
    let mut rng = SplitMix64(u64::from(id) ^ SEED_TAG);
    let degree = dist.sample(&mut rng).min(k);
    let mut out = Vec::with_capacity(degree);
    while out.len() < degree {
        let v = (rng.next_u64() % k as u64) as usize;
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

fn check_block_len(len: usize) -> Result<(), CodeError> {
    if len == 0 {
        return Err(CodeError::InvalidParameter { name: "block_len" });
    }
    Ok(())
}

/// Produces encoded symbols from a fixed source.
#[derive(Clone, Copy, Debug)]
pub struct LtEncoder<'a> {
    dist: &'a DegreeDistribution,
    block_len: usize,
    sources: &'a [u8],
    k: usize,
}

impl<'a> LtEncoder<'a> {
    /// Encoder over `sources`, which holds `k` blocks of `block_len` bytes back to back.
    pub fn new(
        dist: &'a DegreeDistribution,
        block_len: usize,
        sources: &'a [u8],
    ) -> Result<Self, CodeError> {
        check_block_len(block_len)?;
        if sources.is_empty() || !sources.len().is_multiple_of(block_len) {
            return Err(CodeError::InvalidParameter { name: "sources" });
        }
        Ok(Self {
            dist,
            block_len,
            sources,
            k: sources.len() / block_len,
        })
    }

    /// Number of source blocks.
    pub fn k(&self) -> usize {
        self.k
    }

    /// Writes symbol `id` (XOR of its neighbour blocks) into `out`, which must be `block_len`
    /// bytes long.
    pub fn symbol(&self, id: u32, out: &mut [u8]) -> Result<(), CodeError> {
        if out.len() != self.block_len {
            return Err(CodeError::LengthMismatch {
                expected: self.block_len,
                actual: out.len(),
            });
        }
        out.fill(0);
        for s in neighbours(self.dist, self.k, id) {
            let block = &self.sources[s * self.block_len..(s + 1) * self.block_len];
            for (o, &b) in out.iter_mut().zip(block) {
                *o ^= b;
            }
        }
        Ok(())
    }
}

/// A received symbol that still depends on at least one unrecovered source block.
#[derive(Clone, Debug)]
struct Pending {
    neighbours: Vec<usize>,
    data: Vec<u8>,
}

/// Decoding session: feed symbols in any order, then read the recovered source.
#[derive(Clone, Debug)]
pub struct LtDecoder<'a> {
    dist: &'a DegreeDistribution,
    k: usize,
    block_len: usize,
    source: Vec<u8>,
    known: Vec<bool>,
    known_count: usize,
    pending: Vec<Pending>,
    /// For each source block, the pending symbols that still reference it.
    by_source: Vec<Vec<usize>>,
    /// Pending symbols that have reached degree one.
    ready: Vec<usize>,
}

impl<'a> LtDecoder<'a> {
    /// A session for `k` source blocks of `block_len` bytes.
    pub fn new(
        dist: &'a DegreeDistribution,
        k: usize,
        block_len: usize,
    ) -> Result<Self, CodeError> {
        if k == 0 {
            return Err(CodeError::InvalidParameter { name: "k" });
        }
        check_block_len(block_len)?;
        Ok(Self {
            dist,
            k,
            block_len,
            source: alloc::vec![0u8; k * block_len],
            known: alloc::vec![false; k],
            known_count: 0,
            pending: Vec::new(),
            by_source: alloc::vec![Vec::new(); k],
            ready: Vec::new(),
        })
    }

    /// Accepts encoded symbol `id` with payload `data` (`block_len` bytes). A symbol received
    /// more than once, or one that adds nothing, is accepted and ignored.
    pub fn add_symbol(&mut self, id: u32, data: &[u8]) -> Result<(), CodeError> {
        if data.len() != self.block_len {
            return Err(CodeError::LengthMismatch {
                expected: self.block_len,
                actual: data.len(),
            });
        }
        let mut buf = data.to_vec();
        let mut remaining = Vec::new();
        for s in neighbours(self.dist, self.k, id) {
            if self.known[s] {
                // Remove an already-recovered block from this symbol.
                let block = &self.source[s * self.block_len..(s + 1) * self.block_len];
                for (b, &x) in buf.iter_mut().zip(block) {
                    *b ^= x;
                }
            } else {
                remaining.push(s);
            }
        }
        if remaining.is_empty() {
            return Ok(());
        }
        let idx = self.pending.len();
        if remaining.len() == 1 {
            self.ready.push(idx);
        }
        for &s in &remaining {
            self.by_source[s].push(idx);
        }
        self.pending.push(Pending {
            neighbours: remaining,
            data: buf,
        });
        self.peel();
        Ok(())
    }

    /// Resolves degree-one symbols, and each one's effect on the others, until none remain.
    fn peel(&mut self) {
        while let Some(idx) = self.ready.pop() {
            if self.pending[idx].neighbours.len() != 1 {
                continue;
            }
            let s = self.pending[idx].neighbours[0];
            self.pending[idx].neighbours.clear();
            if self.known[s] {
                continue;
            }
            let value = self.pending[idx].data.clone();
            self.source[s * self.block_len..(s + 1) * self.block_len].copy_from_slice(&value);
            self.known[s] = true;
            self.known_count += 1;

            for q in core::mem::take(&mut self.by_source[s]) {
                let p = &mut self.pending[q];
                if let Some(pos) = p.neighbours.iter().position(|&x| x == s) {
                    p.neighbours.swap_remove(pos);
                    for (b, &x) in p.data.iter_mut().zip(&value) {
                        *b ^= x;
                    }
                    if p.neighbours.len() == 1 {
                        self.ready.push(q);
                    }
                }
            }
        }
    }

    /// Number of source blocks recovered so far.
    pub fn known_count(&self) -> usize {
        self.known_count
    }

    /// True once every source block is recovered.
    pub fn is_complete(&self) -> bool {
        self.known_count == self.k
    }

    /// Recovered block `i`, if it is known.
    pub fn block(&self, i: usize) -> Option<&[u8]> {
        if i < self.k && self.known[i] {
            Some(&self.source[i * self.block_len..(i + 1) * self.block_len])
        } else {
            None
        }
    }

    /// The full source, once complete.
    pub fn into_source(self) -> Option<Vec<u8>> {
        if self.is_complete() {
            Some(self.source)
        } else {
            None
        }
    }
}

#[cfg(all(test, feature = "std"))]
#[allow(clippy::expect_used)] // test helper below is not a `#[test]` function
mod tests {
    use super::*;

    fn soliton(k: usize) -> DegreeDistribution {
        DegreeDistribution::robust_soliton(k, 0.1, 0.5).expect("valid")
    }

    #[test]
    fn degree_one_has_real_mass_and_is_sampled() {
        let d = soliton(100);
        assert!(d.cumulative[0] > 0);
        let mut rng = SplitMix64(42);
        let ones = (0..10_000).filter(|_| d.sample(&mut rng) == 1).count();
        // Analytic mass is roughly 0.05 to 0.1; require a generous lower bound.
        assert!(ones > 200, "only {ones} of 10000 samples had degree 1");
    }

    #[test]
    fn neighbour_sets_are_distinct_and_in_range() {
        let k = 37;
        let d = soliton(k);
        for id in 0..2_000 {
            let n = neighbours(&d, k, id);
            assert!(!n.is_empty() && n.len() <= k);
            assert!(n.iter().all(|&v| v < k));
            let mut sorted = n.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), n.len(), "id {id} repeats a block");
        }
    }

    #[test]
    fn neighbours_depend_only_on_id() {
        let d = soliton(20);
        assert_eq!(neighbours(&d, 20, 9), neighbours(&d, 20, 9));
    }
}
