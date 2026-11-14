// SPDX-License-Identifier: MIT OR Apache-2.0
//! The deep-space terms of SGP4 (SDP4), for orbits of 225 minutes or more: lunar and solar
//! long-period terms, resonance coefficients for 12- and 24-hour orbits, and the resonance
//! integrator. Written from the equations of AIAA 2006-6753; the names follow the report.
//!
//! The report's resonance integrator keeps its state between calls. Here each propagation starts
//! from the epoch and steps to the requested time on the same 720-minute grid. That gives the same
//! values without depending on the order of earlier calls.

use core::f64::consts::PI;

const TWO_PI: f64 = 2.0 * PI;
/// Earth's rotation rate in rad/min, as the report uses it.
const RPTIM: f64 = 4.375_269_088_011_3e-3;
/// Integrator step (minutes) and its Euler-Maclaurin second-order weight.
const STEP: f64 = 720.0;
const STEP2: f64 = 259_200.0;

/// The lunar-solar terms of the epoch (`dscom`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dscom {
    pub(crate) sinim: f64,
    pub(crate) cosim: f64,
    pub(crate) emsq: f64,
    pub(crate) e3: f64,
    pub(crate) ee2: f64,
    pub(crate) se2: f64,
    pub(crate) se3: f64,
    pub(crate) sgh2: f64,
    pub(crate) sgh3: f64,
    pub(crate) sgh4: f64,
    pub(crate) sh2: f64,
    pub(crate) sh3: f64,
    pub(crate) si2: f64,
    pub(crate) si3: f64,
    pub(crate) sl2: f64,
    pub(crate) sl3: f64,
    pub(crate) sl4: f64,
    pub(crate) xgh2: f64,
    pub(crate) xgh3: f64,
    pub(crate) xgh4: f64,
    pub(crate) xh2: f64,
    pub(crate) xh3: f64,
    pub(crate) xi2: f64,
    pub(crate) xi3: f64,
    pub(crate) xl2: f64,
    pub(crate) xl3: f64,
    pub(crate) xl4: f64,
    pub(crate) zmol: f64,
    pub(crate) zmos: f64,
    pub(crate) s1: f64,
    pub(crate) s2: f64,
    pub(crate) s3: f64,
    pub(crate) s4: f64,
    pub(crate) s5: f64,
    pub(crate) ss1: f64,
    pub(crate) ss2: f64,
    pub(crate) ss3: f64,
    pub(crate) ss4: f64,
    pub(crate) ss5: f64,
    pub(crate) sz1: f64,
    pub(crate) sz3: f64,
    pub(crate) sz11: f64,
    pub(crate) sz13: f64,
    pub(crate) sz21: f64,
    pub(crate) sz23: f64,
    pub(crate) sz31: f64,
    pub(crate) sz33: f64,
    pub(crate) z1: f64,
    pub(crate) z3: f64,
    pub(crate) z11: f64,
    pub(crate) z13: f64,
    pub(crate) z21: f64,
    pub(crate) z23: f64,
    pub(crate) z31: f64,
    pub(crate) z33: f64,
    pub(crate) nm: f64,
}

