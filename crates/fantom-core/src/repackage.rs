//! Extract and merge scenes from FANTOM scene-export SVD files.
//!
//! Self-contained scene exports bundle exactly the user tones referenced by their scenes in
//! `PATa`. Repackaging rebuilds that bundle and rewrites user-tone ids while leaving factory preset
//! ids untouched. Files without that exact mapping (including full backups) are deliberately
//! rejected.

use std::collections::{BTreeSet, HashMap};

use crate::container::{ascii_trim, Raw, RawZone, Svd, ZoneSettings};
use crate::{Error, Result};

const HEADER_LEN: usize = 0x10;
const COUNT_OFFSET: usize = 0;
const RECORD_SIZE_OFFSET: usize = 4;
const NAME_LEN: usize = 16;
const TONE_ID_OFFSET: usize = 1;
const PRESET_FLAG: u16 = 0x4000;

/// Build a scene-export SVD containing only the requested 1-based scene numbers, in the order
/// given. All referenced user tones are copied and their ids are rebased for the new bank.
pub fn extract_scenes(raw: &Raw, scene_numbers: &[usize]) -> Result<Raw> {
    if scene_numbers.is_empty() {
        return Err(Error::Unrecognized(
            "at least one scene number is required".into(),
        ));
    }
    let bank = ExportBank::parse(raw)?;
    let records = scene_numbers
        .iter()
        .map(|number| {
            bank.scenes
                .get(number.wrapping_sub(1))
                .map(|record| (&bank, record))
                .ok_or_else(|| {
                    Error::Unrecognized(format!(
                        "scene {number} out of range (file has {})",
                        bank.scenes.len()
                    ))
                })
        })
        .collect::<Result<Vec<_>>>()?;
    rebuild(raw, &bank, records)
}

/// Extract one scene and give its scene and bundled-tone names visible `CNY` markers. Tone payloads
/// are left untouched, making this a hardware test for whether the embedded `PATa` bundle is used.
pub fn canary_scene(raw: &Raw, scene_number: usize) -> Result<Raw> {
    let scenes = crate::codec::read_scenes(raw)?;
    let original_name = scenes
        .get(scene_number.wrapping_sub(1))
        .map(|scene| scene.name.clone())
        .ok_or_else(|| {
            Error::Unrecognized(format!(
                "scene {scene_number} out of range (file has {})",
                scenes.len()
            ))
        })?;
    let mut canary = extract_scenes(raw, &[scene_number])?;
    crate::codec::set_scene_name(&mut canary, 1, &format!("CNY {original_name}"))?;
    mark_bundled_tone_names(&mut canary)?;
    Ok(canary)
}

/// Append every scene from `source` to `target`, rebundling and de-duplicating user tones. Both
/// inputs must be self-contained scene exports with matching scene/tone record sizes.
pub fn merge_scenes(target: &Raw, source: &Raw) -> Result<Raw> {
    let target_bank = ExportBank::parse(target)?;
    let source_bank = ExportBank::parse(source)?;
    if target_bank.scene_record_size != source_bank.scene_record_size {
        return Err(Error::Unrecognized(format!(
            "PRFa record sizes differ (target {}, source {})",
            target_bank.scene_record_size, source_bank.scene_record_size
        )));
    }
    if target_bank.tone_record_size != source_bank.tone_record_size {
        return Err(Error::Unrecognized(format!(
            "PATa record sizes differ (target {}, source {})",
            target_bank.tone_record_size, source_bank.tone_record_size
        )));
    }
    if target_bank.scene_format != source_bank.scene_format
        || target_bank.tone_format != source_bank.tone_format
    {
        return Err(Error::Unrecognized(
            "source and target use different PRFa/PATa format versions".into(),
        ));
    }

    let records = target_bank
        .scenes
        .iter()
        .map(|record| (&target_bank, record))
        .chain(
            source_bank
                .scenes
                .iter()
                .map(|record| (&source_bank, record)),
        )
        .collect();
    rebuild(target, &target_bank, records)
}

struct ExportBank {
    scenes: Vec<Vec<u8>>,
    tones: Vec<Vec<u8>>,
    tone_by_gid: HashMap<u16, usize>,
    scene_header: [u8; HEADER_LEN],
    tone_header: [u8; HEADER_LEN],
    scene_format: [u8; 4],
    tone_format: [u8; 4],
    scene_record_size: usize,
    tone_record_size: usize,
}

