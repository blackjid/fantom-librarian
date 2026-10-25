//! Reading and editing MODEL (`MDLa`) tone parameters by name.
//!
//! A record is 2048 bytes: a header whose `+8` type picks the engine, a ZEN-Core tone at `0x10`,
//! and the engine block at `0x670` described by [`params::mdl`]. The instrument keeps derived
//! values of some engine fields in the ZEN-Core tone, so an edit must move those too; only JP-8
//! edits whose mirror is proven against recorded tones are accepted. Layout and the mirror rules
//! are in `docs/FORMAT.md`; the instrument's setters are ported in [`jp8`].

mod jp8;

use std::collections::BTreeMap;
use std::fmt;

use crate::container::{patch_record, Raw, RecordTable, Svd};
use crate::params::mdl::{ANALOG, ENGINE_BASE, JD800, JP8_PINNED, RECORD_LEN, WT};
use crate::params::{MdlMirror, MdlParam};
use crate::{Error, Result};

const TAG: &[u8; 4] = b"MDLa";
const TYPE_AT: usize = 8;
/// Record type of a JUPITER-8 tone, the one engine whose mirrors are decoded.
const JP8: u32 = 7;

/// Which engine a record's type selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdlEngine {
    Analog,
    Jd800,
    Wt,
}

impl MdlEngine {
    fn table(self) -> &'static [MdlParam] {
        match self {
            Self::Analog => ANALOG,
            Self::Jd800 => JD800,
            Self::Wt => WT,
        }
    }
}

/// Whether a parameter can be set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdlEdit {
    /// Reaches nothing else in the record.
    Free,
    /// Also moves its proven mirror in the ZEN-Core tone.
    Mirrored,
    /// Refused: [`set_mdl_param`] says why.
    Locked,
}

/// One engine parameter of a record: its table entry and stored value.
#[derive(Debug)]
pub struct MdlField {
    pub param: &'static MdlParam,
    pub value: i32,
    pub edit: MdlEdit,
}

impl MdlField {
    /// The enumeration name of the value, if the parameter has one.
    pub fn label(&self) -> Option<&'static str> {
        let at = usize::try_from(self.value - self.param.min).ok()?;
        self.param.labels.get(at).copied()
    }
}

impl fmt::Display for MdlField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.label() {
            Some(label) => write!(f, "{} {label}", self.value),
            None => write!(f, "{}", self.value),
        }
    }
}

/// Every engine parameter of MODEL record `index`, in record order, and its record type.
pub fn read_mdl_params(raw: &Raw, index: usize) -> Result<(u32, Vec<MdlField>)> {
    let svd = Svd::parse(raw)?;
    let table = table(raw, &svd)?;
    let record = record(&table, index)?;
    let (engine, kind) = engine_of(record)?;
    let fields = engine
        .table()
        .iter()
        .map(|param| MdlField {
            param,
            value: read(record, param),
            edit: edit_of(kind, param),
        })
        .collect();
    Ok((kind, fields))
}

/// Set parameter `name` of MODEL record `index` from a number or an enumeration name.
///
/// Names match case-insensitively with `_` for a space. The value is range-checked, and the
/// ZEN-Core mirror of the parameter moves with it. A parameter whose mirror is not proven, and any
/// parameter of an engine other than JP-8, is refused with the reason. The per-record CRC-32 an
/// `.svz` area carries is refreshed; an SVD area has none.
pub fn set_mdl_param(raw: &mut Raw, index: usize, name: &str, value: &str) -> Result<()> {
    let patches = {
        let svd = Svd::parse(raw)?;
        let table = table(raw, &svd)?;
        let record = record(&table, index)?;
        let (engine, kind) = engine_of(record)?;
        let param = find(engine, name)?;
        refuse_locked(kind, param)?;
        let new = parse(param, value)?;
        let mut patches = vec![(param.offset as usize, encode(param, new))];
        patches.extend(mirror(record, param, new)?);
        patches
    };
    let patches: Vec<(usize, &[u8])> = patches.iter().map(|(at, b)| (*at, b.as_slice())).collect();
    patch_record(raw, TAG, index, &patches)
}