/// The lunar-solar terms at the epoch. `epoch` is days from 1950 (the report's convention), `ep`
/// the eccentricity, `argpp` the argument of perigee, `inclp` the inclination, `nodep` the node,
/// and `np` the mean motion (rad/min).
pub(crate) fn dscom(epoch: f64, ep: f64, argpp: f64, inclp: f64, nodep: f64, np: f64) -> Dscom {
    const ZES: f64 = 0.01675;
    const ZEL: f64 = 0.05490;
    const C1SS: f64 = 2.986_479_7e-6;
    const C1L: f64 = 4.796_806_5e-7;
    const ZSINIS: f64 = 0.397_854_16;
    const ZCOSIS: f64 = 0.917_448_67;
    const ZCOSGS: f64 = 0.194_590_5;
    const ZSINGS: f64 = -0.980_884_58;

    let nm = np;
    let em = ep;
    let snodm = nodep.sin();
    let cnodm = nodep.cos();
    let sinomm = argpp.sin();
    let cosomm = argpp.cos();
    let sinim = inclp.sin();
    let cosim = inclp.cos();
    let emsq = em * em;
    let betasq = 1.0 - emsq;
    let rtemsq = betasq.sqrt();

    // The lunar node, inclination and longitude at this epoch (the report's day count, tc = 0).
    let day = epoch + 18_261.5;
    let xnodce = (4.523_602_0 - 9.242_202_9e-4 * day) % TWO_PI;
    let stem = xnodce.sin();
    let ctem = xnodce.cos();
    let zcosil = 0.913_751_64 - 0.035_680_96 * ctem;
    let zsinil = (1.0 - zcosil * zcosil).sqrt();
    let zsinhl = 0.089_683_511 * stem / zsinil;
    let zcoshl = (1.0 - zsinhl * zsinhl).sqrt();
    let gam = 5.835_151_4 + 0.001_944_368_0 * day;
    let mut zx = 0.397_854_16 * stem / zsinil;
    let zy = zcoshl * ctem + 0.917_448_67 * zsinhl * stem;
    zx = zx.atan2(zy);
    zx = gam + zx - xnodce;
    let zcosgl = zx.cos();
    let zsingl = zx.sin();

    // Two passes: the solar terms first, then the lunar ones. Each pass yields s1..s7 and z terms.
    let (mut zcosg, mut zsing, mut zcosi, mut zsini) = (ZCOSGS, ZSINGS, ZCOSIS, ZSINIS);
    let (mut zcosh, mut zsinh) = (cnodm, snodm);
    let mut cc = C1SS;
    let xnoi = 1.0 / nm;
    let mut solar_s = [0.0f64; 8];
    let mut solar_z = [0.0f64; 13];
    let mut s = [0.0f64; 8];
    let mut z = [0.0f64; 13];
    for pass in 1..=2 {
        let a1 = zcosg * zcosh + zsing * zcosi * zsinh;
        let a3 = -zsing * zcosh + zcosg * zcosi * zsinh;
        let a7 = -zcosg * zsinh + zsing * zcosi * zcosh;
        let a8 = zsing * zsini;
        let a9 = zsing * zsinh + zcosg * zcosi * zcosh;
        let a10 = zcosg * zsini;
        let a2 = cosim * a7 + sinim * a8;
        let a4 = cosim * a9 + sinim * a10;
        let a5 = -sinim * a7 + cosim * a8;
        let a6 = -sinim * a9 + cosim * a10;
        let x1 = a1 * cosomm + a2 * sinomm;
        let x2 = a3 * cosomm + a4 * sinomm;
        let x3 = -a1 * sinomm + a2 * cosomm;
        let x4 = -a3 * sinomm + a4 * cosomm;
        let x5 = a5 * sinomm;
        let x6 = a6 * sinomm;
        let x7 = a5 * cosomm;
        let x8 = a6 * cosomm;
        let z31 = 12.0 * x1 * x1 - 3.0 * x3 * x3;
        let z32 = 24.0 * x1 * x2 - 6.0 * x3 * x4;
        let z33 = 12.0 * x2 * x2 - 3.0 * x4 * x4;
        let mut z1 = 3.0 * (a1 * a1 + a2 * a2) + z31 * emsq;
        let mut z2 = 6.0 * (a1 * a3 + a2 * a4) + z32 * emsq;
        let mut z3 = 3.0 * (a3 * a3 + a4 * a4) + z33 * emsq;
        let z11 = -6.0 * a1 * a5 + emsq * (-24.0 * x1 * x7 - 6.0 * x3 * x5);
        let z12 = -6.0 * (a1 * a6 + a3 * a5)
            + emsq * (-24.0 * (x2 * x7 + x1 * x8) - 6.0 * (x3 * x6 + x4 * x5));
        let z13 = -6.0 * a3 * a6 + emsq * (-24.0 * x2 * x8 - 6.0 * x4 * x6);
        let z21 = 6.0 * a2 * a5 + emsq * (24.0 * x1 * x5 - 6.0 * x3 * x7);
        let z22 = 6.0 * (a4 * a5 + a2 * a6)
            + emsq * (24.0 * (x2 * x5 + x1 * x6) - 6.0 * (x4 * x7 + x3 * x8));
        let z23 = 6.0 * a4 * a6 + emsq * (24.0 * x2 * x6 - 6.0 * x4 * x8);
        z1 = z1 + z1 + betasq * z31;
        z2 = z2 + z2 + betasq * z32;
        z3 = z3 + z3 + betasq * z33;
        let s3 = cc * xnoi;
        let s2 = -0.5 * s3 / rtemsq;
        let s4 = s3 * rtemsq;
        let s1 = -15.0 * em * s4;
        let s5 = x1 * x3 + x2 * x4;
        let s6 = x2 * x3 + x1 * x4;
        let s7 = x2 * x4 - x1 * x3;
        s = [0.0, s1, s2, s3, s4, s5, s6, s7];
        // z[k] for k = 1..=12 is z1, z2, z3, z11, z12, z13, z21, z22, z23, z31, z32, z33.
        z = [0.0, z1, z2, z3, z11, z12, z13, z21, z22, z23, z31, z32, z33];
        if pass == 1 {
            solar_s = s;
            solar_z = z;
            zcosg = zcosgl;
            zsing = zsingl;
            zcosi = zcosil;
            zsini = zsinil;
            zcosh = zcoshl * cnodm + zsinhl * snodm;
            zsinh = snodm * zcoshl - cnodm * zsinhl;
            cc = C1L;
        }
    }
    let zmol = (4.719_967_2 + 0.229_971_50 * day - gam) % TWO_PI;
    let zmos = (6.256_583_7 + 0.017_201_977 * day) % TWO_PI;

    // Solar copies (ss, sz) and lunar values (s, z).
    let (ss1, ss2, ss3, ss4, ss5, ss6, ss7) = (
        solar_s[1], solar_s[2], solar_s[3], solar_s[4], solar_s[5], solar_s[6], solar_s[7],
    );
    let (sz1, sz2, sz3) = (solar_z[1], solar_z[2], solar_z[3]);
    let (sz11, sz12, sz13) = (solar_z[4], solar_z[5], solar_z[6]);
    let (sz21, sz22, sz23) = (solar_z[7], solar_z[8], solar_z[9]);
    let (sz31, sz32, sz33) = (solar_z[10], solar_z[11], solar_z[12]);
    let (s1, s2, s3, s4, s5, s6, s7) = (s[1], s[2], s[3], s[4], s[5], s[6], s[7]);
    let (z1, z2, z3) = (z[1], z[2], z[3]);
    let (z11, z12, z13) = (z[4], z[5], z[6]);
    let (z21, z22, z23) = (z[7], z[8], z[9]);
    let (z31, z32, z33) = (z[10], z[11], z[12]);

    // Solar long-period terms.
    let se2 = 2.0 * ss1 * ss6;
    let se3 = 2.0 * ss1 * ss7;
    let si2 = 2.0 * ss2 * sz12;
    let si3 = 2.0 * ss2 * (sz13 - sz11);
    let sl2 = -2.0 * ss3 * sz2;
    let sl3 = -2.0 * ss3 * (sz3 - sz1);
    let sl4 = -2.0 * ss3 * (-21.0 - 9.0 * emsq) * ZES;
    let sgh2 = 2.0 * ss4 * sz32;
    let sgh3 = 2.0 * ss4 * (sz33 - sz31);
    let sgh4 = -18.0 * ss4 * ZES;
    let sh2 = -2.0 * ss2 * sz22;
    let sh3 = -2.0 * ss2 * (sz23 - sz21);
    // Lunar long-period terms.
    let ee2 = 2.0 * s1 * s6;
    let e3 = 2.0 * s1 * s7;
    let xi2 = 2.0 * s2 * z12;
    let xi3 = 2.0 * s2 * (z13 - z11);
    let xl2 = -2.0 * s3 * z2;
    let xl3 = -2.0 * s3 * (z3 - z1);
    let xl4 = -2.0 * s3 * (-21.0 - 9.0 * emsq) * ZEL;
    let xgh2 = 2.0 * s4 * z32;
    let xgh3 = 2.0 * s4 * (z33 - z31);
    let xgh4 = -18.0 * s4 * ZEL;
    let xh2 = -2.0 * s2 * z22;
    let xh3 = -2.0 * s2 * (z23 - z21);

    Dscom {
        sinim,
        cosim,
        emsq,
        e3,
        ee2,
        se2,
        se3,
        sgh2,
        sgh3,
        sgh4,
        sh2,
        sh3,
        si2,
        si3,
        sl2,
        sl3,
        sl4,
        xgh2,
        xgh3,
        xgh4,
        xh2,
        xh3,
        xi2,
        xi3,
        xl2,
        xl3,
        xl4,
        zmol,
        zmos,
        s1,
        s2,
        s3,
        s4,
        s5,
        ss1,
        ss2,
        ss3,
        ss4,
        ss5,
        sz1,
        sz3,
        sz11,
        sz13,
        sz21,
        sz23,
        sz31,
        sz33,
        z1,
        z3,
        z11,
        z13,
        z21,
        z23,
        z31,
        z33,
        nm,
    }
}