impl ExportBank {
    fn parse(raw: &Raw) -> Result<Self> {
        let svd = Svd::parse(raw)?;
        if svd.area(b"MDLa").is_some() {
            return Err(Error::Unrecognized(
                "full backups cannot be repackaged: their user-tone mapping is unresolved".into(),
            ));
        }

        let prfa = svd
            .area(b"PRFa")
            .ok_or_else(|| Error::Unrecognized("no PRFa (performance) area in file".into()))?;
        let pata = svd
            .area(b"PATa")
            .ok_or_else(|| Error::Unrecognized("no PATa (tone) area in file".into()))?;
        let (scene_header, scene_record_size, scenes) =
            parse_records(svd.area_bytes(raw, prfa)?, "PRFa", true)?;
        let (tone_header, tone_record_size, tones) =
            parse_records(svd.area_bytes(raw, pata)?, "PATa", false)?;

        let gids: BTreeSet<u16> = scenes
            .iter()
            .flat_map(|record| user_tone_ids(record))
            .collect();
        if gids.len() != tones.len() {
            return Err(Error::Unrecognized(format!(
                "not a self-contained scene export: {} referenced user-tone ids but PATa has {} records",
                gids.len(),
                tones.len()
            )));
        }
        let tone_by_gid = gids
            .into_iter()
            .enumerate()
            .map(|(index, gid)| (gid, index))
            .collect();

        Ok(Self {
            scenes,
            tones,
            tone_by_gid,
            scene_header,
            tone_header,
            scene_format: prfa.format,
            tone_format: pata.format,
            scene_record_size,
            tone_record_size,
        })
    }

    fn tone_for_gid(&self, gid: u16) -> Result<&[u8]> {
        let index = self
            .tone_by_gid
            .get(&gid)
            .ok_or_else(|| Error::Unrecognized(format!("user-tone id {gid} has no PATa record")))?;
        Ok(&self.tones[*index])
    }
}

fn parse_records(
    area: &[u8],
    tag: &str,
    skip_empty_names: bool,
) -> Result<([u8; HEADER_LEN], usize, Vec<Vec<u8>>)> {
    let header: [u8; HEADER_LEN] = area
        .get(..HEADER_LEN)
        .ok_or_else(|| Error::Unrecognized(format!("{tag} area is shorter than its header")))?
        .try_into()
        .unwrap();
    let count = read_u32(area, COUNT_OFFSET, tag)? as usize;
    let record_size = read_u32(area, RECORD_SIZE_OFFSET, tag)? as usize;
    if record_size == 0 {
        return Err(Error::Unrecognized(format!("{tag} record size is zero")));
    }
    if skip_empty_names && record_size < NAME_LEN {
        return Err(Error::Unrecognized(format!(
            "{tag} record size {record_size} is shorter than its name field"
        )));
    }
    if !skip_empty_names && record_size <= NAME_LEN {
        return Err(Error::Unrecognized(format!(
            "{tag} record size {record_size} is shorter than a tone record"
        )));
    }

    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let start = HEADER_LEN + index * record_size;
        let record = area.get(start..start + record_size).ok_or_else(|| {
            Error::Unrecognized(format!("{tag} record {} is truncated", index + 1))
        })?;
        if !skip_empty_names || record[..NAME_LEN].iter().any(|b| *b != 0 && *b != b' ') {
            records.push(record.to_vec());
        }
    }
    Ok((header, record_size, records))
}

fn rebuild<'a>(
    base: &Raw,
    header_bank: &ExportBank,
    records: Vec<(&'a ExportBank, &'a Vec<u8>)>,
) -> Result<Raw> {
    let mut tones: Vec<Vec<u8>> = Vec::new();
    let mut tone_ids: HashMap<Vec<u8>, u16> = HashMap::new();
    let mut scenes = Vec::with_capacity(records.len());

    for (bank, original) in records {
        let mut scene = original.clone();
        let slots: Vec<_> = valid_zone_slots(&scene).collect();
        for slot in slots {
            let at = tone_id_offset(slot);
            let old_id = u16::from_be_bytes([scene[at], scene[at + 1]]);
            if old_id & PRESET_FLAG != 0 {
                continue;
            }
            let tone = bank.tone_for_gid(old_id)?;
            let new_id = match tone_ids.get(tone) {
                Some(id) => *id,
                None => {
                    let id = u16::try_from(tones.len())
                        .map_err(|_| Error::Unrecognized("too many bundled user tones".into()))?;
                    if id & PRESET_FLAG != 0 {
                        return Err(Error::Unrecognized(
                            "too many bundled user tones for the 14-bit id space".into(),
                        ));
                    }
                    tones.push(tone.to_vec());
                    tone_ids.insert(tone.to_vec(), id);
                    id
                }
            };
            scene[at..at + 2].copy_from_slice(&new_id.to_be_bytes());
        }
        scenes.push(scene);
    }

    let prfa = build_area(
        header_bank.scene_header,
        header_bank.scene_record_size,
        &scenes,
    )?;
    let pata = build_area(
        header_bank.tone_header,
        header_bank.tone_record_size,
        &tones,
    )?;
    replace_areas(base, &[(b"PRFa", prfa), (b"PATa", pata)])
}

