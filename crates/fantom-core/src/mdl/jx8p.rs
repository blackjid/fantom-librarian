//! The JX-8P model: its per-field setters and dependency switch. The machinery is in
//! [`super::engine`]. The setters write partials 0, 1 and 3 of the tone.

use super::engine::*;
use crate::params::mdl::*;

pub(super) static MODEL: Model = Model { setter, depend };

/// The partials of the tone the JX-8P writes.
const PARTIALS: [u8; 3] = [1, 0, 3];

static FILTER: Filter = Filter {
    freq: &JX8P_FREQ_CURVE,
    reso: &JX8P_RESO_CURVE,
    env_depth: &JX8P_FENV_DEPTH_CURVE,
    partials: &PARTIALS,
};

// Indices of the fields the setters read besides their own.
const OSC1_ENV_SELECT: usize = 7;
const OSC1_WAVE: usize = 8;
const OSC1_FEET: usize = 9;
const OSC1_COARSE: usize = 10;
const OSC2_WAVE: usize = 19;
const OSC2_FEET: usize = 20;
const OSC2_COARSE: usize = 21;
const MOD_MODE: usize = 36;
const OSC1_LEVEL: usize = 37;
const OSC2_LEVEL: usize = 38;
const MIXER_ENV_SELECT: usize = 42;
const MIXER_DYNAMICS: usize = 43;
const FILTER_TYPE: usize = 44;
const HPF_STEP: usize = 45;
const FILTER_FREQ: usize = 47;
const FILTER_RESO: usize = 48;
const FILTER_ENV_DEPTH: usize = 49;
const FILTER_ENV_SELECT: usize = 50;
const AMP_ENV_SELECT: usize = 57;
const ENV1_KEY_FOLLOW_STEP: usize = 63;
const ENV2_KEY_FOLLOW_STEP: usize = 70;
const EXPANSION: usize = 82;

/// MIXER DYNAMICS to the tone's level curve and
/// velocity range.
const DYNAMICS_CURVE: [i32; 4] = [0, 0, 1, 0];
const DYNAMICS_RANGE_LO: [i32; 4] = [1, 50, 127, 110];
const DYNAMICS_FADE_LO: [i32; 4] = [0, 127, 127, 127];
/// KEY FOLLOW STEP to the tone's key follow.
const KEY_FOLLOW: [i32; 4] = [0, 25, 50, 100];
/// DYNAMICS PITCH to the pitch envelope's velocity sensitivity.
const DYN_PITCH: [i32; 4] = [0, 32, 50, 48];
/// DYNAMICS FILTER to the filter envelope's velocity sensitivity.
const DYN_FILTER: [i32; 4] = [0, 35, 50, 40];
/// DYNAMICS AMP to the level velocity sensitivity.
const DYN_AMP: [i32; 4] = [0, 27, 47, 45];
/// DYNAMICS to the velocity curve.
const DYN_CURVE: [i32; 4] = [0, 1, 1, 2];
/// BENDER PITCH DEPTH STEP to semitones.
const BENDER_STEP: [i32; 4] = [2, 3, 4, 7];
/// PORTAMENT CURVE TYPE to the tone's curve.
const PORTA_CURVE: [i32; 4] = [2, 0, 1, 2];

fn pick(table: &[i32; 4], v: i32) -> i32 {
    table[v as usize & 3]
}

fn put3(c: &mut Ctx, block: u8, param: u16, value: i32) {
    for inst in PARTIALS {
        c.put(block, inst, param, value);
    }
}

