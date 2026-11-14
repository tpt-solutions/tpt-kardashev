// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Earth's penumbra: the lit fraction of the Sun's disk against brute-force sampling of that
//! disk, its limits in full sun and in the umbra, and its continuity and monotonicity across the
//! shadow edge.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::Vec3;
use tpt_kardashev_astrodynamics::ephemeris::AU_KM;
use tpt_kardashev_astrodynamics::force::WGS84_EQUATORIAL_RADIUS_KM as R_E;
use tpt_kardashev_astrodynamics::force::{SUN_RADIUS_KM, lit_fraction};

/// The Sun's direction from Earth: a unit vector scaled to one AU, not on an axis, so that the
/// shadow is not aligned with the coordinate planes.
fn sun() -> Vec3 {
    let v = Vec3::new(0.9, 0.3, 0.2);
    v * (AU_KM / v.norm())
}

/// The fraction of the Sun's disk visible from `position`, by sampling a square grid of directions
/// over the disk and counting those that do not point into the Earth's disk. Independent of
/// `lit_fraction`: no lens formula is used.
fn sampled_lit(position: Vec3, sun: Vec3, n: usize) -> f64 {
    let to_sun = sun - position;
    let d_sun = to_sun.norm();
    let s_hat = to_sun * (1.0 / d_sun);
    let r = position.norm();
    let toward_earth = position * (-1.0 / r);
    let a_s = (SUN_RADIUS_KM / d_sun).asin();
    let a_e = (R_E / r).asin();
    // An orthonormal pair perpendicular to the Sun direction.
    let helper = if s_hat.x.abs() < 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let e1_raw = s_hat.cross(helper);
    let e1 = e1_raw * (1.0 / e1_raw.norm());
    let e2 = s_hat.cross(e1);
    let (mut total, mut visible) = (0usize, 0usize);
    let step = 2.0 * a_s / n as f64;
    for i in 0..n {
        for j in 0..n {
            let dx = -a_s + (i as f64 + 0.5) * step;
            let dy = -a_s + (j as f64 + 0.5) * step;
            if dx * dx + dy * dy > a_s * a_s {
                continue;
            }
            total += 1;
            let raw = s_hat + e1 * dx + e2 * dy;
            let dir = raw * (1.0 / raw.norm());
            let angle = dir.dot(toward_earth).clamp(-1.0, 1.0).acos();
            if angle >= a_e {
                visible += 1;
            }
        }
    }
    visible as f64 / total as f64
}

#[test]
fn matches_brute_force_sampling_of_the_sun_disk() {
    // Positions across the shadow edge: a range of distances behind the Earth and offsets from the
    // Sun–Earth axis that straddle the umbra and penumbra boundaries.
    let u = sun() * (1.0 / sun().norm());
    let p = Vec3::new(-u.y, u.x, 0.0) * (1.0 / (u.x * u.x + u.y * u.y).sqrt());
    let mut worst = 0.0f64;
    for distance in [7_000.0, 10_000.0, 30_000.0] {
        for offset in [
            R_E - 150.0,
            R_E - 40.0,
            R_E - 5.0,
            R_E,
            R_E + 5.0,
            R_E + 40.0,
        ] {
            let position = u * -distance + p * offset;
            let formula = lit_fraction(position, sun());
            let sampled = sampled_lit(position, sun(), 500);
            let diff = (formula - sampled).abs();
            worst = worst.max(diff);
            assert!(
                diff <= 3e-3,
                "distance {distance}, offset {offset}: formula {formula}, sampled {sampled}"
            );
        }
    }
    eprintln!("worst difference from sampling: {worst:e}");
}

#[test]
fn full_sunlight_and_the_umbra_are_exact() {
    let u = sun() * (1.0 / sun().norm());
    let p = Vec3::new(-u.y, u.x, 0.0) * (1.0 / (u.x * u.x + u.y * u.y).sqrt());
    // Toward the Sun from the Earth: fully lit.
    assert_eq!(lit_fraction(u * 7_000.0, sun()), 1.0);
    // Far outside the shadow, on the night side: fully lit.
    assert_eq!(lit_fraction(u * -10_000.0 + p * 20_000.0, sun()), 1.0);
    // Deep in the umbra, on the axis: no light.
    assert_eq!(lit_fraction(u * -7_000.0, sun()), 0.0);
    // Well inside the umbra, halfway to the axis.
    assert_eq!(lit_fraction(u * -10_000.0 + p * (0.5 * R_E), sun()), 0.0);
}

#[test]
fn the_fraction_is_continuous_and_monotone_across_the_edge() {
    // Walk across the shadow edge 10 000 km behind the Earth, from the umbra into full sun.
    let u = sun() * (1.0 / sun().norm());
    let p = Vec3::new(-u.y, u.x, 0.0) * (1.0 / (u.x * u.x + u.y * u.y).sqrt());
    let mut previous = -1.0;
    let mut saw_partial = false;
    let mut offset = R_E - 60.0;
    while offset <= R_E + 60.0 {
        let value = lit_fraction(u * -10_000.0 + p * offset, sun());
        assert!(
            value >= previous - 1e-12,
            "not monotone at offset {offset}: {value} after {previous}"
        );
        if previous >= 0.0 {
            assert!((value - previous).abs() < 0.05, "jump at offset {offset}");
        }
        if value > 0.0 && value < 1.0 {
            saw_partial = true;
        }
        previous = value;
        offset += 0.5;
    }
    assert!(saw_partial, "the walk never entered the penumbra");
    assert_eq!(previous, 1.0);
}

#[test]
fn a_position_inside_the_earth_gives_no_light() {
    assert_eq!(lit_fraction(Vec3::new(1_000.0, 0.0, 0.0), sun()), 0.0);
}
