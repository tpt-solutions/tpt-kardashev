// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_dtn::cbor::Reader;

fuzz_target!(|data: &[u8]| {
    // Exercise every reader entry point; none may panic.
    let mut r = Reader::new(data);
    let _ = r.read_array_header();
    let _ = r.read_uint();
    let _ = r.read_bytes();
    let _ = r.read_text();
});
