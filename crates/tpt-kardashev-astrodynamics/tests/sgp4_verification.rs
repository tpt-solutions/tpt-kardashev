// SPDX-License-Identifier: MIT OR Apache-2.0
//! SGP4 against further near-earth cases of the verification set of Vallado, Crawford, Hujsak and
//! Kelso (AIAA 2006-6753), under WGS-72: a normal-drag case, a low-eccentricity case, the
//! simplified-drag branches for perigees under 220 km, and the sub-orbital and decaying cases, which
//! the paper says must be trapped. The printed states are the paper's; they are cited, not copied
//! from a data file.
#![cfg(feature = "sgp4")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::Error;
use tpt_kardashev_astrodynamics::sgp4::{GravityConstants, Sgp4, parse_tle};

/// One case: its two lines, and rows of (minutes from epoch, position km, velocity km/s).
struct Case {
    line1: &'static str,
    line2: &'static str,
    rows: &'static [(f64, [f64; 6])],
}

const CASE_06251: Case = Case {
    line1: "1 06251U 62025E   06176.82412014   .00008885   00000-0   12808-3 0   3985",
    line2: "2 06251   58.0579   54.0425 0030035 139.1568 221.1854 15.56387291   6774   0.0   2880.0   120.00",
    rows: &[
        (
            0.0,
            [
                3988.31022699,
                5498.96657235,
                0.90055879,
                -3.290032738,
                2.357652820,
                6.496623475,
            ],
        ),
        (
            120.0,
            [
                -3935.69800083,
                409.10980837,
                5471.33577327,
                -3.374784183,
                -6.635211043,
                -1.942056221,
            ],
        ),
        (
            480.0,
            [
                -1115.07959514,
                4015.11691491,
                5326.99727718,
                -5.524279443,
                -4.765738774,
                2.402255961,
            ],
        ),
        (
            1440.0,
            [
                -2777.14682335,
                -5663.16031708,
                -2462.54889123,
                4.915493146,
                0.123328992,
                -5.896495091,
            ],
        ),
        (
            2880.0,
            [
                1159.27802897,
                5056.60175495,
                4353.49418579,
                -5.968060341,
                -2.314790406,
                4.230722669,
            ],
        ),
    ],
};

const CASE_28057: Case = Case {
    line1: "1 28057U 03049A   06177.78615833   .00000060   00000-0   35940-4 0   1836",
    line2: "2 28057   98.4283 247.6961 0000884   88.1964 271.9322 14.35478080140550   0.0   2880.0   120.00",
    rows: &[
        (
            0.0,
            [
                -2715.28237486,
                -6619.26436889,
                -0.01341443,
                -1.008587273,
                0.422782003,
                7.385272942,
            ],
        ),
        (
            1440.0,
            [
                688.16056594,
                4124.87618964,
                5794.55994449,
                2.810973665,
                5.479585563,
                -4.224866316,
            ],
        ),
        (
            2880.0,
            [
                1788.42334580,
                1990.50530957,
                -6640.59337725,
                -2.074169091,
                -6.683381288,
                -2.562777776,
            ],
        ),
    ],
};

const CASE_28350: Case = Case {
    line1: "1 28350U 04020A   06167.21788666   .16154492   76267-5   18678-3 0   8894",
    line2: "2 28350   64.9977 345.6130 0024870 260.7578   99.9590 16.47856722116490   0.0   2880.0   120.00",
    rows: &[
        (
            0.0,
            [
                6333.08123128,
                -1580.82852326,
                90.69355720,
                0.714634423,
                3.224246550,
                7.083128132,
            ],
        ),
        (
            720.0,
            [
                -446.42460916,
                2932.28872588,
                5759.19389757,
                -7.561000245,
                1.550975493,
                -1.374970885,
            ],
        ),
        (
            1440.0,
            [
                -4527.90871828,
                -723.29199041,
                -4527.44608319,
                5.121674217,
                -3.909895427,
                -4.500218556,
            ],
        ),
    ],
};