/// The elements the deep-space initialization and integration take.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EpochElements {
    /// Eccentricity.
    pub(crate) ecco: f64,
    /// Inclination (rad).
    pub(crate) inclo: f64,
    /// Node (rad).
    pub(crate) nodeo: f64,
    /// Argument of perigee (rad).
    pub(crate) argpo: f64,
    /// Mean anomaly (rad).
    pub(crate) mo: f64,
    /// Mean motion (rad/min), after the un-Kozai correction.
    pub(crate) no: f64,
    /// Secular rates from the near-earth model (rad/min, rad/min²).
    pub(crate) mdot: f64,
    pub(crate) argpdot: f64,
    pub(crate) nodedot: f64,
    /// Argument-of-perigee rate plus node rate (rad/min).
    pub(crate) xpidot: f64,
    /// Greenwich sidereal time at the epoch (rad).
    pub(crate) gsto: f64,
    /// Canonical mean-motion unit, 60/√(R³/μ).
    pub(crate) xke: f64,
}

/// The resonance and secular rates fixed at the epoch (`dsinit`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DsInit {
    /// Resonance type: 0 none, 1 synchronous (about one day), 2 half-day (about 12 hours).
    pub(crate) irez: u8,
    pub(crate) d2201: f64,
    pub(crate) d2211: f64,
    pub(crate) d3210: f64,
    pub(crate) d3222: f64,
    pub(crate) d4410: f64,
    pub(crate) d4422: f64,
    pub(crate) d5220: f64,
    pub(crate) d5232: f64,
    pub(crate) d5421: f64,
    pub(crate) d5433: f64,
    pub(crate) dedt: f64,
    pub(crate) didt: f64,
    pub(crate) dmdt: f64,
    pub(crate) dnodt: f64,
    pub(crate) domdt: f64,
    pub(crate) del1: f64,
    pub(crate) del2: f64,
    pub(crate) del3: f64,
    pub(crate) xfact: f64,
    pub(crate) xlamo: f64,
}