/// The dependency switch: fields whose setters depend on field `k`.
fn depend(c: &mut Ctx, k: usize) {
    match k {
        OSC1_ENV_SELECT | FILTER_ENV_SELECT => {
            env1_key_follow(c, c.f(ENV1_KEY_FOLLOW_STEP));
            env1_attack(c, c.f(58));
            env1_decay(c, c.f(59));
            env1_sustain(c, c.f(60));
            env1_release(c, c.f(61));
            env2_key_follow(c, c.f(ENV2_KEY_FOLLOW_STEP));
            env2_stages(c);
        }
        OSC1_WAVE => osc1_level(c, c.f(OSC1_LEVEL)),
        OSC2_WAVE => osc2_level(c, c.f(OSC2_LEVEL)),
        MOD_MODE => {
            osc1_level(c, c.f(OSC1_LEVEL));
            osc2_level(c, c.f(OSC2_LEVEL));
            mixer_env_select(c, c.f(MIXER_ENV_SELECT));
        }
        MIXER_ENV_SELECT => {
            env1_key_follow(c, c.f(ENV1_KEY_FOLLOW_STEP));
            mixer_dynamics(c, c.f(MIXER_DYNAMICS));
        }
        FILTER_TYPE => FILTER.all(c),
        HPF_STEP => {
            osc1_level(c, c.f(OSC1_LEVEL));
            osc2_level(c, c.f(OSC2_LEVEL));
        }
        AMP_ENV_SELECT => {
            env1_attack(c, c.f(58));
            env1_decay(c, c.f(59));
            env1_sustain(c, c.f(60));
            env1_release(c, c.f(61));
            env2_stages(c);
        }
        EXPANSION if c.f(EXPANSION) == 0 => {
            lfo_rate(c, c.f(2));
            FILTER.all(c);
        }
        _ => {}
    }
}

fn env2_stages(c: &mut Ctx) {
    env2_attack(c, c.f(64));
    env2_decay(c, c.f(65));
    env2_sustain(c, c.f(66));
    env2_release(c, c.f(67));
}

