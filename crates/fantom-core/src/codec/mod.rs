//! Mapping between container bytes and the [`crate::model`] types.
//!
//! This is where a confirmed byte layout becomes [`crate::model`] values: scene name, comment,
//! tempo, level, and 16 zones from the `PRFa` area, with bundled user-tone names resolved from
//! their engine areas where possible (see `docs/FORMAT.md`).
//!
//! The handful of offsets this module names as constants were found by controlled single-variable
//! edits; everything else is read through [`crate::params::scene`], which maps the whole record.

use std::collections::HashMap;
use std::io::Cursor;

use binrw::BinRead as _;

use crate::address::{self, AreaSpec};
use crate::container::{ascii_trim, Raw, RawZone, RecordTable, Svd, ZoneSettings};
use crate::model::{Scene, ToneRef, ToneType, Zone};
use crate::params;
use crate::{Error, Result};

/// The area tag holding Performances/Scenes in a FANTOM-6 SVD backup.
const PRFA: &[u8; 4] = b"PRFa";

/// Offsets of the per-zone MIDI bank/program tuple within a settings-table record (`0x194`).
const TONE_ID_OFFSET: usize = 0x01;
const TONE_MSB_OFFSET: usize = 0x00;
const ZEN_CORE_MSB: u8 = 87;

/// The `PRFa` area opens with a fixed 16-byte header, then fixed-stride records begin.
/// `+0x00` = declared scene count; `+0x04` = record stride in bytes.
const AREA_HEADER_LEN: usize = 0x10;
const AREA_COUNT_OFFSET: usize = 0x00;
const AREA_RECORD_SIZE_OFFSET: usize = 0x04;

/// Each record begins with its 16-byte ASCII name (space/NUL padded).
const NAME_LEN: usize = 16;

/// A 64-byte free-text scene comment/memo follows the name at this record offset.
const COMMENT_OFFSET: usize = 0x40;
const COMMENT_LEN: usize = 64;

/// Number of zone slots in every scene.
const ZONE_COUNT: usize = 16;

/// One named user-tone record physically bundled in an SVD area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundledTone {
    /// Four-byte SVD area tag containing the record.
    pub area: [u8; 4],
    /// Sound engine that owns the record.
    pub tone_type: ToneType,
    /// Zero-based record index within the area.
    pub index: usize,
    /// Decoded 16-byte tone name.
    pub name: String,
}

/// Whether a record's name is the factory's placeholder for an unused slot.
///
/// Roland names an empty slot `INITIAL` plus its engine — `INITIAL TONE`, `INITIAL ORGAN`,
/// `INITIAL PIANO`, `INITIAL KIT` — so the prefix is the rule rather than the four seen so far.
/// One FANTOM backup carries over four thousand such records against a few hundred real sounds,
/// which is why anything listing a bundle's contents needs to be able to tell them apart.
///
/// This only reports what a name *is*; whether to skip such a record is the caller's decision.
pub fn is_placeholder_name(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    lower == "initial"
        || lower.starts_with("initial ")
        || lower == "init"
        || lower.starts_with("init ")
        || matches!(lower.as_str(), "inittone" | "initscene" | "----" | "---")
}

/// Decode every named user-tone record bundled in an SVD.
pub fn read_bundled_tones(raw: &Raw) -> Result<Vec<BundledTone>> {
    let svd = Svd::parse(raw)?;
    Ok(bundled_tones_from_svd(raw, &svd))
}

/// The raw bytes of each scene record, in the same order and numbering as [`read_scenes`].
///
/// Lets a caller identify a scene by its stored bytes rather than by the decoded subset.
pub fn read_scene_records(raw: &Raw) -> Result<Vec<&[u8]>> {
    let svd = Svd::parse(raw)?;
    let prfa = svd
        .area(PRFA)
        .ok_or_else(|| Error::Unrecognized("no PRFa (performance) area in file".into()))?;
    let area = svd.area_bytes(raw, prfa)?;
    let declared = read_u32(area, AREA_COUNT_OFFSET)? as usize;
    let record_size = read_u32(area, AREA_RECORD_SIZE_OFFSET)? as usize;
    if record_size == 0 {
        return Err(Error::Unrecognized("PRFa record size is zero".into()));
    }
    Ok(scene_records(area, declared, record_size)
        .into_iter()
        .map(|(_, record)| record)
        .collect())
}

