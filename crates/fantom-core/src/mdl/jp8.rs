//! The JUPITER-8 model: how an engine field lands in
//! the ZEN-Core tone of the same record.
//!
//! The per-field setters and the dependency switch.
//! The instrument sets a field by running its setter with the new value, storing the value, then
//! running the switch for that field index, which re-runs the setters of the fields that depend
//! on it. Each setter writes tone parameters named by the instrument's (block, parameter) pair;
//! [`locate`] turns a pair into a record offset. Names and offsets are in `docs/FORMAT.md`.

use std::cell::Cell;

use crate::params::mdl::*;
use crate::params::MdlCurve;
use crate::params::MdlTarget;

/// Number of engine fields of an analog model.
pub(super) const FIELDS: usize = 88;

/// The engine fields of a record, by index, as the setters see them.
pub(super) type Fields = [i32; FIELDS];

// Indices of the fields the setters read besides their own.
const OSC1_DEST: usize = 6;
const OSC1_WAVE: usize = 8;
const OSC1_PW: usize = 15;
const OSC1_PW_MODE: usize = 16;
const OSC2_WAVE: usize = 19;
const OSC2_MODE: usize = 24;
const MOD_MODE: usize = 36;
const FILTER_TYPE: usize = 44;
const FILTER_ENV_SELECT: usize = 50;
const AMP_ENV_SELECT: usize = 57;
const EXPANSION: usize = 82;

// Block ids.
const CMN: u8 = 22;
const PTL: u8 = 25;
const PENV: u8 = 26;
const FENV: u8 = 27;
const AENV: u8 = 28;
const LFO: u8 = 29;
const S_CMN: u8 = 31;
const S_PMT: u8 = 32;
const S_PTL: u8 = 33;

/// AMP MOD STEP to LFO 1 amp depth.
const AMP_MOD_STEP: [i32; 4] = [0, 25, 70, 98];
/// PORTAMENT CURVE TYPE to the tone's curve.
const PORTA_CURVE: [i32; 4] = [0, 0, 1, 2];

/// One tone parameter a setter wrote: the instrument's block, instance (partial) and parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Write {
    pub block: u8,
    pub inst: u8,
    pub param: u16,
    pub value: i32,
}

/// Where a write lands in the record: offset and width in bytes. `None` for a parameter whose
/// place in the tone layout is not known.
pub(super) fn locate(w: &Write) -> Option<(usize, usize)> {
    let t: &MdlTarget = TONE_TARGETS
        .iter()
        .find(|t| t.block == w.block && t.param == w.param)?;
    if t.offset == MdlTarget::UNLOCATED {
        return None;
    }
    let starts = TONE_BLOCKS.iter().find(|(b, _)| *b == w.block)?.1;
    let inst = if starts.len() == 1 {
        0
    } else {
        usize::from(w.inst)
    };
    Some((
        0x10 + usize::from(*starts.get(inst)?) + usize::from(t.offset),
        usize::from(t.size),
    ))
}

/// The instrument's piecewise-cubic conversion, in its own float32 arithmetic.
pub(super) fn curve(c: &MdlCurve, p: i32) -> i32 {
    let neg = c.signed && p < 0;
    let p = if neg {
        i32::from((p as i16).wrapping_neg())
    } else {
        p
    };
    let (lo, hi) = (i32::from(c.lo), i32::from(c.hi));
    let p = p.min(hi).max(lo);
    let last = c.y[c.y.len() - 1];
    let signed = |v: i16| i32::from(if neg { v.wrapping_neg() } else { v });
    if p == hi {
        return signed(last);
    }
    let pos = ((c.y.len() - 1) as f32 / hi as f32) * (p + 1) as f32;
    let i = if pos > 0.0 {
        pos as i32 as usize & 0xff
    } else {
        0
    };
    let dx = (p - i32::from(c.x[i])) as f32;
    let width = i32::from(c.x[i + 1]) - i32::from(c.x[i]);
    let t = dx / width as f32;
    let shape = c.c[i] + (c.b[i] + t * c.a[i]) * t;
    signed((f32::from(c.y[i]) + shape * dx) as i32 as i16)
}