/// The deep-space initialization at the epoch (`dsinit`).
pub(crate) fn dsinit(d: &Dscom, e: &EpochElements) -> DsInit {
    const Q22: f64 = 1.789_167_9e-6;
    const Q31: f64 = 2.146_074_8e-6;
    const Q33: f64 = 2.212_301_5e-7;
    const ROOT22: f64 = 1.789_167_9e-6;
    const ROOT44: f64 = 7.363_695_3e-9;
    const ROOT54: f64 = 2.176_580_3e-9;
    const ROOT32: f64 = 3.739_379_2e-7;
    const ROOT52: f64 = 1.142_863_9e-7;
    const ZNL: f64 = 1.583_521_8e-4;
    const ZNS: f64 = 1.194_59e-5;
    let x2o3 = 2.0 / 3.0;
    let nm = d.nm;
    let em = e.ecco;
    let sinim = d.sinim;
    let cosim = d.cosim;
    let emsq = d.emsq;
    let inclm = e.inclo;
    let theta = e.gsto % TWO_PI;

    let mut irez = 0u8;
    if nm < 0.005_235_987_7 && nm > 0.003_490_658_5 {
        irez = 1;
    }
    if (8.26e-3..=9.24e-3).contains(&nm) && em >= 0.5 {
        irez = 2;
    }

    // Solar terms.
    let ses = d.ss1 * ZNS * d.ss5;
    let sis = d.ss2 * ZNS * (d.sz11 + d.sz13);
    let sls = -ZNS * d.ss3 * (d.sz1 + d.sz3 - 14.0 - 6.0 * emsq);
    let sghs = d.ss4 * ZNS * (d.sz31 + d.sz33 - 6.0);
    let mut shs = -ZNS * d.ss2 * (d.sz21 + d.sz23);
    if !(5.235_987_7e-2..=PI - 5.235_987_7e-2).contains(&inclm) {
        shs = 0.0;
    }
    if sinim != 0.0 {
        shs /= sinim;
    }
    let sgs = sghs - cosim * shs;

    // Lunar terms.
    let dedt = ses + d.s1 * ZNL * d.s5;
    let didt = sis + d.s2 * ZNL * (d.z11 + d.z13);
    let dmdt = sls - ZNL * d.s3 * (d.z1 + d.z3 - 14.0 - 6.0 * emsq);
    let sghl = d.s4 * ZNL * (d.z31 + d.z33 - 6.0);
    let mut shll = -ZNL * d.s2 * (d.z21 + d.z23);
    if !(5.235_987_7e-2..=PI - 5.235_987_7e-2).contains(&inclm) {
        shll = 0.0;
    }
    let mut domdt = sgs + sghl;
    let mut dnodt = shs;
    if sinim != 0.0 {
        domdt -= cosim / sinim * shll;
        dnodt += shll / sinim;
    }

    let mut out = DsInit {
        irez,
        d2201: 0.0,
        d2211: 0.0,
        d3210: 0.0,
        d3222: 0.0,
        d4410: 0.0,
        d4422: 0.0,
        d5220: 0.0,
        d5232: 0.0,
        d5421: 0.0,
        d5433: 0.0,
        dedt,
        didt,
        dmdt,
        dnodt,
        domdt,
        del1: 0.0,
        del2: 0.0,
        del3: 0.0,
        xfact: 0.0,
        xlamo: 0.0,
    };

    if irez != 0 {
        let aonv = (nm / e.xke).powf(x2o3);
        if irez == 2 {
            // Twelve-hour (half-day) resonance: the coefficients of the report.
            let cosisq = cosim * cosim;
            let eoc = em * emsq;
            let g201 = -0.306 - (em - 0.64) * 0.440;
            let (g211, g310, g322, g410, g422, g520);
            if em <= 0.65 {
                g211 = 3.616 - 13.2470 * em + 16.2900 * emsq;
                g310 = -19.302 + 117.3900 * em - 228.4190 * emsq + 156.5910 * eoc;
                g322 = -18.9068 + 109.7927 * em - 214.6334 * emsq + 146.5816 * eoc;
                g410 = -41.122 + 242.6940 * em - 471.0940 * emsq + 313.9530 * eoc;
                g422 = -146.407 + 841.8800 * em - 1629.014 * emsq + 1083.4350 * eoc;
                g520 = -532.114 + 3017.977 * em - 5740.032 * emsq + 3708.2760 * eoc;
            } else {
                g211 = -72.099 + 331.819 * em - 508.738 * emsq + 266.724 * eoc;
                g310 = -346.844 + 1582.851 * em - 2415.925 * emsq + 1246.113 * eoc;
                g322 = -342.585 + 1554.908 * em - 2366.899 * emsq + 1215.972 * eoc;
                g410 = -1052.797 + 4758.686 * em - 7193.992 * emsq + 3651.957 * eoc;
                g422 = -3581.690 + 16178.110 * em - 24462.770 * emsq + 12422.520 * eoc;
                g520 = if em > 0.715 {
                    -5149.66 + 29936.92 * em - 54087.36 * emsq + 31324.56 * eoc
                } else {
                    1464.74 - 4664.75 * em + 3763.64 * emsq
                };
            }
            let (g533, g521, g532) = if em < 0.7 {
                (
                    -919.22770 + 4988.6100 * em - 9064.7700 * emsq + 5542.21 * eoc,
                    -822.71072 + 4568.6173 * em - 8491.4146 * emsq + 5337.524 * eoc,
                    -853.66600 + 4690.2500 * em - 8624.7700 * emsq + 5341.4 * eoc,
                )
            } else {
                (
                    -37995.780 + 161616.52 * em - 229838.20 * emsq + 109377.94 * eoc,
                    -51752.104 + 218913.95 * em - 309468.16 * emsq + 146349.42 * eoc,
                    -40023.880 + 170470.89 * em - 242699.48 * emsq + 115605.82 * eoc,
                )
            };
            let sini2 = sinim * sinim;
            let f220 = 0.75 * (1.0 + 2.0 * cosim + cosisq);
            let f221 = 1.5 * sini2;
            let f321 = 1.875 * sinim * (1.0 - 2.0 * cosim - 3.0 * cosisq);
            let f322 = -1.875 * sinim * (1.0 + 2.0 * cosim - 3.0 * cosisq);
            let f441 = 35.0 * sini2 * f220;
            let f442 = 39.3750 * sini2 * sini2;
            let f522 = 9.843_75
                * sinim
                * (sini2 * (1.0 - 2.0 * cosim - 5.0 * cosisq)
                    + 0.333_333_33 * (-2.0 + 4.0 * cosim + 6.0 * cosisq));
            let f523 = sinim
                * (4.921_875_12 * sini2 * (-2.0 - 4.0 * cosim + 10.0 * cosisq)
                    + 6.562_500_12 * (1.0 + 2.0 * cosim - 3.0 * cosisq));
            let f542 = 29.531_25
                * sinim
                * (2.0 - 8.0 * cosim + cosisq * (-12.0 + 8.0 * cosim + 10.0 * cosisq));
            let f543 = 29.531_25
                * sinim
                * (-2.0 - 8.0 * cosim + cosisq * (12.0 + 8.0 * cosim - 10.0 * cosisq));
            let xno2 = nm * nm;
            let ainv2 = aonv * aonv;
            let mut temp1 = 3.0 * xno2 * ainv2;
            let mut temp = temp1 * ROOT22;
            out.d2201 = temp * f220 * g201;
            out.d2211 = temp * f221 * g211;
            temp1 *= aonv;
            temp = temp1 * ROOT32;
            out.d3210 = temp * f321 * g310;
            out.d3222 = temp * f322 * g322;
            temp1 *= aonv;
            temp = 2.0 * temp1 * ROOT44;
            out.d4410 = temp * f441 * g410;
            out.d4422 = temp * f442 * g422;
            temp1 *= aonv;
            temp = temp1 * ROOT52;
            out.d5220 = temp * f522 * g520;
            out.d5232 = temp * f523 * g532;
            temp = 2.0 * temp1 * ROOT54;
            out.d5421 = temp * f542 * g521;
            out.d5433 = temp * f543 * g533;
            out.xlamo = (e.mo + e.nodeo + e.nodeo - theta - theta) % TWO_PI;
            out.xfact = e.mdot + dmdt + 2.0 * (e.nodedot + dnodt - RPTIM) - e.no;
        }
        if irez == 1 {
            // Synchronous (one-day) resonance.
            let g200 = 1.0 + emsq * (-2.5 + 0.8125 * emsq);
            let g310 = 1.0 + 2.0 * emsq;
            let g300 = 1.0 + emsq * (-6.0 + 6.609_37 * emsq);
            let f220 = 0.75 * (1.0 + cosim) * (1.0 + cosim);
            let f311 = 0.9375 * sinim * sinim * (1.0 + 3.0 * cosim) - 0.75 * (1.0 + cosim);
            let f330 = (1.0 + cosim) * (1.0 + cosim) * (1.0 + cosim) * 1.875;
            out.del1 = 3.0 * nm * nm * aonv * aonv;
            out.del2 = 2.0 * out.del1 * f220 * g200 * Q22;
            out.del3 = 3.0 * out.del1 * f330 * g300 * Q33 * aonv;
            out.del1 = out.del1 * f311 * g310 * Q31 * aonv;
            out.xlamo = (e.mo + e.nodeo + e.argpo - theta) % TWO_PI;
            out.xfact = e.mdot + e.xpidot - RPTIM + dmdt + domdt + dnodt - e.no;
        }
    }
    // The report's dsinit also sets its integrator's start values here. This version starts the
    // integrator from the epoch on each call, so they are not kept.
    out.domdt = domdt;
    out.dnodt = dnodt;
    out
}