/// Decode every scene in an SVD file — name, comment, and 16 zones (switch, key range, level, and
/// tone, with bundled user-tone names resolved; see [`read_zones`]).
pub fn read_scenes(raw: &Raw) -> Result<Vec<Scene>> {
    let svd = Svd::parse(raw)?;
    let prfa = svd
        .area(PRFA)
        .ok_or_else(|| Error::Unrecognized("no PRFa (performance) area in file".into()))?;
    let area = svd.area_bytes(raw, prfa)?;

    let declared_count = read_u32(area, AREA_COUNT_OFFSET)? as usize;
    let record_size = read_u32(area, AREA_RECORD_SIZE_OFFSET)? as usize;
    if record_size == 0 {
        return Err(Error::Unrecognized("PRFa record size is zero".into()));
    }

    let records = scene_records(area, declared_count, record_size);
    let resolver = ToneResolver::new(raw, &svd);

    records
        .into_iter()
        .map(|(_, record)| {
            let common = scene_block("Scene Common", 0);
            Ok(Scene {
                name: record.get(..NAME_LEN).map(ascii_trim).unwrap_or_default(),
                comment: record
                    .get(COMMENT_OFFSET..COMMENT_OFFSET + COMMENT_LEN)
                    .map(ascii_trim)
                    .unwrap_or_default(),
                tempo: common
                    .and_then(|b| b.read(record, "Scene_Tempo"))
                    .unwrap_or(0) as u16,
                level: common
                    .and_then(|b| b.read(record, "Scene_Level"))
                    .unwrap_or(0) as u8,
                zones: read_zones(record, Some(&resolver))?,
            })
        })
        .collect()
}

/// Area-relative offset and byte slice of each non-empty scene record in the PRFa `area`, in file
/// order, capped at `declared`. The single source of truth for which records are scenes and where,
/// so scene reading and in-place editing can never disagree.
fn scene_records(area: &[u8], declared: usize, record_size: usize) -> Vec<(usize, &[u8])> {
    let mut records = Vec::new();
    if record_size == 0 {
        return records;
    }
    let mut pos = AREA_HEADER_LEN;
    while pos + NAME_LEN <= area.len() && records.len() < declared {
        // Empty-named slots pad the bank to a fixed capacity; keep only real scenes.
        if !ascii_trim(&area[pos..pos + NAME_LEN]).is_empty() {
            records.push((pos, &area[pos..(pos + record_size).min(area.len())]));
        }
        pos += record_size;
    }
    records
}

/// The absolute file offset of the `scene_number` (1-based) scene record.
fn scene_record_offset(raw: &Raw, scene_number: usize) -> Result<usize> {
    let svd = Svd::parse(raw)?;
    let prfa = svd
        .area(PRFA)
        .ok_or_else(|| Error::Unrecognized("no PRFa (performance) area in file".into()))?;
    let area = svd.area_bytes(raw, prfa)?;
    let declared = read_u32(area, AREA_COUNT_OFFSET)? as usize;
    let record_size = read_u32(area, AREA_RECORD_SIZE_OFFSET)? as usize;
    let records = scene_records(area, declared, record_size);
    records
        .get(scene_number.wrapping_sub(1))
        .map(|&(pos, _)| prfa.offset as usize + pos)
        .ok_or_else(|| {
            Error::Unrecognized(format!(
                "scene {scene_number} out of range (file has {})",
                records.len()
            ))
        })
}

/// Rename scene `scene_number` (1-based) in place. The 16-byte name field is overwritten and
/// space-padded; call [`Raw::save`] to persist. Only the name bytes change (see `docs/FORMAT.md`).
pub fn set_scene_name(raw: &mut Raw, scene_number: usize, name: &str) -> Result<()> {
    let name = check_name(name)?;
    let at = scene_record_offset(raw, scene_number)?;
    raw.patch_ascii(at, NAME_LEN, name);
    Ok(())
}

/// Check a name against what the instrument's name fields can hold, returning it trimmed.
///
/// The field is a fixed run of 7-bit ASCII, space-padded. Writing goes through [`Raw::patch_ascii`],
/// which copies bytes and pads — so without this check a too-long name is silently cut mid-word and
/// a non-ASCII one has its UTF-8 bytes written verbatim, putting `café` on the panel as `cafÃ©`.
/// Refusing beats writing something the device will render as garbage.
pub fn check_name(name: &str) -> Result<&str> {
    check_ascii_field(name, NAME_LEN, "name")
}

/// The same check for a scene's longer free-text memo.
pub fn check_comment(comment: &str) -> Result<&str> {
    check_ascii_field(comment, COMMENT_LEN, "comment")
}

fn check_ascii_field<'a>(text: &'a str, limit: usize, what: &str) -> Result<&'a str> {
    let text = text.trim_end();
    if let Some(bad) = text.chars().find(|c| !(' '..='~').contains(c)) {
        return Err(Error::Unrecognized(format!(
            "a Fantom {what} cannot contain {bad:?}; only printable ASCII"
        )));
    }
    if text.len() > limit {
        return Err(Error::Unrecognized(format!(
            "a Fantom {what} is at most {limit} characters, got {}",
            text.len()
        )));
    }
    Ok(text)
}

