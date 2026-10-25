//! What the instrument's analog model engines have in common: how
//! an engine field lands in the ZEN-Core tone of the same record.
//!
//! Each class has a table of per-field setters and a dependency switch (the second virtual
//! method). The instrument sets a field by running its setter with the new value, storing the value,
//! then running the switch for that field index, which re-runs the setters of the fields that
//! depend on it. Each setter writes tone parameters named by the instrument's (block, parameter)
//! pair; [`locate`] turns a pair into a record offset. The classes are in [`super::jp8`],
//! [`super::juno`], [`super::jx8p`] and [`super::sh101`]; names and offsets are in
//! `docs/FORMAT.md`.

use std::cell::Cell;

use crate::params::mdl::{TONE_BLOCKS, TONE_TARGETS};
pub(super) use crate::params::MdlCurve;
use crate::params::MdlTarget;

/// Number of engine fields of an analog model.
pub(super) const FIELDS: usize = 88;

/// The engine fields of a record, by index, as the setters see them.
pub(super) type Fields = [i32; FIELDS];

// Block ids of the tone parameter store.
pub(super) const CMN: u8 = 22;
pub(super) const PMT: u8 = 24;
pub(super) const PTL: u8 = 25;
pub(super) const PENV: u8 = 26;
pub(super) const FENV: u8 = 27;
pub(super) const AENV: u8 = 28;
pub(super) const LFO: u8 = 29;
pub(super) const EQ: u8 = 30;
pub(super) const S_CMN: u8 = 31;
pub(super) const S_PMT: u8 = 32;
pub(super) const S_PTL: u8 = 33;

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
    if c.mode == 2 {
        let t = dx / c.scale;
        let shape = (c.c[i] + (c.b[i] + t * c.a[i]) * t) * t * c.scale;
        return signed((shape as i32 as i16).wrapping_add(c.y[i]));
    }
    let width = i32::from(c.x[i + 1]) - i32::from(c.x[i]);
    let t = dx / width as f32;
    let shape = c.c[i] + (c.b[i] + t * c.a[i]) * t;
    signed((f32::from(c.y[i]) + shape * dx) as i32 as i16)
}

/// What a setter sees: the engine fields, the tone read back, and where it writes.
pub(super) struct Ctx<'a> {
    f: &'a Fields,
    /// The record's tone bytes, for the setters that read the tone back.
    tone: &'a [u8],
    reads: Cell<u128>,
    out: Vec<Write>,
}