struct Ctx<'a> {
    f: &'a Fields,
    /// The record's tone bytes, for the one setter that reads the tone back.
    tone: &'a [u8],
    reads: Cell<u128>,
    out: Vec<Write>,
}

impl Ctx<'_> {
    /// Another field's value; remembered, for the proven-state check.
    fn f(&self, i: usize) -> i32 {
        self.reads.set(self.reads.get() | 1u128 << i);
        self.f[i]
    }

    fn put(&mut self, block: u8, inst: u8, param: u16, value: i32) {
        self.out.push(Write {
            block,
            inst,
            param,
            value,
        });
    }

    /// `put` to partial 1 and partial 2.
    fn put2(&mut self, block: u8, param: u16, value: i32) {
        self.put(block, 0, param, value);
        self.put(block, 1, param, value);
    }

    fn tone(&self, block: u8, param: u16) -> i32 {
        let w = Write {
            block,
            inst: 0,
            param,
            value: 0,
        };
        locate(&w).map_or(0, |(at, size)| read_le(self.tone, at, size))
    }
}

/// A C `short` parameter: the low 16 bits, sign-extended.
fn short(v: i32) -> i32 {
    i32::from(v as i16)
}

pub(super) fn read_le(bytes: &[u8], at: usize, size: usize) -> i32 {
    bytes[at..at + size]
        .iter()
        .rev()
        .fold(0, |v, &b| v << 8 | i32::from(b))
}

/// What setting one field does.
pub(super) struct Effect {
    pub writes: Vec<Write>,
    /// Bit `i` set: the setters read field `i`.
    pub reads: u128,
}

/// Setting field `k` to `new` while the others hold `fields` (`fields[k]` is the old value): the
/// field's setter with the old state, then the dependency switch with the new one.
pub(super) fn set_field(fields: &Fields, tone: &[u8], k: usize, new: i32) -> Effect {
    let mut after = *fields;
    after[k] = new;
    let mut c = Ctx {
        f: fields,
        tone,
        reads: Cell::new(0),
        out: Vec::new(),
    };
    setter(&mut c, k, new);
    let (mut writes, reads) = (c.out, c.reads.get());
    let mut c = Ctx {
        f: &after,
        tone,
        reads: Cell::new(reads),
        out: Vec::new(),
    };
    depend(&mut c, k);
    writes.append(&mut c.out);
    Effect {
        writes,
        reads: c.reads.get(),
    }
}

/// Every write of loading `fields`: each field set in index order with all of them in place.
pub(super) fn all_writes(fields: &Fields, tone: &[u8]) -> Vec<Write> {
    let mut c = Ctx {
        f: fields,
        tone,
        reads: Cell::new(0),
        out: Vec::new(),
    };
    for (k, &v) in fields.iter().enumerate() {
        setter(&mut c, k, v);
        depend(&mut c, k);
    }
    c.out
}

/// The dependency switch: fields whose setters depend on field `k`.
fn depend(c: &mut Ctx, k: usize) {
    match k {
        OSC1_DEST => {
            osc1_mod(c, c.f(4));
            osc1_pitch_env(c, c.f(5));
        }
        OSC1_WAVE => {
            osc1_level(c, c.f(37));
            pulse_width(c, c.f(OSC1_PW));
        }
        OSC1_PW_MODE => pulse_width(c, c.f(OSC1_PW)),
        OSC2_WAVE => {
            osc2_level(c, c.f(38));
            pulse_width(c, c.f(OSC1_PW));
        }
        OSC2_MODE => {
            osc2_coarse(c, c.f(21));
            if c.f(OSC2_MODE) == 0 {
                c.put(S_PTL, 1, 0x86e, 0);
            } else {
                let v = low_freq_depth(c.f(23));
                c.put(S_PTL, 1, 0x86e, v);
            }
        }
        MOD_MODE => {
            osc1_level(c, c.f(37));
            osc2_level(c, c.f(38));
        }
        FILTER_TYPE => {
            filter_freq(c, c.f(47));
            filter_reso(c, c.f(48));
            filter_env_depth(c, c.f(49));
        }
        FILTER_ENV_SELECT => {
            env1_key_follow(c, c.f(63));
            env1_attack(c, c.f(58));
            env1_decay(c, c.f(59));
            env1_sustain(c, c.f(60));
            env1_release(c, c.f(61));
            amp_env_select_dependents(c);
        }
        AMP_ENV_SELECT => amp_env_select_dependents(c),
        EXPANSION if c.f(EXPANSION) == 0 => {
            let v = curve(&LFO_RATE_CURVE, short(c.f(2)));
            c.put(LFO, 0, 0x670, v);
            c.put(LFO, 0, 0x6a1, v);
            filter_freq(c, c.f(47));
            filter_reso(c, c.f(48));
            filter_env_depth(c, c.f(49));
        }
        _ => {}
    }
}

