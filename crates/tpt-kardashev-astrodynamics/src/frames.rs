// SPDX-License-Identifier: MIT OR Apache-2.0
//! The radial–transverse–normal (RTN) frame of a state.
//!
//! RTN is the frame of a spacecraft's own position: R points along the position vector, N along
//! the orbit normal `h = r × v`, and T completes the right-handed triad, `T = N × R`. For a
//! circular orbit the velocity is purely along T.
//!
//! Not written: LVLH. Its axis conventions differ between users (some take the along-track axis
//! as x, others the velocity direction), so it needs a choice made explicitly. Not written
//! either: heliocentric ecliptic and Earth-fixed frames. The first needs a heliocentric origin,
//! and the second needs UT1.

use crate::elements::StateVector;
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
