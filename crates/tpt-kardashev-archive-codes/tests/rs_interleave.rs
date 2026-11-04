// SPDX-License-Identifier: MIT OR Apache-2.0
//! End-to-end: four RS(255, 223) codewords, interleaved, hit by a channel burst, then recovered.

use tpt_kardashev_archive_codes::interleave::{deinterleave, interleave};
use tpt_kardashev_archive_codes::{BlockDecoder, BlockEncoder, ReedSolomon};

const DEPTH: usize = 4;
const K: usize = 223;
const N: usize = 255;

#[test]
fn interleaved_rs_survives_a_burst_of_depth_bytes() {
    let code = ReedSolomon::ccsds_255_223().expect("valid parameters");

    // Encode DEPTH codewords back to back.
    let mut codewords = [0u8; DEPTH * N];
    for c in 0..DEPTH {
        let msg: [u8; K] = core::array::from_fn(|i| (i * 7 + c * 31) as u8);
        code.encode(&msg, &mut codewords[c * N..(c + 1) * N])
            .expect("encode");
    }
    let original = codewords;

    // Interleave, then corrupt a burst of DEPTH consecutive channel bytes.
    let mut channel = [0u8; DEPTH * N];
    interleave(&codewords, DEPTH, &mut channel).expect("interleave");
    for byte in &mut channel[1000..1000 + DEPTH] {
        *byte ^= 0xA5;
    }

    // Deinterleave: each codeword now carries at most one bad byte.
    let mut received = [0u8; DEPTH * N];
    deinterleave(&channel, DEPTH, &mut received).expect("deinterleave");
    for c in 0..DEPTH {
        let report = code
            .decode(&mut received[c * N..(c + 1) * N])
            .expect("one error per codeword is correctable");
        assert!(report.corrected <= 1, "codeword {c}: {report:?}");
    }
    assert_eq!(received, original);
}