fn amp_env_select_dependents(c: &mut Ctx) {
    env2_attack(c, c.f(64));
    env2_decay(c, c.f(65));
    env2_sustain(c, c.f(66));
    env2_release(c, c.f(67));
}

/// The setter of field `k`; fields without one do nothing.
fn setter(c: &mut Ctx, k: usize, v: i32) {
    match k {
        1 => lfo_wave(c, v),
        2 => lfo_rate(c, v),
        3 => lfo_delay(c, v),
        4 => osc1_mod(c, v),
        5 => osc1_pitch_env(c, v),
        8 => osc1_wave(c, v),
        9 => osc1_feet(c, v),
        OSC1_PW => pulse_width(c, v),
        OSC2_WAVE => osc2_wave(c, v),
        21 => osc2_coarse(c, v),
        22 => c.put(PTL, 1, 0x747, v),
        23 => {
            let d = if c.f(OSC2_MODE) != 0 {
                low_freq_depth(v)
            } else {
                0
            };
            c.put(S_PTL, 1, 0x86e, d);
        }
        35 => c.put(S_PMT, 0, 0x851, v),
        MOD_MODE => c.put(S_PMT, 0, 0x849, if v == 1 || v == 3 { v } else { 0 }),
        37 => osc1_level(c, v),
        38 => osc2_level(c, v),
        FILTER_TYPE => {
            if (v.wrapping_sub(1) as u32) <= 2 {
                c.put(S_PTL, 0, 0x866, v);
                c.put(S_PTL, 1, 0x866, v);
            }
        }
        46 => {
            let u = curve(&HPF_CURVE, short(v));
            c.put2(S_PTL, 0x861, u);
        }
        47 => filter_freq(c, v),
        48 => filter_reso(c, v),
        49 => filter_env_depth(c, v),
        51 => {
            if v == 0 || v == 2 {
                c.put2(S_PTL, 0x863, v);
            }
        }
        52 => {
            if v >= 0 {
                let u = curve(&KEY_FOLLOW_CURVE, short(v));
                c.put2(PTL, 0x765, u);
            }
        }
        53 => {
            if v >= 0 {
                let u = curve(&FILTER_MOD_CURVE, short(v));
                c.put2(LFO, 0x679, u);
            }
        }
        54 => {
            let u = curve(&AMP_LEVEL_CURVE, short(v));
            c.put(CMN, 0, 0x705, u);
        }
        56 => c.put2(LFO, 0x67a, AMP_MOD_STEP[v as usize & 3]),
        58 => env1_attack(c, v),
        59 => env1_decay(c, v),
        60 => env1_sustain(c, v),
        61 => env1_release(c, v),
        63 => env1_key_follow(c, v),
        64 => env2_attack(c, v),
        65 => env2_decay(c, v),
        66 => env2_sustain(c, v),
        67 => env2_release(c, v),
        70 => {
            if v == 0 {
                c.put2(PTL, 0x771, 0);
            } else if v == 2 {
                c.put2(PTL, 0x771, 0x32);
            }
        }
        74 => c.put(S_CMN, 0, 0x846, v),
        76 => {
            let u = curve(&BENDER_FILTER_CURVE, short(v));
            c.put2(PTL, 0x77b, u);
        }
        77 => {
            let u = curve(&MOD_LFO_CURVE, short(v));
            c.put2(PTL, 0x780, u);
        }
        78 => {
            if (v as u32) <= 0x4b0 {
                c.put(S_CMN, 0, 0x844, v % 100);
                c.put(S_CMN, 0, 0x845, v % 100);
                c.put(CMN, 0, 0x715, v / 100);
                c.put(CMN, 0, 0x716, v / 100);
            }
        }
        79 => {
            let u = curve(&PORTA_TIME_CURVE, short(v));
            c.put(CMN, 0, 0x714, u);
        }
        80 => porta_mode(c, v),
        81 => key_mode(c, v),
        83 => c.put(S_CMN, 0, 0x848, v),
        84 => c.put(CMN, 0, 0x71d, PORTA_CURVE[v as usize & 3]),
        85 => c.put2(PTL, 0x792, v),
        86 => c.put2(PTL, 0x794, v),
        87 => c.put2(PTL, 0x796, v),
        _ => {}
    }
}

