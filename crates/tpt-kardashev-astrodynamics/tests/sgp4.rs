// SPDX-License-Identifier: MIT OR Apache-2.0
//! SGP4 against the verification case of Vallado, Crawford, Hujsak and Kelso (AIAA 2006-6753): the
//! element set of satellite 00005 under WGS-72, and the state the paper prints at seven epochs.
//! Also: element parsing, deep-space rejection, and malformed input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::sgp4::{GravityConstants, Sgp4, parse_tle};
use tpt_kardashev_astrodynamics::{Error, StateVector, Vec3};

/// The verification element set (the paper's "TEME example", satellite 00005).
const LINE1: &str = "1 00005U 58002B   00179.78495062   .00000023   00000-0   28098-4 0   4753";
const LINE2: &str = "2 00005   34.2682 348.7242 1859667 331.7664   19.3264 10.82419157413667   0.00   4320.0   360.00";

/// The paper's printed states (km, km/s in TEME) at these minutes after epoch.
const REFERENCE: [(f64, [f64; 6]); 7] = [
    (
        0.0,
        [
            7022.46529266,
            -1400.08296755,
            0.03995155,
            1.893841015,
            6.405893759,
            4.534807250,
        ],
    ),
    (
        360.0,
        [
            -7154.03120202,
            -3783.17682504,
            -3536.19412294,
            4.741887409,
            -4.151817765,
            -2.093935425,
        ],
    ),
    (
        720.0,
        [
            -7134.59340119,
            6531.68641334,
            3260.27186483,
            -4.113793027,
            -2.911922039,
            -2.557327851,
        ],
    ),
    (
        1080.0,
        [
            5568.53901181,
            4492.06992591,
            3863.87641983,
            -4.209106476,
            5.159719888,
            2.744852980,
        ],
    ),
    (
        1440.0,
        [
            -938.55923943,
            -6268.18748831,
            -4294.02924751,
            7.536105209,
            -0.427127707,
            0.989878080,
        ],
    ),
    (
        1800.0,
        [
            -9680.56121728,
            2802.47771354,
            124.10688038,
            -0.905874102,
            -4.659467970,
            -3.227347517,
        ],
    ),
    (
        2160.0,
        [
            190.19796988,
            7746.96653614,
            5110.00675412,
            -6.112325142,
            1.527008184,
            -0.139152358,
        ],
    ),
];

fn model() -> Sgp4 {
    let elements = parse_tle(LINE1, LINE2).expect("parses");
    Sgp4::new(GravityConstants::WGS72, elements).expect("near-earth")
}

#[test]
fn the_verification_element_set_parses_to_its_printed_values() {
    let e = parse_tle(LINE1, LINE2).expect("parses");
    assert!((e.eccentricity - 0.1859667).abs() <= 1e-12);
    assert!((e.inclination.to_degrees() - 34.2682).abs() <= 1e-9);
    assert!((e.raan.to_degrees() - 348.7242).abs() <= 1e-9);
    assert!((e.arg_perigee.to_degrees() - 331.7664).abs() <= 1e-9);
    assert!((e.mean_anomaly.to_degrees() - 19.3264).abs() <= 1e-9);
    // 10.82419157 revolutions a day, in radians a minute.
    let rad_per_min = 10.824_191_57 * 2.0 * core::f64::consts::PI / 1440.0;
    assert!((e.mean_motion - rad_per_min).abs() <= 1e-12 * rad_per_min);
    // B* = 28098-4, read as 0.28098 × 10⁻⁴.
    assert!((e.bstar - 2.8098e-5).abs() <= 1e-15);
}

#[test]
fn wgs72_gives_the_published_canonical_mean_motion_unit() {
    // XKE for WGS-72 is 0.074 366 916 133 17 per minute in the paper's Table 2.
    let c = GravityConstants::WGS72;
    let xke = 60.0 / (c.radius_km.powi(3) / c.mu).sqrt();
    assert!((xke - 0.074_366_916_133_17).abs() <= 1e-12, "xke = {xke}");
}

#[test]
fn states_match_the_published_verification_case() {
    let sgp4 = model();
    for (minutes, expected) in REFERENCE {
        let state = sgp4.propagate(minutes).expect("propagates");
        let got = [
            state.position.x,
            state.position.y,
            state.position.z,
            state.velocity.x,
            state.velocity.y,
            state.velocity.z,
        ];
        for (axis, (g, e)) in got.iter().zip(expected.iter()).enumerate() {
            let tol = if axis < 3 { 1e-6 } else { 1e-8 };
            assert!(
                (g - e).abs() <= tol,
                "t = {minutes} min, component {axis}: got {g}, paper {e}"
            );
        }
    }
}

#[test]
fn the_epoch_state_is_the_papers_first_row() {
    let sgp4 = model();
    let forward = sgp4.propagate(360.0).expect("propagates");
    let back: StateVector = sgp4.propagate(0.0).expect("propagates");
    // The state at the epoch is the paper's first row, reached directly.
    assert!(
        (back.position - Vec3::new(7_022.465_292_66, -1_400.082_967_55, 0.039_951_55)).norm()
            <= 1e-6
    );
    assert!(forward.position.norm() > 0.0);
}

#[test]
fn deep_space_orbits_initialize_and_propagate() {
    // Two revolutions a day: a 720-minute period, which takes the deep-space terms. The state must
    // be finite and at a plausible radius (above the surface, below the sphere of geostationary orbit).
    let line2 = "2 00005   34.2682 348.7242 1859667 331.7664   19.3264  2.00000000413667   0.00   4320.0   360.00";
    let elements = parse_tle(LINE1, line2).expect("parses");
    let model = Sgp4::new(GravityConstants::WGS72, elements).expect("initializes");
    for minutes in [0.0, 360.0, 1440.0] {
        let r = model
            .propagate(minutes)
            .expect("propagates")
            .position
            .norm();
        assert!(
            (6_378.0..50_000.0).contains(&r),
            "radius {r} km at {minutes} min"
        );
    }
}

#[test]
fn malformed_element_sets_are_rejected() {
    assert!(matches!(
        parse_tle("3 short", LINE2),
        Err(Error::BadTle { .. })
    ));
    assert!(matches!(
        parse_tle(LINE1, "2 short"),
        Err(Error::BadTle { .. })
    ));
    // An eccentricity field that is not seven digits.
    let bad_ecc = "2 00005   34.2682 348.7242 18596X7 331.7664   19.3264 10.82419157413667   0.00   4320.0   360.00";
    assert!(matches!(
        parse_tle(LINE1, bad_ecc),
        Err(Error::BadTle { .. })
    ));
}
