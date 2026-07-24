//! Mapping between container bytes and the [`crate::model`] types.
//!
//! This is where a confirmed byte layout becomes [`crate::model`] values: scene name, comment, and
//! 16 zones (switch, key range, level, tone) from the `PRFa` area, with user-tone names resolved
//! from `PATa` where possible (see `docs/FORMAT.md`).

use std::collections::{BTreeSet, HashMap};
use std::io::Cursor;

use binrw::BinRead as _;

use crate::container::{ascii_trim, PatArea, Raw, RawZone, Svd, ZoneSettings};
use crate::model::{Scene, ToneRef, Zone};
use crate::{Error, Result};

/// The area tag holding Performances/Scenes in a FANTOM-6 SVD backup.
const PRFA: &[u8; 4] = b"PRFa";

/// Offset of the per-zone tone id within a settings-table record (`0x194`), read big-endian.
const TONE_ID_OFFSET: usize = 0x01;

/// Tone ids with this bit set are factory ROM presets, not stored in the file's `PATa`.
const PRESET_FLAG: u16 = 0x4000;

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

/// Decode every scene in an SVD file — name, comment, and 16 zones (switch, key range, level, and
/// tone, with user-tone names resolved for scene exports; see [`read_zones`]).
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

    let pat = PatArea::from_svd(raw, &svd).ok();
    let is_backup = svd.area(b"MDLa").is_some();

    // Pass 1: collect the scene records and every referenced user tone id.
    let mut records: Vec<(String, String, &[u8])> = Vec::new();
    let mut user_gids: BTreeSet<u16> = BTreeSet::new();
    let mut pos = AREA_HEADER_LEN;
    while pos + NAME_LEN <= area.len() && records.len() < declared_count {
        let name = ascii_trim(&area[pos..pos + NAME_LEN]);
        // Empty-named slots pad the bank to a fixed capacity; keep only real scenes.
        if !name.is_empty() {
            let record = &area[pos..(pos + record_size).min(area.len())];
            let comment = record
                .get(COMMENT_OFFSET..COMMENT_OFFSET + COMMENT_LEN)
                .map(ascii_trim)
                .unwrap_or_default();
            for n in 0..ZONE_COUNT {
                if let Some((_, _, id)) = decode_zone_slot(record, n) {
                    if id & PRESET_FLAG == 0 {
                        user_gids.insert(id);
                    }
                }
            }
            records.push((name, comment, record));
        }
        pos += record_size;
    }

    // A user tone indexes `PATa` by the **rank** of its gid among all referenced gids: `PATa` is
    // stored gid-sorted and de-duplicated, so the Nth-smallest gid is `PATa[N]`. This holds only
    // when `PATa` contains exactly the referenced tones — true for scene exports, detected by the
    // unique-gid count matching the `PATa` count. Full backups keep every tone in `PATa` (and carry
    // an `MDLa` area), so the rank does not apply and their user tones stay unresolved.
    let rank: HashMap<u16, usize> = user_gids.iter().enumerate().map(|(i, &g)| (g, i)).collect();
    let resolver = pat
        .as_ref()
        .filter(|_| !is_backup)
        .filter(|p| p.tones().len() == rank.len())
        .map(|p| ToneResolver { pat: p, rank: &rank });

    records
        .into_iter()
        .map(|(name, comment, record)| {
            Ok(Scene {
                name,
                comment,
                zones: read_zones(record, resolver.as_ref())?,
            })
        })
        .collect()
}

/// Resolves user-tone gids to `PATa` names for one file (see [`read_scenes`]).
struct ToneResolver<'a> {
    pat: &'a PatArea,
    rank: &'a HashMap<u16, usize>,
}

/// Read a zone's raw tone id (settings table `+0x01`, big-endian), if the record is long enough.
fn zone_tone_id(record: &[u8], n: usize) -> Option<u16> {
    let at = ZoneSettings::TABLE_OFFSET + n * ZoneSettings::LEN + TONE_ID_OFFSET;
    record.get(at..at + 2).map(|s| u16::from_be_bytes([s[0], s[1]]))
}

