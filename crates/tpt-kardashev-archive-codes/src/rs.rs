// SPDX-License-Identifier: MIT OR Apache-2.0
//! Systematic Reed-Solomon codes over GF(2^8), with a Berlekamp-Massey decoder.
//!
//! A code is defined by its parity count `nroots`, the first consecutive root exponent `fcr`,
//! and the root step `prim`. The generator is
//! `g(x) = prod_{i=0}^{nroots-1} (x - alpha^{prim (fcr + i)})`.
//!
//! The codeword is `message || parity`, with the message in the highest-power positions. A
//! shortened code is any length `n` with `nroots < n <= 255`.
//!
//! [`ReedSolomon::ccsds_255_223`] uses the parameters of the CCSDS TM synchronous and channel
//! coding Reed-Solomon code: `nroots = 32`, `fcr = 112`, `prim = 11` in the conventional basis,
//! with field polynomial 0x187. These were confirmed against the ECSS-adopted draft of
//! CCSDS 131.0-B-3. They have not yet been checked against the current revision (B-5) or against
//! published known-answer vectors. Both checks are open todo items.

use crate::codec::{BlockDecoder, BlockEncoder};
use crate::error::{CodeError, DecodeReport};
use crate::gf::{alpha_pow, inv, mul};

/// Largest supported number of parity symbols.
pub const MAX_PARITY: usize = 64;

/// A Reed-Solomon code over GF(2^8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReedSolomon {
    generator: [u8; MAX_PARITY + 1],
    nroots: usize,
    fcr: u32,
    prim: u32,
}

impl ReedSolomon {
    /// Creates a code with `nroots` parity symbols, first root exponent `fcr`, and root step `prim`.
    ///
    /// Requires `1 <= nroots <= MAX_PARITY` and `prim` coprime to 255, so that `alpha^prim` is primitive.
    pub fn new(nroots: usize, fcr: u32, prim: u32) -> Result<Self, CodeError> {
        if nroots == 0 || nroots > MAX_PARITY {
            return Err(CodeError::InvalidParameter { name: "nroots" });
        }
        if prim == 0 || gcd(prim, 255) != 1 {
            return Err(CodeError::InvalidParameter { name: "prim" });
        }
        let mut generator = [0u8; MAX_PARITY + 1];
        generator[0] = 1;
        for i in 0..nroots {
            let root = alpha_pow(i64::from(prim) * i64::from(fcr + i as u32));
            // Multiply the current generator by (x + root).
            for j in (1..=i + 1).rev() {
                generator[j] = generator[j - 1] ^ mul(root, generator[j]);
            }
            generator[0] = mul(root, generator[0]);
        }
        Ok(Self {
            generator,
            nroots,
            fcr,
            prim,
        })
    }

    /// CCSDS TM Reed-Solomon RS(255, 223): 32 parity symbols, `fcr = 112`, `prim = 11`.
    ///
    /// See the module documentation for the verification status of these parameters.
    pub fn ccsds_255_223() -> Result<Self, CodeError> {
        Self::new(32, 112, 11)
    }

    /// Number of parity symbols.
    pub fn parity_len(&self) -> usize {
        self.nroots
    }

    /// Maximum number of symbol errors correctable: `nroots / 2`.
    pub fn correctable(&self) -> usize {
        self.nroots / 2
    }

    fn root(&self, i: usize) -> u8 {
        alpha_pow(i64::from(self.prim) * i64::from(self.fcr + i as u32))
    }

    /// Syndromes `S_i = c(alpha^{prim (fcr + i)})` of a received word.
    fn syndromes(&self, word: &[u8]) -> [u8; MAX_PARITY] {
        let mut syn = [0u8; MAX_PARITY];
        for (i, s) in syn.iter_mut().enumerate().take(self.nroots) {
            let x = self.root(i);
            let mut acc = 0u8;
            for &c in word {
                acc = mul(acc, x) ^ c;
            }
            *s = acc;
        }
        syn
    }

    /// Berlekamp-Massey over `len` syndromes: returns the locator `sigma` (with `sigma[0] = 1`) and its degree `L`.
    fn berlekamp_massey(syn: &[u8], len: usize) -> ([u8; MAX_PARITY + 1], usize) {
        let mut c = [0u8; MAX_PARITY + 1];
        let mut b = [0u8; MAX_PARITY + 1];
        c[0] = 1;
        b[0] = 1;
        let mut l = 0usize;
        let mut m = 1usize;
        let mut bb = 1u8;
        for n in 0..len {
            let mut d = syn[n];
            for i in 1..=l {
                d ^= mul(c[i], syn[n - i]);
            }
            if d == 0 {
                m += 1;
                continue;
            }
            let coef = mul(d, inv(bb));
            let prev = c;
            for i in 0..(MAX_PARITY + 1 - m) {
                c[i + m] ^= mul(coef, b[i]);
            }
            if 2 * l <= n {
                l = n + 1 - l;
                b = prev;
                bb = d;
                m = 1;
            } else {
                m += 1;
            }
        }
        (c, l)
    }