fn lfo_wave(c: &mut Ctx, v: i32) {
    if (v as u32) < 8 && (1 << v) & 0x99 != 0 {
        c.put2(LFO, 0x66c, v);
        c.put2(LFO, 0x69d, v);
    }
}

fn lfo_rate(c: &mut Ctx, v: i32) {
    let v = if c.f(EXPANSION) == 0 {
        curve(&LFO_RATE_CURVE, short(v))
    } else {
        v
    };
    c.put(LFO, 0, 0x670, v);
    c.put(LFO, 0, 0x6a1, v);
}

fn lfo_delay(c: &mut Ctx, v: i32) {
    let v = short(v);
    let delay = curve(&LFO_DELAY_CURVE, v);
    let fade = curve(&LFO_FADE_CURVE, v);
    c.put2(LFO, 0x673, delay);
    c.put2(LFO, 0x676, fade);
}

/// OSC1 MOD and OSC1 PITCH ENV DEPTH: the value goes to the partial(s) the destination names.
fn by_destination(c: &mut Ctx, block: u8, param: u16, v: i32) {
    match c.f(OSC1_DEST) {
        0 => {
            c.put(block, 0, param, v);
            c.put(block, 1, param, 0);
        }
        1 => {
            c.put(block, 0, param, v);
            c.put(block, 1, param, v);
        }
        2 => {
            c.put(block, 0, param, 0);
            c.put(block, 1, param, v);
        }
        _ => {}
    }
}

fn osc1_mod(c: &mut Ctx, v: i32) {
    if v >= 0 {
        let u = curve(&OSC1_MOD_CURVE, short(v));
        by_destination(c, LFO, 0x678, u);
    }
}

fn osc1_pitch_env(c: &mut Ctx, v: i32) {
    let v = short(v);
    let u = curve(&OSC1_PENV_CURVE, v);
    by_destination(c, PENV, 0x6ce, u);
}

fn osc1_wave(c: &mut Ctx, v: i32) {
    let (kind, form, sync) = match v {
        0xb => (2, 0, 0x2d),
        0xc => (1, 0, 0),
        0xd | 0xe => (1, 1, 0),
        _ => return,
    };
    c.put(S_PTL, 0, 0x85a, kind);
    c.put(S_PTL, 0, 0x85b, form);
    c.put(S_PTL, 0, 0x85c, sync);
}

fn osc2_wave(c: &mut Ctx, v: i32) {
    c.put(S_PTL, 1, 0x85d, 0x41);
    let (kind, form, sync) = match v {
        10 => (2, 0, 0x2e),
        0xc => (1, 0, 0),
        0xd => (1, 1, 0),
        0xf => (4, 0, 0),
        _ => return,
    };
    c.put(S_PTL, 1, 0x85a, kind);
    c.put(S_PTL, 1, 0x85b, form);
    c.put(S_PTL, 1, 0x85c, sync);
}

