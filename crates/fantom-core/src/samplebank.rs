//! Building a sample-only SVZ: the one container that moves user audio between instruments.
//!
//! A scene export carries no audio, only slot references, so a sampled scene is incomplete
//! anywhere but the instrument it came from. The fix is not to stuff samples into the scene bank —
//! Roland's own scene exports do not, and nothing reads them there — but to hand the destination a
//! second file it *does* read: an `.svz` holding `DIFa`, `USPa`, and `USDa` and no tone area at
//! all, which **MENU → IMPORT SAMPLE** loads into a slot range the user picks.
//!
//! The audio for that file lives in a full backup, in a different shape: `SMPa` slot records and
//! one `USDa` holding `SMPd` sections. Converting is a **header rewrite around an unchanged audio
//! payload**, and every rule below is derived from a matched pair — `FFC SAMPLES 1-50.svz` and the
//! `2023.4.8+topandprisma` backup hold the same 50 recordings, so the two shapes could be diffed
//! field by field. All 50 agree.

use crate::container::{Raw, RecordTable, Svd};
use crate::tonebank::BuiltArea;
use crate::{Error, Result};

/// `USPa` record length. `SMPa`'s is 84.
const USP_LEN: usize = 64;
/// `SMPa`'s parameter block — in-use, level, loop mode, original key, start, loop point, end.
const SMPA_PARAMS: std::ops::Range<usize> = 0x40..0x54;
/// Where that same block sits in a `USPa` record: the whole thing shifted by `-0x2c`.
const USP_PARAMS_AT: usize = 0x14;
/// Bytes `0x2c..0x3c` of every `USPa` record the instrument writes, in both fixtures that have one.
const USP_TAIL: [u8; 16] = [
    0x02, 0x00, 0x32, 0x32, 0x00, 0x01, 0xe0, 0x2e, 0x00, 0x00, 0x10, 0x00, 0x01, 0x10, 0x00, 0x00,
];
const USP_TAIL_AT: usize = 0x2c;
const NAME_LEN: usize = 16;

/// An `SMPd` section header, in each of its two layouts.
pub(crate) mod smpd {
    /// Backup: flags `+0x04`, size `+0x08`, words `+0x0c`, name `+0x10`, rate `+0x20`, id `+0x24`.
    pub mod backup {
        pub const FLAGS: usize = 0x04;
        pub const SIZE: usize = 0x08;
        pub const WORDS: usize = 0x0c;
        pub const NAME: usize = 0x10;
        pub const RATE: usize = 0x20;
        pub const ID: usize = 0x24;
        pub const AUDIO: usize = 0x80;
        /// A backup's declared section size excludes the 64-byte gap before the next section, so
        /// the audio actually available runs 64 bytes past it.
        pub const TRAILING_GAP: usize = 64;
    }
    /// SVZ: words `+0x04`, flags `+0x08`, rate `+0x0c`, name `+0x10`.
    pub mod svz {
        pub const WORDS: usize = 0x04;
        pub const FLAGS: usize = 0x08;
        pub const RATE: usize = 0x0c;
        pub const NAME: usize = 0x10;
        pub const AUDIO: usize = 0x60;
        /// The flags word is this, plus one bit carried from the backup's own flags.
        pub const FLAGS_BASE: u32 = 0x0000_1002;
        /// Backup `0x4000_0000` becomes SVZ `0x0001_0000`. The pack's 50 samples split 46/4 on
        /// exactly this bit, matching their backup sections one for one; what it *means* is not
        /// decoded, which is why it is carried rather than derived from anything.
        ///
        /// **Only one family of samples has been checked.** Every section in every matched pair
        /// reads `0x0201_0020` or `0x4201_0020`. One backup also holds 44 sections reading
        /// `0x0101_0020` — a bit this conversion neither carries nor reproduces, because no
        /// instrument-written export of such a sample exists to compare against. A file built
        /// from those is structurally sound and unverified; see `docs/FORMAT.md`.
        pub const CARRIED_BIT_FROM: u32 = 0x4000_0000;
        pub const CARRIED_BIT_TO: u32 = 0x0001_0000;
    }
}