/// The long-period lunar and solar terms, applied at `t` minutes (`dpper` with its `init` flag
/// clear). Takes the elements `ep`, `inclp`, `nodep`, `argpp` and `mp`, and returns them corrected.
pub(crate) fn dpper(
    d: &Dscom,
    t: f64,
    ep: f64,
    inclp: f64,
    nodep: f64,
    argpp: f64,
    mp: f64,
) -> (f64, f64, f64, f64, f64) {
    const ZNS: f64 = 1.194_59e-5;
    const ZES: f64 = 0.01675;
    const ZNL: f64 = 1.583_521_8e-4;
    const ZEL: f64 = 0.05490;

    // Solar periodics at the solar anomaly.
    let zm = d.zmos + ZNS * t;
    let zf = zm + 2.0 * ZES * zm.sin();
    let sinzf = zf.sin();
    let f2 = 0.5 * sinzf * sinzf - 0.25;
    let f3 = -0.5 * sinzf * zf.cos();
    let ses = d.se2 * f2 + d.se3 * f3;
    let sis = d.si2 * f2 + d.si3 * f3;
    let sls = d.sl2 * f2 + d.sl3 * f3 + d.sl4 * sinzf;
    let sghs = d.sgh2 * f2 + d.sgh3 * f3 + d.sgh4 * sinzf;
    let shs = d.sh2 * f2 + d.sh3 * f3;

    // Lunar periodics at the lunar anomaly.
    let zm = d.zmol + ZNL * t;
    let zf = zm + 2.0 * ZEL * zm.sin();
    let sinzf = zf.sin();
    let f2 = 0.5 * sinzf * sinzf - 0.25;
    let f3 = -0.5 * sinzf * zf.cos();
    let sel = d.ee2 * f2 + d.e3 * f3;
    let sil = d.xi2 * f2 + d.xi3 * f3;
    let sll = d.xl2 * f2 + d.xl3 * f3 + d.xl4 * sinzf;
    let sghl = d.xgh2 * f2 + d.xgh3 * f3 + d.xgh4 * sinzf;
    let shll = d.xh2 * f2 + d.xh3 * f3;

    // The periodics are measured from their values at the epoch, which are zero.
    let pe = ses + sel;
    let pinc = sis + sil;
    let pl = sls + sll;
    let pgh = sghs + sghl;
    let mut ph = shs + shll;

    let ep = ep + pe;
    let inclp = inclp + pinc;
    let sinip = inclp.sin();
    let cosip = inclp.cos();
    let mut argpp = argpp;
    let mut nodep = nodep;
    let mut mp = mp;
    if inclp >= 0.2 {
        ph /= sinip;
        let pgh = pgh - cosip * ph;
        argpp += pgh;
        nodep += ph;
        mp += pl;
    } else {
        // Low inclination: the Lyddane modification, which works on the node vector.
        let sinop = nodep.sin();
        let cosop = nodep.cos();
        let alfdp = sinip * sinop + (ph * cosop + pinc * cosip * sinop);
        let betdp = sinip * cosop + (-ph * sinop + pinc * cosip * cosop);
        nodep %= TWO_PI;
        let mut xls = mp + argpp + cosip * nodep;
        let dls = pl + pgh - pinc * nodep * sinip;
        xls += dls;
        let xnoh = nodep;
        nodep = alfdp.atan2(betdp);
        if (xnoh - nodep).abs() > PI {
            if nodep < xnoh {
                nodep += TWO_PI;
            } else {
                nodep -= TWO_PI;
            }
        }
        mp += pl;
        argpp = xls - mp - cosip * nodep;
    }
    (ep, inclp, nodep, argpp, mp)
}

