// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ground-station visibility: a satellite's elevation above a station, and the windows in which it
//! is above a minimum elevation.
//!
//! A station is a point on the WGS 84 ellipsoid, given in geodetic latitude, longitude and
//! altitude. Its position and its local vertical are taken in the Earth-fixed frame and turned into
//! the inertial frame by the Earth's rotation, which the caller starts with an angle at the first
//! instant ([`EarthRotation`]). Using that angle in place of Greenwich sidereal time avoids UT1,
//! which the crate does not have: the angle is an input, and so is its relation to the calendar.
//!
//! Windows are found by sampling the elevation at a fixed step and refining each change of state
//! by bisection to within a millisecond. A window shorter than the step can be missed, so the step
//! should be well under the shortest pass of interest.
//!
//! Not modelled: atmospheric refraction, terrain masking, and the station's motion (plate tectonics
//! and tides).

use crate::force::{EARTH_ROTATION_RATE_RAD_S, WGS84_EQUATORIAL_RADIUS_KM};
use crate::{Error, Radians, Seconds, StateVector, Vec3};
use std::vec::Vec;

/// The square of the first eccentricity of the WGS 84 ellipsoid (WGS 84).
pub const WGS84_ECCENTRICITY_SQ: f64 = 0.006_694_379_990_14;

/// Bisection stops when a change of state is bracketed within this many seconds.
const EDGE_TOLERANCE_S: f64 = 1e-3;
/// Bisection steps allowed: enough to reach `EDGE_TOLERANCE_S` from any step.
const MAX_BISECTIONS: usize = 80;

/// A ground station on the WGS 84 ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundStation {
    latitude: f64,
    longitude: f64,
    altitude: f64,
}

impl GroundStation {
    /// A station at geodetic `latitude` and `longitude`, and `altitude` km above the ellipsoid.
    /// Fails with [`Error::OutOfRange`] for a latitude beyond ±90°, and [`Error::NonFinite`] for a
    /// non-finite altitude.
    pub fn new(latitude: Radians, longitude: Radians, altitude: f64) -> Result<Self, Error> {
        if latitude.get().abs() > core::f64::consts::FRAC_PI_2 {
            return Err(Error::OutOfRange { what: "latitude" });
        }
        if !altitude.is_finite() {
            return Err(Error::NonFinite {
                what: "station altitude (km)",
            });
        }
        Ok(Self {
            latitude: latitude.get(),
            longitude: longitude.get(),
            altitude,
        })
    }

    /// The station's position in the Earth-fixed frame, in km.
    pub fn ecef(&self) -> Vec3 {
        let (sin_lat, cos_lat) = self.latitude.sin_cos();
        let (sin_lon, cos_lon) = self.longitude.sin_cos();
        let n =
            WGS84_EQUATORIAL_RADIUS_KM / (1.0 - WGS84_ECCENTRICITY_SQ * sin_lat * sin_lat).sqrt();
        let h = self.altitude;
        Vec3::new(
            (n + h) * cos_lat * cos_lon,
            (n + h) * cos_lat * sin_lon,
            (n * (1.0 - WGS84_ECCENTRICITY_SQ) + h) * sin_lat,
        )
    }

    /// The local vertical (unit) at the station, in the Earth-fixed frame.
    pub fn vertical_ecef(&self) -> Vec3 {
        let (sin_lat, cos_lat) = self.latitude.sin_cos();
        let (sin_lon, cos_lon) = self.longitude.sin_cos();
        Vec3::new(cos_lat * cos_lon, cos_lat * sin_lon, sin_lat)
    }
}

/// The Earth's rotation: the angle of the Earth-fixed x axis from the inertial x axis at the first
/// instant, advancing at the WGS 84 rotation rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EarthRotation {
    angle_at_start: f64,
}

impl EarthRotation {
    /// Rotation that starts at `angle_at_start` (the Earth-fixed x axis's inertial angle at `t = 0`).
    pub fn new(angle_at_start: Radians) -> Self {
        Self {
            angle_at_start: angle_at_start.get(),
        }
    }

