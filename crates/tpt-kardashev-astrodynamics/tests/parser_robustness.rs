// SPDX-License-Identifier: MIT OR Apache-2.0
//! Deterministic robustness checks for the TLE and SPK parsers: mutated and random input must
//! come back as `Ok` or `Err`, never as a panic. The generator is a fixed-seed LCG, so every run
//! sees the same inputs. This is not a coverage-guided fuzzer; a `cargo-fuzz` target is a
//! separate todo item.
#![cfg(feature = "sgp4")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::sgp4::parse_tle;
use tpt_kardashev_astrodynamics::spk::read_segments;

const LINE1: &str = "1 00005U 58002B   00179.78495062   .00000023   00000-0   28098-4 0   4753";
const LINE2: &str = "2 00005   34.2682 348.7242 1859667 331.7664   19.3264 10.82419157413667   0.00   4320.0   360.00";

/// A 64-bit linear congruential generator (Knuth's constants): deterministic, no dependency.
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Characters that appear in element sets, plus some that do not, so that both the happy path
/// and the rejection paths are reached.
const ALPHABET: &[u8] = b"0123456789 .-+UBAabcXYZ\t\n";

#[test]
fn mutated_element_sets_never_panic() {
    let mut rng = Lcg(0x5eed_0001);
    let (mut ok, mut err) = (0usize, 0usize);
    for _ in 0..20_000 {
        let mut a = LINE1.as_bytes().to_vec();
        let mut b = LINE2.as_bytes().to_vec();
        for _ in 0..1 + rng.below(4) {
            let line = if rng.below(2) == 0 { &mut a } else { &mut b };
            let pos = rng.below(line.len());
            line[pos] = ALPHABET[rng.below(ALPHABET.len())];
        }
        if rng.below(8) == 0 {
            let cut = rng.below(b.len());
            b.truncate(cut);
        }
        let line1 = String::from_utf8_lossy(&a).into_owned();
        let line2 = String::from_utf8_lossy(&b).into_owned();
        match parse_tle(&line1, &line2) {
            Ok(_) => ok += 1,
            Err(_) => err += 1,
        }
    }
    // Both outcomes must occur, or the mutations are not reaching the interesting paths.
    assert!(ok > 0 && err > 0, "ok {ok}, err {err}");
}

#[test]
fn random_element_sets_never_panic() {
    let mut rng = Lcg(0x5eed_0002);
    for _ in 0..20_000 {
        let len1 = rng.below(80);
        let len2 = rng.below(80);
        let line1: String = (0..len1)
            .map(|_| ALPHABET[rng.below(ALPHABET.len())] as char)
            .collect();
        let line2: String = (0..len2)
            .map(|_| ALPHABET[rng.below(ALPHABET.len())] as char)
            .collect();
        let _ = parse_tle(&line1, &line2);
    }
}

#[test]
fn random_bytes_never_panic_the_spk_reader() {
    let mut rng = Lcg(0x5eed_0003);
    for _ in 0..5_000 {
        let len = rng.below(2_048);
        let mut bytes: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        // Half the inputs start with the DAF magic, so the reader gets past the file record.
        if rng.below(2) == 0 && bytes.len() >= 8 {
            bytes[..8].copy_from_slice(b"DAF/SPK ");
        }
        let _ = read_segments(&bytes);
    }
}

#[test]
fn truncations_of_a_valid_element_set_never_panic() {
    for cut in 0..LINE2.len() {
        let _ = parse_tle(LINE1, &LINE2[..cut]);
    }
    for cut in 0..LINE1.len() {
        let _ = parse_tle(&LINE1[..cut], LINE2);
    }
}

/// Regression: a two-byte character inside a line used to make a fixed-column slice fall inside it
/// and panic (found by `cargo fuzz run tle_parse`). Every such insertion must now be an error.
#[test]
fn multibyte_characters_in_a_line_are_rejected_not_sliced() {
    for pos in 0..LINE1.len() {
        let line = format!("{}ϸ{}", &LINE1[..pos], &LINE1[pos..]);
        assert!(parse_tle(&line, LINE2).is_err(), "insertion at {pos}");
    }
    for pos in 0..LINE2.len() {
        let line = format!("{}ϸ{}", &LINE2[..pos], &LINE2[pos..]);
        assert!(parse_tle(LINE1, &line).is_err(), "insertion at {pos}");
    }
}
