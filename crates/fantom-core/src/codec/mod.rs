//! Mapping between container bytes and the [`crate::model`] types.
//!
//! This is where a confirmed byte layout becomes [`crate::model`] values. It reads scene *names*
//! from the `PRFa` area today; the zone/tone contents of each record are still being
//! reverse-engineered (see `docs/FORMAT.md`).

use std::io::Cursor;

use binrw::BinRead as _;

use crate::container::{ascii_trim, Raw, RawZone, Svd, ZoneSettings};
use crate::model::{Scene, Zone};
use crate::{Error, Result};

/// The area tag holding Performances/Scenes in a Fantom-0 SVD backup.
const PRFA: &[u8; 4] = b"PRFa";

/// The `PRFa` area opens with a fixed 16-byte header, then fixed-stride records begin.
/// `+0x00` = declared scene count; `+0x04` = record stride in bytes.
const AREA_HEADER_LEN: usize = 0x10;
const AREA_COUNT_OFFSET: usize = 0x00;
const AREA_RECORD_SIZE_OFFSET: usize = 0x04;

/// Each record begins with its 16-byte ASCII name (space/NUL padded).
const NAME_LEN: usize = 16;

/// Number of zone slots in every scene.
const ZONE_COUNT: usize = 16;

/// Decode every scene (name only, for now) contained in an SVD backup.
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

    let mut scenes = Vec::new();
    // Records start right after the header and are capped by the declared count.
    let mut pos = AREA_HEADER_LEN;
    while pos + NAME_LEN <= area.len() && scenes.len() < declared_count {
        let name = ascii_trim(&area[pos..pos + NAME_LEN]);
        // Empty-named slots pad the bank to a fixed capacity; keep only real scenes.
        if !name.is_empty() {
            let record = &area[pos..(pos + record_size).min(area.len())];
            scenes.push(Scene {
                name,
                zones: read_zones(record)?,
            });
        }
        pos += record_size;
    }
    Ok(scenes)
}

/// Decode the 16 zones from a single scene `record` (the slice starting at the zone name).
///
/// Combines the zone table (`0x6d0`, switch + key range) with the settings table (`0x194`, level).
/// Returns an empty vector when the record is too short to hold the zone tables — real Fantom
/// records are 3572 bytes; synthetic/truncated records simply carry no zones.
pub fn read_zones(record: &[u8]) -> Result<Vec<Zone>> {
    let zones_end = RawZone::TABLE_OFFSET + ZONE_COUNT * RawZone::LEN;
    let settings_end = ZoneSettings::TABLE_OFFSET + ZONE_COUNT * ZoneSettings::LEN;
    if record.len() < zones_end.max(settings_end) {
        return Ok(Vec::new());
    }

    let mut zones = Vec::with_capacity(ZONE_COUNT);
    for n in 0..ZONE_COUNT {
        let z = RawZone::read(&mut cursor_at(record, RawZone::TABLE_OFFSET + n * RawZone::LEN))?;
        let s = ZoneSettings::read(&mut cursor_at(
            record,
            ZoneSettings::TABLE_OFFSET + n * ZoneSettings::LEN,
        ))?;
        zones.push(Zone {
            number: n as u8,
            enabled: z.enable != 0,
            key_low: z.key_low,
            key_high: z.key_high,
            level: s.level,
        });
    }
    Ok(zones)
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
    /// following a 16-byte area header. Mirrors the confirmed Fantom-0 layout.
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
        let zones = read_zones(&rec).unwrap();
        assert_eq!(zones.len(), 16);
        let z = &zones[2];
        assert_eq!(
            (z.number, z.enabled, z.key_low, z.key_high, z.level),
            (2, true, 60, 72, 50)
        );
        assert!(!zones[0].enabled);
    }
}