/// The mean anomaly, node, argument of perigee, eccentricity and mean motion at time `t`
/// minutes, with the resonance integrated from the epoch (`dspace`). Takes the secular state
/// `(em, argpm, inclm, mm, nodem, nm)` and returns it updated, together with the mean-motion rate
/// `dndt`.
pub(crate) fn dspace(
    d: &DsInit,
    e: &EpochElements,
    t: f64,
    secular: (f64, f64, f64, f64, f64, f64),
) -> (f64, f64, f64, f64, f64, f64, f64) {
    const FASX2: f64 = 0.131_309_08;
    const FASX4: f64 = 2.884_319_8;
    const FASX6: f64 = 0.374_480_87;
    const G22: f64 = 5.768_639_6;
    const G32: f64 = 0.952_408_98;
    const G44: f64 = 1.801_499_8;
    const G52: f64 = 1.050_833_0;
    const G54: f64 = 4.410_889_8;

    let (em0, argpm0, inclm0, mm0, nodem0, nm0) = secular;
    let theta = (e.gsto + t * RPTIM) % TWO_PI;
    let em = em0 + d.dedt * t;
    let inclm = inclm0 + d.didt * t;
    let argpm = argpm0 + d.domdt * t;
    let nodem = nodem0 + d.dnodt * t;
    let mm = mm0 + d.dmdt * t;

    if d.irez == 0 {
        return (em, argpm, inclm, mm, nodem, nm0, 0.0);
    }

    // Integrate the resonance from the epoch to `t` on the 720-minute grid, as the report does.
    let mut atime = 0.0f64;
    let mut xli = d.xlamo;
    let mut xni = e.no;
    let mut ft = 0.0f64;
    let mut xndt = 0.0f64;
    let mut xnddt = 0.0f64;
    let mut xldot = 0.0f64;
    let mut iret = 0u32;
    let mut iretn = 381u32;
    while iretn == 381 {
        let delt;
        if t.abs() < atime.abs() || iret == 351 {
            delt = if t >= 0.0 { -STEP } else { STEP };
            iret = 351;
            iretn = 381;
        } else {
            delt = if t > 0.0 { STEP } else { -STEP };
            if (t - atime).abs() >= STEP {
                iret = 0;
                iretn = 381;
            } else {
                ft = t - atime;
                iretn = 0;
            }
        }
        if d.irez != 2 {
            xndt = d.del1 * (xli - FASX2).sin()
                + d.del2 * (2.0 * (xli - FASX4)).sin()
                + d.del3 * (3.0 * (xli - FASX6)).sin();
            xldot = xni + d.xfact;
            xnddt = d.del1 * (xli - FASX2).cos()
                + 2.0 * d.del2 * (2.0 * (xli - FASX4)).cos()
                + 3.0 * d.del3 * (3.0 * (xli - FASX6)).cos();
            xnddt *= xldot;
        } else {
            let xomi = e.argpo + e.argpdot * atime;
            let x2omi = xomi + xomi;
            let x2li = xli + xli;
            xndt = d.d2201 * (x2omi + xli - G22).sin()
                + d.d2211 * (xli - G22).sin()
                + d.d3210 * (xomi + xli - G32).sin()
                + d.d3222 * (-xomi + xli - G32).sin()
                + d.d4410 * (x2omi + x2li - G44).sin()
                + d.d4422 * (x2li - G44).sin()
                + d.d5220 * (xomi + xli - G52).sin()
                + d.d5232 * (-xomi + xli - G52).sin()
                + d.d5421 * (xomi + x2li - G54).sin()
                + d.d5433 * (-xomi + x2li - G54).sin();
            xldot = xni + d.xfact;
            xnddt = d.d2201 * (x2omi + xli - G22).cos()
                + d.d2211 * (xli - G22).cos()
                + d.d3210 * (xomi + xli - G32).cos()
                + d.d3222 * (-xomi + xli - G32).cos()
                + d.d5220 * (xomi + xli - G52).cos()
                + d.d5232 * (-xomi + xli - G52).cos()
                + 2.0
                    * (d.d4410 * (x2omi + x2li - G44).cos()
                        + d.d4422 * (x2li - G44).cos()
                        + d.d5421 * (xomi + x2li - G54).cos()
                        + d.d5433 * (-xomi + x2li - G54).cos());
            xnddt *= xldot;
        }
        if iretn == 381 {
            xli += xldot * delt + xndt * STEP2;
            xni += xndt * delt + xnddt * STEP2;
            atime += delt;
        }
    }
    let nm = xni + xndt * ft + xnddt * ft * ft * 0.5;
    let xl = xli + xldot * ft + xndt * ft * ft * 0.5;
    let dndt = nm - e.no;
    let mm = if d.irez != 1 {
        xl - 2.0 * nodem + 2.0 * theta
    } else {
        xl - nodem - argpm + theta
    };
    (em, argpm, inclm, mm, nodem, e.no + dndt, dndt)
}

/// Greenwich sidereal time, in radians in `[0, 2π)`, at `days` days from J2000 (UT1, as the report
/// uses it). Taking the day count from J2000 rather than a Julian date keeps its precision: a Julian
/// date near 2.4 million days is good only to about 5e-10 day. The polynomial is the report's.
pub(crate) fn gstime(days: f64) -> f64 {
    let tut1 = days / 36_525.0;
    let temp = -6.2e-6 * tut1 * tut1 * tut1
        + 0.093_104 * tut1 * tut1
        + (876_600.0 * 3600.0 + 8_640_184.812_866) * tut1
        + 67_310.548_41;
    let mut t = (temp * (PI / 180.0) / 240.0) % TWO_PI;
    if t < 0.0 {
        t += TWO_PI;
    }
    t
}
