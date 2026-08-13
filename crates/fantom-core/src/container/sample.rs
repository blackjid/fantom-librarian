//! The user sample bank: `SMPa` slots, `MLSa` multisamples, and the `USDa` waveform payload.
//!
//! Three areas describe user sampling in a full backup:
//!
//! - **`SMPa`** — 8000 fixed slots, 84 bytes each: name, in-use flag, level, loop mode, original
//!   key, and start/loop/end points in frames. One record per panel sample slot, whether used or not.
//! - **`USDa`** — the audio. A directory of 8-byte `{sample_index, offset}` entries terminated by
//!   `0xFFFFFFFF`, each offset pointing at an `SMPd` section holding 16-bit PCM.
//! - **`MLSa`** — 128 multisamples, 1040 bytes each: a 16-byte name plus 128 eight-byte entries,
//!   one per MIDI key. The shape is certain; the entry fields are **not decoded**, because every
//!   record in every fixture is still the factory `INITIAL MSMPL` default.
//!
//! Confirmed against three FANTOM-6 backups: 50 named `SMPa` slots, 50 `USDa` directory entries,
//! and exactly 50 `SMPd` sections, matching by name and position. A backup with no user samples has
//! an 8-byte `USDa` holding only the terminator.
//!
//! **The link in the other direction lives elsewhere and is only half known.** Nothing in these
//! three areas says which sample a *tone* plays: a ZEN-Core tone's partials do, via the wave group
//! and slot number [`crate::container::sample_slots`] reads, but a drum kit's per-key instruments in
//! `INSa` do not — their wave blocks are located, their group field is not. So a `PATa` tone's
//! samples can be selected when repackaging and a drum kit's cannot; see [`crate::tonebank`].

use crate::container::{ascii_trim, Raw, RecordTable, Svd};
use crate::Result;

/// How many user sample slots the panel has. `SMPa` holds exactly this many records, and a tone
/// addresses them 1-based, so `1..=PANEL_SLOTS` is the whole range a reference may name.
pub const PANEL_SLOTS: u16 = 8000;

/// One of the 8000 user sample slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleSlot {
    /// Zero-based slot number.
    pub index: usize,
    pub name: String,
    /// Whether the slot holds a sample.
    pub in_use: bool,
    pub level: u8,
    /// 0 = off, 1 and 2 seen; the exact loop modes are not confirmed.
    pub loop_mode: u8,
    /// MIDI note the sample plays at its recorded pitch (60 = C4 by default).
    pub original_key: u8,
    /// Playback start, in frames.
    pub start: u32,
    /// Loop point, in frames.
    pub loop_point: u32,
    /// Playback end, in frames. Equals the recorded length unless the sample was trimmed.
    pub end: u32,
}

/// One `SMPd` waveform section inside `USDa`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleData {
    /// The `SMPa` slot this audio belongs to.
    pub slot: u32,
    /// Offset of the `SMPd` magic within the `USDa` body.
    pub offset: usize,
    /// Name as stored in the section header. May differ from the slot name if the user renamed the
    /// slot after importing.
    pub name: String,
    /// Total size of the section's audio, both channels including their padding.
    pub size: u32,
    /// Bytes of real PCM **per channel** — 16-bit mono each, so twice the frame count.
    ///
    /// The two channels are stored as separate contiguous blocks, left then right, each padded up
    /// to a multiple of 512 (and given another 512 if that would leave under 128 bytes of slack).
    /// So `size == 2 * pad512(channel_bytes)`, which holds for all 50 samples of a FANTOM-6 backup.
    /// The audio is *not* interleaved, which an earlier reading of this field as "16-bit words, two
    /// per frame" implied — the frame count works out the same either way.
    pub channel_bytes: u32,
    pub sample_rate: u32,
}

impl SampleData {
    /// Length in frames.
    pub fn frames(&self) -> u32 {
        self.channel_bytes / 2
    }

    /// Duration in seconds.
    pub fn seconds(&self) -> f32 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.frames() as f32 / self.sample_rate as f32
    }
}

/// One of the 128 multisample slots.
///
/// Only the name is decoded. The 128 eight-byte entries that follow it are one per MIDI key and
/// almost certainly map keys to `SMPa` slots, but no fixture has a populated multisample to confirm
/// it against, so this deliberately reports nothing it cannot prove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Multisample {
    pub index: usize,
    pub name: String,
    /// Whether the record differs from the factory `INITIAL MSMPL` default.
    pub edited: bool,
}

