// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `ErasureCoder` and `SoftDecoder` traits, used through their public interfaces.

use tpt_kardashev_archive_codes::conv::{self, ConvolutionalK7};
use tpt_kardashev_archive_codes::{BlockEncoder, ErasureCoder, ReedSolomon, SoftDecoder};

#[test]
fn erasure_coder_recovers_nroots_known_erasures() {
    let code = ReedSolomon::ccsds_255_223().expect("valid parameters");
    let message: [u8; 223] = core::array::from_fn(|i| (i * 11 + 5) as u8);
    let mut codeword = [0u8; 255];
    code.encode(&message, &mut codeword).expect("encode");
    let original = codeword;

    // 32 erasures (2t + e <= 32 with t = 0) at known positions, values scrambled.
    let erasures: [usize; 32] = core::array::from_fn(|i| i * 7);
    for &j in &erasures {
        codeword[j] = 0xEE;
    }
    let report = ErasureCoder::decode_erasures(&code, &mut codeword, &erasures).expect("decode");
    assert_eq!(codeword, original);
    assert!(report.corrected <= 32);
}

#[test]
fn erasure_coder_rejects_more_erasures_than_parity() {
    let code = ReedSolomon::ccsds_255_223().expect("valid parameters");
    let mut word = [0u8; 255];
    let erasures: [usize; 33] = core::array::from_fn(|i| i);
    assert!(ErasureCoder::decode_erasures(&code, &mut word, &erasures).is_err());
}

#[test]
fn soft_decoder_recovers_message_from_confident_soft_input() {
    let bits: [u8; 40] = core::array::from_fn(|i| ((i * 5 + 3) % 7 % 2) as u8);
    let mut coded = [0u8; 80];
    conv::encode(&bits, &mut coded).expect("encode");

    // Map each coded bit to a confident soft value.
    let soft: [u8; 80] = core::array::from_fn(|i| if coded[i] == 1 { 255 } else { 0 });
    let mut out = [0u8; 40];
    let mut scratch = [0u64; 40];
    let metric =
        SoftDecoder::decode_soft(&ConvolutionalK7, &soft, &mut out, &mut scratch).expect("decode");
    assert_eq!(out, bits);
    assert_eq!(metric, 0);
}

#[test]
fn soft_decoder_matches_free_function() {
    let soft: [u8; 40] = core::array::from_fn(|i| ((i * 37) % 256) as u8);
    let mut a = [0u8; 20];
    let mut b = [0u8; 20];
    let mut s1 = [0u64; 20];
    let mut s2 = [0u64; 20];
    let m1 = conv::viterbi_decode_soft(&soft, &mut a, &mut s1).expect("free fn");
    let m2 = SoftDecoder::decode_soft(&ConvolutionalK7, &soft, &mut b, &mut s2).expect("trait");
    assert_eq!((a, m1), (b, m2));
}
