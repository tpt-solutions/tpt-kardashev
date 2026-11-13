// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_archive_codes::frame;

fuzz_target!(|data: &[u8]| {
    // Must never panic. A successful parse must describe exactly the bytes it was given.
    if let Ok(parsed) = frame::decode(data) {
        assert_eq!(frame::encoded_len(parsed.payload.len()), data.len());
    }
});