/// The user sample bank of one file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SampleBank {
    /// Slots that hold a sample, in slot order. Empty slots are omitted.
    pub slots: Vec<SampleSlot>,
    /// Waveform sections found in `USDa`, in directory order.
    pub data: Vec<SampleData>,
    /// Multisamples that differ from the factory default.
    pub multisamples: Vec<Multisample>,
}

impl SampleBank {
    /// Whether the file carries any user sampling at all.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty() && self.data.is_empty() && self.multisamples.is_empty()
    }

    /// Audio present in `USDa` with no matching `SMPa` slot, or a slot with no audio — either
    /// means the two areas disagree and the bank should not be trusted for copying.
    pub fn orphans(&self) -> Vec<String> {
        let mut out = Vec::new();
        for slot in &self.slots {
            if !self.data.iter().any(|d| d.slot as usize == slot.index) {
                out.push(format!(
                    "slot {} {:?} has no waveform data",
                    slot.index, slot.name
                ));
            }
        }
        for data in &self.data {
            if !self.slots.iter().any(|s| s.index == data.slot as usize) {
                out.push(format!(
                    "waveform {:?} points at unused slot {}",
                    data.name, data.slot
                ));
            }
        }
        out
    }
}

const IN_USE: usize = 0x40;
const LEVEL: usize = 0x41;
const LOOP_MODE: usize = 0x44;
const ORIGINAL_KEY: usize = 0x45;
const START: usize = 0x48;
const LOOP_POINT: usize = 0x4c;
const END: usize = 0x50;
const SLOT_LEN: usize = 0x54;

/// `USPa` — the sample slot table of an SVZ tone export. A 64-byte record, laid out differently
/// from `SMPa`: in-use and level at `+0x14`, original key at `+0x19`, end point at `+0x24`.
///
/// Confirmed against `Z-Core_20260623.svz`: slot 0 is named `Sample003;F#3-F#` and stores key
/// `0x36` = 54 = F#3, and `384 + end * 4` is exactly its `SMPd` section size for both slots.
mod usp {
    pub const IN_USE: usize = 0x14;
    pub const LEVEL: usize = 0x15;
    pub const ORIGINAL_KEY: usize = 0x19;
    pub const END: usize = 0x24;
    pub const LEN: usize = 0x28;
}

const DIRECTORY_END: u32 = u32::MAX;
const SMPD_MAGIC: &[u8; 4] = b"SMPd";

/// SVZ `USDa` directory entry: `{u32 slot, u32 offset, u32 size, u32 word}`.
pub(crate) const SVZ_DIRECTORY_ENTRY: usize = 16;
/// In an SVZ `SMPd`, the per-channel byte count and rate sit closer to the front than in a backup's.
const SVZ_SMPD_CHANNEL_BYTES: usize = 0x04;
const SVZ_SMPD_RATE: usize = 0x0c;
const SMPD_SIZE: usize = 0x08;
const SMPD_CHANNEL_BYTES: usize = 0x0c;
const SMPD_NAME: usize = 0x10;
const SMPD_RATE: usize = 0x20;
const SMPD_HEADER_LEN: usize = 0x24;

/// Read whatever user sampling a file carries. A file with no sample areas yields an empty bank.
///
/// Two shapes exist. A backup stores slots in `SMPa` and audio in a single `USDa` record; an SVZ
/// tone export stores slots in `USPa` and audio as variable-size `USDa` records. The presence of
/// `USPa` selects between them.
pub fn read(raw: &Raw, svd: &Svd) -> Result<SampleBank> {
    if svd.area(b"USPa").is_some() {
        return Ok(SampleBank {
            slots: read_svz_slots(raw, svd)?,
            data: read_svz_data(raw, svd)?,
            multisamples: Vec::new(),
        });
    }
    Ok(SampleBank {
        slots: read_slots(raw, svd)?,
        data: read_data(raw, svd)?,
        multisamples: read_multisamples(raw, svd)?,
    })
}