impl Ctx<'_> {
    /// Another field's value; remembered, for the proven-state check.
    pub fn f(&self, i: usize) -> i32 {
        self.reads.set(self.reads.get() | 1u128 << i);
        self.f[i]
    }

    pub fn put(&mut self, block: u8, inst: u8, param: u16, value: i32) {
        self.out.push(Write {
            block,
            inst,
            param,
            value,
        });
    }

    /// `put` to partial 1 and partial 2.
    pub fn put2(&mut self, block: u8, param: u16, value: i32) {
        self.put(block, 0, param, value);
        self.put(block, 1, param, value);
    }

    /// `put` to all four partials.
    pub fn put4(&mut self, block: u8, param: u16, value: i32) {
        for inst in 0..4 {
            self.put(block, inst, param, value);
        }
    }

    /// The tone parameter's current value in the record; 0 where its place is not known.
    pub fn tone(&self, block: u8, param: u16) -> i32 {
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
pub(super) fn short(v: i32) -> i32 {
    i32::from(v as i16)
}

/// `(v * k) >> 10` with the compiler's round-toward-zero, floored at zero.
pub(super) fn scale10(v: i32, k: i32) -> i32 {
    let t = v * k;
    let t = if t < 0 { t + 0x3ff } else { t };
    (t >> 10) & !(t >> 31)
}

/// `(v * k) >> 10`, rounding toward zero.
pub(super) fn scale_signed(v: i32, k: i32) -> i32 {
    let t = v * k;
    (if t < 0 { t + 0x3ff } else { t }) >> 10
}

pub(super) fn read_le(bytes: &[u8], at: usize, size: usize) -> i32 {
    bytes[at..at + size]
        .iter()
        .rev()
        .fold(0, |v, &b| v << 8 | i32::from(b))
}

/// The filter of a model whose FILTER TYPE 0 is the curve (JUNO-106, JX-8P, SH-101): types 2 and 3
/// are straight lines with a floor and a ceiling, type 1 writes nothing, and with the parameter
/// expansion switch on the value goes through unchanged.
pub(super) struct Filter {
    pub freq: &'static MdlCurve,
    pub reso: &'static MdlCurve,
    pub env_depth: &'static MdlCurve,
    /// The partials of the tone the model writes.
    pub partials: &'static [u8],
}

impl Filter {
    const TYPE: usize = 44;
    const FREQ: usize = 47;
    const RESO: usize = 48;
    const ENV_DEPTH: usize = 49;
    const EXPANSION: usize = 82;

    /// All three, with the fields' own values.
    pub fn all(&self, c: &mut Ctx) {
        self.set_freq(c, c.f(Self::FREQ));
        self.set_reso(c, c.f(Self::RESO));
        self.set_env_depth(c, c.f(Self::ENV_DEPTH));
    }

    fn put(&self, c: &mut Ctx, block: u8, param: u16, v: i32) {
        for &inst in self.partials {
            c.put(block, inst, param, v);
        }
    }

    pub fn set_freq(&self, c: &mut Ctx, v: i32) {
        let mut v = v;
        if c.f(Self::EXPANSION) == 0 {
            v = match c.f(Self::TYPE) {
                0 => curve(self.freq, short(v)),
                2 => (scale_signed(v, 0x3ae) + 0xb).clamp(0xb, 0x3b8),
                3 => (scale_signed(v, 0x3c5) + 0x1c).clamp(0x1c, 0x3e0),
                _ => return,
            };
        }
        self.put(c, PTL, 0x764, v);
    }

    pub fn set_reso(&self, c: &mut Ctx, v: i32) {
        let mut v = v;
        if c.f(Self::EXPANSION) == 0 {
            v = match c.f(Self::TYPE) {
                0 => curve(self.reso, short(v)),
                2 => scale10(v, 0x2b4).min(0x2b3),
                3 => scale10(v, 0x231).min(0x230),
                _ => return,
            };
        }
        self.put(c, PTL, 0x768, v);
    }

    /// The result is split over the filter envelope depth and the synth partial's fine depth.
    pub fn set_env_depth(&self, c: &mut Ctx, v: i32) {
        let u = if c.f(Self::EXPANSION) == 0 && c.f(Self::TYPE) == 0 {
            curve(self.env_depth, short(v))
        } else if c.f(Self::EXPANSION) == 0 && !matches!(c.f(Self::TYPE), 2 | 3) {
            return;
        } else {
            let u = scale10(v.abs(), 0xf85).min(0xf81);
            if v < 0 {
                -u
            } else {
                u
            }
        };
        self.put(c, S_PTL, 0x86a, u % 0x3f);
        self.put(c, FENV, 0x6dc, u / 0x3f);
    }
}

/// PORTAMENT MODE: OFF and ON; AUTO is ON with the second flag set.
pub(super) fn porta_mode(c: &mut Ctx, v: i32) {
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

/// BENDER PITCH DEPTH in cents: the tone keeps semitones in the common block and the remainder in
/// the synth block.
pub(super) fn bender_depth(c: &mut Ctx, v: i32, max: u32) {
    if (v as u32) <= max {
        c.put(S_CMN, 0, 0x844, v % 100);
        c.put(S_CMN, 0, 0x845, v % 100);
        c.put(CMN, 0, 0x715, v / 100);
        c.put(CMN, 0, 0x716, v / 100);
    }
}

/// KEY MODE: mono/poly and analog feel in the common block; the two unison modes also switch the
/// unison on (if it is off) and set its detune. Reads the unison switch back from the tone.
pub(super) fn key_mode(c: &mut Ctx, v: i32, detune: i32) {
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
            c.put(S_CMN, 0, 0x843, detune);
        }
        _ => {}
    }
}

/// An ENV2 stage of a model that sends it to every partial's amp envelope unless AMP ENV SELECT
/// is 2 or 3, and always to the filter envelope (with its own curve when the amp one is off).
pub(super) fn env2_all(
    c: &mut Ctx,
    v: i32,
    (amp_curve, filter_curve): (&MdlCurve, &MdlCurve),
    (amp, filter): (u16, u16),
) {
    let v = short(v);
    let amp_on = c.f(57) < 2;
    let u = curve(amp_curve, v);
    let f = if amp_on { u } else { curve(filter_curve, v) };
    if amp_on {
        c.put4(AENV, amp, u);
    }
    c.put4(FENV, filter, f);
}

/// What setting one field does.
pub(super) struct Effect {
    pub writes: Vec<Write>,
    /// Bit `i` set: the setters read field `i`.
    pub reads: u128,
}

/// One model class: the setter of each field (a field without one does nothing) and the
/// dependency switch.
pub(super) struct Model {
    pub setter: fn(&mut Ctx, usize, i32),
    pub depend: fn(&mut Ctx, usize),
}

impl Model {
    /// Setting field `k` to `new` while the others hold `fields` (`fields[k]` is the old value):
    /// the field's setter with the old state, then the dependency switch with the new one.
    pub fn set_field(&self, fields: &Fields, tone: &[u8], k: usize, new: i32) -> Effect {
        let mut after = *fields;
        after[k] = new;
        let mut c = Ctx {
            f: fields,
            tone,
            reads: Cell::new(0),
            out: Vec::new(),
        };
        (self.setter)(&mut c, k, new);
        let (mut writes, reads) = (c.out, c.reads.get());
        let mut c = Ctx {
            f: &after,
            tone,
            reads: Cell::new(reads),
            out: Vec::new(),
        };
        (self.depend)(&mut c, k);
        writes.append(&mut c.out);
        Effect {
            writes,
            reads: c.reads.get(),
        }
    }

    /// Every write of loading `fields`: each field set in index order with all of them in place.
    pub fn all_writes(&self, fields: &Fields, tone: &[u8]) -> Vec<Write> {
        let mut c = Ctx {
            f: fields,
            tone,
            reads: Cell::new(0),
            out: Vec::new(),
        };
        for (k, &v) in fields.iter().enumerate() {
            (self.setter)(&mut c, k, v);
            (self.depend)(&mut c, k);
        }
        c.out
    }
}
