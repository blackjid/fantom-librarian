//! The JUNO-106 model: its per-field setters and dependency switch. The machinery is in
//! [`super::engine`]. Every setter writes all four partials of the tone.

use super::engine::*;
use crate::params::mdl::*;

pub(super) static MODEL: Model = Model { setter, depend };

static FILTER: Filter = Filter {
    freq: &JUNO_FREQ_CURVE,
    reso: &JUNO_RESO_CURVE,
    env_depth: &JUNO_FENV_DEPTH_CURVE,
    partials: &[0, 1, 2, 3],
};

// Indices of the fields the setters read besides their own.
const OSC1_PW: usize = 15;
const OSC1_PW_MODE: usize = 16;
const MOD_MODE: usize = 36;
const OSC1_LEVEL: usize = 37;
const OSC2_LEVEL: usize = 38;
const FILTER_TYPE: usize = 44;
const HPF_STEP: usize = 45;
const FILTER_FREQ: usize = 47;
const FILTER_RESO: usize = 48;
const FILTER_ENV_DEPTH: usize = 49;
const AMP_ENV_SELECT: usize = 57;
const EXPANSION: usize = 82;

/// `UNK_00dada90`: PORTAMENT CURVE TYPE to the tone's curve.
const PORTA_CURVE: [i32; 4] = [0, 0, 1, 2];