    /// Field element alpha^(prim * p) locating a symbol at power p.
    fn locator(&self, p: usize) -> u8 {
        alpha_pow(i64::from(self.prim) * p as i64)
    }

    fn check_length(&self, len: usize) -> Result<(), CodeError> {
        if len <= self.nroots || len > 255 {
            return Err(CodeError::InvalidParameter {
                name: "codeword length",
            });
        }
        Ok(())
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl BlockEncoder for ReedSolomon {
    /// Writes `message || parity` to `out`. `out.len()` must be `message.len() + parity_len()`.
    fn encode(&self, message: &[u8], out: &mut [u8]) -> Result<(), CodeError> {
        let n = message.len() + self.nroots;
        self.check_length(n)?;
        if out.len() != n {
            return Err(CodeError::LengthMismatch {
                expected: n,
                actual: out.len(),
            });
        }
        let mut par = [0u8; MAX_PARITY];
        for &m in message {
            let fb = m ^ par[self.nroots - 1];
            for j in (1..self.nroots).rev() {
                par[j] = par[j - 1] ^ mul(self.generator[j], fb);
            }
            par[0] = mul(self.generator[0], fb);
        }
        out[..message.len()].copy_from_slice(message);
        for (i, slot) in out[message.len()..].iter_mut().enumerate() {
            *slot = par[self.nroots - 1 - i];
        }
        Ok(())
    }
}

impl BlockDecoder for ReedSolomon {
    /// Corrects up to `nroots / 2` symbol errors in place, with no erasure information.
    ///
    /// On `Uncorrectable` the word is left unchanged. A word with more errors than the code can
    /// correct may still decode to a different valid codeword. This is inherent to bounded-distance decoding.
    fn decode(&self, word: &mut [u8]) -> Result<DecodeReport, CodeError> {
        self.correct(word, &[])
    }
}

impl ReedSolomon {
    /// Corrects errors and erasures in place.
    ///
    /// `erasures` lists codeword indices known to be unreliable. The code corrects `e` erasures and
    /// `t` errors whenever 2t + e <= nroots. An erasure whose value happens to be correct is not
    /// counted as corrected. On failure the word is left unchanged.
    pub fn decode_with_erasures(
        &self,
        word: &mut [u8],
        erasures: &[usize],
    ) -> Result<DecodeReport, CodeError> {
        self.correct(word, erasures)
    }

    fn correct(&self, word: &mut [u8], erasures: &[usize]) -> Result<DecodeReport, CodeError> {
        let n = word.len();
        self.check_length(n)?;
        let e = erasures.len();
        if e > self.nroots {
            return Err(CodeError::Uncorrectable);
        }
        let mut erasure_pow = [0usize; MAX_PARITY];
        for (k, &j) in erasures.iter().enumerate() {
            if j >= n {
                return Err(CodeError::InvalidParameter {
                    name: "erasure index",
                });
            }
            if erasures[..k].contains(&j) {
                return Err(CodeError::InvalidParameter {
                    name: "duplicate erasure",
                });
            }
            erasure_pow[k] = n - 1 - j;
        }

        let syn = self.syndromes(word);
        if syn[..self.nroots].iter().all(|&s| s == 0) {
            return Ok(DecodeReport { corrected: 0 });
        }

        // Erasure locator gamma(x) = prod (1 + X_k x), with X_k the locator of each erasure.
        let mut gamma = [0u8; MAX_PARITY + 1];
        gamma[0] = 1;
        for &p in &erasure_pow[..e] {
            let x = self.locator(p);
            for i in (1..=MAX_PARITY).rev() {
                gamma[i] ^= mul(x, gamma[i - 1]);
            }
        }

        // Modified syndromes T = S * gamma mod x^nroots. Berlekamp-Massey runs on T[e..], which
        // removes the known erasures from the problem.
        let mut modified = [0u8; MAX_PARITY];
        for (i, t) in modified.iter_mut().enumerate().take(self.nroots) {
            let mut acc = 0u8;
            for k in 0..=i.min(e) {
                acc ^= mul(gamma[k], syn[i - k]);
            }
            *t = acc;
        }
        let (sigma, l) = Self::berlekamp_massey(&modified[e..self.nroots], self.nroots - e);
        if l == 0 && e == 0 {
            return Err(CodeError::Uncorrectable);
        }
        if 2 * l + e > self.nroots {
            return Err(CodeError::Uncorrectable);
        }

        // Full locator lambda = sigma * gamma, and evaluator omega = S * lambda mod x^nroots.
        let mut lambda = [0u8; MAX_PARITY + 1];
        for (i, &si) in sigma[..=l].iter().enumerate() {
            for (j, &gj) in gamma[..=e].iter().enumerate() {
                if i + j <= MAX_PARITY {
                    lambda[i + j] ^= mul(si, gj);
                }
            }
        }
        let deg = lambda.iter().rposition(|&c| c != 0).unwrap_or(0);
        let mut omega = [0u8; MAX_PARITY];
        for (j, o) in omega.iter_mut().enumerate().take(self.nroots) {
            for i in 0..=j {
                *o ^= mul(lambda[i], syn[j - i]);
            }
        }

        // Chien search over every position power p (0 = last symbol of the word).
        let mut positions = [0usize; MAX_PARITY];
        let mut count = 0usize;
        for p in 0..n {
            let xi = alpha_pow(-i64::from(self.prim) * p as i64);
            if eval(&lambda[..=deg], xi) == 0 {
                if count >= deg {
                    return Err(CodeError::Uncorrectable);
                }
                positions[count] = p;
                count += 1;
            }
        }
        if count != deg {
            return Err(CodeError::Uncorrectable);
        }

        // Forney: value = X^(1 - fcr) * omega(X^-1) / lambda'(X^-1), with X = alpha^(prim p).
        let mut values = [0u8; MAX_PARITY];
        for (k, &p) in positions[..count].iter().enumerate() {
            let xi = alpha_pow(-i64::from(self.prim) * p as i64);
            let num = eval(&omega[..self.nroots], xi);
            let den = eval_derivative(&lambda[..=deg], xi);
            if den == 0 {
                return Err(CodeError::Uncorrectable);
            }
            let x_pow = alpha_pow(i64::from(self.prim) * p as i64 * (1 - i64::from(self.fcr)));
            values[k] = mul(x_pow, mul(num, inv(den)));
        }

        for (k, &p) in positions[..count].iter().enumerate() {
            word[n - 1 - p] ^= values[k];
        }
        if self.syndromes(word)[..self.nroots].iter().any(|&s| s != 0) {
            // Undo the attempted correction so the caller sees the original word.
            for (k, &p) in positions[..count].iter().enumerate() {
                word[n - 1 - p] ^= values[k];
            }
            return Err(CodeError::Uncorrectable);
        }
        let corrected = values[..count].iter().filter(|&&v| v != 0).count();
        Ok(DecodeReport { corrected })
    }
}

/// Horner evaluation of a polynomial with coefficients in increasing power order.
fn eval(coeffs: &[u8], x: u8) -> u8 {
    let mut acc = 0u8;
    for &c in coeffs.iter().rev() {
        acc = mul(acc, x) ^ c;
    }
    acc
}

/// Formal derivative evaluated at `x`. In characteristic 2 only odd-power terms survive.
fn eval_derivative(coeffs: &[u8], x: u8) -> u8 {
    let mut acc = 0u8;
    let mut i = 1;
    while i < coeffs.len() {
        // d/dx of c_i x^i = i c_i x^(i-1); i is odd so the coefficient is 1 in GF(2).
        let mut term = coeffs[i];
        for _ in 0..(i - 1) {
            term = mul(term, x);
        }
        acc ^= term;
        i += 2;
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Small deterministic generator for test data.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u8 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 56) as u8
        }
    }

