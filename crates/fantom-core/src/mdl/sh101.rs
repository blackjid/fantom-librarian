//! The SH-101 model: its per-field setters and dependency switch. The machinery is in
//! [`super::engine`]. Every setter writes all four partials of the tone.

use super::engine::*;
use crate::params::mdl::*;

pub(super) static MODEL: Model = Model { setter, depend };

static FILTER: Filter = Filter {
    freq: &SH101_FREQ_CURVE,
    reso: &SH101_RESO_CURVE,
    env_depth: &SH101_FENV_DEPTH_CURVE,
    partials: &[0, 1, 2, 3],
};

// Firmware indices of the fields the setters read besides their own.
const OSC1_PW: usize = 15;
const OSC1_PW_MODE: usize = 16;
const OSC3_WAVE: usize = 28;
const OSC3_LEVEL: usize = 39;
const MOD_MODE: usize = 36;
const FILTER_TYPE: usize = 44;
const FILTER_FREQ: usize = 47;
const FILTER_RESO: usize = 48;
const FILTER_ENV_DEPTH: usize = 49;
const AMP_ENV_SELECT: usize = 57;
const ENV2_MODE: usize = 68;
const EXPANSION: usize = 82;
const OSC1_FEET: usize = 9;

/// `UNK_00dae9e0`: PORTAMENT CURVE TYPE to the tone's curve.
const PORTA_CURVE: [i32; 4] = [2, 0, 1, 2];

/// The dependency switch: fields whose setters depend on field `k`.
fn depend(c: &mut Ctx, k: usize) {
    match k {
        OSC1_PW_MODE => pulse_width(c, c.f(OSC1_PW)),
        OSC3_WAVE => osc3_level(c, c.f(OSC3_LEVEL)),
        FILTER_TYPE => FILTER.all(c),
        AMP_ENV_SELECT => {
            env2_attack(c, c.f(64));
            env2_decay(c, c.f(65));
            env2_sustain(c, c.f(66));
            env2_release(c, c.f(67));
            env2_mode(c, c.f(ENV2_MODE));
        }
        EXPANSION if c.f(EXPANSION) == 0 => {
            lfo_rate(c, c.f(2));
            FILTER.all(c);
        }
        _ => {}
    }
}

/// The setter of field `k`; fields without one do nothing.
fn setter(c: &mut Ctx, k: usize, v: i32) {
    match k {
        1 => {
            if (v as u32) < 8 && (1 << v) & 0x92 != 0 {
                c.put4(LFO, 0x66c, v);
            }
        }
        2 => lfo_rate(c, v),
        4 => {
            if v >= 0 {
                let u = curve(&SH101_OSC1_MOD_CURVE, short(v));
                c.put4(LFO, 0x678, u);
            }
        }
        OSC1_FEET => osc1_feet(c, v),
        OSC1_PW => pulse_width(c, v),
        OSC3_WAVE => osc3_wave(c, v),
        37 => {
            if c.f(MOD_MODE) == 0 {
                c.put(S_PTL, 0, 0x867, scale10(v, 0x2a7).min(0xa9));
            }
        }
        38 => {
            if c.f(MOD_MODE) == 0 {
                c.put(S_PTL, 1, 0x867, scale10(v, 0x1ea).min(0x7a));
            }
        }
        OSC3_LEVEL => osc3_level(c, v),
        40 => c.put(S_PTL, 3, 0x867, v),
        FILTER_TYPE => {
            if (v as u32) < 4 && (1 << v) & 0xd != 0 {
                c.put4(S_PTL, 0x866, v);
            }
        }
        FILTER_FREQ => FILTER.set_freq(c, v),
        FILTER_RESO => FILTER.set_reso(c, v),
        FILTER_ENV_DEPTH => FILTER.set_env_depth(c, v),
        52 => {
            if v >= 0 {
                let u = curve(&SH101_KEY_FOLLOW_CURVE, short(v));
                c.put4(PTL, 0x765, u);
            }
        }
        53 => {
            if v >= 0 {
                let u = curve(&SH101_FILTER_MOD_CURVE, short(v));
                c.put4(LFO, 0x679, u);
            }
        }
        54 => c.put(CMN, 0, 0x705, v),
        AMP_ENV_SELECT => {
            if v == 2 {
                for inst in 0..4 {
                    c.put(AENV, inst, 0x6ec, 0);
                    c.put(AENV, inst, 0x6ee, 2);
                    c.put(AENV, inst, 0x6f2, 0x3ff);
                    c.put(AENV, inst, 0x6ef, 0);
                }
            }
        }
        64 => env2_attack(c, v),
        65 => env2_decay(c, v),
        66 => env2_sustain(c, v),
        67 => env2_release(c, v),
        ENV2_MODE => env2_mode(c, v),
        74 => c.put(S_CMN, 0, 0x846, v),
        76 => {
            let u = curve(&SH101_BENDER_FILTER_CURVE, short(v));
            c.put4(PTL, 0x77b, u);
        }
        77 => {
            let u = curve(&SH101_MOD_LFO_CURVE, short(v));
            c.put4(PTL, 0x780, u);
        }
        78 => bender_depth(c, v, 0x5dc),
        79 => {
            let u = curve(&SH101_PORTA_TIME_CURVE, short(v));
            c.put(CMN, 0, 0x714, u);
        }
        80 => porta_mode(c, v),
        81 => key_mode(c, v, 10),
        83 => c.put(S_CMN, 0, 0x848, v),
        84 => c.put(CMN, 0, 0x71d, PORTA_CURVE[v as usize & 3]),
        85 => c.put4(PTL, 0x792, v),
        86 => c.put4(PTL, 0x794, v),
        87 => c.put4(PTL, 0x796, v),
        _ => {}
    }
}