fn table<'a>(raw: &'a Raw, svd: &Svd) -> Result<RecordTable<'a>> {
    RecordTable::from_svd(raw, svd, TAG)?
        .ok_or_else(|| Error::Unrecognized("no MDLa area in file".into()))
}

fn record<'a>(table: &RecordTable<'a>, index: usize) -> Result<&'a [u8]> {
    let record = table.record(index).ok_or_else(|| {
        Error::Unrecognized(format!(
            "MODEL tone {index} out of range (file has {})",
            table.len()
        ))
    })?;
    if record.len() != RECORD_LEN {
        return Err(Error::Unrecognized(format!(
            "MDLa record size is {}, expected {RECORD_LEN}",
            record.len()
        )));
    }
    Ok(record)
}

/// The engine a record's type selects, and that type.
fn engine_of(record: &[u8]) -> Result<(MdlEngine, u32)> {
    let kind = u32::from_le_bytes(record[TYPE_AT..TYPE_AT + 4].try_into().unwrap());
    let engine = match kind {
        7..=10 => MdlEngine::Analog,
        15 => MdlEngine::Jd800,
        16 => MdlEngine::Wt,
        _ => {
            return Err(Error::Unrecognized(format!(
                "MODEL record type {kind} is not a known engine"
            )))
        }
    };
    if engine == MdlEngine::Analog && u32::from(record[ENGINE_BASE]) + 6 != kind {
        return Err(Error::Unrecognized(format!(
            "MODEL record type {kind} disagrees with its model byte {}",
            record[ENGINE_BASE]
        )));
    }
    Ok((engine, kind))
}

fn edit_of(kind: u32, param: &MdlParam) -> MdlEdit {
    if kind != JP8 {
        return MdlEdit::Locked;
    }
    match param.mirror {
        MdlMirror::Free => MdlEdit::Free,
        MdlMirror::Locked | MdlMirror::Unproven => MdlEdit::Locked,
        _ => MdlEdit::Mirrored,
    }
}

fn refuse_locked(kind: u32, param: &MdlParam) -> Result<()> {
    let id = param.id;
    let why = if kind != JP8 {
        match kind {
            7..=10 => {
                "only JP8 edits are supported: the other analog models' mirrors are not decoded"
            }
            _ => "JD-800 and WT edits are not supported: their engine side effects are not decoded",
        }
        .to_string()
    } else {
        match param.mirror {
            MdlMirror::Locked => "it selects the engine".to_string(),
            MdlMirror::Unproven => {
                "it moves fields of the ZEN-Core tone, and that transform is not proven".to_string()
            }
            _ => return Ok(()),
        }
    };
    Err(Error::Unrecognized(format!("cannot set {id}: {why}")))
}

fn find(engine: MdlEngine, name: &str) -> Result<&'static MdlParam> {
    let table = engine.table();
    let norm = |s: &str| s.trim().replace('_', " ").to_ascii_uppercase();
    let needle = norm(name);
    if let Some(param) = table.iter().find(|p| norm(p.id) == needle) {
        return Ok(param);
    }
    let near: Vec<&str> = table
        .iter()
        .filter(|p| !needle.is_empty() && norm(p.id).contains(&needle))
        .map(|p| p.id)
        .take(8)
        .collect();
    Err(Error::Unrecognized(if near.is_empty() {
        format!("no MODEL parameter {name:?}")
    } else {
        format!("no MODEL parameter {name:?}; similar: {}", near.join(", "))
    }))
}

fn read(record: &[u8], param: &MdlParam) -> i32 {
    let at = param.offset as usize;
    match (param.size, param.min < 0) {
        (1, false) => i32::from(record[at]),
        (1, true) => i32::from(record[at] as i8),
        (_, false) => i32::from(u16::from_le_bytes([record[at], record[at + 1]])),
        (_, true) => i32::from(i16::from_le_bytes([record[at], record[at + 1]])),
    }
}

fn encode(param: &MdlParam, value: i32) -> Vec<u8> {
    value.to_le_bytes()[..param.size as usize].to_vec()
}