fn build_area(
    mut header: [u8; HEADER_LEN],
    record_size: usize,
    records: &[Vec<u8>],
) -> Result<Vec<u8>> {
    let count = u32::try_from(records.len())
        .map_err(|_| Error::Unrecognized("too many records to encode".into()))?;
    header[COUNT_OFFSET..COUNT_OFFSET + 4].copy_from_slice(&count.to_le_bytes());
    header[RECORD_SIZE_OFFSET..RECORD_SIZE_OFFSET + 4]
        .copy_from_slice(&(record_size as u32).to_le_bytes());
    let mut area = Vec::with_capacity(HEADER_LEN + records.len() * record_size);
    area.extend_from_slice(&header);
    for record in records {
        if record.len() != record_size {
            return Err(Error::Unrecognized(
                "record size changed while repackaging".into(),
            ));
        }
        area.extend_from_slice(record);
    }
    Ok(area)
}

fn mark_bundled_tone_names(raw: &mut Raw) -> Result<()> {
    let svd = Svd::parse(raw)?;
    let pata = svd
        .area(b"PATa")
        .ok_or_else(|| Error::Unrecognized("no PATa (tone) area in file".into()))?;
    let area = svd.area_bytes(raw, pata)?;
    let count = read_u32(area, COUNT_OFFSET, "PATa")? as usize;
    let record_size = read_u32(area, RECORD_SIZE_OFFSET, "PATa")? as usize;
    if record_size < NAME_LEN {
        return Err(Error::Unrecognized(
            "PATa record is shorter than its tone-name field".into(),
        ));
    }

    let mut names = Vec::with_capacity(count);
    for index in 0..count {
        let start = HEADER_LEN + index * record_size;
        let name = area.get(start..start + NAME_LEN).ok_or_else(|| {
            Error::Unrecognized(format!("PATa record {} is truncated", index + 1))
        })?;
        names.push((
            pata.offset as usize + start,
            format!("CNY{:02} {}", index + 1, ascii_trim(name)),
        ));
    }
    for (offset, name) in names {
        raw.patch_ascii(offset, NAME_LEN, &name);
    }
    Ok(())
}

fn replace_areas(raw: &Raw, replacements: &[(&[u8; 4], Vec<u8>)]) -> Result<Raw> {
    let svd = Svd::parse(raw)?;
    let mut ordered: Vec<_> = svd.areas.iter().enumerate().collect();
    ordered.sort_by_key(|(_, area)| area.offset);
    let first_offset = ordered
        .first()
        .map(|(_, area)| area.offset as usize)
        .ok_or_else(|| Error::Unrecognized("SVD has no areas".into()))?;
    if first_offset > raw.len() {
        return Err(Error::Unrecognized(
            "first SVD area begins beyond the end of the file".into(),
        ));
    }
    let table_end = 0x10 + svd.areas.len() * 16;
    if first_offset < table_end {
        return Err(Error::Unrecognized(
            "first SVD area overlaps the area table".into(),
        ));
    }
    let mut bytes = raw.bytes()[..first_offset].to_vec();

    for (position, (table_index, area)) in ordered.iter().enumerate() {
        svd.area_bytes(raw, area)?;
        let new_offset = u32::try_from(bytes.len())
            .map_err(|_| Error::Unrecognized("repackaged file is too large".into()))?;
        let body = replacements
            .iter()
            .find(|(tag, _)| *tag == &area.tag)
            .map(|(_, body)| body.as_slice())
            .unwrap_or(svd.area_bytes(raw, area)?);
        let new_size = u32::try_from(body.len())
            .map_err(|_| Error::Unrecognized("repackaged area is too large".into()))?;
        bytes.extend_from_slice(body);

        let original_end = area.range().end;
        let next_offset = ordered
            .get(position + 1)
            .map(|(_, next)| next.offset as usize)
            .unwrap_or(raw.len());
        if next_offset < original_end {
            return Err(Error::Unrecognized("SVD areas overlap".into()));
        }
        if next_offset > raw.len() {
            return Err(Error::Unrecognized(
                "SVD area gap extends beyond the end of the file".into(),
            ));
        }
        bytes.extend_from_slice(&raw.bytes()[original_end..next_offset]);

        let entry = 0x10 + *table_index * 16;
        bytes[entry + 8..entry + 12].copy_from_slice(&new_offset.to_le_bytes());
        bytes[entry + 12..entry + 16].copy_from_slice(&new_size.to_le_bytes());
    }
    Ok(Raw::from_bytes(bytes))
}