/// Decode zone slot `n` of a scene `record`, or `None` when the record is too short to hold it or
/// the slot lacks the `cf cd` alignment marker (uninitialized, corrupt, or a layout we don't
/// recognise). This is the single gate for "is this a real zone" — used both to decide which zones
/// to emit ([`read_zones`]) and which tone ids count toward the gid-rank resolver ([`read_scenes`]),
/// so an unpopulated slot can never leak a spurious tone id into the resolver.
fn decode_zone_slot(record: &[u8], n: usize) -> Option<(RawZone, ZoneSettings, u16)> {
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
    let tone_id = zone_tone_id(record, n).unwrap_or(0);
    Some((z, s, tone_id))
}

/// Decode the 16 zones from a single scene `record` (the slice starting at the zone name).
///
/// Combines the zone table (`0x6d0`, switch + key range) with the settings table (`0x194`, level +
/// tone id). `resolver` supplies user-tone names when the file's `PATa` maps by gid rank (scene
/// exports); it is `None` for full backups, which leave user tones unresolved.
///
/// Resilient by design: a record too short to hold the zone tables yields no zones, and any
/// individual zone slot that lacks the `cf cd` alignment marker is skipped rather than failing the
/// whole file (see [`decode_zone_slot`]).
fn read_zones(record: &[u8], resolver: Option<&ToneResolver>) -> Result<Vec<Zone>> {
    let mut zones = Vec::with_capacity(ZONE_COUNT);
    for n in 0..ZONE_COUNT {
        let Some((z, s, tone_id)) = decode_zone_slot(record, n) else {
            continue;
        };
        zones.push(Zone {
            number: n as u8,
            enabled: z.enable != 0,
            tone: resolve_tone(tone_id, resolver),
            key_low: z.key_low,
            key_high: z.key_high,
            level: s.level,
        });
    }
    Ok(zones)
}