/// The dependency switch: fields whose setters depend on field `k`.
fn depend(c: &mut Ctx, k: usize) {
    match k {
        OSC1_PW_MODE => pulse_width(c, c.f(OSC1_PW)),
        HPF_STEP if c.f(MOD_MODE) == 0 => {
            osc1_level(c, c.f(OSC1_LEVEL));
            osc2_level(c, c.f(OSC2_LEVEL));
        }
        FILTER_TYPE => FILTER.all(c),
        AMP_ENV_SELECT => {
            env2_attack(c, c.f(64));
            env2_decay(c, c.f(65));
            env2_sustain(c, c.f(66));
            env2_release(c, c.f(67));
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
        2 => lfo_rate(c, v),
        3 => {
            let v = short(v);
            let delay = curve(&JUNO_LFO_DELAY_CURVE, v);
            let fade = curve(&JUNO_LFO_FADE_CURVE, v);
            for inst in 0..4 {
                c.put(LFO, inst, 0x673, delay);
                c.put(LFO, inst, 0x676, fade);
            }
        }
        4 => {
            if v >= 0 {
                let u = curve(&JUNO_OSC1_MOD_CURVE, short(v));
                c.put4(LFO, 0x678, u);
            }
        }
        9 => osc1_feet(c, v),
        OSC1_PW => pulse_width(c, v),
        OSC1_LEVEL => osc1_level(c, v),
        OSC2_LEVEL => osc2_level(c, v),
        39 => c.put(S_PTL, 2, 0x867, v),
        40 => c.put(S_PTL, 3, 0x867, v),
        FILTER_TYPE => {
            if (v as u32) < 4 && (1 << v) & 0xd != 0 {
                c.put4(S_PTL, 0x866, v);
            }
        }
        HPF_STEP => hpf_step(c, v),
        FILTER_FREQ => FILTER.set_freq(c, v),
        FILTER_RESO => FILTER.set_reso(c, v),
        FILTER_ENV_DEPTH => FILTER.set_env_depth(c, v),
        52 => {
            if v >= 0 {
                let u = curve(&JUNO_KEY_FOLLOW_CURVE, short(v));
                c.put4(PTL, 0x765, u);
            }
        }
        53 => {
            if v >= 0 {
                let u = curve(&JUNO_FILTER_MOD_CURVE, short(v));
                c.put4(LFO, 0x679, u);
            }
        }
        54 => {
            let u = curve(&JUNO_AMP_LEVEL_CURVE, short(v));
            c.put(CMN, 0, 0x705, u);
        }
        AMP_ENV_SELECT => {
            if v == 2 {
                for inst in 0..4 {
                    c.put(AENV, inst, 0x6ec, 0);
                    c.put(AENV, inst, 0x6ee, 8);
                    c.put(AENV, inst, 0x6f2, 0x3ff);
                    c.put(AENV, inst, 0x6ef, 0);
                }
            }
        }
        64 => env2_attack(c, v),
        65 => env2_decay(c, v),
        66 => env2_sustain(c, v),
        67 => env2_release(c, v),
        74 => c.put(S_CMN, 0, 0x846, v),
        76 => {
            let u = curve(&JUNO_BENDER_FILTER_CURVE, short(v));
            c.put4(PTL, 0x77b, u);
        }
        77 => {
            let u = curve(&JUNO_MOD_LFO_CURVE, short(v));
            c.put4(PTL, 0x780, u);
        }
        78 => bender_depth(c, v, 0x4b0),
        79 => {
            let u = curve(&JUNO_PORTA_TIME_CURVE, short(v));
            c.put(CMN, 0, 0x714, u);
        }
        80 => {
            if v == 0 || v == 1 {
                c.put(CMN, 0, 0x710, v);
                c.put(CMN, 0, 0x711, 0);
            }
        }
        81 => key_mode(c, v, 2),
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
        curve(&JUNO_LFO_RATE_CURVE, short(v))
    } else {
        v
    };
    c.put(LFO, 0, 0x670, v);
    c.put(LFO, 0, 0x6a1, v);
}

/// OSC1 FEET: the coarse tune of the two oscillators and of the sub-oscillator.
fn osc1_feet(c: &mut Ctx, v: i32) {
    let (osc, sub) = match v {
        1 => (-12, -24),
        2 => (0, -12),
        3 => (12, 0),
        _ => return,
    };
    c.put(PTL, 0, 0x746, osc);
    c.put(PTL, 1, 0x746, osc);
    c.put(PTL, 2, 0x746, sub);
}

/// OSC1 PULSE WIDTH from the pulse width mode: LFO sweeps (two depths), MANUAL, ENV sets none.
fn pulse_width(c: &mut Ctx, v: i32) {
    let v = short(v);
    let (width, depth) = match c.f(OSC1_PW_MODE) {
        0 => (curve(&JUNO_PW_A_CURVE, v), curve(&JUNO_PW_B_CURVE, v)),
        1 => (curve(&JUNO_PW_MANUAL_CURVE, v), 0),
        2 => (0, 0),
        _ => return,
    };
    c.put(S_PTL, 0, 0x85d, width);
    c.put(S_PTL, 0, 0x85e, depth);
    c.put(PTL, 0, 0x789, 0);
}

/// OSC1/2 LEVEL as the partial's attenuation; only with MOD MODE 0 (OFF).
fn level(c: &mut Ctx, v: i32, inst: u8, m: i32, top: i32, scale: i32) {
    if c.f(MOD_MODE) != 0 {
        return;
    }
    let mut u = scale10(v, m).min(top);
    if c.f(HPF_STEP) == 0 {
        u = u * scale / 100;
    }
    c.put(S_PTL, inst, 0x867, u);
}

fn osc1_level(c: &mut Ctx, v: i32) {
    level(c, v, 0, 0x27b, 0x9e, 0x82);
}

fn osc2_level(c: &mut Ctx, v: i32) {
    level(c, v, 1, 0x17a, 0x5e, 0x8c);
}

/// FILTER HPF STEP: the high-pass cutoff of the synth partials and the EQ of the tone.
fn hpf_step(c: &mut Ctx, v: i32) {
    let (cutoff, lo_gain, lo_freq) = match v {
        0 => (0, 0xf0, 300),
        1 => (100, 100, 200),
        2 => (500, 100, 200),
        3 => (600, 100, 200),
        _ => return,
    };
    c.put4(S_PTL, 0x861, cutoff);
    for inst in 0..3 {
        c.put(EQ, inst, 0x66b, i32::from(v == 0));
    }
    c.put(EQ, 3, 0x664, lo_gain);
    c.put(EQ, 3, 0x667, lo_freq);
    c.put(EQ, 3, 0x66b, 1);
}

fn env2_attack(c: &mut Ctx, v: i32) {
    let curves = (&JUNO_ENV2_ATTACK_CURVE, &JUNO_ENV2_ATTACK_CURVE);
    env2_all(c, v, curves, (0x6ec, 0x6e1));
}

fn env2_decay(c: &mut Ctx, v: i32) {
    let curves = (&JUNO_ENV2_DECAY_A_CURVE, &JUNO_ENV2_DECAY_F_CURVE);
    env2_all(c, v, curves, (0x6ee, 0x6e3));
}

fn env2_sustain(c: &mut Ctx, v: i32) {
    let curves = (&JUNO_ENV2_SUSTAIN_CURVE, &JUNO_ENV2_SUSTAIN_CURVE);
    env2_all(c, v, curves, (0x6f2, 0x6e8));
}

fn env2_release(c: &mut Ctx, v: i32) {
    let curves = (&JUNO_ENV2_RELEASE_CURVE, &JUNO_ENV2_RELEASE_CURVE);
    env2_all(c, v, curves, (0x6ef, 0x6e4));
}
