// SPDX-License-Identifier: MIT OR Apache-2.0
//! The heliocentric ecliptic frame: the equatorial/ecliptic rotations invert each other, the Earth
//! sits about one astronomical unit from the Sun, and a point at the Sun's position maps to the
//! origin.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::ephemeris::{AU_KM, Body, Ephemeris, LowPrecision};
use tpt_kardashev_astrodynamics::frames::{
    ecliptic_to_equatorial, equatorial_to_ecliptic, heliocentric_ecliptic,
};
use tpt_kardashev_astrodynamics::{Epoch, Error, Vec3};

const J2000_JD: f64 = 2_451_545.0;

fn j2000() -> Epoch {
    Epoch::from_tt_julian_date(J2000_JD).expect("finite epoch")
}

fn norm(v: Vec3) -> f64 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

#[test]
fn equatorial_to_ecliptic_inverts_ecliptic_to_equatorial() {
    let samples = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(3.0e5, -4.0e5, 1.2e5),
    ];
    for v in samples {
        let back = equatorial_to_ecliptic(ecliptic_to_equatorial(v));
        assert!((back.x - v.x).abs() <= 1e-9 * norm(v).max(1.0));
        assert!((back.y - v.y).abs() <= 1e-9 * norm(v).max(1.0));
        assert!((back.z - v.z).abs() <= 1e-9 * norm(v).max(1.0));
    }
}

#[test]
fn the_ecliptic_pole_maps_to_the_equatorial_pole_direction() {
    // The ecliptic pole (0,0,1) in equatorial coordinates is (0, -sin e, cos e): tilted by the
    // obliquity from the celestial pole, so its z component is cos(23.44 degrees).
    let pole = ecliptic_to_equatorial(Vec3::new(0.0, 0.0, 1.0));
    let obliquity = (84_381.406_f64 / 3600.0).to_radians();
    assert!((pole.z - obliquity.cos()).abs() < 1e-12);
}

#[test]
fn earth_sits_about_one_astronomical_unit_from_the_sun() {
    // The Earth's centre is the origin of the geocentric input, so the result is the Earth's
    // heliocentric position: -(Sun's geocentric position). Its ecliptic latitude is zero.
    let r = heliocentric_ecliptic(Vec3::new(0.0, 0.0, 0.0), j2000(), &LowPrecision)
        .expect("finite input");
    let au = norm(r) / AU_KM;
    assert!(
        (0.983..=1.017).contains(&au),
        "Earth-Sun distance {au} AU is outside the perihelion/aphelion range"
    );
    assert_eq!(r.z, 0.0, "the Earth lies in the ecliptic plane");
}

#[test]
fn a_point_at_the_suns_position_maps_to_the_origin() {
    let epoch = j2000();
    let sun_geo_ecliptic = LowPrecision
        .geocentric_position(Body::Sun, epoch)
        .expect("sun");
    // Express the Sun's geocentric position in the equatorial frame, as the input expects.
    let sun_geo_equatorial = ecliptic_to_equatorial(sun_geo_ecliptic);
    let r = heliocentric_ecliptic(sun_geo_equatorial, epoch, &LowPrecision).expect("finite");
    assert!(norm(r) < 1e-6, "residual {} km", norm(r));
}

#[test]
fn non_finite_input_is_rejected() {
    let bad = Vec3::new(f64::NAN, 0.0, 0.0);
    assert_eq!(
        heliocentric_ecliptic(bad, j2000(), &LowPrecision),
        Err(Error::NonFinite { what: "position" })
    );
}
