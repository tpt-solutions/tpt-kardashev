// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_archive_codes::conv::viterbi_decode_soft;

fuzz_target!(|data: &[u8]| {
    // Even-length soft input, one byte per coded bit. Any length must be handled without panic.
    let coded = &data[..data.len() & !1];
    let steps = coded.len() / 2;
    let mut out = std::vec![0u8; steps];
    let mut scratch = std::vec![0u64; steps];
    let _ = viterbi_decode_soft(coded, &mut out, &mut scratch);
});
