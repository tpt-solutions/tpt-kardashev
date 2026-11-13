// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_dtn::ltp::segment::Segment;

fuzz_target!(|data: &[u8]| {
    // Must never panic. A segment that decodes must re-encode to the same bytes.
    if let Ok(seg) = Segment::decode(data) {
        let again = seg.encode().expect("a decoded segment is valid");
        assert_eq!(again, data, "decode and encode disagree");
    }
});
