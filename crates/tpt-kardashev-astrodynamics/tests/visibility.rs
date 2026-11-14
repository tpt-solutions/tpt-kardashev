// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ground-station visibility: station geometry against exact cases, elevation at the zenith and
//! nadir, contact windows of a circular equatorial orbit with their edges on the elevation limit,
//! and rejection of invalid input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::FRAC_PI_2;
use tpt_kardashev_astrodynamics::force::WGS84_EQUATORIAL_RADIUS_KM;
use tpt_kardashev_astrodynamics::propagate::propagate;
use tpt_kardashev_astrodynamics::visibility::{
    EarthRotation, GroundStation, WGS84_ECCENTRICITY_SQ, contact_windows, elevation,
};
use tpt_kardashev_astrodynamics::{Error, Radians, Seconds, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;

fn rad(value: f64) -> Radians {
    Radians::new(value).expect("finite")
}

fn secs(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

#[test]
fn a_station_on_the_equator_lies_on_the_equatorial_radius() {
    let s = GroundStation::new(rad(0.0), rad(0.0), 0.0).expect("valid");
    let p = s.ecef();
    assert!((p.x - WGS84_EQUATORIAL_RADIUS_KM).abs() <= 1e-9);
    assert!(p.y.abs() <= 1e-9 && p.z.abs() <= 1e-9);
}

#[test]
fn a_station_at_the_pole_lies_on_the_polar_radius() {
    // The polar radius of WGS 84 is a·√(1 − e²) = 6356.752 314 km.
    let s = GroundStation::new(rad(FRAC_PI_2), rad(0.0), 0.0).expect("valid");
    let polar = WGS84_EQUATORIAL_RADIUS_KM * (1.0 - WGS84_ECCENTRICITY_SQ).sqrt();
    assert!((s.ecef().z - polar).abs() <= 1e-6, "z = {}", s.ecef().z);
    assert!((polar - 6_356.752_314).abs() <= 1e-3);
}

#[test]
fn a_satellite_overhead_is_at_ninety_degrees_and_one_below_is_at_minus_ninety() {
    let s = GroundStation::new(rad(0.3), rad(1.0), 0.0).expect("valid");
    let rot = EarthRotation::new(rad(0.0));
    let up = s.vertical_ecef();
    let overhead = s.ecef() + up * 500.0;
    let below = s.ecef() - up * 500.0;
    let zenith = elevation(&s, &rot, 0.0, overhead).expect("elevation");
    let nadir = elevation(&s, &rot, 0.0, below).expect("elevation");
    assert!((zenith - FRAC_PI_2).abs() <= 1e-9, "zenith {zenith}");
    assert!((nadir + FRAC_PI_2).abs() <= 1e-9, "nadir {nadir}");
}

#[test]
fn contact_windows_of_an_equatorial_leo_have_plausible_passes_with_edges_on_the_limit() {
    // A 500 km circular equatorial orbit, a station on the equator, and a 10° elevation limit over
    // one day. The Earth turns under the orbit, so the satellite passes the station about every
    // hour and a half; each pass is shorter than 12 minutes.
    let r = WGS84_EQUATORIAL_RADIUS_KM + 500.0;
    let start = StateVector {
        position: Vec3::new(r, 0.0, 0.0),
        velocity: Vec3::new(0.0, (MU / r).sqrt(), 0.0),
    };
    let station = GroundStation::new(rad(0.0), rad(0.0), 0.0).expect("valid");
    let rot = EarthRotation::new(rad(0.0));
    let limit = 10.0_f64.to_radians();
    let windows = contact_windows(
        &station,
        &rot,
        rad(limit),
        |t| propagate(&start, MU, t),
        secs(86_400.0),
        secs(30.0),
    )
    .expect("windows");

    assert!(
        windows.len() >= 10,
        "only {} passes in a day",
        windows.len()
    );
    let mut previous_end = -1.0;
    for &(open, close) in &windows {
        let (a, b) = (open.get(), close.get());
        assert!(a >= previous_end, "windows overlap or are out of order");
        assert!(b > a, "empty window");
        let length = b - a;
        assert!(
            length <= 720.0,
            "a pass of {length} s is too long for 500 km"
        );
        // The window ends sit on the elevation limit, to within the bisection tolerance (about a
        // thousandth of a second of satellite motion, well under 0.01°).
        for (edge, inside) in [(a, a > 0.0), (b, b < 86_400.0)] {
            if inside {
                let state = propagate(&start, MU, secs(edge)).expect("propagates");
                let e = elevation(&station, &rot, edge, state.position).expect("elevation");
                assert!(
                    (e - limit).abs() <= 0.01_f64.to_radians(),
                    "elevation {} deg at an edge, limit 10 deg",
                    e.to_degrees()
                );
            }
        }
        // The middle of each window is above the limit.
        let mid = 0.5 * (a + b);
        let state = propagate(&start, MU, secs(mid)).expect("propagates");
        assert!(elevation(&station, &rot, mid, state.position).expect("elevation") >= limit);
        previous_end = b;
    }
}

#[test]
fn a_satellite_always_above_the_limit_gives_one_window_over_the_whole_span() {
    // A satellite placed directly above the station, hovering: every sample is visible.
    let station = GroundStation::new(rad(0.0), rad(0.0), 0.0).expect("valid");
    let rot = EarthRotation::new(rad(0.0));
    let p = station.ecef() + station.vertical_ecef() * 1_000.0;
    let windows = contact_windows(
        &station,
        &rot,
        rad(0.0),
        |_| {
            Ok(StateVector {
                position: p,
                velocity: Vec3::new(0.0, 0.0, 0.0),
            })
        },
        secs(600.0),
        secs(60.0),
    )
    .expect("windows");
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].0.get(), 0.0);
    assert_eq!(windows[0].1.get(), 600.0);
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(
        GroundStation::new(rad(FRAC_PI_2 + 0.01), rad(0.0), 0.0).map(|_| ()),
        Err(Error::OutOfRange { what: "latitude" })
    );
    assert!(matches!(
        GroundStation::new(rad(0.0), rad(0.0), f64::NAN).map(|_| ()),
        Err(Error::NonFinite { .. })
    ));
    let station = GroundStation::new(rad(0.0), rad(0.0), 0.0).expect("valid");
    let rot = EarthRotation::new(rad(0.0));
    let still = |_: Seconds| -> Result<StateVector, Error> {
        Ok(StateVector {
            position: Vec3::new(7_000.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
        })
    };
    assert_eq!(
        contact_windows(&station, &rot, rad(0.0), still, secs(10.0), secs(0.0)).map(|_| ()),
        Err(Error::InvalidStep)
    );
    assert_eq!(
        contact_windows(&station, &rot, rad(0.0), still, secs(0.0), secs(1.0)).map(|w| w.len()),
        Ok(0)
    );
}
