// SPDX-License-Identifier: MIT OR Apache-2.0
//! LDPC decoding by normalised min-sum belief propagation, from soft input.
//!
//! A code is given by its parity-check matrix in sparse form: one slice of variable indices per
//! check. The crate does not bundle any standard's matrix, so no compliance is claimed. Encoding is
//! not provided; the decoder works for any linear code given its checks.
//!
//! Soft input is a log-likelihood ratio (LLR) per bit: positive favours 0, negative favours 1.
//! Min-sum replaces the sum-product check update with a minimum, scaled by [`NORMALISATION`]. It
//! is cheaper and avoids transcendental functions, at a small loss in performance.
//!
//! The decoder needs caller-provided scratch of length [`LdpcCode::scratch_len`], so it does not
//! allocate.

use crate::error::CodeError;

/// Scale applied to every check-to-variable message (normalised min-sum).
pub const NORMALISATION: f32 = 0.75;

/// Inputs and messages are clamped to ±`LLR_LIMIT` so that sums stay finite.
pub const LLR_LIMIT: f32 = 64.0;

/// A binary linear code given by its sparse parity-check matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LdpcCode<'a> {
    n: usize,
    checks: &'a [&'a [usize]],
    edges: usize,
}

impl<'a> LdpcCode<'a> {
    /// Builds a code of `n` variables with the given checks.
    ///
    /// Each check lists distinct variable indices below `n`. A check must involve at least two
    /// variables; a single-variable check would force that bit and is not a useful constraint for
    /// min-sum.
    pub fn new(n: usize, checks: &'a [&'a [usize]]) -> Result<Self, CodeError> {
        if n == 0 {
            return Err(CodeError::InvalidParameter { name: "n" });
        }
        if checks.is_empty() {
            return Err(CodeError::InvalidParameter { name: "checks" });
        }
        let mut edges = 0usize;
        for check in checks {
            if check.len() < 2 {
                return Err(CodeError::InvalidParameter {
                    name: "check degree",
                });
            }
            for (k, &v) in check.iter().enumerate() {
                if v >= n {
                    return Err(CodeError::InvalidParameter {
                        name: "variable index",
                    });
                }
                if check[..k].contains(&v) {
                    return Err(CodeError::InvalidParameter {
                        name: "duplicate index",
                    });
                }
            }
            edges += check.len();
        }
        Ok(Self { n, checks, edges })
    }

    /// Number of variables (code length).
    pub fn len(&self) -> usize {
        self.n
    }

    /// Always false: a code has at least one variable.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Number of checks.
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }

    /// Scratch length required by [`LdpcCode::decode_llr`]: one message per edge plus one
    /// posterior per variable.
    pub fn scratch_len(&self) -> usize {
        self.edges + self.n
    }

    /// Returns true when `bits` satisfies every check. `bits` has one value 0 or 1 per variable.
    pub fn is_codeword(&self, bits: &[u8]) -> Result<bool, CodeError> {
        check_len(bits.len(), self.n)?;
        check_bits(bits)?;
        Ok(self.checks.iter().all(|check| {
            let parity = check.iter().fold(0u8, |acc, &v| acc ^ bits[v]);
            parity == 0
        }))
    }

    /// Decodes soft input into hard decisions in `bits`, using at most `max_iterations` rounds of
    /// message passing. Returns the number of rounds used, which is 0 when the hard decisions from
    /// the input already form a codeword.
    ///
    /// `llr` holds one LLR per variable (positive favours 0). `scratch` must have length
    /// [`LdpcCode::scratch_len`]; its contents on entry are ignored.
    ///
    /// Returns [`CodeError::Uncorrectable`] if no codeword is reached within `max_iterations`.
    /// `bits` then holds the last hard decisions and is not a corrected word.
    pub fn decode_llr(
        &self,
        llr: &[f32],
        bits: &mut [u8],
        scratch: &mut [f32],
        max_iterations: u32,
    ) -> Result<u32, CodeError> {
        if max_iterations == 0 {
            return Err(CodeError::InvalidParameter {
                name: "max_iterations",
            });
        }
        check_len(llr.len(), self.n)?;
        check_len(bits.len(), self.n)?;
        check_len(scratch.len(), self.scratch_len())?;
        if llr.iter().any(|x| !x.is_finite()) {
            return Err(CodeError::InvalidSymbol);
        }

        let (c2v, post) = scratch.split_at_mut(self.edges);
        c2v.fill(0.0);

        for iteration in 0..=max_iterations {
            self.posterior(llr, c2v, post);
            for (b, &p) in bits.iter_mut().zip(post.iter()) {
                *b = u8::from(p < 0.0);
            }
            if self.is_satisfied(bits) {
                return Ok(iteration);
            }
            if iteration < max_iterations {
                self.check_update(post, c2v);
            }
        }
        Err(CodeError::Uncorrectable)
    }

    /// Posterior LLR per variable: channel LLR plus all incoming check messages.
    fn posterior(&self, llr: &[f32], c2v: &[f32], post: &mut [f32]) {
        for (p, &l) in post.iter_mut().zip(llr) {
            *p = l.clamp(-LLR_LIMIT, LLR_LIMIT);
        }
        let mut e = 0;
        for check in self.checks {
            for &v in check.iter() {
                post[v] = (post[v] + c2v[e]).clamp(-LLR_LIMIT, LLR_LIMIT);
                e += 1;
            }
        }
    }

    /// Normalised min-sum check update, from the variable-to-check messages `post - c2v`.
    ///
    /// Each message reads its own old value before overwriting it, so no extra buffer is needed.
    fn check_update(&self, post: &[f32], c2v: &mut [f32]) {
        let mut base = 0;
        for check in self.checks {
            let mut min1 = f32::INFINITY;
            let mut min2 = f32::INFINITY;
            let mut argmin = 0;
            let mut negatives = false;
            for (k, &v) in check.iter().enumerate() {
                let m = post[v] - c2v[base + k];
                negatives ^= m < 0.0;
                let a = m.abs();
                if a < min1 {
                    min2 = min1;
                    min1 = a;
                    argmin = k;
                } else if a < min2 {
                    min2 = a;
                }
            }
            for (k, &v) in check.iter().enumerate() {
                let m = post[v] - c2v[base + k];
                let magnitude = if k == argmin { min2 } else { min1 };
                let negative = negatives ^ (m < 0.0);
                let out = NORMALISATION * magnitude;
                c2v[base + k] = if negative { -out } else { out };
            }
            base += check.len();
        }
    }

    fn is_satisfied(&self, bits: &[u8]) -> bool {
        self.checks
            .iter()
            .all(|check| check.iter().fold(0u8, |acc, &v| acc ^ bits[v]) == 0)
    }
}

fn check_len(actual: usize, expected: usize) -> Result<(), CodeError> {
    if actual != expected {
        return Err(CodeError::LengthMismatch { expected, actual });
    }
    Ok(())
}

fn check_bits(bits: &[u8]) -> Result<(), CodeError> {
    if bits.iter().any(|&b| b > 1) {
        return Err(CodeError::InvalidSymbol);
    }
    Ok(())
}