fn read_svz_slots(raw: &Raw, svd: &Svd) -> Result<Vec<SampleSlot>> {
    let Some(table) = RecordTable::from_svd(raw, svd, b"USPa")? else {
        return Ok(Vec::new());
    };
    let mut slots = Vec::new();
    for index in 0..table.len() {
        let Some(record) = table.record(index) else {
            break;
        };
        if record.len() < usp::LEN {
            break;
        }
        slots.push(SampleSlot {
            index,
            name: ascii_trim(&record[..16]),
            in_use: record[usp::IN_USE] != 0,
            level: record[usp::LEVEL],
            loop_mode: 0,
            original_key: record[usp::ORIGINAL_KEY],
            start: 0,
            loop_point: 0,
            end: le_u32(record, usp::END),
        });
    }
    Ok(slots)
}

/// An SVZ `USDa` holds variable-size records: a 16-byte directory entry per section
/// (`{slot, offset, size, word}`) followed by the `SMPd` sections themselves.
fn read_svz_data(raw: &Raw, svd: &Svd) -> Result<Vec<SampleData>> {
    let Some(area) = svd.area(b"USDa") else {
        return Ok(Vec::new());
    };
    let bytes = svd.area_bytes(raw, area)?;
    let count = le_u32(bytes, 0) as usize;

    let mut out = Vec::new();
    for index in 0..count {
        let entry = RecordTable::HEADER_LEN + index * SVZ_DIRECTORY_ENTRY;
        let Some(entry) = bytes.get(entry..entry + SVZ_DIRECTORY_ENTRY) else {
            break;
        };
        let slot = le_u32(entry, 0);
        let offset = le_u32(entry, 4) as usize;
        let Some(section) = bytes.get(offset..offset + SMPD_HEADER_LEN) else {
            break;
        };
        if &section[..4] != SMPD_MAGIC {
            break;
        }
        out.push(SampleData {
            slot,
            offset,
            name: ascii_trim(&section[SMPD_NAME..SMPD_NAME + 16]),
            size: le_u32(entry, 8),
            channel_bytes: le_u32(section, SVZ_SMPD_CHANNEL_BYTES),
            sample_rate: le_u32(section, SVZ_SMPD_RATE),
        });
    }
    Ok(out)
}

fn read_slots(raw: &Raw, svd: &Svd) -> Result<Vec<SampleSlot>> {
    let Some(table) = RecordTable::from_svd(raw, svd, b"SMPa")? else {
        return Ok(Vec::new());
    };
    let mut slots = Vec::new();
    for index in 0..table.len() {
        let Some(record) = table.record(index) else {
            break;
        };
        if record.len() < SLOT_LEN {
            break;
        }
        let name = ascii_trim(&record[..16]);
        if name.is_empty() && record[IN_USE] == 0 {
            continue;
        }
        slots.push(SampleSlot {
            index,
            name,
            in_use: record[IN_USE] != 0,
            level: record[LEVEL],
            loop_mode: record[LOOP_MODE],
            original_key: record[ORIGINAL_KEY],
            start: le_u32(record, START),
            loop_point: le_u32(record, LOOP_POINT),
            end: le_u32(record, END),
        });
    }
    Ok(slots)
}

fn read_data(raw: &Raw, svd: &Svd) -> Result<Vec<SampleData>> {
    let Some(area) = svd.area(b"USDa") else {
        return Ok(Vec::new());
    };
    let bytes = svd.area_bytes(raw, area)?;
    let Some(body) = bytes.get(RecordTable::HEADER_LEN..) else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for entry in body.chunks_exact(8) {
        let slot = u32::from_le_bytes(entry[..4].try_into().unwrap());
        let offset = u32::from_le_bytes(entry[4..].try_into().unwrap()) as usize;
        if slot == DIRECTORY_END {
            break;
        }
        let Some(section) = body.get(offset..offset + SMPD_HEADER_LEN) else {
            break;
        };
        if &section[..4] != SMPD_MAGIC {
            break;
        }
        out.push(SampleData {
            slot,
            offset,
            name: ascii_trim(&section[SMPD_NAME..SMPD_NAME + 16]),
            size: le_u32(section, SMPD_SIZE),
            channel_bytes: le_u32(section, SMPD_CHANNEL_BYTES),
            sample_rate: le_u32(section, SMPD_RATE),
        });
    }
    Ok(out)
}