    /// The Earth-fixed vector `v` turned into the inertial frame at `t` seconds past the start.
    pub fn to_inertial(&self, t: f64, v: Vec3) -> Vec3 {
        let angle = self.angle_at_start + EARTH_ROTATION_RATE_RAD_S * t;
        let (s, c) = angle.sin_cos();
        Vec3::new(c * v.x - s * v.y, s * v.x + c * v.y, v.z)
    }
}

/// The elevation of `satellite` (inertial, km) above the station's horizon at `t` seconds past the
/// start, in radians: positive above the horizon, zero on it, negative below.
///
/// Fails with [`Error::Degenerate`] if the satellite is at the station.
pub fn elevation(
    station: &GroundStation,
    rotation: &EarthRotation,
    t: f64,
    satellite: Vec3,
) -> Result<f64, Error> {
    let at_station = rotation.to_inertial(t, station.ecef());
    let vertical = rotation.to_inertial(t, station.vertical_ecef());
    let line_of_sight = satellite - at_station;
    let distance = line_of_sight.norm();
    if distance.is_nan() || distance <= 0.0 {
        return Err(Error::Degenerate);
    }
    Ok((line_of_sight.dot(vertical) / distance)
        .clamp(-1.0, 1.0)
        .asin())
}

/// The windows in `[0, duration]` in which the satellite is at least `min_elevation` above the
/// horizon. `satellite(t)` gives its inertial state `t` seconds past the start; each window is a
/// `(start, end)` pair, with `start` at 0 if the satellite is already visible.
///
/// Fails with [`Error::InvalidWindow`] for a negative duration, [`Error::InvalidStep`] for a step
/// that is not positive and finite, and [`Error::NonFinite`] for a non-finite elevation limit.
/// Errors from `satellite` are returned as they arise.
pub fn contact_windows<F>(
    station: &GroundStation,
    rotation: &EarthRotation,
    min_elevation: Radians,
    satellite: F,
    duration: Seconds,
    step: Seconds,
) -> Result<Vec<(Seconds, Seconds)>, Error>
where
    F: Fn(Seconds) -> Result<StateVector, Error>,
{
    let total = duration.get();
    let h = step.get();
    if total < 0.0 {
        return Err(Error::InvalidWindow);
    }
    if !(h.is_finite() && h > 0.0) {
        return Err(Error::InvalidStep);
    }
    let limit = min_elevation.get();
    if !limit.is_finite() {
        return Err(Error::NonFinite {
            what: "minimum elevation",
        });
    }
    let visible_at = |t: f64| -> Result<bool, Error> {
        let state = satellite(Seconds::new(t)?)?;
        Ok(elevation(station, rotation, t, state.position)? >= limit)
    };

    let mut windows = Vec::new();
    let mut start: Option<f64> = None;
    let mut previous_t = 0.0;
    let mut previous = visible_at(0.0)?;
    if previous {
        start = Some(0.0);
    }
    while previous_t < total {
        let t = (previous_t + h).min(total);
        let current = visible_at(t)?;
        if current != previous {
            // Bisect for the instant the state changes, keeping `lo` on the previous state.
            let (mut lo, mut hi) = (previous_t, t);
            for _ in 0..MAX_BISECTIONS {
                if hi - lo <= EDGE_TOLERANCE_S {
                    break;
                }
                let mid = 0.5 * (lo + hi);
                if visible_at(mid)? == previous {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let edge = 0.5 * (lo + hi);
            if current {
                start = Some(edge);
            } else if let Some(open) = start.take() {
                push_window(&mut windows, open, edge)?;
            }
        }
        previous = current;
        previous_t = t;
    }
    if let Some(open) = start {
        push_window(&mut windows, open, total)?;
    }
    Ok(windows)
}

/// Adds the window `[open, close]`, unless it is empty.
fn push_window(windows: &mut Vec<(Seconds, Seconds)>, open: f64, close: f64) -> Result<(), Error> {
    if close > open {
        windows.push((Seconds::new(open)?, Seconds::new(close)?));
    }
    Ok(())
}
