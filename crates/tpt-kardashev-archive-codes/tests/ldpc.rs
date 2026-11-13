// SPDX-License-Identifier: MIT OR Apache-2.0
//! LDPC min-sum decoding on a small Hamming-style code and a seeded random sparse code.

use tpt_kardashev_archive_codes::{CodeError, LdpcCode};

/// Checks of the (7,4) Hamming code, as sparse rows.
const HAMMING_CHECKS: [&[usize]; 3] = [&[0, 2, 4, 6], &[1, 2, 5, 6], &[3, 4, 5, 6]];

/// Deterministic LCG, so every run uses the same code and noise.
struct Lcg(u64);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u32() as usize) % n
    }

    /// Uniform in [-1, 1).
    fn unit(&mut self) -> f32 {
        (self.next_u32() as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Random sparse checks: `m` checks of degree `d` over `n` variables, distinct within a check.
fn random_checks(seed: u64, n: usize, m: usize, d: usize) -> Vec<Vec<usize>> {
    let mut rng = Lcg(seed);
    (0..m)
        .map(|_| {
            let mut check = Vec::with_capacity(d);
            while check.len() < d {
                let v = rng.below(n);
                if !check.contains(&v) {
                    check.push(v);
                }
            }
            check
        })
        .collect()
}

#[test]
fn hamming_all_zero_input_is_a_codeword() {
    let code = LdpcCode::new(7, &HAMMING_CHECKS).expect("valid code");
    let llr = [4.0f32; 7];
    let mut bits = [1u8; 7];
    let mut scratch = [0.0f32; 7 + 12];
    // Length 7 variables, 12 edges: scratch is edges + n.
    assert_eq!(code.scratch_len(), 19);
    let rounds = code
        .decode_llr(&llr, &mut bits, &mut scratch, 10)
        .expect("decode");
    assert_eq!(rounds, 0);
    assert_eq!(bits, [0u8; 7]);
}

#[test]
fn hamming_corrects_one_wrong_hard_decision() {
    let code = LdpcCode::new(7, &HAMMING_CHECKS).expect("valid code");
    for flipped in 0..7 {
        let mut llr = [4.0f32; 7];
        llr[flipped] = -1.0; // weakly wrong
        let mut bits = [0u8; 7];
        let mut scratch = vec![0.0f32; code.scratch_len()];
        code.decode_llr(&llr, &mut bits, &mut scratch, 10)
            .unwrap_or_else(|e| panic!("bit {flipped}: {e:?}"));
        assert_eq!(bits, [0u8; 7], "bit {flipped} not corrected");
    }
}

#[test]
fn reports_failure_when_no_codeword_is_reached() {
    // One check over two variables, with input 0 and 1 (violating the check). With LLRs +1 and -1
    // the min-sum messages are symmetric and the hard decisions stay 0,1 on every round, so the
    // decoder must give up.
    let checks: [&[usize]; 1] = [&[0, 1]];
    let code = LdpcCode::new(2, &checks).expect("valid code");
    let llr = [1.0f32, -1.0];
    let mut bits = [0u8; 2];
    let mut scratch = vec![0.0f32; code.scratch_len()];
    assert_eq!(
        code.decode_llr(&llr, &mut bits, &mut scratch, 5),
        Err(CodeError::Uncorrectable)
    );
    assert_eq!(bits, [0, 1]);
    assert!(!code.is_codeword(&bits).expect("length"));
}

#[test]
fn random_code_decodes_zero_codeword_under_moderate_noise() {
    let checks_owned = random_checks(0xC0DE, 96, 48, 6);
    let checks: Vec<&[usize]> = checks_owned.iter().map(Vec::as_slice).collect();
    let code = LdpcCode::new(96, &checks).expect("valid code");
    let mut scratch = vec![0.0f32; code.scratch_len()];
    let mut bits = vec![0u8; 96];
    let mut rng = Lcg(7);
    let mut converged = 0;
    for _ in 0..50 {
        // All-zero codeword sent as +4, with uniform noise of amplitude 2.
        let llr: Vec<f32> = (0..96).map(|_| 4.0 + 2.0 * rng.unit()).collect();
        if code.decode_llr(&llr, &mut bits, &mut scratch, 50).is_ok() {
            converged += 1;
            assert!(code.is_codeword(&bits).expect("length"));
            assert!(
                bits.iter().all(|&b| b == 0),
                "converged to a different codeword"
            );
        }
    }
    assert!(converged >= 45, "only {converged}/50 frames converged");
}

#[test]
fn any_successful_decode_is_a_codeword() {
    let checks_owned = random_checks(0xBEEF, 60, 30, 5);
    let checks: Vec<&[usize]> = checks_owned.iter().map(Vec::as_slice).collect();
    let code = LdpcCode::new(60, &checks).expect("valid code");
    let mut scratch = vec![0.0f32; code.scratch_len()];
    let mut bits = vec![0u8; 60];
    let mut rng = Lcg(99);
    for _ in 0..200 {
        let llr: Vec<f32> = (0..60).map(|_| 3.0 * rng.unit()).collect();
        if code.decode_llr(&llr, &mut bits, &mut scratch, 30).is_ok() {
            assert!(code.is_codeword(&bits).expect("length"));
        }
    }
}

#[test]
fn rejects_bad_construction() {
    assert_eq!(
        LdpcCode::new(0, &HAMMING_CHECKS),
        Err(CodeError::InvalidParameter { name: "n" })
    );
    let out_of_range: [&[usize]; 1] = [&[0, 9]];
    assert!(matches!(
        LdpcCode::new(7, &out_of_range),
        Err(CodeError::InvalidParameter {
            name: "variable index"
        })
    ));
    let duplicate: [&[usize]; 1] = [&[1, 1]];
    assert!(matches!(
        LdpcCode::new(7, &duplicate),
        Err(CodeError::InvalidParameter {
            name: "duplicate index"
        })
    ));
    let degree_one: [&[usize]; 1] = [&[3]];
    assert!(matches!(
        LdpcCode::new(7, &degree_one),
        Err(CodeError::InvalidParameter {
            name: "check degree"
        })
    ));
}

#[test]
fn rejects_bad_decode_arguments() {
    let code = LdpcCode::new(7, &HAMMING_CHECKS).expect("valid code");
    let mut scratch = vec![0.0f32; code.scratch_len()];
    let mut bits = [0u8; 7];
    let llr = [1.0f32; 7];
    assert!(matches!(
        code.decode_llr(&llr, &mut bits, &mut scratch, 0),
        Err(CodeError::InvalidParameter {
            name: "max_iterations"
        })
    ));
    assert!(matches!(
        code.decode_llr(&llr[..6], &mut bits, &mut scratch, 5),
        Err(CodeError::LengthMismatch {
            expected: 7,
            actual: 6
        })
    ));
    let mut short = [0.0f32; 3];
    assert!(matches!(
        code.decode_llr(&llr, &mut bits, &mut short, 5),
        Err(CodeError::LengthMismatch { .. })
    ));
    let nan = [f32::NAN, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    assert_eq!(
        code.decode_llr(&nan, &mut bits, &mut scratch, 5),
        Err(CodeError::InvalidSymbol)
    );
}