fn lfo_rate(c: &mut Ctx, v: i32) {
    let v = if c.f(EXPANSION) == 0 {
        curve(&SH101_LFO_RATE_CURVE, short(v))
    } else {
        v
    };
    c.put(LFO, 0, 0x670, v);
    c.put(LFO, 0, 0x6a1, v);
}

/// OSC1 FEET: the coarse tune of partials 0 and 1, and of the sub-oscillator (partial 2) by its
/// waveform.
fn osc1_feet(c: &mut Ctx, v: i32) {
    let osc = match v {
        1 => -12,
        2 => 0,
        3 => 12,
        4 => 24,
        _ => return,
    };
    c.put(PTL, 0, 0x746, osc);
    c.put(PTL, 1, 0x746, osc);
    // (feet 1, 2, 3, 4) of the sub-oscillator
    let sub = match c.f(OSC3_WAVE) {
        2 => [-24, -12, 0, 12],
        3 | 4 => [-36, -24, -12, 0],
        _ => return,
    };
    c.put(PTL, 2, 0x746, sub[v as usize - 1]);
}

/// OSC3 WAVE FORM (2 pulse, 3 square, 4 sub): the sub-oscillator's wave and its tune from OSC1 FEET.
fn osc3_wave(c: &mut Ctx, v: i32) {
    let (kind, form, sync, sub) = match v {
        2 => (1, 1, 0, [-24, -12, 0, 12]),
        3 => (1, 1, 0, [-36, -24, -12, 0]),
        4 => (2, 0, 0x2c, [-36, -24, -12, 0]),
        _ => return,
    };
    c.put(S_PTL, 2, 0x85a, kind);
    c.put(S_PTL, 2, 0x85b, form);
    c.put(S_PTL, 2, 0x85c, sync);
    let feet = c.f(OSC1_FEET);
    if (1..=4).contains(&feet) {
        c.put(PTL, 2, 0x746, sub[feet as usize - 1]);
    }
}

/// OSC1 PULSE WIDTH by mode: LFO (two depths), MANUAL, ENV.
fn pulse_width(c: &mut Ctx, v: i32) {
    let v = short(v);
    let (width, depth, env) = match c.f(OSC1_PW_MODE) {
        0 => (curve(&SH101_PW_A_CURVE, v), curve(&SH101_PW_B_CURVE, v), 0),
        1 => (curve(&SH101_PW_MANUAL_CURVE, v), 0, 0),
        2 => (0x40, 0, curve(&SH101_PW_ENV_CURVE, v)),
        _ => return,
    };
    c.put(S_PTL, 0, 0x85d, width);
    c.put(S_PTL, 0, 0x85e, depth);
    c.put(PTL, 0, 0x789, env);
}

/// OSC3 LEVEL as the sub-oscillator's attenuation, by its waveform.
fn osc3_level(c: &mut Ctx, v: i32) {
    let u = match c.f(OSC3_WAVE) {
        2 | 3 => scale10(v, 0x20b).min(0x82),
        4 => scale10(v, 0x146).min(0x51),
        _ => return,
    };
    c.put(S_PTL, 2, 0x867, u);
}

fn env2_attack(c: &mut Ctx, v: i32) {
    let curves = (&SH101_ENV2_ATTACK_CURVE, &SH101_ENV2_ATTACK_CURVE);
    env2_all(c, v, curves, (0x6ec, 0x6e1));
}

fn env2_decay(c: &mut Ctx, v: i32) {
    let curves = (&SH101_ENV2_DECAY_A_CURVE, &SH101_ENV2_DECAY_F_CURVE);
    env2_all(c, v, curves, (0x6ee, 0x6e3));
}

fn env2_sustain(c: &mut Ctx, v: i32) {
    let curves = (&SH101_ENV2_SUSTAIN_CURVE, &SH101_ENV2_SUSTAIN_CURVE);
    env2_all(c, v, curves, (0x6f2, 0x6e8));
}

fn env2_release(c: &mut Ctx, v: i32) {
    let curves = (&SH101_ENV2_RELEASE_CURVE, &SH101_ENV2_RELEASE_CURVE);
    env2_all(c, v, curves, (0x6ef, 0x6e4));
}

/// ENV2 MODE: legato in the common block, and the LFO triggers of the amp and filter envelopes
/// (GATE+TRIG, GATE, LFO — the LFO mode only with AMP ENV SELECT 0 or 2).
fn env2_mode(c: &mut Ctx, v: i32) {
    let (legato, amp_trig, filter_trig) = match v {
        0 | 1 => (v, 0, 0),
        2 => match c.f(AMP_ENV_SELECT) {
            0 => (0, 1, 0),
            2 => (0, 0, 1),
            _ => return,
        },
        _ => return,
    };
    c.put(CMN, 0, 0x70e, legato);
    c.put4(S_PTL, 0x86d, amp_trig);
    c.put4(S_PTL, 0x86c, filter_trig);
}