/// The new value from a number or an enumeration name, checked against its range.
fn parse(param: &MdlParam, text: &str) -> Result<i32> {
    let id = param.id;
    let text = text.trim();
    let value = match text.parse::<i32>() {
        Ok(v) => v,
        Err(_) => {
            let label = param
                .labels
                .iter()
                .position(|l| l.eq_ignore_ascii_case(text))
                .ok_or_else(|| {
                    Error::Unrecognized(format!("{id}: {text:?} is not a number or a known name"))
                })?;
            param.min + label as i32
        }
    };
    if !(param.min..=param.max).contains(&value) {
        return Err(Error::Unrecognized(format!(
            "{id}: {value} is outside {}..={}",
            param.min, param.max
        )));
    }
    Ok(value)
}

/// The engine fields of a JP-8 record, as the instrument's setters see them.
fn jp8_fields(record: &[u8]) -> jp8::Fields {
    let mut fields = [0; jp8::FIELDS];
    for (f, param) in fields.iter_mut().zip(ANALOG) {
        *f = read(record, param);
    }
    fields
}

/// The ZEN-Core fields that must change with `param` set to `value`, as (offset, bytes).
///
/// The instrument's own setters decide (`jp8`). A setter that reads a field the recorded tones never
/// vary is trusted only at the recorded value, a write to a tone parameter whose place is not
/// known is refused unless it writes zero, and an edit whose writes differ from loading the whole
/// new state is refused rather than guessed.
fn mirror(record: &[u8], param: &MdlParam, value: i32) -> Result<Vec<(usize, Vec<u8>)>> {
    if param.mirror != MdlMirror::Mirrored {
        return Ok(Vec::new());
    }
    let id = param.id;
    let k = ANALOG
        .iter()
        .position(|p| std::ptr::eq(p, param))
        .expect("a JP-8 table entry");
    let fields = jp8_fields(record);
    let effect = jp8::set_field(&fields, record, k, value);
    let outside: Vec<String> = JP8_PINNED
        .iter()
        .filter(|&&(i, pinned)| i != k && effect.reads >> i & 1 == 1 && fields[i] != pinned)
        .map(|&(i, pinned)| format!("{} {pinned}", ANALOG[i].id))
        .collect();
    if !outside.is_empty() {
        return Err(Error::Unrecognized(format!(
            "cannot set {id}: its mirror is proven only with {}",
            outside.join(" and ")
        )));
    }
    let mut targets = BTreeMap::new();
    for w in &effect.writes {
        match jp8::locate(w) {
            Some(place) => {
                targets.insert(place, w.value);
            }
            None if w.value == 0 => {}
            None => {
                return Err(Error::Unrecognized(format!(
                    "cannot set {id}: it would write {} to tone parameter {:#x} of block {}, \
                     whose place in the tone is not known",
                    w.value, w.param, w.block
                )))
            }
        }
    }
    let mut after = fields;
    after[k] = value;
    let whole = final_writes(&after, record);
    for (place, v) in &targets {
        if whole.get(place).is_some_and(|w| w != v) {
            return Err(Error::Unrecognized(format!(
                "cannot set {id}: the edit and a full reload of the new state disagree at \
                 {:#x}, so the mirror is not proven for this state",
                place.0
            )));
        }
    }
    Ok(targets
        .into_iter()
        .map(|((at, size), v)| (at, v.to_le_bytes()[..size].to_vec()))
        .collect())
}

/// What loading `fields` leaves in each tone field the setters write: the last write wins.
fn final_writes(fields: &jp8::Fields, record: &[u8]) -> BTreeMap<(usize, usize), i32> {
    let mut last = BTreeMap::new();
    for w in jp8::all_writes(fields, record) {
        if let Some(place) = jp8::locate(&w) {
            last.insert(place, w.value);
        }
    }
    last
}

/// A ZEN-Core tone field that does not hold what the engine block implies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorMismatch {
    /// Record offset and width in bytes.
    pub offset: usize,
    pub size: usize,
    pub stored: i32,
    pub expected: i32,
}