/// `DIFa` is 32 bytes and **all zero** in every SVZ the instrument writes — both the pack's
/// sample-only file and a fresh single-tone export. It is not recomputed here because there is
/// nothing to recompute.
const DIF_LEN: usize = 32;

/// The area order an SVZ sample file uses.
const AREA_ORDER: [[u8; 4]; 3] = [*b"DIFa", *b"USPa", *b"USDa"];
/// Areas of an SVZ tone/sample export are stamped `ZCOR`.
const FORMAT: [u8; 4] = *b"ZCOR";

/// One sample lifted out of a backup: its slot record and the audio section that belongs to it.
struct Sample<'a> {
    smpa: &'a [u8],
    section: &'a [u8],
}

/// Build a sample-only `.svz` carrying `slots` from `source`, in the order given.
///
/// `slots` are 0-based `SMPa` slot numbers — the panel's "0001" is 0 here, matching what
/// [`crate::container::read_samples`] reports. The output numbers them densely from 0, exactly as
/// an instrument-written export does, so the destination imports them as one contiguous run.
///
/// `source` must be a full backup: a scene export carries no audio to copy.
pub fn export_samples(source: &Raw, slots: &[usize]) -> Result<Raw> {
    if slots.is_empty() {
        return Err(Error::Unrecognized(
            "at least one sample slot is required".into(),
        ));
    }
    let svd = Svd::parse(source)?;
    let mut areas = vec![dif_area()];
    areas.extend(sample_areas(source, &svd, slots)?);

    let order: Vec<[u8; 4]> = areas.iter().map(|(tag, _, _)| *tag).collect();
    crate::tonebank::assemble(&crate::tonebank::preamble(&order), &AREA_ORDER, areas)
}

/// `DIFa`, which every SVZ opens with.
pub(crate) fn dif_area() -> BuiltArea {
    (
        *b"DIFa",
        FORMAT,
        crate::tonebank::record_area(&[&[0u8; DIF_LEN]], DIF_LEN),
    )
}

/// The `USPa` slot table and `USDa` audio carrying `slots` out of a backup, numbered densely from
/// zero in the order given.
///
/// This is the sample half of *any* SVZ built from a backup — on its own it is a sample companion,
/// beside a tone area it is a self-contained tone export (see [`crate::convert`]).
pub(crate) fn sample_areas(source: &Raw, svd: &Svd, slots: &[usize]) -> Result<Vec<BuiltArea>> {
    let samples = collect(source, svd, slots)?;

    let usp: Vec<Vec<u8>> = samples.iter().map(|s| usp_record(s.smpa)).collect();
    let sections: Vec<(u32, Vec<u8>)> = samples
        .iter()
        .map(|s| Ok((section_id(s.section)?, svz_section(s.section)?)))
        .collect::<Result<_>>()?;

    Ok(vec![
        (
            *b"USPa",
            FORMAT,
            crate::tonebank::record_area(
                &usp.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                USP_LEN,
            ),
        ),
        (
            *b"USDa",
            FORMAT,
            crate::tonebank::build_waveform_area(
                sections
                    .iter()
                    .enumerate()
                    .map(|(i, (id, bytes))| (i as u32, *id, bytes.as_slice())),
            )?,
        ),
    ])
}

/// Look up each requested slot's `SMPa` record and `SMPd` section.
fn collect<'a>(raw: &'a Raw, svd: &Svd, slots: &[usize]) -> Result<Vec<Sample<'a>>> {
    let table = RecordTable::from_svd(raw, svd, b"SMPa")?.ok_or_else(|| {
        Error::Unrecognized(
            "this file has no SMPa area — only a full backup carries user sample audio".into(),
        )
    })?;
    let sections = read_sections(raw, svd)?;

    let mut out = Vec::with_capacity(slots.len());
    for &slot in slots {
        let smpa = table.record(slot).ok_or_else(|| {
            Error::Unrecognized(format!(
                "sample slot {} is out of range (the file has {})",
                slot + 1,
                table.len()
            ))
        })?;
        // Whether the slot's in-use flag is set is not the test — one of the 50 slots the FFC pack
        // ships has it clear, and the pack carries that sample regardless. Holding audio is what
        // makes a slot exportable, and the flag travels verbatim like every other field.
        let section = sections
            .iter()
            .find(|(index, _)| *index == slot)
            .map(|(_, bytes)| *bytes)
            .ok_or_else(|| {
                Error::Unrecognized(format!("no waveform data for sample slot {}", slot + 1))
            })?;
        out.push(Sample { smpa, section });
    }
    Ok(out)
}