/// Set scene `scene_number`'s (1-based) 64-byte comment/memo in place; call [`Raw::save`] to persist.
pub fn set_scene_comment(raw: &mut Raw, scene_number: usize, comment: &str) -> Result<()> {
    let comment = check_comment(comment)?;
    let at = scene_record_offset(raw, scene_number)?;
    raw.patch_ascii(at + COMMENT_OFFSET, COMMENT_LEN, comment);
    Ok(())
}

/// The bundled user-tone names of one file, keyed by the area holding them.
///
/// Every user reference — in a scene export or a full backup alike — indexes its engine's area
/// directly (see [`crate::address`]), so one list of names per area answers them all.
struct ToneResolver {
    names: HashMap<[u8; 4], Vec<String>>,
}

impl ToneResolver {
    fn new(raw: &Raw, svd: &Svd) -> Self {
        let mut names: HashMap<[u8; 4], Vec<String>> = HashMap::new();
        for tone in bundled_tones_from_svd(raw, svd) {
            names.entry(tone.area).or_default().push(tone.name);
        }
        Self { names }
    }

    /// The name of the record a tone address points at, if the file carries it.
    fn name(&self, msb: u8, lsb: u8, pc: u8) -> Option<&str> {
        let (spec, index) = address::resolve(msb, lsb, pc)?;
        self.names.get(&spec.tag)?.get(index).map(String::as_str)
    }
}

fn bundled_tones_from_svd(raw: &Raw, svd: &Svd) -> Vec<BundledTone> {
    address::AREAS
        .iter()
        .flat_map(|spec| {
            let names = record_names(raw, svd, spec).unwrap_or_default();
            names
                .into_iter()
                .enumerate()
                .map(move |(index, name)| BundledTone {
                    area: spec.tag,
                    tone_type: spec.tone_type,
                    index,
                    name,
                })
        })
        .collect()
}

/// The 16-byte name of every record in `spec`'s area, in storage order.
fn record_names(raw: &Raw, svd: &Svd, spec: &AreaSpec) -> Option<Vec<String>> {
    let table = RecordTable::from_svd(raw, svd, &spec.tag).ok()??;
    if table.record_size < spec.name_offset + NAME_LEN {
        return None;
    }
    Some(
        table
            .records()
            .filter_map(|record| {
                let bytes = record.get(spec.name_offset..spec.name_offset + NAME_LEN)?;
                Some(spec.decode_name(bytes))
            })
            .collect(),
    )
}

/// The `n`th instance of a named block within a scene record, from the parameter table.
///
/// The table is the map for everything past the handful of offsets this module found by
/// controlled edit; `params::scene` asserts the two agree where they overlap.
fn scene_block(name: &str, n: usize) -> Option<&'static params::Instance> {
    params::scene::SCENE
        .iter()
        .filter(|i| i.block.name == name)
        .nth(n)
}

/// Read a zone's raw MSB plus LSB/PC pair, if the record is long enough.
fn zone_tone_bank(record: &[u8], n: usize) -> Option<(u8, u16)> {
    let at = ZoneSettings::TABLE_OFFSET + n * ZoneSettings::LEN + TONE_ID_OFFSET;
    let msb_at = ZoneSettings::TABLE_OFFSET + n * ZoneSettings::LEN + TONE_MSB_OFFSET;
    let id = record
        .get(at..at + 2)
        .map(|s| u16::from_be_bytes([s[0], s[1]]))?;
    Some((*record.get(msb_at)?, id))
}

/// Decode zone slot `n` of a scene `record`, or `None` when the record is too short to hold it or
/// the slot lacks the `cf cd` alignment marker (uninitialized, corrupt, or a layout we don't
/// recognise). This is the single gate for "is this a real zone", so an unpopulated slot can never
/// leak a spurious tone reference into the output.
fn decode_zone_slot(record: &[u8], n: usize) -> Option<(RawZone, ZoneSettings, u8, u16)> {
    let zone_off = RawZone::TABLE_OFFSET + n * RawZone::LEN;
    let settings_off = ZoneSettings::TABLE_OFFSET + n * ZoneSettings::LEN;
    if record.len() < (zone_off + RawZone::LEN).max(settings_off + ZoneSettings::LEN) {
        return None;
    }
    let z = RawZone::read(&mut cursor_at(record, zone_off)).ok()?;
    if !z.is_valid() {
        return None;
    }
    let s = ZoneSettings::read(&mut cursor_at(record, settings_off)).ok()?;
    let (msb, tone_id) = zone_tone_bank(record, n).unwrap_or((0, 0));
    Some((z, s, msb, tone_id))
}