/// The factory-default multisample record: the `INITIAL MSMPL` name followed by 128 identical
/// per-key entries. Byte-identical in all 384 records across three backups.
fn factory_multisample(record_size: usize) -> Vec<u8> {
    let mut record = Vec::with_capacity(record_size);
    let mut name = [b' '; 16];
    name[..13].copy_from_slice(b"INITIAL MSMPL");
    record.extend_from_slice(&name);
    while record.len() < record_size {
        record.extend_from_slice(&[0x00, 0x00, 0x7f, 0x00, 0x80, 0x00, 0x00, 0x00]);
    }
    record.truncate(record_size);
    record
}

fn read_multisamples(raw: &Raw, svd: &Svd) -> Result<Vec<Multisample>> {
    let Some(table) = RecordTable::from_svd(raw, svd, b"MLSa")? else {
        return Ok(Vec::new());
    };
    // Compare against the known factory bytes rather than against this file's own record 0 —
    // using record 0 as the template would hide it if it were the one the user edited, and would
    // then report every untouched slot as edited.
    let default = factory_multisample(table.record_size);
    let mut out = Vec::new();
    for index in 0..table.len() {
        let Some(record) = table.record(index) else {
            break;
        };
        if record == default {
            continue;
        }
        out.push(Multisample {
            index,
            name: ascii_trim(&record[..16.min(record.len())]),
            edited: true,
        });
    }
    Ok(out)
}