/// The setter of field `k`; fields without one do nothing.
fn setter(c: &mut Ctx, k: usize, v: i32) {
    match k {
        1 => {
            if (v as u32) < 8 && (1 << v) & 0x91 != 0 {
                for inst in PARTIALS {
                    c.put(LFO, inst, 0x66c, v);
                    c.put(LFO, inst, 0x69d, 1);
                }
            }
        }
        2 => lfo_rate(c, v),
        3 => {
            let v = short(v);
            let delay = curve(&JX8P_LFO_DELAY_CURVE, v);
            let fade = curve(&JX8P_LFO_FADE_CURVE, v);
            put3(c, LFO, 0x673, delay);
            put3(c, LFO, 0x676, fade);
        }
        4 => {
            if v >= 0 {
                let u = curve(&JX8P_MOD_CURVE, short(v));
                c.put(LFO, 1, 0x678, u);
            }
        }
        5 => {
            let u = curve(&JX8P_PENV_DEPTH_CURVE, short(v));
            c.put(PENV, 1, 0x6ce, u);
        }
        OSC1_ENV_SELECT => c.put(PTL, 1, 0x790, if v != 0 { 0x6c } else { 0x6b }),
        OSC1_WAVE => wave(c, 1, v),
        OSC1_FEET => {
            let coarse = c.f(OSC1_COARSE);
            c.put(PTL, 1, 0x746, tune(v, coarse));
        }
        OSC1_COARSE => {
            let feet = c.f(OSC1_FEET);
            c.put(PTL, 1, 0x746, tune(feet, v));
        }
        17 => {
            if v >= 0 {
                let u = curve(&JX8P_MOD_CURVE, short(v));
                c.put(LFO, 0, 0x678, u);
            }
        }
        18 => {
            let u = curve(&JX8P_PENV_DEPTH_CURVE, short(v));
            c.put(PENV, 0, 0x6ce, u);
        }
        OSC2_WAVE => wave(c, 0, v),
        OSC2_FEET => {
            let coarse = c.f(OSC2_COARSE);
            c.put(PTL, 0, 0x746, tune(v, coarse));
        }
        OSC2_COARSE => {
            let feet = c.f(OSC2_FEET);
            c.put(PTL, 0, 0x746, tune(feet, v));
        }
        22 => c.put(PTL, 0, 0x747, v),
        MOD_MODE => c.put(S_PMT, 0, 0x849, if v == 1 || v == 3 { v } else { 0 }),
        OSC1_LEVEL => osc1_level(c, v),
        OSC2_LEVEL => osc2_level(c, v),
        41 => {
            if v >= 0 {
                c.put(PTL, 0, 0x792, scale10(v, 0x219).min(0x21));
            }
        }
        MIXER_ENV_SELECT => mixer_env_select(c, v),
        MIXER_DYNAMICS => mixer_dynamics(c, v),
        FILTER_TYPE => {
            if (v as u32) < 4 && (1 << v) & 0xd != 0 {
                put3(c, S_PTL, 0x866, v);
            }
        }
        HPF_STEP => hpf_step(c, v),
        FILTER_FREQ => FILTER.set_freq(c, v),
        FILTER_RESO => FILTER.set_reso(c, v),
        FILTER_ENV_DEPTH => FILTER.set_env_depth(c, v),
        52 => {
            if v >= 0 {
                let u = curve(&JX8P_KEY_FOLLOW_CURVE, short(v));
                put3(c, PTL, 0x765, u);
            }
        }
        53 => {
            if v >= 0 {
                let u = curve(&JX8P_FILTER_MOD_CURVE, short(v));
                put3(c, LFO, 0x679, u);
            }
        }
        54 => {
            let u = curve(&JX8P_AMP_LEVEL_CURVE, short(v));
            c.put(CMN, 0, 0x705, u);
        }
        AMP_ENV_SELECT => {
            if v == 3 {
                for inst in PARTIALS {
                    c.put(AENV, inst, 0x6ec, 3);
                    c.put(AENV, inst, 0x6ee, 5);
                    c.put(AENV, inst, 0x6f2, 0x3ff);
                    c.put(AENV, inst, 0x6ef, 0);
                }
            }
        }
        58 => env1_attack(c, v),
        59 => env1_decay(c, v),
        60 => env1_sustain(c, v),
        61 => env1_release(c, v),
        ENV1_KEY_FOLLOW_STEP => env1_key_follow(c, v),
        64 => env2_attack(c, v),
        65 => env2_decay(c, v),
        66 => env2_sustain(c, v),
        67 => env2_release(c, v),
        ENV2_KEY_FOLLOW_STEP => env2_key_follow(c, v),
        71 => {
            for inst in [1, 0] {
                c.put(PENV, inst, 0x6cf, pick(&DYN_PITCH, v));
                c.put(PENV, inst, 0x6db, pick(&DYN_CURVE, v));
            }
        }
        72 => {
            put3(c, FENV, 0x6de, pick(&DYN_FILTER, v));
            put3(c, FENV, 0x6dd, pick(&DYN_CURVE, v));
        }
        73 => {
            put3(c, PTL, 0x76e, pick(&DYN_AMP, v));
            put3(c, PTL, 0x76d, pick(&DYN_CURVE, v));
        }
        74 => c.put(S_CMN, 0, 0x846, v),
        75 => {
            c.put(S_CMN, 0, 0x844, 0);
            c.put(S_CMN, 0, 0x845, 0);
            c.put(CMN, 0, 0x715, pick(&BENDER_STEP, v));
            c.put(CMN, 0, 0x716, pick(&BENDER_STEP, v));
        }
        77 => {
            let u = curve(&JX8P_MOD_LFO_CURVE, short(v));
            c.put(PTL, 1, 0x780, u);
            c.put(PTL, 0, 0x780, u);
        }
        79 => {
            let u = curve(&JX8P_PORTA_TIME_CURVE, short(v));
            c.put(CMN, 0, 0x714, u);
        }
        80 => porta_mode(c, v),
        81 => key_mode(c, v, 0x14),
        83 => c.put(S_CMN, 0, 0x848, v),
        84 => c.put(CMN, 0, 0x71d, pick(&PORTA_CURVE, v)),
        85 => put3(c, PTL, 0x789, v),
        86 => put3(c, PTL, 0x78b, v),
        87 => {
            c.put(PTL, 0, 0x78d, v);
            c.put(PTL, 1, 0x78d, v);
            c.put(PTL, 3, 0x78d, v / 16);
        }
        _ => {}
    }
}

fn lfo_rate(c: &mut Ctx, v: i32) {
    let v = if c.f(EXPANSION) == 0 {
        curve(&JX8P_LFO_RATE_CURVE, short(v))
    } else {
        v
    };
    c.put(LFO, 0, 0x670, v);
    c.put(LFO, 0, 0x6a1, v);
}

/// An oscillator's waveform (codes 0x10…0x13) as the partial's wave type, variation and sync.
fn wave(c: &mut Ctx, inst: u8, v: i32) {
    let (kind, form, sync) = match v {
        0x10 => (1, 0, 0),
        0x11 => (2, 0, 0x2f),
        0x12 => (1, 1, 0),
        0x13 => (4, 0, 0),
        _ => return,
    };
    c.put(S_PTL, inst, 0x85a, kind);
    c.put(S_PTL, inst, 0x85b, form);
    c.put(S_PTL, inst, 0x85c, sync);
}