/// Decode the 16 zones from a single scene `record` (the slice starting at the zone name).
///
/// Combines the zone table (`0x6d0`, switch + key range) with the settings table (`0x194`, level +
/// tone address). `resolver` supplies the names of user tones the file bundles; without it, zones
/// still decode and keep their raw addresses.
///
/// Resilient by design: a record too short to hold the zone tables yields no zones, and any
/// individual zone slot that lacks the `cf cd` alignment marker is skipped rather than failing the
/// whole file (see [`decode_zone_slot`]).
fn read_zones(record: &[u8], resolver: Option<&ToneResolver>) -> Result<Vec<Zone>> {
    let mut zones = Vec::with_capacity(ZONE_COUNT);
    for n in 0..ZONE_COUNT {
        let Some((z, s, msb, tone_id)) = decode_zone_slot(record, n) else {
            continue;
        };
        let settings = scene_block("Scene Zone", n);
        let control = scene_block("Zone Control", n);
        let read = |b: Option<&'static params::Instance>, id: &str| {
            b.and_then(|b| b.read(record, id)).unwrap_or(0)
        };
        let signed = |b: Option<&'static params::Instance>, id: &str| {
            b.and_then(|b| b.read_display(record, id)).unwrap_or(0) as i8
        };
        zones.push(Zone {
            number: n as u8,
            enabled: z.enable != 0,
            muted: read(settings, "Mute_Switch") != 0,
            tone: resolve_tone(msb, tone_id, resolver),
            key_low: z.key_low,
            key_high: z.key_high,
            velocity_low: read(control, "Velocity_Control_Range_Lower") as u8,
            velocity_high: read(control, "Velocity_Control_Range_Upper") as u8,
            level: s.level,
            pan: signed(settings, "Zone_Pan"),
            transpose: signed(control, "Zone_Transpose"),
            octave: signed(settings, "Zone_Octave_Shift"),
            midi_channel: read(settings, "Receive_Channel") as u8,
            arpeggio: read(control, "Arpeggio_Switch") != 0,
        });
    }
    Ok(zones)
}

/// Name a zone's tone address: a bundled user record if the file holds one, else the ZEN-Core
/// factory sound list. Addresses we cannot place keep their raw MSB/LSB/PC and no name — showing
/// the wrong name would be worse than showing none.
fn resolve_tone(msb: u8, id: u16, resolver: Option<&ToneResolver>) -> ToneRef {
    let [lsb, pc] = id.to_be_bytes();
    let name = match resolver.and_then(|r| r.name(msb, lsb, pc)) {
        Some(name) => Some(name.to_owned()),
        None if msb == ZEN_CORE_MSB => {
            crate::presets::lookup(id).map(|preset| preset.name.to_owned())
        }
        None => None,
    };
    ToneRef::new(msb, lsb, pc, name)
}