const CASE_22312: Case = Case {
    line1: "1 22312U 93002D   06094.46235912   .99999999   81888-5   49949-3 0   3953",
    line2: "2 22312   62.1486   77.4698 0308723 267.9229   88.7392 15.95744531 98783   54.2028672   1440.0   20.00",
    rows: &[
        (
            0.0,
            [
                1442.10132912,
                6510.23625449,
                8.83145885,
                -3.475714837,
                0.997262768,
                6.835860345,
            ],
        ),
        (
            54.20286720,
            [
                306.10478453,
                -5816.45655525,
                -2979.55846068,
                3.950663855,
                3.415332543,
                -5.879974329,
            ],
        ),
        (
            474.20286720,
            [
                -3181.54698042,
                -3831.29976506,
                4096.80242787,
                1.114159970,
                -6.104773578,
                -4.829967400,
            ],
        ),
    ],
};

const CASE_29238: Case = Case {
    line1: "1 29238U 06022G   06177.28732010   .00766286   10823-4   13334-2 0   101",
    line2: "2 29238   51.5595 213.7903 0202579   95.2503 267.9010 15.73823839   1061   0.0   1440.0   120.00",
    rows: &[
        (
            0.0,
            [
                -5566.59512819,
                -3789.75991159,
                67.60382245,
                2.873759367,
                -3.825340523,
                6.023253926,
            ],
        ),
        (
            720.0,
            [
                -5776.81371622,
                -118.64155319,
                -3641.22052418,
                -2.539917207,
                -5.622701582,
                4.403125405,
            ],
        ),
        (
            1440.0,
            [
                -2629.55011449,
                3400.98040158,
                -5344.38217129,
                -6.368548448,
                -3.998963509,
                0.577253064,
            ],
        ),
    ],
};

fn model(case: &Case) -> Sgp4 {
    let elements = parse_tle(case.line1, case.line2).expect("parses");
    Sgp4::new(GravityConstants::WGS72, elements).expect("initializes")
}

fn assert_rows(case: &Case) {
    let sgp4 = model(case);
    for (minutes, expected) in case.rows {
        let state = sgp4.propagate(*minutes).expect("propagates");
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
fn normal_drag_case_06251_matches_the_paper() {
    assert_rows(&CASE_06251);
}

#[test]
fn low_eccentricity_case_28057_matches_the_paper() {
    assert_rows(&CASE_28057);
}

#[test]
fn low_perigee_case_28350_matches_the_paper() {
    // Perigee 127 km: the drag branch for perigees under 156 km.
    assert_rows(&CASE_28350);
}

#[test]
fn very_low_perigee_case_22312_matches_the_paper() {
    // Perigee 87 km: the drag branch for perigees under 98 km.
    assert_rows(&CASE_22312);
}

#[test]
fn simplified_drag_case_29238_matches_the_paper() {
    // Perigee 212 km: the simplified drag equations for perigees under 220 km.
    assert_rows(&CASE_29238);
}

#[test]
fn the_paper_says_two_low_perigee_cases_must_trap_eccentricity_too_low() {
    // 28350 is trapped beyond about 1460 minutes, and 22312 beyond about 2840 minutes.
    let s = model(&CASE_28350);
    assert!(matches!(s.propagate(1500.0), Err(Error::Sgp4 { .. })));
    let s = model(&CASE_22312);
    assert!(matches!(s.propagate(2880.0), Err(Error::Sgp4 { .. })));
}

#[test]
fn the_sub_orbital_case_is_refused_at_initialization() {
    // 28872 has a perigee of about −51 km: the paper uses it to test error handling.
    let elements = parse_tle(
        "1 28872U 05037B   05333.02012661   .25992681   00000-0   24476-3 0   1534",
        "2 28872   96.4736 157.9986 0303955 244.0492 110.6523 16.46015938 10708   0.0   50.0   5.00",
    )
    .expect("parses");
    assert!(matches!(
        Sgp4::new(GravityConstants::WGS72, elements),
        Err(Error::Sgp4 { .. })
    ));
}

#[test]
fn the_decaying_case_is_trapped_before_440_minutes() {
    // 29141 is in its last stages of decay: the paper says it fails before 440 minutes.
    let s = model_from(
        "1 29141U 85108AA   06170.26783845   .99999999   00000-0   13519-0 0   718",
        "2 29141   82.4288 273.4882 0015848 277.2124   83.9133 15.93343074   6828   0.0   440.0   20.00",
    );
    assert!(matches!(s.propagate(440.0), Err(Error::Sgp4 { .. })));
}

fn model_from(line1: &str, line2: &str) -> Sgp4 {
    let elements = parse_tle(line1, line2).expect("parses");
    Sgp4::new(GravityConstants::WGS72, elements).expect("initializes")
}