/// The partial's coarse tune from FEET (octaves above 8') and COARSE TUNE, each clamped.
fn tune(feet: i32, coarse: i32) -> i32 {
    let octave = ((feet - 2) * 12).min(24);
    let coarse = coarse.clamp(-12, 12);
    if octave < -12 {
        coarse - 12
    } else {
        coarse + octave
    }
}

/// OSC1/2 LEVEL as the partial's attenuation (by waveform and MOD MODE), and the mixer's level
/// into the modulation stage.
fn level(c: &mut Ctx, v: i32, wave: i32, inst: u8, ring: u16) {
    // per waveform: (multiplier, ceiling, factor when HPF STEP is 0, MOD MODE 3 value)
    let (m, top, factor) = match wave {
        0x12 => (0x37c, 0xde, 0x73),
        0x13 => (0xdd, 0x37, 0x82),
        0x11 => (0x360, 0xd7, 0x76),
        _ => (0x21b, 0x86, 0x7c),
    };
    let mut u = match c.f(MOD_MODE) {
        2 => 0,
        3 => top,
        _ => scale10(v, m).min(top),
    };
    if c.f(HPF_STEP) == 0 {
        u = u * factor / 100;
    }
    c.put(S_PTL, inst, 0x867, u);
    c.put(S_PMT, 0, 0x84b, 0);
    if c.f(MOD_MODE) == 3 {
        c.put(S_PMT, 0, ring, v / 2);
    }
}

fn osc1_level(c: &mut Ctx, v: i32) {
    let wave = c.f(OSC1_WAVE);
    level(c, v, wave, 1, 0x854);
}

fn osc2_level(c: &mut Ctx, v: i32) {
    let wave = c.f(OSC2_WAVE);
    level(c, v, wave, 0, 0x853);
}

/// MIXER ENV SELECT: the velocity range of partial 0 and 1 by MOD MODE (0 and 1 alike).
fn mixer_env_select(c: &mut Ctx, v: i32) {
    let (a, b) = match (c.f(MOD_MODE), v) {
        (0 | 1, 0) => (0x6b, 0x2c),
        (3, 0) => (0x6b, 0x2f),
        (0 | 1, _) => (0x6c, 0x2c),
        (3, _) => (0x6c, 0x2f),
        _ => return,
    };
    c.put(PTL, 1, 0x790, 0);
    c.put(PTL, 1, 0x791, 0);
    c.put(PTL, 0, 0x790, a);
    c.put(PTL, 0, 0x791, b);
}

fn mixer_dynamics(c: &mut Ctx, v: i32) {
    c.put(PMT, 0, 0x720, pick(&DYNAMICS_CURVE, v));
    c.put(PMT, 0, 0x726, pick(&DYNAMICS_RANGE_LO, v));
    c.put(PMT, 0, 0x728, pick(&DYNAMICS_FADE_LO, v));
}

/// FILTER HPF STEP: the high-pass cutoff of the synth partials and the EQ of the tone.
fn hpf_step(c: &mut Ctx, v: i32) {
    let cutoff = match v {
        0 => 0,
        1 => 100,
        2 => 500,
        3 => 600,
        _ => return,
    };
    for inst in 0..4 {
        c.put(EQ, inst, 0x66b, i32::from(v == 0));
        c.put(S_PTL, inst, 0x861, cutoff);
    }
}

/// ENV1 stage: the pitch envelope (OSC1 ENVT SELECT 0) and the filter envelope (FILTER ENV
/// SELECT 0 — all partials — or, with MIXER ENV SELECT 0, partial 0 alone).
fn env1_stage(
    c: &mut Ctx,
    v: i32,
    pitch_curve: &MdlCurve,
    filter_curve: &MdlCurve,
    p: u16,
    f: u16,
) {
    let v = short(v);
    if c.f(OSC1_ENV_SELECT) == 0 {
        let u = curve(pitch_curve, v);
        c.put(PENV, 1, p, u);
        c.put(PENV, 0, p, u);
    }
    let u = curve(filter_curve, v);
    if c.f(FILTER_ENV_SELECT) == 0 {
        put3(c, FENV, f, u);
    } else if c.f(MIXER_ENV_SELECT) == 0 {
        c.put(FENV, 0, f, u);
    }
}

