// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reed-Solomon with interleaving: a 4-byte channel burst is corrected.
//!
//! Run with `cargo run -p tpt-kardashev-archive-codes --example rs_interleave`.

use tpt_kardashev_archive_codes::interleave::{deinterleave, interleave};
use tpt_kardashev_archive_codes::{BlockDecoder, BlockEncoder, CodeError, ReedSolomon};

const DEPTH: usize = 4;
const K: usize = 223;
const N: usize = 255;

fn main() -> Result<(), CodeError> {
    let code = ReedSolomon::ccsds_255_223()?;

    let mut codewords = [0u8; DEPTH * N];
    for c in 0..DEPTH {
        let msg: [u8; K] = core::array::from_fn(|i| (i + c) as u8);
        code.encode(&msg, &mut codewords[c * N..(c + 1) * N])?;
    }

    let mut channel = [0u8; DEPTH * N];
    interleave(&codewords, DEPTH, &mut channel)?;
    for byte in &mut channel[1000..1000 + DEPTH] {
        *byte ^= 0xFF;
    }

    let mut received = [0u8; DEPTH * N];
    deinterleave(&channel, DEPTH, &mut received)?;
    let mut total = 0;
    for c in 0..DEPTH {
        let report = code.decode(&mut received[c * N..(c + 1) * N])?;
        total += report.corrected;
    }

    println!("Corrected {total} symbols across {DEPTH} codewords after a {DEPTH}-byte burst.");
    println!("Recovered word matches original: {}", received == codewords);
    Ok(())
}