fn le_u32(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svd_with(areas: &[(&[u8; 4], Vec<u8>)]) -> (Raw, Svd) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&((14 + areas.len() * 16) as u16).to_le_bytes());
        bytes.extend_from_slice(b"SVD5");
        bytes.extend_from_slice(&[0u8; 10]);
        let mut offset = 0x10 + areas.len() * 16;
        for (tag, body) in areas {
            bytes.extend_from_slice(*tag);
            bytes.extend_from_slice(b"KY19");
            bytes.extend_from_slice(&(offset as u32).to_le_bytes());
            bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
            offset += body.len();
        }
        for (_, body) in areas {
            bytes.extend_from_slice(body);
        }
        let raw = Raw::from_bytes(bytes);
        let svd = Svd::parse(&raw).unwrap();
        (raw, svd)
    }

    fn slot_area(entries: &[(&str, u32)]) -> Vec<u8> {
        let mut area = Vec::new();
        area.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        area.extend_from_slice(&(SLOT_LEN as u32).to_le_bytes());
        area.extend_from_slice(&16u32.to_le_bytes());
        area.extend_from_slice(&[0u8; 4]);
        for (name, end) in entries {
            let mut record = vec![0u8; SLOT_LEN];
            record[..16].fill(b' ');
            record[..name.len()].copy_from_slice(name.as_bytes());
            if !name.trim().is_empty() {
                record[IN_USE] = 1;
                record[LEVEL] = 127;
                record[ORIGINAL_KEY] = 60;
                record[END..END + 4].copy_from_slice(&end.to_le_bytes());
            }
            area.extend_from_slice(&record);
        }
        area
    }

    /// A `USDa` area: an 8-byte directory entry per section, terminator, then the sections.
    fn waveform_area(sections: &[(u32, &str, u32)]) -> Vec<u8> {
        let directory_len = (sections.len() + 1) * 8;
        let mut directory = Vec::new();
        let mut bodies = Vec::new();
        let mut offset = directory_len;
        for (slot, name, words) in sections {
            directory.extend_from_slice(&slot.to_le_bytes());
            directory.extend_from_slice(&(offset as u32).to_le_bytes());
            let mut section = vec![0u8; SMPD_HEADER_LEN];
            section[..4].copy_from_slice(SMPD_MAGIC);
            section[SMPD_SIZE..SMPD_SIZE + 4].copy_from_slice(&(words * 2).to_le_bytes());
            section[SMPD_CHANNEL_BYTES..SMPD_CHANNEL_BYTES + 4]
                .copy_from_slice(&words.to_le_bytes());
            section[SMPD_NAME..SMPD_NAME + name.len()].copy_from_slice(name.as_bytes());
            section[SMPD_RATE..SMPD_RATE + 4].copy_from_slice(&48000u32.to_le_bytes());
            offset += section.len();
            bodies.extend_from_slice(&section);
        }
        directory.extend_from_slice(&DIRECTORY_END.to_le_bytes());
        directory.extend_from_slice(&(offset as u32).to_le_bytes());

        let mut area = vec![0u8; RecordTable::HEADER_LEN];
        area.extend_from_slice(&directory);
        area.extend_from_slice(&bodies);
        area
    }

    #[test]
    fn reads_used_slots_and_skips_empty_ones() {
        let (raw, svd) = svd_with(&[(
            b"SMPa",
            slot_area(&[("1 Beat It - C2", 278323), ("", 0), ("3 IML Whoa 1", 22778)]),
        )]);
        let bank = read(&raw, &svd).unwrap();

        assert_eq!(bank.slots.len(), 2);
        assert_eq!(bank.slots[0].index, 0);
        assert_eq!(bank.slots[0].name, "1 Beat It - C2");
        assert_eq!(bank.slots[0].end, 278323);
        assert_eq!(bank.slots[0].original_key, 60);
        assert!(bank.slots[0].in_use);
        // The unused slot keeps its number free rather than shifting the ones after it.
        assert_eq!(bank.slots[1].index, 2);
    }

    #[test]
    fn walks_the_waveform_directory_to_its_terminator() {
        let (raw, svd) = svd_with(&[(
            b"USDa",
            waveform_area(&[(0, "1 Beat It - C2", 556646), (2, "3 IML Whoa 1", 45556)]),
        )]);
        let bank = read(&raw, &svd).unwrap();

        assert_eq!(bank.data.len(), 2);
        assert_eq!(bank.data[0].slot, 0);
        assert_eq!(bank.data[0].name, "1 Beat It - C2");
        assert_eq!(bank.data[0].frames(), 278323);
        assert_eq!(bank.data[0].sample_rate, 48000);
        assert!((bank.data[0].seconds() - 5.798).abs() < 0.01);
        assert_eq!(bank.data[1].slot, 2);
    }

    #[test]
    fn reports_slots_and_waveforms_that_disagree() {
        let (raw, svd) = svd_with(&[
            (b"SMPa", slot_area(&[("Recorded", 100)])),
            (b"USDa", waveform_area(&[(7, "Stray", 200)])),
        ]);
        let bank = read(&raw, &svd).unwrap();

        let orphans = bank.orphans();
        assert_eq!(orphans.len(), 2);
        assert!(orphans[0].contains("no waveform data"));
        assert!(orphans[1].contains("unused slot 7"));
    }

    fn multisample_area(records: &[Vec<u8>], record_size: usize) -> Vec<u8> {
        let mut area = Vec::new();
        area.extend_from_slice(&(records.len() as u32).to_le_bytes());
        area.extend_from_slice(&(record_size as u32).to_le_bytes());
        area.extend_from_slice(&16u32.to_le_bytes());
        area.extend_from_slice(&[0u8; 4]);
        for record in records {
            let mut r = record.clone();
            r.resize(record_size, 0);
            area.extend_from_slice(&r);
        }
        area
    }

    /// The default must be the known factory bytes, not this file's own record 0 — otherwise an
    /// edit to slot 0 hides itself and makes every untouched slot look edited.
    #[test]
    fn an_edited_first_multisample_is_reported_and_does_not_taint_the_rest() {
        const LEN: usize = 1040;
        let mut edited = factory_multisample(LEN);
        edited[..16].copy_from_slice(b"My Multisample  ");

        let records = vec![edited, factory_multisample(LEN), factory_multisample(LEN)];
        let (raw, svd) = svd_with(&[(b"MLSa", multisample_area(&records, LEN))]);
        let bank = read(&raw, &svd).unwrap();

        assert_eq!(
            bank.multisamples.len(),
            1,
            "only the edited slot is reported"
        );
        assert_eq!(bank.multisamples[0].index, 0);
        assert_eq!(bank.multisamples[0].name, "My Multisample");
    }

    #[test]
    fn untouched_multisamples_are_all_recognised_as_factory_defaults() {
        const LEN: usize = 1040;
        let records = vec![factory_multisample(LEN); 4];
        let (raw, svd) = svd_with(&[(b"MLSa", multisample_area(&records, LEN))]);
        assert!(read(&raw, &svd).unwrap().multisamples.is_empty());
    }

    /// A file with no sampling at all — every scene export, and a backup whose user never sampled.
    #[test]
    fn a_file_without_sample_areas_yields_an_empty_bank() {
        let (raw, svd) = svd_with(&[(b"PRFa", vec![0u8; 32])]);
        let bank = read(&raw, &svd).unwrap();
        assert!(bank.is_empty());
        assert!(bank.orphans().is_empty());
    }
}