    fn corrupt(rng: &mut Lcg, word: &mut [u8], errors: usize) {
        let n = word.len();
        let mut used = [false; 255];
        let mut placed = 0;
        while placed < errors {
            let pos = (rng.next() as usize * 256 + rng.next() as usize) % n;
            if used[pos] {
                continue;
            }
            used[pos] = true;
            let mut value = rng.next();
            while value == 0 {
                value = rng.next();
            }
            word[pos] ^= value;
            placed += 1;
        }
    }

    #[test]
    fn encoded_word_has_zero_syndromes() {
        let code = ReedSolomon::new(16, 1, 1).expect("valid");
        let msg: [u8; 20] = core::array::from_fn(|i| (i as u8).wrapping_mul(37));
        let mut cw = [0u8; 36];
        code.encode(&msg, &mut cw).expect("ok");
        assert!(code.syndromes(&cw)[..16].iter().all(|&s| s == 0));
    }

    fn round_trip(code: &ReedSolomon, k: usize, seed: u64, max_errors: usize) {
        let n = k + code.parity_len();
        let mut rng = Lcg(seed);
        for trial in 0..40 {
            let msg: std::vec::Vec<u8> = (0..k).map(|_| rng.next()).collect();
            let mut cw = std::vec![0u8; n];
            code.encode(&msg, &mut cw).expect("encode");
            let original = cw.clone();
            let errors = (trial % (max_errors + 1)).min(max_errors);
            corrupt(&mut rng, &mut cw, errors);
            let report = code.decode(&mut cw).expect("within correction capability");
            assert_eq!(report.corrected, errors, "trial {trial}");
            assert_eq!(cw, original, "trial {trial}");
        }
    }