fn valid_zone_slots(record: &[u8]) -> impl Iterator<Item = usize> + '_ {
    (0..16).filter(|slot| {
        let marker = RawZone::TABLE_OFFSET + slot * RawZone::LEN + 0x3e;
        let tone = tone_id_offset(*slot);
        record.get(marker..marker + 2) == Some(&RawZone::MARKER) && tone + 2 <= record.len()
    })
}

fn user_tone_ids(record: &[u8]) -> impl Iterator<Item = u16> + '_ {
    valid_zone_slots(record).filter_map(|slot| {
        let at = tone_id_offset(slot);
        let id = u16::from_be_bytes([record[at], record[at + 1]]);
        (id & PRESET_FLAG == 0).then_some(id)
    })
}

fn tone_id_offset(slot: usize) -> usize {
    ZoneSettings::TABLE_OFFSET + slot * ZoneSettings::LEN + TONE_ID_OFFSET
}

fn read_u32(bytes: &[u8], at: usize, tag: &str) -> Result<u32> {
    let value = bytes
        .get(at..at + 4)
        .ok_or_else(|| Error::Unrecognized(format!("{tag} area truncated at offset {at}")))?;
    Ok(u32::from_le_bytes(value.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::read_scenes;
    use crate::model::ToneRef;

    const SCENE_SIZE: usize = 3572;
    const TONE_SIZE: usize = 32;

    fn scene(name: &str, tone_ids: &[u16]) -> Vec<u8> {
        let mut record = vec![0u8; SCENE_SIZE];
        record[..name.len()].copy_from_slice(name.as_bytes());
        for (slot, tone_id) in tone_ids.iter().enumerate() {
            let marker = RawZone::TABLE_OFFSET + slot * RawZone::LEN + 0x3e;
            record[marker..marker + 2].copy_from_slice(&RawZone::MARKER);
            let at = tone_id_offset(slot);
            record[at..at + 2].copy_from_slice(&tone_id.to_be_bytes());
        }
        record
    }

    fn tone(name: &str, fill: u8) -> Vec<u8> {
        let mut record = vec![fill; TONE_SIZE];
        record[..NAME_LEN].fill(b' ');
        record[..name.len()].copy_from_slice(name.as_bytes());
        record
    }

    fn record_area(records: &[Vec<u8>], record_size: usize) -> Vec<u8> {
        let mut area = vec![0x55; HEADER_LEN];
        area[COUNT_OFFSET..COUNT_OFFSET + 4].copy_from_slice(&(records.len() as u32).to_le_bytes());
        area[RECORD_SIZE_OFFSET..RECORD_SIZE_OFFSET + 4]
            .copy_from_slice(&(record_size as u32).to_le_bytes());
        for record in records {
            area.extend_from_slice(record);
        }
        area
    }

    fn build_svd(areas: &[(&[u8; 4], Vec<u8>)]) -> Raw {
        let header_size = 14 + areas.len() * 16;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(header_size as u16).to_le_bytes());
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
        Raw::from_bytes(bytes)
    }

    fn bank(scenes: Vec<Vec<u8>>, tones: Vec<Vec<u8>>, system: &[u8]) -> Raw {
        build_svd(&[
            (b"PRFa", record_area(&scenes, SCENE_SIZE)),
            (b"SYSa", system.to_vec()),
            (b"PATa", record_area(&tones, TONE_SIZE)),
        ])
    }

    fn area_bytes(raw: &Raw, tag: &[u8; 4]) -> Vec<u8> {
        let svd = Svd::parse(raw).unwrap();
        svd.area_bytes(raw, svd.area(tag).unwrap())
            .unwrap()
            .to_vec()
    }

    #[test]
    fn extract_keeps_selected_scenes_and_rebundles_their_tones() {
        let raw = bank(
            vec![scene("First", &[10]), scene("Second", &[20, 30])],
            vec![
                tone("Tone A", 0xa1),
                tone("Tone B", 0xb2),
                tone("Tone C", 0xc3),
            ],
            b"opaque system bytes",
        );

        let extracted = extract_scenes(&raw, &[2]).unwrap();
        let scenes = read_scenes(&extracted).unwrap();
        assert_eq!(scenes.len(), 1);
        assert_eq!(scenes[0].name, "Second");
        assert_eq!(
            scenes[0].zones[0].tone,
            ToneRef::User {
                id: 0,
                name: Some("Tone B".into())
            }
        );
        assert_eq!(
            scenes[0].zones[1].tone,
            ToneRef::User {
                id: 1,
                name: Some("Tone C".into())
            }
        );

        let pat = crate::container::PatArea::from_svd(&extracted, &Svd::parse(&extracted).unwrap())
            .unwrap();
        let names: Vec<_> = pat.tones().iter().map(|tone| tone.name.as_str()).collect();
        assert_eq!(names, ["Tone B", "Tone C"]);
        assert_eq!(area_bytes(&extracted, b"SYSa"), b"opaque system bytes");
    }

    #[test]
    fn merge_appends_scenes_and_deduplicates_identical_tone_records() {
        let shared = tone("Shared", 0x44);
        let target = bank(
            vec![scene("Target", &[100])],
            vec![shared.clone()],
            b"target system",
        );
        let source = bank(
            vec![scene("Source", &[10, 20])],
            vec![tone("Other", 0x22), shared],
            b"source system",
        );

        let merged = merge_scenes(&target, &source).unwrap();
        let scenes = read_scenes(&merged).unwrap();
        assert_eq!(
            scenes
                .iter()
                .map(|scene| scene.name.as_str())
                .collect::<Vec<_>>(),
            ["Target", "Source"]
        );
        assert_eq!(
            scenes[0].zones[0].tone,
            ToneRef::User {
                id: 0,
                name: Some("Shared".into())
            }
        );
        assert_eq!(
            scenes[1].zones[0].tone,
            ToneRef::User {
                id: 1,
                name: Some("Other".into())
            }
        );
        assert_eq!(
            scenes[1].zones[1].tone,
            ToneRef::User {
                id: 0,
                name: Some("Shared".into())
            }
        );

        let pat =
            crate::container::PatArea::from_svd(&merged, &Svd::parse(&merged).unwrap()).unwrap();
        assert_eq!(pat.tones().len(), 2);
        assert_eq!(area_bytes(&merged, b"SYSa"), b"target system");
    }

    #[test]
    fn canary_marks_scene_and_tone_names_without_changing_tone_payloads() {
        let raw = bank(
            vec![scene("First", &[10]), scene("Sledgehammer", &[20, 30])],
            vec![
                tone("Unused", 0xa1),
                tone("Sledgehammer Sha", 0xb2),
                tone("Sledge Brass 1", 0xc3),
            ],
            b"opaque system bytes",
        );
        let extracted = extract_scenes(&raw, &[2]).unwrap();
        let canary = canary_scene(&raw, 2).unwrap();

        let scenes = read_scenes(&canary).unwrap();
        assert_eq!(scenes[0].name, "CNY Sledgehammer");
        assert_eq!(scenes[0].zones[0].tone.name(), Some("CNY01 Sledgehamm"));
        assert_eq!(scenes[0].zones[1].tone.name(), Some("CNY02 Sledge Bra"));

        let extracted_pat = area_bytes(&extracted, b"PATa");
        let canary_pat = area_bytes(&canary, b"PATa");
        for index in 0..2 {
            let start = HEADER_LEN + index * TONE_SIZE;
            assert_eq!(
                &canary_pat[start + NAME_LEN..start + TONE_SIZE],
                &extracted_pat[start + NAME_LEN..start + TONE_SIZE]
            );
        }
        assert_eq!(area_bytes(&canary, b"SYSa"), b"opaque system bytes");
    }

    #[test]
    fn rejects_files_whose_user_tones_cannot_be_resolved() {
        let raw = bank(
            vec![scene("Broken", &[10, 20])],
            vec![tone("Only one", 0x11)],
            b"",
        );
        let error = extract_scenes(&raw, &[1]).unwrap_err().to_string();
        assert!(error.contains("not a self-contained scene export"));
    }
}