fn osc1_feet(c: &mut Ctx, v: i32) {
    let coarse = match v {
        1 => -12,
        2 => 0,
        3 => 12,
        4 => 24,
        _ => return,
    };
    c.put(PTL, 0, 0x746, coarse);
}

fn osc2_coarse(c: &mut Ctx, v: i32) {
    if c.f(OSC2_MODE) == 0 {
        if (v.wrapping_add(12) as u32) > 0x24 {
            return;
        }
        c.put(PTL, 1, 0x746, v);
        c.put(PTL, 1, 0x761, 100);
    } else {
        c.put(PTL, 1, 0x746, 0);
        c.put(PTL, 1, 0x761, 0);
    }
}

/// OSC2 LOW FREQ PITCH as the tone's pitch-down depth.
fn low_freq_depth(v: i32) -> i32 {
    let mut t = v * -0x11b;
    if t < 0 {
        t += 0x3ff;
    }
    ((t >> 10) + 0x48).clamp(0x25, 0x48)
}

/// `(v * k) >> 10` with the compiler's round-toward-zero, floored at zero.
fn scale10(v: i32, k: i32) -> i32 {
    let t = v * k;
    let t = if t < 0 { t + 0x3ff } else { t };
    (t >> 10) & !(t >> 31)
}

/// The level pair the mixer sends to the ring and cross-mod stages and the partial's attenuation.
fn osc_level(c: &mut Ctx, v: i32, wave: i32, other: bool) {
    let v = short(v);
    let level = curve(&OSC1_LEVEL_CURVE, v);
    let sv = short(level);
    // (multiplier, top) per waveform: the attenuation is `scale10(level, m)` up to `top`
    let (m, top) = match (other, wave) {
        (false, 0xb) => (0x13a, 0x4e),
        (false, 0xc) => (0x1ae, 0x6b),
        (false, 0xd | 0xe) => (0x324, 200),
        (true, 10) => (0x10e, 0x43),
        (true, 0xc) => (0x1ae, 0x6b),
        (true, 0xd) => (0x34c, 0xd2),
        (true, 0xf) => (0x122, 0x48),
        _ => return,
    };
    let att = match c.f(MOD_MODE) {
        1 => scale10(sv, m).min(top),
        3 => top,
        _ => 0,
    };
    let inst = u8::from(other);
    c.put(S_PTL, inst, 0x867, att);
    c.put(S_PMT, inst, 0x84b, 0);
    if c.f(MOD_MODE) == 3 {
        c.put(S_PMT, inst, if other { 0x854 } else { 0x853 }, level / 2);
    }
}

fn osc1_level(c: &mut Ctx, v: i32) {
    let wave = c.f(OSC1_WAVE);
    osc_level(c, v, wave, false);
}

fn osc2_level(c: &mut Ctx, v: i32) {
    let wave = c.f(OSC2_WAVE);
    osc_level(c, v, wave, true);
}