/// Walk a backup's `USDa` directory: 8-byte `{slot, offset}` pairs until the `0xFFFFFFFF` end.
///
/// Each section is returned with the 64-byte gap after it included, because that gap is audio: a
/// backup's declared size stops short of it, and the instrument's own SVZ carries those bytes.
fn read_sections<'a>(raw: &'a Raw, svd: &Svd) -> Result<Vec<(usize, &'a [u8])>> {
    let Some(area) = svd.area(b"USDa") else {
        return Ok(Vec::new());
    };
    let bytes = svd.area_bytes(raw, area)?;
    let Some(body) = bytes.get(RecordTable::HEADER_LEN..) else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for entry in body.as_chunks::<8>().0 {
        let slot = u32::from_le_bytes(entry[..4].try_into().unwrap());
        let offset = u32::from_le_bytes(entry[4..].try_into().unwrap()) as usize;
        if slot == u32::MAX {
            break;
        }
        let Some(size) = body
            .get(offset + smpd::backup::SIZE..offset + smpd::backup::SIZE + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
        else {
            break;
        };
        let end = (offset + size + smpd::backup::TRAILING_GAP).min(body.len());
        let Some(section) = body.get(offset..end) else {
            break;
        };
        out.push((slot as usize, section));
    }
    Ok(out)
}

/// Turn an 84-byte `SMPa` record into a 64-byte `USPa` one.
///
/// Confirmed on all 50 samples of the matched pair: the name copies straight across, `SMPa`'s
/// `0x40..0x54` parameter block lands at `0x14` — a flat `-0x2c` shift — and the tail is fixed.
fn usp_record(smpa: &[u8]) -> Vec<u8> {
    let mut record = vec![0u8; USP_LEN];
    record[..NAME_LEN].copy_from_slice(&smpa[..NAME_LEN]);
    let params = &smpa[SMPA_PARAMS];
    record[USP_PARAMS_AT..USP_PARAMS_AT + params.len()].copy_from_slice(params);
    record[USP_TAIL_AT..USP_TAIL_AT + USP_TAIL.len()].copy_from_slice(&USP_TAIL);
    record
}

/// The per-section word the `USDa` directory stores, carried from the backup rather than computed.
fn section_id(section: &[u8]) -> Result<u32> {
    read_u32(section, smpd::backup::ID)
}

/// Rewrite a backup `SMPd` into SVZ form: a new 96-byte header, then the audio untouched.
fn svz_section(section: &[u8]) -> Result<Vec<u8>> {
    let words = read_u32(section, smpd::backup::WORDS)?;
    let rate = read_u32(section, smpd::backup::RATE)?;
    let carried = read_u32(section, smpd::backup::FLAGS)? & smpd::svz::CARRIED_BIT_FROM;
    let flags = smpd::svz::FLAGS_BASE
        | if carried != 0 {
            smpd::svz::CARRIED_BIT_TO
        } else {
            0
        };
    let audio = section
        .get(smpd::backup::AUDIO..)
        .ok_or_else(|| Error::Unrecognized("SMPd section is shorter than its header".into()))?;

    let mut out = vec![0u8; smpd::svz::AUDIO + audio.len()];
    out[..4].copy_from_slice(b"SMPd");
    write_u32(&mut out, smpd::svz::WORDS, words);
    write_u32(&mut out, smpd::svz::FLAGS, flags);
    write_u32(&mut out, smpd::svz::RATE, rate);
    out[smpd::svz::NAME..smpd::svz::NAME + NAME_LEN]
        .copy_from_slice(&section[smpd::backup::NAME..smpd::backup::NAME + NAME_LEN]);
    out[smpd::svz::AUDIO..].copy_from_slice(audio);
    Ok(out)
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| Error::Unrecognized(format!("SMPd section truncated at offset {at:#x}")))
}

fn write_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