/// Resolve a raw 16-bit tone id into a [`ToneRef`].
fn resolve_tone(id: u16, resolver: Option<&ToneResolver>) -> ToneRef {
    if id & PRESET_FLAG != 0 {
        ToneRef::Preset { id }
    } else {
        let name = resolver.and_then(|r| {
            let idx = *r.rank.get(&id)?;
            Some(r.pat.get(idx)?.name.clone())
        });
        ToneRef::User { id, name }
    }
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
    fn resolve_tone_maps_gid_rank_and_presets() {
        use crate::container::PatArea;
        // A tiny gid-sorted PATa with two tones.
        let mut area = Vec::new();
        area.extend_from_slice(&2u32.to_le_bytes()); // count
        area.extend_from_slice(&32u32.to_le_bytes()); // record_size
        area.extend_from_slice(&[0u8; 8]);
        for name in ["Strings Fall", "Jump Brass EmA"] {
            let mut rec = vec![0u8; 32];
            rec[..name.len()].copy_from_slice(name.as_bytes());
            area.extend_from_slice(&rec);
        }
        let pat = PatArea::parse(&area).unwrap();
        // gids 10 and 42 rank to PATa[0] and PATa[1].
        let rank: HashMap<u16, usize> = [(10u16, 0usize), (42, 1)].into_iter().collect();
        let r = ToneResolver { pat: &pat, rank: &rank };

        assert_eq!(
            resolve_tone(42, Some(&r)),
            ToneRef::User { id: 42, name: Some("Jump Brass EmA".into()) }
        );
        // A gid not in the rank map, or no resolver at all → id only.
        assert_eq!(resolve_tone(99, Some(&r)), ToneRef::User { id: 99, name: None });
        assert_eq!(resolve_tone(10, None), ToneRef::User { id: 10, name: None });
        // Preset flag set → Preset regardless.
        assert_eq!(resolve_tone(0x5c00, Some(&r)), ToneRef::Preset { id: 0x5c00 });
    }

    /// Build a full-size (3572-byte) record with valid markers, so `read_zones` runs the real path.
    fn record_with_zone(number: usize, enable: u8, key_low: u8, key_high: u8, level: u8) -> Vec<u8> {
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
    fn golden_zone(r: &mut [u8], n: usize, enabled: bool, key_low: u8, key_high: u8, level: u8, tone_id: u16) {
        let zb = RawZone::TABLE_OFFSET + n * RawZone::LEN;
        r[zb + 0x04] = enabled as u8;
        r[zb + 0x08] = key_low;
        r[zb + 0x09] = key_high;
        r[zb + 0x3e..zb + 0x40].copy_from_slice(&RawZone::MARKER);

        let sb = ZoneSettings::TABLE_OFFSET + n * ZoneSettings::LEN;
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

    /// End-to-end regression test over a real-size, real-shaped SVD5 file: two PRFa scenes plus a
    /// matching PATa tone list, decoded through the public `read_scenes` entry point. Exercises the
    /// container envelope, scene name/comment, per-zone switch/range/level, the gid-rank user-tone
    /// resolver, factory preset resolution, and the "skip invalid zone slots" behavior — all in one
    /// committable fixture, independent of any real Fantom file.
    #[test]
    fn golden_file_round_trips_through_read_scenes() {
        // JX Cream = PR-A 0061 (LSB 92, PC 61) -> (92<<8)|(61-1) = 0x5c3c; a real bundled preset.
        const JX_CREAM: u16 = 0x5c3c;
        const USER_A_GID: u16 = 100;
        const USER_B_GID: u16 = 200;

        let mut scene1 = golden_record("Golden Scene", "regression fixture");
        golden_zone(&mut scene1, 0, true, 0, 60, 100, USER_A_GID);
        golden_zone(&mut scene1, 1, false, 61, 127, 90, JX_CREAM);
        golden_zone(&mut scene1, 2, true, 0, 127, 127, USER_B_GID);
        // Zones 3..16 are left zeroed (no marker) — expected to be skipped, not errored.

        let scene2 = golden_record("Second Scene", "");
        // No zones populated at all: expect an empty zone list, not a parse failure.

        let mut prfa = Vec::new();
        prfa.extend_from_slice(&2u32.to_le_bytes()); // declared count
        prfa.extend_from_slice(&(scene1.len() as u32).to_le_bytes()); // record_size
        prfa.extend_from_slice(&[0u8; 8]);
        prfa.extend_from_slice(&scene1);
        prfa.extend_from_slice(&scene2);

        // PATa gid-sorted: rank(USER_A_GID)=0, rank(USER_B_GID)=1 (ranks assigned by ascending
        // gid), so PATa[0]/[1] must be named accordingly for the export-style resolver to fire.
        let mut pata = Vec::new();
        pata.extend_from_slice(&2u32.to_le_bytes()); // count
        pata.extend_from_slice(&32u32.to_le_bytes()); // record_size
        pata.extend_from_slice(&[0u8; 8]);
        for name in ["Golden User A", "Golden User B"] {
            let mut rec = vec![0u8; 32];
            write_ascii(&mut rec, 0, NAME_LEN, name);
            pata.extend_from_slice(&rec);
        }

        let raw = build_svd(&[(b"PRFa", b"KY19", &prfa), (b"PATa", b"KY19", &pata)]);
        let scenes = read_scenes(&raw).unwrap();

        assert_eq!(scenes.len(), 2);

        let s1 = &scenes[0];
        assert_eq!(s1.name, "Golden Scene");
        assert_eq!(s1.comment, "regression fixture");
        assert_eq!(s1.zones.len(), 3, "zones without a marker must be skipped");

        assert_eq!(s1.zones[0].number, 0);
        assert!(s1.zones[0].enabled);
        assert_eq!((s1.zones[0].key_low, s1.zones[0].key_high, s1.zones[0].level), (0, 60, 100));
        assert_eq!(
            s1.zones[0].tone,
            ToneRef::User { id: USER_A_GID, name: Some("Golden User A".into()) }
        );

        assert_eq!(s1.zones[1].number, 1);
        assert!(!s1.zones[1].enabled);
        assert_eq!(s1.zones[1].tone, ToneRef::Preset { id: JX_CREAM });
        assert_eq!(s1.zones[1].tone.name(), Some("JX Cream"));
        assert_eq!(s1.zones[1].tone.preset().unwrap().bank, "PR-A");
        assert_eq!(s1.zones[1].tone.preset().unwrap().number, 61);

        assert_eq!(s1.zones[2].number, 2);
        assert_eq!(
            s1.zones[2].tone,
            ToneRef::User { id: USER_B_GID, name: Some("Golden User B".into()) }
        );

        let s2 = &scenes[1];
        assert_eq!(s2.name, "Second Scene");
        assert_eq!(s2.comment, "");
        assert!(s2.zones.is_empty());
    }
}
