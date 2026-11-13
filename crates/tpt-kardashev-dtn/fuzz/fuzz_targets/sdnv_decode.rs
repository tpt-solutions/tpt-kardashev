// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_dtn::sdnv;

fuzz_target!(|data: &[u8]| {
    // Must never panic; a value that decodes must re-encode to exactly the octets it came from.
    if let Ok((value, used)) = sdnv::decode(data) {
        let mut out = Vec::new();
        sdnv::encode(value, &mut out);
        assert_eq!(out, data[..used], "SDNV is not in shortest form");
    }
});
