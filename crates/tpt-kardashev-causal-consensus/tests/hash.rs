// SPDX-License-Identifier: MIT OR Apache-2.0
//! SHA-256 against the published FIPS 180-4 examples, and streaming against one-shot hashing.

use tpt_kardashev_causal_consensus::hash::{Sha256, sha256};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn the_empty_message_has_the_published_digest() {
    assert_eq!(
        hex(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn abc_has_the_published_digest() {
    assert_eq!(
        hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn a_448_bit_message_needs_two_blocks_and_has_the_published_digest() {
    let message = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    assert_eq!(
        hex(&sha256(message)),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn streaming_in_any_split_gives_the_one_shot_digest() {
    // Every length around the block boundaries, split at every point.
    for len in [0usize, 1, 55, 56, 63, 64, 65, 119, 120, 128, 200] {
        let data: Vec<u8> = (0..len).map(|i| (i * 31 + 7) as u8).collect();
        let expected = sha256(&data);
        for split in 0..=len {
            let mut hash = Sha256::new();
            hash.update(&data[..split]);
            hash.update(&data[split..]);
            assert_eq!(hash.finalize(), expected, "len {len}, split {split}");
        }
    }
}

#[test]
fn many_one_byte_updates_match_one_shot() {
    let data: Vec<u8> = (0..1_000u32).map(|i| (i % 251) as u8).collect();
    let mut hash = Sha256::new();
    for byte in &data {
        hash.update(std::slice::from_ref(byte));
    }
    assert_eq!(hash.finalize(), sha256(&data));
}