/// Every tone field a JP-8 record's engine block determines that holds something else.
///
/// The check loads the engine fields through the instrument's setters (every field set, in index
/// order) and compares what they write with the tone bytes; the name, the MFX and the fields no
/// setter writes are not looked at. An empty list means the tone is what the instrument would hold.
pub fn mirror_mismatches(record: &[u8]) -> Result<Vec<MirrorMismatch>> {
    let (engine, kind) = engine_of(record)?;
    if engine != MdlEngine::Analog || kind != JP8 || record.len() != RECORD_LEN {
        return Err(Error::Unrecognized(
            "only JP-8 records have a decoded mirror".into(),
        ));
    }
    let mut mismatches = Vec::new();
    for ((offset, size), expected) in final_writes(&jp8_fields(record), record) {
        let stored = jp8::read_le(record, offset, size);
        let mask = (1i64 << (8 * size)) - 1;
        if i64::from(expected) & mask != i64::from(stored) {
            mismatches.push(MirrorMismatch {
                offset,
                size,
                stored,
                expected: (i64::from(expected) & mask) as i32,
            });
        }
    }
    Ok(mismatches)
}

/// Whether `record` is a JP-8 record whose tone agrees with its engine block everywhere the
/// engine determines it.
pub fn mirror_consistent(record: &[u8]) -> bool {
    mirror_mismatches(record).is_ok_and(|m| m.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::crc32;
    use crate::container::PREAMBLE_LEN;

    fn u16_at(raw: &Raw, record_at: usize) -> u16 {
        let svd = Svd::parse(raw).unwrap();
        let record = table(raw, &svd).unwrap().record(0).unwrap();
        u16::from_le_bytes([record[record_at], record[record_at + 1]])
    }

    /// An SVZ whose `MDLa` area holds one record of `kind` with a JP-8-like filter state.
    fn svz(kind: u32, with_crc: bool) -> Raw {
        let mut record = vec![0u8; RECORD_LEN];
        record[TYPE_AT..TYPE_AT + 4].copy_from_slice(&kind.to_le_bytes());
        if (7..=10).contains(&kind) {
            record[ENGINE_BASE] = (kind - 6) as u8;
        }
        let at = |name: &str| ANALOG.iter().find(|p| p.id == name).unwrap().offset as usize;
        record[at("FILTER TYPE")] = 1;
        let info_len = 16 + if with_crc { 4 } else { 0 };
        let mut body = Vec::new();
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&(RECORD_LEN as u32).to_le_bytes());
        body.extend_from_slice(&(info_len as u32).to_le_bytes());
        body.extend_from_slice(&[0u8; 4]);
        if with_crc {
            body.extend_from_slice(&crc32(&record).to_le_bytes());
        }
        body.extend_from_slice(&record);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"SVZa");
        bytes.push(1);
        bytes.push(3);
        bytes.extend_from_slice(b"KY019$");
        bytes.extend_from_slice(&[0u8; 4]);
        bytes.extend_from_slice(b"MDLa");
        bytes.extend_from_slice(b"ZCOR");
        bytes.extend_from_slice(&((PREAMBLE_LEN + 16) as u32).to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&body);
        Raw::from_bytes(bytes)
    }

    fn get(raw: &Raw, name: &str) -> i32 {
        let (_, fields) = read_mdl_params(raw, 0).unwrap();
        fields
            .into_iter()
            .find(|f| f.param.id == name)
            .unwrap()
            .value
    }

    #[test]
    fn tables_cover_the_engine_blocks() {
        assert_eq!((ANALOG.len(), JD800.len(), WT.len()), (88, 368, 286));
        for (table, len) in [(ANALOG, 112), (JD800, 396), (WT, 376)] {
            for p in table {
                let at = p.offset as usize;
                assert!(
                    at >= ENGINE_BASE && at + p.size as usize <= ENGINE_BASE + len,
                    "{}",
                    p.id
                );
            }
        }
    }

    /// Recorded JP-8 tones: engine value -> value the instrument stored in the ZEN-Core tone.
    #[test]
    fn curves_reproduce_recorded_tones() {
        use crate::params::mdl::{ATTACK_CURVE, FREQ_CURVE, RELEASE_CURVE, RESO_CURVE};
        use crate::params::MdlCurve;
        let cases: [(&MdlCurve, &[(i32, i32)]); 4] = [
            (
                &FREQ_CURVE,
                &[
                    (962, 949),
                    (714, 733),
                    (718, 737),
                    (754, 778),
                    (790, 819),
                    (566, 586),
                    (1023, 959),
                ],
            ),
            (
                &RESO_CURVE,
                &[(863, 795), (643, 623), (807, 752), (120, 105), (0, 0)],
            ),
            (&ATTACK_CURVE, &[(299, 21), (465, 129), (869, 684), (0, 0)]),
            (
                &RELEASE_CURVE,
                &[(489, 456), (582, 592), (613, 631), (0, 0)],
            ),
        ];
        for (c, points) in cases {
            for &(from, to) in points {
                assert_eq!(jp8::curve(c, from), to, "{from}");
            }
        }
    }

    #[test]
    fn a_free_value_round_trips_by_number_and_by_name() {
        let mut raw = svz(7, false);
        set_mdl_param(&mut raw, 0, "OSC1 LOW FREQ PITCH", "100").unwrap();
        assert_eq!(get(&raw, "OSC1 LOW FREQ PITCH"), 100);
        set_mdl_param(&mut raw, 0, "osc1_coarse_tune", "-12").unwrap();
        assert_eq!(get(&raw, "OSC1 COARSE TUNE"), -12);
        set_mdl_param(&mut raw, 0, "NOISE", "1").unwrap();
        assert_eq!(get(&raw, "NOISE"), 1);
    }

    #[test]
    fn a_value_out_of_range_is_refused_and_leaves_the_file_alone() {
        let mut raw = svz(7, false);
        let before = raw.bytes().to_vec();
        for bad in ["128", "-1", "x"] {
            assert!(set_mdl_param(&mut raw, 0, "OSC1 LOW FREQ PITCH", bad).is_err());
        }
        assert_eq!(raw.bytes(), before);
    }

    #[test]
    fn setting_the_filter_moves_the_tone_mirror_of_both_partials() {
        let mut raw = svz(7, false);
        set_mdl_param(&mut raw, 0, "FILTER FREQ", "566").unwrap();
        set_mdl_param(&mut raw, 0, "FILTER RESO", "863").unwrap();
        set_mdl_param(&mut raw, 0, "ENV2 ATTACK", "299").unwrap();
        set_mdl_param(&mut raw, 0, "ENV2 RELEASE", "489").unwrap();
        set_mdl_param(&mut raw, 0, "MOD DEPTH", "1068").unwrap();
        assert_eq!(get(&raw, "FILTER FREQ"), 566);
        for (offsets, want) in [
            ([0x100, 0x17c].as_slice(), 586),
            ([0x106, 0x182].as_slice(), 795),
            ([0x38a, 0x39a].as_slice(), 21),
            ([0x390, 0x3a0].as_slice(), 456),
            ([0x5c8].as_slice(), 1068),
        ] {
            for &at in offsets {
                assert_eq!(u16_at(&raw, at), want, "{at:#x}");
            }
        }
    }

    #[test]
    fn the_other_proven_fields_move_their_mirrors() {
        let mut raw = svz(7, false);
        for (name, value) in [
            ("LFO WAVE FORM", "7"),
            ("LFO RATE", "505"),
            ("OSC1 FEET", "3"),
            ("OSC2 COARSE TUNE", "7"),
            ("FILTER HPF", "88"),
            ("AMP LEVEL", "85"),
            ("ENV1 DECAY", "320"),
            ("PITCH DRIFT", "35"),
        ] {
            set_mdl_param(&mut raw, 0, name, value).unwrap();
        }
        let b = |at| u16_at(&raw, at) & 0xff;
        assert_eq!([b(0x3c8), b(0x3fc), b(0x430), b(0x464)], [7; 4]);
        assert_eq!(b(0xda), 12);
        assert_eq!(b(0x156), 7);
        assert_eq!(b(0x24), 112);
        assert_eq!(b(0x5ae), 35);
        assert_eq!(u16_at(&raw, 0x3cc), 544);
        assert_eq!(u16_at(&raw, 0x5e8), 311);
        // ENV1 DECAY goes to the pitch and filter envelopes of both partials
        assert_eq!(u16_at(&raw, 0x2d0), u16_at(&raw, 0x2e8));
        assert_eq!(u16_at(&raw, 0x332), u16_at(&raw, 0x34a));
        assert_ne!(u16_at(&raw, 0x332), 0);
    }

    #[test]
    fn a_mirror_that_reads_a_pinned_field_is_refused_off_its_recorded_value() {
        let mut raw = svz(7, false);
        let before = raw.bytes().to_vec();
        let err = set_mdl_param(&mut raw, 0, "OSC1 WAVE FORM", "12").unwrap_err();
        assert!(err.to_string().contains("MOD MODE 3"), "{err}");
        assert_eq!(raw.bytes(), before);
    }

    #[test]
    fn a_parameter_with_an_unproven_mirror_is_refused() {
        let mut raw = svz(7, false);
        let before = raw.bytes().to_vec();
        let err = set_mdl_param(&mut raw, 0, "FILTER KEY FOLLOW", "1").unwrap_err();
        assert!(err.to_string().contains("not proven"), "{err}");
        assert!(set_mdl_param(&mut raw, 0, "Model", "2").is_err());
        assert_eq!(raw.bytes(), before);
    }

    #[test]
    fn a_mirror_is_refused_outside_the_recorded_state() {
        let mut raw = svz(7, false);
        set_mdl_param(&mut raw, 0, "PARAMETER EXPANSION SWITCH", "1").unwrap_err();
        let at = ANALOG
            .iter()
            .find(|p| p.id == "FILTER TYPE")
            .unwrap()
            .offset as usize;
        let mut bytes = raw.bytes().to_vec();
        let svd = Svd::parse(&raw).unwrap();
        let start = table(&raw, &svd).unwrap().record_offset(0);
        bytes[start + at] = 2;
        let mut raw = Raw::from_bytes(bytes);
        let err = set_mdl_param(&mut raw, 0, "FILTER FREQ", "100").unwrap_err();
        assert!(err.to_string().contains("FILTER TYPE 1"), "{err}");
    }

    #[test]
    fn other_engines_are_read_only() {
        for kind in [9, 15, 16] {
            let mut raw = svz(kind, false);
            let (_, fields) = read_mdl_params(&raw, 0).unwrap();
            assert!(fields.iter().all(|f| f.edit == MdlEdit::Locked));
            let id = fields.iter().find(|f| f.param.max >= 1).unwrap().param.id;
            let err = set_mdl_param(&mut raw, 0, id, "1").unwrap_err();
            assert!(err.to_string().contains("cannot set"), "{err}");
        }
    }

    #[test]
    fn a_model_byte_that_disagrees_with_the_type_is_refused() {
        let mut raw = svz(7, false);
        let svd = Svd::parse(&raw).unwrap();
        let start = table(&raw, &svd).unwrap().record_offset(0);
        raw.patch_bytes(start + ENGINE_BASE, &[3]);
        assert!(read_mdl_params(&raw, 0).is_err());
    }

    #[test]
    fn an_unknown_name_lists_near_matches_and_a_missing_record_is_refused() {
        let err = set_mdl_param(&mut svz(7, false), 0, "filter f", "1").unwrap_err();
        assert!(err.to_string().contains("FILTER FREQ"), "{err}");
        assert!(set_mdl_param(&mut svz(7, false), 1, "NOISE", "1").is_err());
    }

    #[test]
    fn an_edit_refreshes_the_record_checksum() {
        let mut raw = svz(7, true);
        set_mdl_param(&mut raw, 0, "FILTER FREQ", "700").unwrap();
        let report = crate::verify::check(&raw).unwrap();
        assert_eq!(report.checked, 1);
        assert!(report.is_ok(), "{:?}", report.problems);
    }
}
