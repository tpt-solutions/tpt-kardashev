// SPDX-License-Identifier: MIT OR Apache-2.0
//! Time dilation, length contraction, relativistic Doppler and proper time along paths.

use crate::error::{Error, validate_speed};
use crate::event::{Event, IntervalKind, interval};
use crate::math::sqrt;
use crate::units::{C, Metres, Seconds};

/// Lorentz factor `gamma = 1 / sqrt(1 - v^2 / c^2)` for a speed `v` (m/s).
pub fn lorentz_gamma(speed: f64) -> Result<f64, Error> {
    let v = validate_speed(speed)?;
    let beta = v / C;
    Ok(1.0 / sqrt(1.0 - beta * beta))
}

/// Coordinate-frame duration of a clock that ticks `proper` in its own rest frame and moves at `speed`.
pub fn dilated_duration(proper: Seconds, speed: f64) -> Result<Seconds, Error> {
    Ok(Seconds(lorentz_gamma(speed)? * proper.0))
}

/// Proper duration of a moving clock that shows `coordinate` in the observer's frame.
pub fn proper_duration(coordinate: Seconds, speed: f64) -> Result<Seconds, Error> {
    Ok(Seconds(coordinate.0 / lorentz_gamma(speed)?))
}

/// Length measured in the observer's frame of a rod with rest length `rest` moving at `speed`.
pub fn contracted_length(rest: Metres, speed: f64) -> Result<Metres, Error> {
    Ok(Metres(rest.0 / lorentz_gamma(speed)?))
}

/// Ratio `f_observed / f_emitted` of the longitudinal Doppler shift.
///
/// `beta` is the radial velocity over c, positive when the source recedes. For a receding
/// source the ratio is `sqrt((1 - beta) / (1 + beta)) = exp(-eta)`, with `eta = atanh(beta)`.
pub fn doppler_ratio_radial(beta: f64) -> Result<f64, Error> {
    if !beta.is_finite() {
        return Err(Error::NonFinite);
    }
    if beta.abs() >= 1.0 {
        return Err(Error::SpeedNotSubluminal { speed: beta * C });
    }
    Ok(sqrt((1.0 - beta) / (1.0 + beta)))
}

/// Proper time along a piecewise-straight path of events, in seconds.
///
/// Each segment must be future-directed and non-spacelike. Lightlike segments contribute zero.
/// A single event has zero proper time.
pub fn proper_time(path: &[Event]) -> Result<Seconds, Error> {
    if path.is_empty() {
        return Err(Error::EmptyPath);
    }
    let mut total = 0.0;
    for pair in path.windows(2) {
        if let [a, b] = pair {
            if b.t.0 < a.t.0 {
                return Err(Error::PastDirectedSegment);
            }
            let iv = interval(a, b);
            match iv.kind() {
                IntervalKind::Spacelike => return Err(Error::SpacelikeSegment),
                IntervalKind::Lightlike => {}
                IntervalKind::Timelike => total += sqrt(iv.squared()) / C,
            }
        }
    }
    Ok(Seconds(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamma_known_values() {
        let g = lorentz_gamma(0.6 * C).expect("ok");
        assert!((g - 1.25).abs() < 1e-14);
        assert!((lorentz_gamma(0.0).expect("ok") - 1.0).abs() < 1e-15);
    }

    #[test]
    fn dilation_and_contraction_are_inverse() {
        let v = 0.8 * C;
        let dilated = dilated_duration(Seconds(10.0), v).expect("ok");
        assert!((dilated.0 - 50.0 / 3.0).abs() < 1e-12);
        let back = proper_duration(dilated, v).expect("ok");
        assert!((back.0 - 10.0).abs() < 1e-12);
        let len = contracted_length(Metres(10.0), v).expect("ok");
        assert!((len.0 - 6.0).abs() < 1e-12);
    }

    #[test]
    fn doppler_equals_exp_neg_rapidity() {
        // exp(-atanh(beta)) = sqrt((1-beta)/(1+beta)) for every |beta| < 1.
        for &beta in &[-0.9, -0.2, 0.0, 0.3, 0.75] {
            let ratio = doppler_ratio_radial(beta).expect("ok");
            let eta = 0.5 * crate::math::ln((1.0 + beta) / (1.0 - beta));
            assert!(
                (ratio - crate::math::exp(-eta)).abs() < 1e-14,
                "beta {beta}"
            );
        }
        assert!(doppler_ratio_radial(1.0).is_err());
    }

    #[test]
    fn empty_and_invalid_paths() {
        assert_eq!(proper_time(&[]), Err(Error::EmptyPath));
    }
}