fn env1_attack(c: &mut Ctx, v: i32) {
    env1_stage(
        c,
        v,
        &JX8P_ENV1_ATTACK_CURVE,
        &JX8P_ENV1_ATTACK_CURVE,
        0x6d2,
        0x6e1,
    );
}

fn env1_decay(c: &mut Ctx, v: i32) {
    env1_stage(
        c,
        v,
        &JX8P_ENV1_DECAY_CURVE,
        &JX8P_ENV1_DECAY_CURVE,
        0x6d4,
        0x6e3,
    );
}

fn env1_sustain(c: &mut Ctx, v: i32) {
    env1_stage(
        c,
        v,
        &JX8P_ENV1_SUSTAIN_P_CURVE,
        &JX8P_ENV1_SUSTAIN_F_CURVE,
        0x6d9,
        0x6e8,
    );
}

fn env1_release(c: &mut Ctx, v: i32) {
    let v = short(v);
    let u = curve(&JX8P_ENV1_RELEASE_CURVE, v);
    if c.f(OSC1_ENV_SELECT) == 0 {
        c.put(PENV, 1, 0x6d5, u);
        c.put(PENV, 0, 0x6d5, u);
    }
    if c.f(MIXER_ENV_SELECT) == 0 {
        if c.f(FILTER_ENV_SELECT) == 0 {
            put3(c, FENV, 0x6e4, u);
        } else {
            c.put(FENV, 0, 0x6e4, u);
        }
    }
}

fn env1_key_follow(c: &mut Ctx, v: i32) {
    let u = pick(&KEY_FOLLOW, v);
    let (mixer, filter) = (c.f(MIXER_ENV_SELECT), c.f(FILTER_ENV_SELECT));
    if (mixer == 0 && filter != 1) || (mixer != 0 && filter == 0) {
        put3(c, PTL, 0x770, u);
    }
    put3(c, PTL, 0x76f, if c.f(OSC1_ENV_SELECT) == 0 { u } else { 0 });
}

fn env2_key_follow(c: &mut Ctx, v: i32) {
    let u = pick(&KEY_FOLLOW, v);
    put3(c, PTL, 0x771, u);
    if c.f(OSC1_ENV_SELECT) == 1 {
        put3(c, PTL, 0x76f, u);
    }
    if c.f(FILTER_ENV_SELECT) == 1 {
        put3(c, PTL, 0x770, u);
    }
}

/// ENV2 stage: the pitch envelope (OSC1 ENVT SELECT 1), the filter envelope (FILTER ENV SELECT
/// 1) and the amp envelope (AMP ENV SELECT 0 or 1).
fn env2_stage(
    c: &mut Ctx,
    v: i32,
    pitch_curve: &MdlCurve,
    other: &MdlCurve,
    p: u16,
    f: u16,
    a: u16,
) {
    let v = short(v);
    if c.f(OSC1_ENV_SELECT) == 1 {
        let u = curve(pitch_curve, v);
        c.put(PENV, 1, p, u);
        c.put(PENV, 0, p, u);
    }
    let u = curve(other, v);
    if c.f(FILTER_ENV_SELECT) == 1 {
        put3(c, FENV, f, u);
    }
    if c.f(AMP_ENV_SELECT) < 2 {
        put3(c, AENV, a, u);
    }
}

fn env2_attack(c: &mut Ctx, v: i32) {
    env2_stage(
        c,
        v,
        &JX8P_ENV2_ATTACK_CURVE,
        &JX8P_ENV2_ATTACK_CURVE,
        0x6d2,
        0x6e1,
        0x6ec,
    );
}

fn env2_decay(c: &mut Ctx, v: i32) {
    env2_stage(
        c,
        v,
        &JX8P_ENV2_DECAY_CURVE,
        &JX8P_ENV2_DECAY_CURVE,
        0x6d4,
        0x6e3,
        0x6ee,
    );
}

fn env2_sustain(c: &mut Ctx, v: i32) {
    env2_stage(
        c,
        v,
        &JX8P_ENV2_SUSTAIN_P_CURVE,
        &JX8P_ENV2_SUSTAIN_F_CURVE,
        0x6d9,
        0x6e8,
        0x6f2,
    );
}

fn env2_release(c: &mut Ctx, v: i32) {
    env2_stage(
        c,
        v,
        &JX8P_ENV2_RELEASE_CURVE,
        &JX8P_ENV2_RELEASE_CURVE,
        0x6d5,
        0x6e4,
        0x6ef,
    );
}
