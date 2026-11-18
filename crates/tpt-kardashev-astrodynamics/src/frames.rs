// SPDX-License-Identifier: MIT OR Apache-2.0
//! The radial–transverse–normal (RTN) frame of a state.
//!
//! RTN is the frame of a spacecraft's own position: R points along the position vector, N along
//! the orbit normal `h = r × v`, and T completes the right-handed triad, `T = N × R`. For a
//! circular orbit the velocity is purely along T.
//!
//! LVLH is written with the axis convention documented on [`lvlh_matrix`]. The heliocentric
//! ecliptic frame is written as a conversion from the Earth-centred equatorial frame,
//! [`heliocentric_ecliptic`]. The Earth-fixed position transform is [`inertial_to_earth_fixed`],
//! which takes the Earth rotation angle as an input: the angle itself needs UT1, which the crate
//! does not bundle.

use crate::elements::StateVector;
use crate::ephemeris::{Body, Ephemeris};
use crate::time::Epoch;
use crate::{Error, Mat3, Vec3};

/// The obliquity of the ecliptic at J2000, in arcseconds (IAU 2006: 84 381.406″).
pub const OBLIQUITY_J2000_ARCSEC: f64 = 84_381.406;

/// Rotates a vector from the mean ecliptic of J2000 to the mean equator of J2000: a rotation about
/// the x axis by the obliquity. The ecliptic pole `(0, 0, 1)` maps to `(0, −sin ε, cos ε)`.
pub fn ecliptic_to_equatorial(v: Vec3) -> Vec3 {
    let eps = (OBLIQUITY_J2000_ARCSEC / 3600.0).to_radians();
    let (s, c) = eps.sin_cos();
    Vec3::new(v.x, c * v.y - s * v.z, s * v.y + c * v.z)
}

/// Rotates a vector from the mean equator of J2000 to the mean ecliptic of J2000. This is the
/// inverse of [`ecliptic_to_equatorial`]: a rotation about the x axis by minus the obliquity.
pub fn equatorial_to_ecliptic(v: Vec3) -> Vec3 {
    let eps = (OBLIQUITY_J2000_ARCSEC / 3600.0).to_radians();
    let (s, c) = eps.sin_cos();
    Vec3::new(v.x, c * v.y + s * v.z, -s * v.y + c * v.z)
}

/// Converts a position from the Earth-centred inertial frame (mean equator of J2000) to the
/// heliocentric ecliptic frame of J2000: the origin is the Sun's centre, the fundamental plane is
/// the mean ecliptic of J2000, and the x axis points to the J2000 equinox.
///
/// `geocentric_equatorial` is the position relative to the Earth's centre, in km, in the
/// equatorial frame that the rest of this crate uses. The Sun's geocentric position comes from
/// `ephemeris` at `epoch`, so the accuracy is that of the ephemeris (about 0.01° for the Sun's
/// direction with [`crate::ephemeris::LowPrecision`]).
///
/// Fails with [`Error::NonFinite`] when the input is not finite.
pub fn heliocentric_ecliptic<E: Ephemeris>(
    geocentric_equatorial: Vec3,
    epoch: Epoch,
    ephemeris: &E,
) -> Result<Vec3, Error> {
    let p = geocentric_equatorial;
    if !(p.x.is_finite() && p.y.is_finite() && p.z.is_finite()) {
        return Err(Error::NonFinite { what: "position" });
    }
    let sun = ephemeris.geocentric_position(Body::Sun, epoch)?;
    let e = equatorial_to_ecliptic(p);
    // The Earth's heliocentric position is minus the Sun's geocentric position.
    Ok(Vec3::new(e.x - sun.x, e.y - sun.y, e.z - sun.z))
}

/// Turns an inertial position into the Earth-fixed frame, given the angle of the Earth-fixed x axis
/// from the inertial x axis (the Earth rotation angle). The same convention as
/// [`crate::visibility::EarthRotation`].
///
/// The angle is an input: the crate does not compute it from UT1 or sidereal time, and bundles no
/// Earth orientation data. Velocity is not transformed here, because the Earth-fixed velocity also
/// needs the rotation rate and the position (`v_ef = R v - ω × r_ef`).
pub fn inertial_to_earth_fixed(position: Vec3, earth_rotation_angle: f64) -> Vec3 {
    let (s, c) = earth_rotation_angle.sin_cos();
    Vec3::new(
        c * position.x + s * position.y,
        -s * position.x + c * position.y,
        position.z,
    )
}

/// The matrix whose rows are the RTN axes, as unit vectors in the state's own frame. Applying it
/// to a vector gives that vector's `(radial, transverse, normal)` components.
///
/// Fails with [`Error::NonFinite`] for non-finite inputs and [`Error::Degenerate`] when the
/// position is zero or position and velocity are parallel.
pub fn rtn_matrix(state: &StateVector) -> Result<Mat3, Error> {
    if !state.position.is_finite() {
        return Err(Error::NonFinite {
            what: "position (km)",
        });
    }
    if !state.velocity.is_finite() {
        return Err(Error::NonFinite {
            what: "velocity (km/s)",
        });
    }
    let r = state.position;
    let h = r.cross(state.velocity);
    let r_norm = r.norm();
    let h_norm = h.norm();
    if !(r_norm > 0.0 && h_norm > 0.0) {
        return Err(Error::Degenerate);
    }
    let radial = r * (1.0 / r_norm);
    let normal = h * (1.0 / h_norm);
    let transverse = normal.cross(radial);
    Ok(Mat3::from_rows([
        [radial.x, radial.y, radial.z],
        [transverse.x, transverse.y, transverse.z],
        [normal.x, normal.y, normal.z],
    ]))
}

/// The matrix whose rows are the local-vertical, local-horizontal axes of a state, as unit vectors
/// in the state's own frame. The convention, chosen for this crate:
///
/// - `x`: along-track, `T = N × R` (for a circular orbit, along the velocity);
/// - `y`: cross-track, the negative orbit normal, `−N`;
/// - `z`: nadir, toward the centre of the body, `−R`.
///
/// The triad is right-handed: `x × y = z`. It is the RTN triad reordered and signed, so the
/// components of a vector in LVLH are `(T, −N, −R)` of its RTN components. Other sources use other
/// axis orders and signs; this one is fixed here so that each state has one LVLH matrix.
///
/// Fails as [`rtn_matrix`] does.
pub fn lvlh_matrix(state: &StateVector) -> Result<Mat3, Error> {
    let rtn = rtn_matrix(state)?.rows;
    let neg = |v: [f64; 3]| [-v[0], -v[1], -v[2]];
    Ok(Mat3::from_rows([rtn[1], neg(rtn[2]), neg(rtn[0])]))
}