/// Pulse width and PWM depth of both oscillators from OSC1 PULSE WIDTH.
fn pulse_width(c: &mut Ctx, v: i32) {
    let v = short(v);
    let (w1, w2) = (c.f(OSC1_WAVE), c.f(OSC2_WAVE));
    // what each oscillator's pulse width and the tone's unlocated parameter 0x789 take
    let pw = |wave: i32, square: i32| -> bool { wave == square };
    match c.f(OSC1_PW_MODE) {
        1 => {
            let a = if pw(w1, 0xd) {
                curve(&PW_A_CURVE, v)
            } else if w1 == 0xe {
                0x41
            } else {
                0x40
            };
            let b = if pw(w2, 0xd) {
                curve(&PW_A_CURVE, v)
            } else if w2 == 0xf {
                0x41
            } else {
                0x40
            };
            c.put(S_PTL, 0, 0x85d, a);
            c.put(S_PTL, 1, 0x85d, b);
            c.put2(PTL, 0x789, 0);
            c.put2(S_PTL, 0x85e, 0);
        }
        2 => {
            let (n1, a) = if pw(w1, 0xd) {
                (curve(&PW_B_CURVE, v), 0x41)
            } else {
                (0, if w1 == 0xe { 0x41 } else { 0x40 })
            };
            let (n2, b) = if pw(w2, 0xd) {
                (curve(&PW_B_CURVE, v), 0x41)
            } else {
                (0, 0x40)
            };
            c.put(S_PTL, 0, 0x85d, a);
            c.put(S_PTL, 1, 0x85d, b);
            c.put2(S_PTL, 0x85e, 0);
            c.put(PTL, 0, 0x789, n1);
            c.put(PTL, 1, 0x789, n2);
        }
        0 => {
            let (a, d1) = if pw(w1, 0xd) {
                (curve(&PW_C_CURVE, v), curve(&PW_D_CURVE, v))
            } else {
                (if w1 == 0xe { 0x41 } else { 0x40 }, 0)
            };
            let (b, d2) = if pw(w2, 0xd) {
                (curve(&PW_C_CURVE, v), curve(&PW_D_CURVE, v))
            } else {
                (if w2 == 0xf { 0x41 } else { 0x40 }, 0)
            };
            c.put(S_PTL, 0, 0x85d, a);
            c.put(S_PTL, 1, 0x85d, b);
            c.put(S_PTL, 0, 0x85e, d1);
            c.put(S_PTL, 1, 0x85e, d2);
            c.put2(PTL, 0x789, 0);
        }
        _ => {}
    }
}

/// Type 1 is the curve; types 2 and 3 a straight line with a floor and a ceiling. With the
/// parameter expansion switch on, the value goes through unchanged.
fn filter_freq(c: &mut Ctx, v: i32) {
    let mut v = v;
    if c.f(EXPANSION) == 0 {
        v = match c.f(FILTER_TYPE) {
            2 => ((scale_signed(v, 0x3ae)) + 0xb).clamp(0xb, 0x3b8),
            3 => ((scale_signed(v, 0x3c5)) + 0x1c).clamp(0x1c, 0x3e0),
            1 => curve(&FREQ_CURVE, short(v)),
            _ => return,
        };
    }
    c.put2(PTL, 0x764, v);
}

/// `(v * k) >> 10`, rounding toward zero.
fn scale_signed(v: i32, k: i32) -> i32 {
    let t = v * k;
    (if t < 0 { t + 0x3ff } else { t }) >> 10
}

fn filter_reso(c: &mut Ctx, v: i32) {
    let mut v = v;
    if c.f(EXPANSION) == 0 {
        v = match c.f(FILTER_TYPE) {
            2 => scale10(v, 0x2b4).min(0x2b3),
            3 => scale10(v, 0x231).min(0x230),
            1 => curve(&RESO_CURVE, short(v)),
            _ => return,
        };
    }
    c.put2(PTL, 0x768, v);
}

/// FILTER ENV DEPTH: type 1 uses the curve, the others (and the expansion switch) a line; the
/// result is split over the filter envelope depth and the synth partial's fine depth.
fn filter_env_depth(c: &mut Ctx, v: i32) {
    let u = if c.f(EXPANSION) == 0 && c.f(FILTER_TYPE) == 1 {
        curve(&FENV_DEPTH_CURVE, short(v))
    } else if c.f(EXPANSION) == 0 && !(1..=3).contains(&c.f(FILTER_TYPE)) {
        return;
    } else {
        let u = scale10(v.abs(), 0xf85).min(0xf81);
        if v < 0 {
            -u
        } else {
            u
        }
    };
    c.put2(S_PTL, 0x86a, u % 0x3f);
    c.put2(FENV, 0x6dc, u / 0x3f);
}

fn env1_attack(c: &mut Ctx, v: i32) {
    let v = short(v);
    let u = curve(&ENV1_ATTACK_CURVE, v);
    c.put2(PENV, 0x6d2, u);
    if c.f(FILTER_ENV_SELECT) == 0 {
        c.put2(FENV, 0x6e1, u);
        c.put2(PTL, 0x751, 0);
    }
}

