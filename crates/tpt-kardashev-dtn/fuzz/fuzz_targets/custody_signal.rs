// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_kardashev_dtn::custody::{CustodySignal, parse_request};

fuzz_target!(|data: &[u8]| {
    // Must never panic. A signal or request that decodes must re-encode to exactly the octets it came from.
    if let Ok(signal) = CustodySignal::decode(data) {
        assert_eq!(signal.encode(), data, "custody signal is not in canonical form");
    }
    let _ = parse_request(data);
});
