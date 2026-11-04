// SPDX-License-Identifier: MIT OR Apache-2.0
//! Net GPS clock offset from gravity and motion, against a static ground clock.
//!
//! Run with `cargo run -p tpt-kardashev-relativity --example gps_clock`.

use tpt_kardashev_relativity::Error;
use tpt_kardashev_relativity::gravity::weak_field_clock_rate;
use tpt_kardashev_relativity::units::SECONDS_PER_DAY;

const EARTH_GM: f64 = 3.986_004_418e14;
const EARTH_RADIUS_M: f64 = 6.371e6;
const GPS_SEMI_MAJOR_AXIS_M: f64 = 2.656_175e7;

fn main() -> Result<(), Error> {
    let v_gps = (EARTH_GM / GPS_SEMI_MAJOR_AXIS_M).sqrt();

    let gps = weak_field_clock_rate(EARTH_GM, GPS_SEMI_MAJOR_AXIS_M, v_gps)?;
    let ground = weak_field_clock_rate(EARTH_GM, EARTH_RADIUS_M, 0.0)?;

    let offset_us_per_day = (gps - ground) * SECONDS_PER_DAY * 1.0e6;
    println!("GPS orbital speed: {v_gps:.1} m/s");
    println!("Net GPS clock offset: {offset_us_per_day:+.2} us/day (GPS runs fast)");
    println!(
        "Published figure is about +38 us/day; this model omits Earth rotation and eccentricity."
    );
    Ok(())
}