    #[test]
    fn small_shortened_code_corrects_up_to_t() {
        let code = ReedSolomon::new(8, 1, 1).expect("valid");
        round_trip(&code, 12, 0xA5A5, code.correctable());
    }

    #[test]
    fn ccsds_255_223_corrects_up_to_sixteen_errors() {
        let code = ReedSolomon::ccsds_255_223().expect("valid parameters");
        assert_eq!(code.correctable(), 16);
        round_trip(&code, 223, 0xC0FFEE, 16);
    }

    #[test]
    fn corrects_nroots_erasures_at_known_positions() {
        let code = ReedSolomon::ccsds_255_223().expect("valid");
        let msg: std::vec::Vec<u8> = (0..223).map(|i| (i * 13 + 5) as u8).collect();
        let mut cw = std::vec![0u8; 255];
        code.encode(&msg, &mut cw).expect("encode");
        let original = cw.clone();
        let erasures: std::vec::Vec<usize> = (0..32).map(|i| i * 7 + 1).collect();
        for &j in &erasures {
            cw[j] = 0xEE;
        }
        code.decode_with_erasures(&mut cw, &erasures)
            .expect("32 erasures are correctable");
        assert_eq!(cw, original);
    }

    #[test]
    fn corrects_mixed_erasures_and_errors_within_budget() {
        // 10 erasures and 11 errors: 2 * 11 + 10 = 32 = nroots.
        let code = ReedSolomon::ccsds_255_223().expect("valid");
        let msg: std::vec::Vec<u8> = (0..223).map(|i| (i * 31 + 7) as u8).collect();
        let mut cw = std::vec![0u8; 255];
        code.encode(&msg, &mut cw).expect("encode");
        let original = cw.clone();
        let erasures: std::vec::Vec<usize> = (0..10).map(|i| 20 + i * 11).collect();
        for &j in &erasures {
            cw[j] ^= 0x55;
        }
        for k in 0..11 {
            cw[200 + k * 4] ^= 0xA1 + k as u8;
        }
        code.decode_with_erasures(&mut cw, &erasures)
            .expect("within budget");
        assert_eq!(cw, original);
    }

    #[test]
    fn rejects_bad_erasure_lists_and_leaves_word_unchanged() {
        let code = ReedSolomon::new(8, 1, 1).expect("valid");
        let mut cw = [0u8; 20];
        let before = cw;
        assert!(code.decode_with_erasures(&mut cw, &[25]).is_err());
        assert!(code.decode_with_erasures(&mut cw, &[3, 3]).is_err());
        let too_many: std::vec::Vec<usize> = (0..9).collect();
        assert!(code.decode_with_erasures(&mut cw, &too_many).is_err());
        assert_eq!(cw, before);
    }

    #[test]
    fn detects_too_many_errors_or_restores_word() {
        let code = ReedSolomon::new(8, 1, 1).expect("valid");
        let msg = [7u8; 12];
        let mut cw = [0u8; 20];
        code.encode(&msg, &mut cw).expect("ok");
        let mut rng = Lcg(99);
        let mut rejected = 0;
        for _ in 0..200 {
            let mut bad = cw;
            corrupt(&mut rng, &mut bad, 5);
            let before = bad;
            match code.decode(&mut bad) {
                Err(CodeError::Uncorrectable) => {
                    rejected += 1;
                    assert_eq!(bad, before, "failed decode must not modify the word");
                }
                Ok(_) => {}
                Err(other) => panic!("unexpected {other:?}"),
            }
        }
        assert!(rejected > 0);
    }

    #[test]
    fn rejects_invalid_parameters() {
        assert!(ReedSolomon::new(0, 1, 1).is_err());
        assert!(ReedSolomon::new(MAX_PARITY + 1, 1, 1).is_err());
        assert!(ReedSolomon::new(8, 1, 0).is_err());
        assert!(ReedSolomon::new(8, 1, 5).is_err()); // gcd(5, 255) = 5
    }
}
