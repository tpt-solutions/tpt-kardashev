// SPDX-License-Identifier: MIT OR Apache-2.0
//! An exponential model of the upper atmosphere, for atmospheric drag.
//!
//! Density is piecewise exponential in altitude: `ρ(h) = ρ_i·exp(−(h − h_i)/H_i)`, where row `i`
//! is the highest tabulated altitude `h_i` not above `h`. The table lists reference densities and
//! scale heights from 0 to 1000 km. Above 1000 km the density is taken as zero, which understates
//! drag at the top of the table's range and is stated in the README.
//!
//! The table values are the standard exponential reference set, as recalled; they should be checked
//! against their source before release (see the provenance file). The model is not a
//! solar-activity-dependent one, so it does not vary with the solar cycle.

/// Reference rows `(altitude km, density kg/m³, scale height km)`, from the standard exponential
/// table.
const TABLE: [(f64, f64, f64); 27] = [
    (0.0, 1.225, 7.249),
    (25.0, 3.899e-2, 6.349),
    (30.0, 1.774e-2, 6.682),
    (40.0, 3.972e-3, 7.554),
    (50.0, 1.057e-3, 8.382),
    (60.0, 3.206e-4, 7.714),
    (70.0, 8.770e-5, 6.549),
    (80.0, 1.905e-5, 5.799),
    (90.0, 3.396e-6, 5.382),
    (100.0, 5.297e-7, 5.877),
    (110.0, 9.661e-8, 7.263),
    (120.0, 2.438e-8, 9.473),
    (130.0, 8.484e-9, 12.636),
    (140.0, 3.845e-9, 16.149),
    (150.0, 2.070e-9, 22.523),
    (180.0, 5.464e-10, 29.740),
    (200.0, 2.789e-10, 37.105),
    (250.0, 7.248e-11, 45.546),
    (300.0, 2.418e-11, 53.628),
    (350.0, 9.518e-12, 53.298),
    (400.0, 3.725e-12, 58.515),
    (450.0, 1.585e-12, 60.828),
    (500.0, 6.967e-13, 63.822),
    (600.0, 1.454e-13, 71.835),
    (700.0, 3.614e-14, 88.667),
    (800.0, 1.170e-14, 124.64),
    (900.0, 5.245e-15, 181.05),
];

/// The top of the table, in km. Above it the density is taken as zero.
pub const TOP_KM: f64 = 1000.0;

/// The air density (kg/m³) at `altitude` km above the reference surface. Altitudes below zero use
/// the surface row; altitudes above [`TOP_KM`] give zero.
pub fn density(altitude: f64) -> f64 {
    if altitude > TOP_KM {
        return 0.0;
    }
    let h = altitude.max(0.0);
    // The last row whose altitude is not above h. The table starts at 0 km, so one exists.
    let (h_i, rho_i, scale) = TABLE
        .iter()
        .rev()
        .find(|(h_row, _, _)| *h_row <= h)
        .copied()
        .unwrap_or(TABLE[0]);
    rho_i * (-(h - h_i) / scale).exp()
}