fn env1_decay(c: &mut Ctx, v: i32) {
    let v = short(v);
    let u = curve(&ENV1_DECAY_CURVE, v);
    c.put2(PENV, 0x6d4, u);
    if c.f(FILTER_ENV_SELECT) == 0 {
        c.put2(FENV, 0x6e3, u);
    }
}

fn env1_sustain(c: &mut Ctx, v: i32) {
    let v = short(v);
    let u = curve(&ENV1_SUSTAIN_P_CURVE, v);
    c.put2(PENV, 0x6d9, u);
    if c.f(FILTER_ENV_SELECT) == 0 {
        let u = curve(&ENV1_SUSTAIN_F_CURVE, v);
        c.put2(FENV, 0x6e8, u);
    }
}

fn env1_release(c: &mut Ctx, v: i32) {
    let v = short(v);
    let u = curve(&ENV1_RELEASE_CURVE, v);
    c.put2(PENV, 0x6d5, u);
    if c.f(FILTER_ENV_SELECT) == 0 {
        c.put2(FENV, 0x6e4, u);
    }
}

fn env1_key_follow(c: &mut Ctx, v: i32) {
    let step = if c.f(FILTER_ENV_SELECT) == 0 {
        match v {
            2 => -50,
            5 => 50,
            0 => 0,
            _ => return,
        }
    } else {
        0
    };
    c.put2(PTL, 0x770, step);
}

/// ENV2 stage: AMP ENV SELECT 0 and 1 send it to the amp envelope; FILTER ENV SELECT 1 also to
/// the filter envelope.
fn env2(c: &mut Ctx, curve_of: &MdlCurve, amp: u16, filter: u16, v: i32) {
    if c.f(AMP_ENV_SELECT) > 1 {
        return;
    }
    let u = curve(curve_of, short(v));
    c.put2(AENV, amp, u);
    if c.f(FILTER_ENV_SELECT) == 1 {
        c.put2(FENV, filter, u);
    }
}

fn env2_attack(c: &mut Ctx, v: i32) {
    env2(c, &ATTACK_CURVE, 0x6ec, 0x6e1, v);
}

fn env2_decay(c: &mut Ctx, v: i32) {
    env2(c, &ENV2_DECAY_CURVE, 0x6ee, 0x6e3, v);
}

fn env2_sustain(c: &mut Ctx, v: i32) {
    env2(c, &ENV2_SUSTAIN_CURVE, 0x6f2, 0x6e8, v);
}

fn env2_release(c: &mut Ctx, v: i32) {
    env2(c, &RELEASE_CURVE, 0x6ef, 0x6e4, v);
}

fn porta_mode(c: &mut Ctx, v: i32) {
    match v {
        0 | 1 => {
            c.put(CMN, 0, 0x710, v);
            c.put(CMN, 0, 0x711, 0);
        }
        2 => {
            c.put(CMN, 0, 0x710, 1);
            c.put(CMN, 0, 0x711, 1);
        }
        _ => {}
    }
}

/// KEY MODE: mono/poly and analog feel in the common block; the two unison modes also switch the
/// unison on (if it is off) and set its detune. Reads the unison switch back from the tone.
fn key_mode(c: &mut Ctx, v: i32) {
    let unison = c.tone(S_CMN, 0x841);
    match v {
        0 | 1 => {
            c.put(CMN, 0, 0x70d, 1 - v);
            c.put(CMN, 0, 0x70c, 0);
            if unison != 0 {
                c.put(S_CMN, 0, 0x841, 0);
            }
        }
        2 | 3 => {
            c.put(CMN, 0, 0x70d, 3 - v);
            c.put(CMN, 0, 0x70c, 5);
            if unison == 0 {
                c.put(S_CMN, 0, 0x841, 1);
            }
            c.put(S_CMN, 0, 0x843, 0x1e);
        }
        _ => {}
    }
}