fn cursor_at(bytes: &[u8], at: usize) -> Cursor<&[u8]> {
    Cursor::new(&bytes[at..])
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32> {
    let slice = bytes
        .get(at..at + 4)
        .ok_or_else(|| Error::Unrecognized(format!("PRFa area truncated at offset {at}")))?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn a_name_is_checked_against_what_the_field_can_hold() {
        assert_eq!(check_name("Ballad Rhodes").unwrap(), "Ballad Rhodes");
        // Exactly the field width is fine; one more is not.
        assert!(check_name("0123456789abcdef").is_ok());
        assert!(check_name("0123456789abcdefg").is_err());
        // `patch_ascii` would write the UTF-8 bytes verbatim, so non-ASCII has to be refused.
        assert!(check_name("café").is_err());
        assert!(check_name("tab\there").is_err());
        // Trailing space is how the field pads, so trimming it is not a rejection.
        assert_eq!(check_name("Pad   ").unwrap(), "Pad");
    }

    #[test]
    fn the_placeholder_rule_matches_the_factory_convention() {
        for blank in [
            "",
            "   ",
            "INITIAL TONE",
            "INITIAL ORGAN",
            "INITIAL PIANO",
            "INITIAL KIT",
            "initial",
            "INIT TONE",
            "----",
        ] {
            assert!(is_placeholder_name(blank), "{blank:?} should read as blank");
        }
        for real in [
            "Mk1 Rhodes",
            "Initials",
            "INITIATE",
            "24k Logic I",
            "Init'l",
        ] {
            assert!(!is_placeholder_name(real), "{real:?} is a real name");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic SVD5 whose PRFa area holds `names`, each in a `record_size`-byte record
    /// following a 16-byte area header. Mirrors the confirmed FANTOM-6 layout.
    fn svd_with_scenes(names: &[&str], record_size: usize) -> Raw {
        let area_len = AREA_HEADER_LEN + names.len() * record_size;

        let mut area = Vec::new();
        area.extend_from_slice(&(names.len() as u32).to_le_bytes()); // declared count
        area.extend_from_slice(&(record_size as u32).to_le_bytes());
        area.extend_from_slice(&[0u8; 8]);
        for name in names {
            let mut rec = vec![0u8; record_size];
            let bytes = name.as_bytes();
            rec[..bytes.len().min(NAME_LEN)].copy_from_slice(&bytes[..bytes.len().min(NAME_LEN)]);
            area.extend_from_slice(&rec);
        }

        let mut file = Vec::new();
        file.extend_from_slice(&30u16.to_le_bytes()); // header_size: 14 + one 16-byte area entry
        file.extend_from_slice(b"SVD5");
        file.extend_from_slice(&[0u8; 10]);
        file.extend_from_slice(b"PRFa");
        file.extend_from_slice(b"KY19");
        file.extend_from_slice(&0x20u32.to_le_bytes()); // area offset (right after the table)
        file.extend_from_slice(&(area_len as u32).to_le_bytes());
        file.extend_from_slice(&area);
        Raw::from_bytes(file)
    }

    #[test]
    fn reads_multiple_scene_names_and_skips_empty_slots() {
        let raw = svd_with_scenes(&["DSOTM Breathe", "On The Run", "", ""], 64);
        let scenes = read_scenes(&raw).unwrap();
        let names: Vec<_> = scenes.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["DSOTM Breathe", "On The Run"]);
        // Records too short to hold zone tables carry no zones (rather than erroring).
        assert!(scenes[0].zones.is_empty());
    }

    #[test]
    fn reads_scene_comment_at_0x40() {
        // One 0x80-byte record: name at 0x00, comment at 0x40.
        let rsize = 0x80;
        let mut area = Vec::new();
        area.extend_from_slice(&1u32.to_le_bytes());
        area.extend_from_slice(&(rsize as u32).to_le_bytes());
        area.extend_from_slice(&[0u8; 8]);
        let mut rec = vec![b' '; rsize];
        rec[..5].copy_from_slice(b"Scene");
        rec[COMMENT_OFFSET..COMMENT_OFFSET + 9].copy_from_slice(b"a memo   ".as_slice());
        area.extend_from_slice(&rec);

        let mut file = Vec::new();
        file.extend_from_slice(&30u16.to_le_bytes());
        file.extend_from_slice(b"SVD5");
        file.extend_from_slice(&[0u8; 10]);
        file.extend_from_slice(b"PRFa");
        file.extend_from_slice(b"KY19");
        file.extend_from_slice(&0x20u32.to_le_bytes());
        file.extend_from_slice(&(area.len() as u32).to_le_bytes());
        file.extend_from_slice(&area);

        let scenes = read_scenes(&Raw::from_bytes(file)).unwrap();
        assert_eq!(scenes[0].name, "Scene");
        assert_eq!(scenes[0].comment, "a memo");
    }

    #[test]
    fn resolve_tone_names_bundled_records_and_falls_back_to_presets() {
        let r = ToneResolver {
            names: HashMap::from([
                (
                    *b"PATa",
                    vec!["Strings Fall".to_owned(), "Jump Brass EmA".to_owned()],
                ),
                (*b"ZEPa", vec!["Time Intro EP".to_owned()]),
            ]),
        };

        // A user reference indexes its area directly: lsb 0 / pc 1 -> PATa[1].
        assert_eq!(
            resolve_tone(ZEN_CORE_MSB, 0x0001, Some(&r)),
            ToneRef::new(87, 0, 1, Some("Jump Brass EmA".into()))
        );
        // An index past the bundled records, or no resolver at all → address only, never a guess.
        assert_eq!(
            resolve_tone(ZEN_CORE_MSB, 0x0009, Some(&r)),
            ToneRef::new(87, 0, 9, None)
        );
        assert_eq!(
            resolve_tone(ZEN_CORE_MSB, 0x0001, None),
            ToneRef::new(87, 0, 1, None)
        );
        // LSB >= 64 is a factory ROM bank → the bundled sound list answers it.
        assert_eq!(
            resolve_tone(ZEN_CORE_MSB, 0x5c00, Some(&r)),
            ToneRef::new(87, 0x5c, 0, Some("AnalogAtmosphere".into()))
        );

        // ZEPa shares MSB 105 with ZAPa; LSB 1 selects it and rebases the index to 0.
        let sn_ep = resolve_tone(105, 0x0100, Some(&r));
        assert_eq!(sn_ep.name(), Some("Time Intro EP"));
        assert_eq!(sn_ep.tone_type(), crate::model::ToneType::SnEp);
        assert_eq!(sn_ep.bank(), Some("USER"));
    }

    /// The rule is the same in a full backup: a `PATa` holding the whole USER bank (and an `MDLa`
    /// area alongside it, which used to switch resolution off entirely) still resolves by index.
    #[test]
    fn resolves_user_tones_in_a_backup_shaped_file() {
        let mut names = vec![String::new(); 600];
        names[443] = "Africa Brass".to_owned();
        names[546] = "Sledgehammer Sha".to_owned();
        let r = ToneResolver {
            names: HashMap::from([(*b"PATa", names)]),
        };

        // Panel truth: NARF "Africa Main" zone 1 stores lsb 3 / pc 59.
        assert_eq!(
            resolve_tone(ZEN_CORE_MSB, 0x033b, Some(&r)).name(),
            Some("Africa Brass")
        );
        // "Sledgehammer" zone 1 stores lsb 4 / pc 34.
        assert_eq!(
            resolve_tone(ZEN_CORE_MSB, 0x0422, Some(&r)).name(),
            Some("Sledgehammer Sha")
        );
    }

    /// Build a full-size (3572-byte) record with valid markers, so `read_zones` runs the real path.
    fn record_with_zone(
        number: usize,
        enable: u8,
        key_low: u8,
        key_high: u8,
        level: u8,
    ) -> Vec<u8> {
        use crate::container::{RawZone, ZoneSettings};
        let mut rec = vec![0u8; 3572];
        // Every zone slot needs a valid marker or parsing fails.
        for n in 0..16 {
            let zb = RawZone::TABLE_OFFSET + n * RawZone::LEN;
            rec[zb + 0x3e] = 0xcf;
            rec[zb + 0x3f] = 0xcd;
            rec[ZoneSettings::TABLE_OFFSET + n * ZoneSettings::LEN] = 0x57;
        }
        let zb = RawZone::TABLE_OFFSET + number * RawZone::LEN;
        rec[zb + 0x04] = enable;
        rec[zb + 0x08] = key_low;
        rec[zb + 0x09] = key_high;
        rec[ZoneSettings::TABLE_OFFSET + number * ZoneSettings::LEN + 0x07] = level;
        rec
    }

    #[test]
    fn read_zones_decodes_switch_key_range_and_level() {
        let rec = record_with_zone(2, 1, 60, 72, 50);
        let zones = read_zones(&rec, None).unwrap();
        assert_eq!(zones.len(), 16);
        let z = &zones[2];
        assert_eq!(
            (z.number, z.enabled, z.key_low, z.key_high, z.level),
            (2, true, 60, 72, 50)
        );
        assert!(!zones[0].enabled);
    }

    /// Write `s` into `buf[at..at+len]`, space-padding the remainder (matches Fantom's ASCII
    /// field convention, so `ascii_trim` round-trips it exactly).
    fn write_ascii(buf: &mut [u8], at: usize, len: usize, s: &str) {
        let b = s.as_bytes();
        let n = b.len().min(len);
        buf[at..at + n].copy_from_slice(&b[..n]);
        buf[at + n..at + len].fill(b' ');
    }

    /// Build a real-size (3572-byte) scene record with a name and comment, all 16 zone slots
    /// zeroed (so they lack the `cf cd` marker and are skipped unless populated by [`golden_zone`]).
    fn golden_record(name: &str, comment: &str) -> Vec<u8> {
        let mut r = vec![0u8; 3572];
        write_ascii(&mut r, 0, NAME_LEN, name);
        write_ascii(&mut r, COMMENT_OFFSET, COMMENT_LEN, comment);
        r
    }

    /// Populate zone slot `n` with a valid marker and the given switch/range/level/tone.
    fn golden_zone(
        r: &mut [u8],
        n: usize,
        enabled: bool,
        key_low: u8,
        key_high: u8,
        level: u8,
        tone_id: u16,
    ) {
        let zb = RawZone::TABLE_OFFSET + n * RawZone::LEN;
        r[zb + 0x04] = enabled as u8;
        r[zb + 0x08] = key_low;
        r[zb + 0x09] = key_high;
        r[zb + 0x3e..zb + 0x40].copy_from_slice(&RawZone::MARKER);

        let sb = ZoneSettings::TABLE_OFFSET + n * ZoneSettings::LEN;
        r[sb + TONE_MSB_OFFSET] = ZEN_CORE_MSB;
        r[sb + 0x07] = level;
        r[sb + TONE_ID_OFFSET..sb + TONE_ID_OFFSET + 2].copy_from_slice(&tone_id.to_be_bytes());
    }

    /// Lay out `areas` (tag, format, body) as a full SVD5 file: header, area table, then bodies
    /// back to back. Mirrors the confirmed FANTOM-6 envelope (see `container::svd`).
    fn build_svd(areas: &[(&[u8; 4], &[u8; 4], &[u8])]) -> Raw {
        let table_len = areas.len() * 16;
        let header_size = 14 + table_len as u16;

        let mut table = Vec::new();
        let mut bodies = Vec::new();
        let mut offset = 0x10 + table_len;
        for (tag, format, body) in areas {
            table.extend_from_slice(*tag);
            table.extend_from_slice(*format);
            table.extend_from_slice(&(offset as u32).to_le_bytes());
            table.extend_from_slice(&(body.len() as u32).to_le_bytes());
            bodies.extend_from_slice(body);
            offset += body.len();
        }

        let mut file = Vec::new();
        file.extend_from_slice(&header_size.to_le_bytes());
        file.extend_from_slice(b"SVD5");
        file.extend_from_slice(&[0u8; 10]);
        file.extend_from_slice(&table);
        file.extend_from_slice(&bodies);
        Raw::from_bytes(file)
    }

    fn named_record_area(record_size: usize, name_offset: usize, names: &[&str]) -> Vec<u8> {
        let mut area = Vec::new();
        area.extend_from_slice(&(names.len() as u32).to_le_bytes());
        area.extend_from_slice(&(record_size as u32).to_le_bytes());
        area.extend_from_slice(&[0u8; 8]);
        for name in names {
            let mut record = vec![0u8; record_size];
            write_ascii(&mut record, name_offset, NAME_LEN, name);
            area.extend_from_slice(&record);
        }
        area
    }

    #[test]
    fn lists_multi_record_model_and_vpiano_tones() {
        let dcwa = named_record_area(684, 0, &["Stage Grand4", "Stage Grand4 3"]);
        let mdla = named_record_area(2048, 0x10, &["Berlin Night  4", "Berlin Night  6"]);
        let raw = build_svd(&[(b"DCWa", b"KY19", &dcwa), (b"MDLa", b"KY19", &mdla)]);

        let tones = read_bundled_tones(&raw).unwrap();
        let tones: Vec<_> = tones
            .iter()
            .map(|tone| (tone.area, tone.tone_type, tone.index, tone.name.as_str()))
            .collect();
        assert_eq!(
            tones,
            [
                (*b"DCWa", ToneType::VPiano, 0, "Stage Grand4"),
                (*b"DCWa", ToneType::VPiano, 1, "Stage Grand4 3"),
                (*b"MDLa", ToneType::Model, 0, "Berlin Night  4"),
                (*b"MDLa", ToneType::Model, 1, "Berlin Night  6"),
            ]
        );
    }

    /// End-to-end regression test over a real-size, real-shaped SVD5 file: two PRFa scenes plus a
    /// matching PATa tone list, decoded through the public `read_scenes` entry point. Exercises the
    /// container envelope, scene name/comment, per-zone switch/range/level, direct user-tone
    /// indexing (including across an LSB page boundary), factory preset resolution, and the "skip
    /// invalid zone slots" behavior — all in one committable fixture, independent of any real file.
    #[test]
    fn golden_file_round_trips_through_read_scenes() {
        // JX Cream = PR-A 0061 (LSB 92, PC 61) -> (92<<8)|(61-1) = 0x5c3c; a real bundled preset.
        const JX_CREAM: u16 = 0x5c3c;
        // User tones index PATa directly: lsb 0 / pc 5 -> PATa[5].
        const USER_A: u16 = 0x0005;
        // lsb 1 / pc 1 -> PATa[129], one past the first LSB page.
        const USER_B: u16 = 0x0101;

        let mut scene1 = golden_record("Golden Scene", "regression fixture");
        golden_zone(&mut scene1, 0, true, 0, 60, 100, USER_A);
        golden_zone(&mut scene1, 1, false, 61, 127, 90, JX_CREAM);
        golden_zone(&mut scene1, 2, true, 0, 127, 127, USER_B);
        // Zones 3..16 are left zeroed (no marker) — expected to be skipped, not errored.

        let scene2 = golden_record("Second Scene", "");
        // No zones populated at all: expect an empty zone list, not a parse failure.

        let mut prfa = Vec::new();
        prfa.extend_from_slice(&2u32.to_le_bytes()); // declared count
        prfa.extend_from_slice(&(scene1.len() as u32).to_le_bytes()); // record_size
        prfa.extend_from_slice(&[0u8; 8]);
        prfa.extend_from_slice(&scene1);
        prfa.extend_from_slice(&scene2);

        // A 130-record PATa, so the referenced indexes land where the addresses point.
        let mut names = vec![""; 130];
        names[5] = "Golden User A";
        names[129] = "Golden User B";
        let pata = named_record_area(32, 0, &names);

        let raw = build_svd(&[(b"PRFa", b"KY19", &prfa), (b"PATa", b"KY19", &pata)]);
        let scenes = read_scenes(&raw).unwrap();

        assert_eq!(scenes.len(), 2);

        let s1 = &scenes[0];
        assert_eq!(s1.name, "Golden Scene");
        assert_eq!(s1.comment, "regression fixture");
        assert_eq!(s1.zones.len(), 3, "zones without a marker must be skipped");

        assert_eq!(s1.zones[0].number, 0);
        assert!(s1.zones[0].enabled);
        assert_eq!(
            (s1.zones[0].key_low, s1.zones[0].key_high, s1.zones[0].level),
            (0, 60, 100)
        );
        assert_eq!(
            s1.zones[0].tone,
            ToneRef::new(87, 0, 5, Some("Golden User A".into()))
        );

        assert_eq!(s1.zones[1].number, 1);
        assert!(!s1.zones[1].enabled);
        assert_eq!(
            s1.zones[1].tone,
            ToneRef::new(87, 0x5c, 0x3c, Some("JX Cream".into()))
        );
        assert_eq!(s1.zones[1].tone.name(), Some("JX Cream"));
        assert_eq!(s1.zones[1].tone.preset().unwrap().bank, "PR-A");
        assert_eq!(s1.zones[1].tone.preset().unwrap().number, 61);

        assert_eq!(s1.zones[2].number, 2);
        assert_eq!(
            s1.zones[2].tone,
            ToneRef::new(87, 1, 1, Some("Golden User B".into()))
        );

        let s2 = &scenes[1];
        assert_eq!(s2.name, "Second Scene");
        assert_eq!(s2.comment, "");
        assert!(s2.zones.is_empty());
    }

    #[test]
    fn resolves_word_swapped_acb_user_name() {
        let mut scene = golden_record("ACB Scene", "");
        golden_zone(&mut scene, 0, true, 0, 127, 100, 0);
        scene[ZoneSettings::TABLE_OFFSET + TONE_MSB_OFFSET] = 107;

        let mut prfa = Vec::new();
        prfa.extend_from_slice(&1u32.to_le_bytes());
        prfa.extend_from_slice(&(scene.len() as u32).to_le_bytes());
        prfa.extend_from_slice(&[0u8; 8]);
        prfa.extend_from_slice(&scene);

        let mut acb_record = vec![0u8; 9984];
        let name = b"Soft & Subtle3  ";
        for (source, target) in name
            .chunks_exact(4)
            .zip(acb_record[0x1c44..0x1c54].chunks_exact_mut(4))
        {
            target.copy_from_slice(&[source[3], source[2], source[1], source[0]]);
        }
        let mut acba = Vec::new();
        acba.extend_from_slice(&1u32.to_le_bytes());
        acba.extend_from_slice(&(acb_record.len() as u32).to_le_bytes());
        acba.extend_from_slice(&[0u8; 8]);
        acba.extend_from_slice(&acb_record);

        let raw = build_svd(&[(b"PRFa", b"KY19", &prfa), (b"ACBa", b"KY19", &acba)]);
        let scenes = read_scenes(&raw).unwrap();
        assert_eq!(scenes[0].zones[0].tone.name(), Some("Soft & Subtle3"));
        assert_eq!(
            read_bundled_tones(&raw).unwrap(),
            [BundledTone {
                area: *b"ACBa",
                tone_type: ToneType::Acb,
                index: 0,
                name: "Soft & Subtle3".into(),
            }]
        );
    }

    #[test]
    fn edit_scene_name_and_comment_touches_only_those_fields() {
        let scene1 = golden_record("Old Name", "old comment");
        let scene2 = golden_record("Keep Me", "keep");
        let mut prfa = Vec::new();
        prfa.extend_from_slice(&2u32.to_le_bytes());
        prfa.extend_from_slice(&(scene1.len() as u32).to_le_bytes());
        prfa.extend_from_slice(&[0u8; 8]);
        prfa.extend_from_slice(&scene1);
        prfa.extend_from_slice(&scene2);
        let raw0 = build_svd(&[(b"PRFa", b"KY19", &prfa)]);
        let before = raw0.bytes().to_vec();

        let mut raw = raw0.clone();
        set_scene_name(&mut raw, 1, "New Name").unwrap();
        set_scene_comment(&mut raw, 1, "new comment").unwrap();

        // A name the field cannot hold is refused rather than written badly, and nothing changes.
        let mut rejected = raw.clone();
        assert!(set_scene_name(&mut rejected, 1, "a name far longer than sixteen").is_err());
        assert!(set_scene_name(&mut rejected, 1, "café").is_err());
        assert_eq!(rejected.bytes(), raw.bytes());

        // Re-parsing shows the edits, and scene 2 is untouched.
        let scenes = read_scenes(&raw).unwrap();
        assert_eq!(
            (scenes[0].name.as_str(), scenes[0].comment.as_str()),
            ("New Name", "new comment")
        );
        assert_eq!(
            (scenes[1].name.as_str(), scenes[1].comment.as_str()),
            ("Keep Me", "keep")
        );

        // Byte-faithful: same length, and every changed byte lies inside scene 1's name or comment.
        let after = raw.bytes();
        assert_eq!(after.len(), before.len());
        let rec = scene_record_offset(&raw0, 1).unwrap();
        let name_field = rec..rec + NAME_LEN;
        let comment_field = rec + COMMENT_OFFSET..rec + COMMENT_OFFSET + COMMENT_LEN;
        for (i, (a, b)) in before.iter().zip(after).enumerate() {
            if a != b {
                assert!(
                    name_field.contains(&i) || comment_field.contains(&i),
                    "byte {i:#x} changed outside the name/comment fields"
                );
            }
        }
    }
}
