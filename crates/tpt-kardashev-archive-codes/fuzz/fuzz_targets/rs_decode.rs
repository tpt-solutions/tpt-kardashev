// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_archive_codes::{BlockDecoder, ErasureCoder, ReedSolomon};

fuzz_target!(|data: &[u8]| {
    let Ok(code) = ReedSolomon::ccsds_255_223() else {
        return;
    };
    // Use the first byte to choose how many erasures to declare, then the rest as the word.
    let (&erasure_count, rest) = match data.split_first() {
        Some(split) => split,
        None => return,
    };
    let mut word = [0u8; 255];
    for (w, &b) in word.iter_mut().zip(rest) {
        *w = b;
    }
    let mut erasures = [0usize; 32];
    let count = usize::from(erasure_count % 33).min(32);
    for (i, e) in erasures.iter_mut().take(count).enumerate() {
        *e = (usize::from(erasure_count) + i * 7) % 255;
    }
    let before = word;
    // A failed decode must leave the word unchanged.
    if code.decode(&mut word).is_err() {
        assert_eq!(word, before);
    }
    let mut again = before;
    if ErasureCoder::decode_erasures(&code, &mut again, &erasures[..count]).is_err() {
        assert_eq!(again, before);
    }
});
