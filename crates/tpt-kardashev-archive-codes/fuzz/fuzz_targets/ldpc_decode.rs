// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_archive_codes::{CodeError, LdpcCode};

const CHECKS: [&[usize]; 3] = [&[0, 2, 4, 6], &[1, 2, 5, 6], &[3, 4, 5, 6]];

fuzz_target!(|data: &[u8]| {
    let Ok(code) = LdpcCode::new(7, &CHECKS) else {
        return;
    };
    // Map each byte to an LLR in [-8, 8]; the 7 LLRs are taken from the start of the input.
    let mut llr = [0f32; 7];
    for (l, &b) in llr.iter_mut().zip(data) {
        *l = f32::from(b as i8) / 16.0;
    }
    let mut bits = [0u8; 7];
    let mut scratch = [0f32; 19];
    match code.decode_llr(&llr, &mut bits, &mut scratch, 8) {
        Ok(_) => assert!(code.is_codeword(&bits).unwrap_or(false)),
        Err(CodeError::Uncorrectable) => {}
        Err(e) => panic!("unexpected error {e:?}"),
    }
});
