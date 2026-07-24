//! Extract and merge scenes from FANTOM scene-export SVD files.
//!
//! Self-contained scene exports bundle the user sounds referenced by their scenes in engine areas
//! such as `PATa`, `RHYa`, and `ZEPa`. Repackaging rebuilds those bundles and rewrites their indexes
//! while leaving factory preset references untouched. Full backups are deliberately rejected.

use std::collections::{BTreeSet, HashMap};

use crate::container::{ascii_trim, Raw, RawZone, Svd, ZoneSettings};
use crate::{Error, Result};

const HEADER_LEN: usize = 0x10;
const COUNT_OFFSET: usize = 0;
const RECORD_SIZE_OFFSET: usize = 4;
const NAME_LEN: usize = 16;
const PC_VALUES_PER_BANK: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum FamilyKind {
    Pat,
    Rhy,
    Sna,
    Vtw,
    Zap,
    Zep,
}

impl FamilyKind {
    const ALL: [Self; 6] = [
        Self::Pat,
        Self::Rhy,
        Self::Vtw,
        Self::Sna,
        Self::Zap,
        Self::Zep,
    ];

    fn tags(self) -> &'static [&'static [u8; 4]] {
        match self {
            Self::Pat => &[b"PATa"],
            Self::Rhy => &[b"RHYa", b"INSa"],
            Self::Sna => &[b"SNAa"],
            Self::Vtw => &[b"VTWa"],
            Self::Zap => &[b"ZAPa"],
            Self::Zep => &[b"ZEPa"],
        }
    }

    fn reference(msb: u8, lsb: u8, pc: u8) -> Option<(Self, usize)> {
        let (kind, first_lsb) = match (msb, lsb) {
            (87, 0..=63) => (Self::Pat, 0),
            (86, 0) => (Self::Rhy, 0),
            (89, 0) => (Self::Sna, 0),
            (91, 0) => (Self::Vtw, 0),
            (105, 0) => (Self::Zap, 0),
            (105, 1) => (Self::Zep, 1),
            _ => return None,
        };
        (pc < 128).then_some((
            kind,
            (lsb - first_lsb) as usize * PC_VALUES_PER_BANK + pc as usize,
        ))
    }

    fn encode(self, index: usize) -> Result<(u8, u8)> {
        let first_lsb = if self == Self::Zep { 1 } else { 0 };
        let pages = if self == Self::Pat { 64 } else { 1 };
        if index >= pages * PC_VALUES_PER_BANK {
            return Err(Error::Unrecognized(format!(
                "too many {} records to encode",
                String::from_utf8_lossy(self.tags()[0])
            )));
        }
        Ok((
            first_lsb + (index / PC_VALUES_PER_BANK) as u8,
            (index % PC_VALUES_PER_BANK) as u8,
        ))
    }
}

struct RecordArea {
    header: [u8; HEADER_LEN],
    format: [u8; 4],
    record_size: usize,
    records: Vec<Vec<u8>>,
}

struct Family {
    areas: Vec<RecordArea>,
    entries: Vec<Vec<Vec<u8>>>,
}

struct OutputFamily {
    headers: Vec<[u8; HEADER_LEN]>,
    formats: Vec<[u8; 4]>,
    record_sizes: Vec<usize>,
    entries: Vec<Vec<Vec<u8>>>,
    indexes: HashMap<Vec<Vec<u8>>, usize>,
}

struct OpaqueFamily {
    area: RecordArea,
}

struct OutputOpaque {
    header: [u8; HEADER_LEN],
    format: [u8; 4],
    record_size: usize,
    entries: Vec<Vec<u8>>,
    indexes: HashMap<Vec<u8>, usize>,
}

impl OutputOpaque {
    fn from_family(family: &OpaqueFamily) -> Self {
        Self {
            header: family.area.header,
            format: family.area.format,
            record_size: family.area.record_size,
            entries: Vec::new(),
            indexes: HashMap::new(),
        }
    }

    fn add(&mut self, family: &OpaqueFamily, index: usize) -> Result<usize> {
        if self.format != family.area.format || self.record_size != family.area.record_size {
            return Err(Error::Unrecognized(
                "opaque dependency record formats differ".into(),
            ));
        }
        let record = family.area.records.get(index).ok_or_else(|| {
            Error::Unrecognized(format!("{} record {index} is out of range", "ACBa"))
        })?;
        if let Some(index) = self.indexes.get(record) {
            return Ok(*index);
        }
        let output_index = self.entries.len();
        let owned = record.clone();
        self.entries.push(owned.clone());
        self.indexes.insert(owned, output_index);
        Ok(output_index)
    }
}

impl OutputFamily {
    fn from_family(family: &Family) -> Self {
        Self {
            headers: family.areas.iter().map(|area| area.header).collect(),
            formats: family.areas.iter().map(|area| area.format).collect(),
            record_sizes: family.areas.iter().map(|area| area.record_size).collect(),
            entries: Vec::new(),
            indexes: HashMap::new(),
        }
    }

    fn add(&mut self, family: &Family, entry: &[Vec<u8>]) -> Result<usize> {
        let compatible = self
            .formats
            .iter()
            .eq(family.areas.iter().map(|area| &area.format))
            && self
                .record_sizes
                .iter()
                .eq(family.areas.iter().map(|area| &area.record_size));
        if !compatible {
            return Err(Error::Unrecognized(
                "dependency area formats or record sizes differ".into(),
            ));
        }
        if let Some(index) = self.indexes.get(entry) {
            return Ok(*index);
        }
        let index = self.entries.len();
        let owned = entry.to_vec();
        self.entries.push(owned.clone());
        self.indexes.insert(owned, index);
        Ok(index)
    }
}

/// Build a scene-export SVD containing only the requested 1-based scene numbers, in the order
/// given. All recognized bundled dependencies are copied and rebased for the new bank.
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

/// Append every scene from `source` to `target`, rebundling and de-duplicating recognized engine
/// dependencies. Both inputs must be self-contained scene exports with matching scene records.
pub fn merge_scenes(target: &Raw, source: &Raw) -> Result<Raw> {
    let target_bank = ExportBank::parse(target)?;
    let source_bank = ExportBank::parse(source)?;
    if target_bank.scene_record_size != source_bank.scene_record_size {
        return Err(Error::Unrecognized(format!(
            "PRFa record sizes differ (target {}, source {})",
            target_bank.scene_record_size, source_bank.scene_record_size
        )));
    }
    if target_bank.scene_format != source_bank.scene_format {
        return Err(Error::Unrecognized(
            "source and target use different PRFa format versions".into(),
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
    families: HashMap<FamilyKind, Family>,
    opaque: HashMap<[u8; 4], OpaqueFamily>,
    scene_header: [u8; HEADER_LEN],
    scene_format: [u8; 4],
    scene_record_size: usize,
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
        let (scene_header, scene_record_size, scenes) =
            parse_records(svd.area_bytes(raw, prfa)?, "PRFa", true)?;
        let references = dependency_references(&scenes);
        let mut families = HashMap::new();
        for kind in FamilyKind::ALL {
            let expected = references.get(&kind).cloned().unwrap_or_default();
            let Some(first) = svd.area(kind.tags()[0]) else {
                if expected.is_empty() {
                    continue;
                }
                return Err(Error::Unrecognized(format!(
                    "scene references missing {} area",
                    String::from_utf8_lossy(kind.tags()[0])
                )));
            };

            let mut areas = Vec::new();
            for tag in kind.tags() {
                let area = svd.area(tag).ok_or_else(|| {
                    Error::Unrecognized(format!(
                        "{} requires paired {} area",
                        String::from_utf8_lossy(kind.tags()[0]),
                        String::from_utf8_lossy(*tag)
                    ))
                })?;
                let (header, record_size, records) =
                    parse_records(svd.area_bytes(raw, area)?, &area.tag_str(), false)?;
                areas.push(RecordArea {
                    header,
                    format: area.format,
                    record_size,
                    records,
                });
            }
            let count = areas[0].records.len();
            if areas.iter().any(|area| area.records.len() != count) {
                return Err(Error::Unrecognized(format!(
                    "{} paired area counts differ",
                    first.tag_str()
                )));
            }
            let complete: BTreeSet<_> = (0..count).collect();
            if expected != complete {
                return Err(Error::Unrecognized(format!(
                    "not a self-contained scene export: {} references {:?}, area has 0..{}",
                    first.tag_str(),
                    expected,
                    count.saturating_sub(1)
                )));
            }
            let entries = (0..count)
                .map(|index| {
                    areas
                        .iter()
                        .map(|area| area.records[index].clone())
                        .collect()
                })
                .collect();
            families.insert(kind, Family { areas, entries });
        }

        let mut opaque = HashMap::new();
        if let Some(area) = svd.area(b"ACBa") {
            let (header, record_size, records) =
                parse_records(svd.area_bytes(raw, area)?, "ACBa", false)?;
            opaque.insert(
                *b"ACBa",
                OpaqueFamily {
                    area: RecordArea {
                        header,
                        format: area.format,
                        record_size,
                        records,
                    },
                },
            );
        }

        Ok(Self {
            scenes,
            families,
            opaque,
            scene_header,
            scene_format: prfa.format,
            scene_record_size,
        })
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
    let mut outputs: HashMap<FamilyKind, OutputFamily> = HashMap::new();
    let mut opaque_outputs: HashMap<[u8; 4], OutputOpaque> = HashMap::new();
    let mut scenes = Vec::with_capacity(records.len());

    for (bank, original) in records {
        let mut scene = original.clone();
        let slots: Vec<_> = valid_zone_slots(&scene).collect();
        for slot in slots {
            let at = tone_bank_offset(slot);
            if scene[at] == 107 && scene[at + 1] == 0 {
                let tag = *b"ACBa";
                let family = bank.opaque.get(&tag).ok_or_else(|| {
                    Error::Unrecognized("scene references missing ACBa dependency".into())
                })?;
                let output = opaque_outputs
                    .entry(tag)
                    .or_insert_with(|| OutputOpaque::from_family(family));
                let new_index = output.add(family, scene[at + 2] as usize)?;
                scene[at + 2] = u8::try_from(new_index)
                    .map_err(|_| Error::Unrecognized("too many ACBa records to encode".into()))?;
                continue;
            }
            let Some((kind, old_index)) =
                FamilyKind::reference(scene[at], scene[at + 1], scene[at + 2])
            else {
                continue;
            };
            let family = bank.families.get(&kind).ok_or_else(|| {
                Error::Unrecognized(format!(
                    "scene references missing {} dependency",
                    String::from_utf8_lossy(kind.tags()[0])
                ))
            })?;
            let entry = family.entries.get(old_index).ok_or_else(|| {
                Error::Unrecognized(format!(
                    "{} record {} is out of range",
                    String::from_utf8_lossy(kind.tags()[0]),
                    old_index
                ))
            })?;
            let output = outputs
                .entry(kind)
                .or_insert_with(|| OutputFamily::from_family(family));
            let new_index = output.add(family, entry)?;
            let (lsb, pc) = kind.encode(new_index)?;
            scene[at + 1] = lsb;
            scene[at + 2] = pc;
        }
        scenes.push(scene);
    }

    let prfa = build_area(
        header_bank.scene_header,
        header_bank.scene_record_size,
        &scenes,
    )?;
    let mut replacements = HashMap::new();
    replacements.insert(*b"PRFa", (header_bank.scene_format, prfa));
    for kind in FamilyKind::ALL {
        let Some(output) = outputs.remove(&kind) else {
            continue;
        };
        for (area_index, tag) in kind.tags().iter().enumerate() {
            let records: Vec<_> = output
                .entries
                .iter()
                .map(|entry| entry[area_index].clone())
                .collect();
            let body = build_area(
                output.headers[area_index],
                output.record_sizes[area_index],
                &records,
            )?;
            replacements.insert(**tag, (output.formats[area_index], body));
        }
    }
    for (tag, output) in opaque_outputs {
        let body = build_area(output.header, output.record_size, &output.entries)?;
        replacements.insert(tag, (output.format, body));
    }
    rebuild_container(base, replacements)
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
    let mut names = Vec::new();
    for kind in FamilyKind::ALL {
        for tag in kind.tags() {
            let Some(area_info) = svd.area(tag) else {
                continue;
            };
            let area = svd.area_bytes(raw, area_info)?;
            let count = read_u32(area, COUNT_OFFSET, &area_info.tag_str())? as usize;
            let record_size = read_u32(area, RECORD_SIZE_OFFSET, &area_info.tag_str())? as usize;
            if record_size < NAME_LEN {
                return Err(Error::Unrecognized(format!(
                    "{} record is shorter than its name field",
                    area_info.tag_str()
                )));
            }
            for index in 0..count {
                let start = HEADER_LEN + index * record_size;
                let name = area.get(start..start + NAME_LEN).ok_or_else(|| {
                    Error::Unrecognized(format!(
                        "{} record {} is truncated",
                        area_info.tag_str(),
                        index + 1
                    ))
                })?;
                names.push((
                    area_info.offset as usize + start,
                    format!("CNY{:02} {}", index + 1, ascii_trim(name)),
                ));
            }
        }
    }
    for (offset, name) in names {
        raw.patch_ascii(offset, NAME_LEN, &name);
    }
    Ok(())
}

fn rebuild_container(
    raw: &Raw,
    mut replacements: HashMap<[u8; 4], ([u8; 4], Vec<u8>)>,
) -> Result<Raw> {
    let svd = Svd::parse(raw)?;
    const ORDER: [&[u8; 4]; 13] = [
        b"PRFa", b"PATa", b"RHYa", b"INSa", b"VTWa", b"SNAa", b"ZAPa", b"ZEPa", b"ACBa", b"DCWa",
        b"MDLa", b"SYSa", b"DIFa",
    ];
    for area in &svd.areas {
        if !ORDER.contains(&&area.tag) {
            return Err(Error::Unrecognized(format!(
                "cannot preserve unknown area {} while rebuilding the area table",
                area.tag_str()
            )));
        }
    }

    let dependency_tags: BTreeSet<[u8; 4]> = FamilyKind::ALL
        .into_iter()
        .flat_map(|kind| kind.tags().iter().map(|tag| **tag))
        .collect();
    let mut areas = Vec::new();
    for tag in ORDER {
        if let Some((format, body)) = replacements.remove(tag) {
            areas.push((*tag, format, body));
        } else if !dependency_tags.contains(tag) && tag != b"PRFa" {
            if let Some(area) = svd.area(tag) {
                areas.push((area.tag, area.format, svd.area_bytes(raw, area)?.to_vec()));
            }
        }
    }

    let header_size = u16::try_from(14 + areas.len() * 16)
        .map_err(|_| Error::Unrecognized("too many SVD areas".into()))?;
    let first_offset = 0x10 + areas.len() * 16;
    let mut bytes = raw
        .bytes()
        .get(..0x10)
        .ok_or_else(|| Error::Unrecognized("SVD header is truncated".into()))?
        .to_vec();
    bytes[0..2].copy_from_slice(&header_size.to_le_bytes());
    let mut offset = first_offset;
    for (tag, format, body) in &areas {
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(format);
        bytes.extend_from_slice(&(offset as u32).to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        offset += body.len();
    }
    for (_, _, body) in areas {
        bytes.extend_from_slice(&body);
    }
    Ok(Raw::from_bytes(bytes))
}

fn valid_zone_slots(record: &[u8]) -> impl Iterator<Item = usize> + '_ {
    (0..16).filter(|slot| {
        let marker = RawZone::TABLE_OFFSET + slot * RawZone::LEN + 0x3e;
        let tone = tone_bank_offset(*slot);
        record.get(marker..marker + 2) == Some(&RawZone::MARKER) && tone + 3 <= record.len()
    })
}

fn dependency_references(scenes: &[Vec<u8>]) -> HashMap<FamilyKind, BTreeSet<usize>> {
    let mut references: HashMap<FamilyKind, BTreeSet<usize>> = HashMap::new();
    for record in scenes {
        for slot in valid_zone_slots(record) {
            let at = tone_bank_offset(slot);
            if let Some((kind, index)) =
                FamilyKind::reference(record[at], record[at + 1], record[at + 2])
            {
                references.entry(kind).or_default().insert(index);
            }
        }
    }
    references
}

fn tone_bank_offset(slot: usize) -> usize {
    ZoneSettings::TABLE_OFFSET + slot * ZoneSettings::LEN
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

    fn scene(name: &str, tone_ids: &[usize]) -> Vec<u8> {
        let banks: Vec<_> = tone_ids
            .iter()
            .map(|index| {
                (
                    87,
                    (index / PC_VALUES_PER_BANK) as u8,
                    (index % PC_VALUES_PER_BANK) as u8,
                )
            })
            .collect();
        scene_with_banks(name, &banks)
    }

    fn scene_with_banks(name: &str, banks: &[(u8, u8, u8)]) -> Vec<u8> {
        let mut record = vec![0u8; SCENE_SIZE];
        record[..name.len()].copy_from_slice(name.as_bytes());
        for (slot, &(msb, lsb, pc)) in banks.iter().enumerate() {
            let marker = RawZone::TABLE_OFFSET + slot * RawZone::LEN + 0x3e;
            record[marker..marker + 2].copy_from_slice(&RawZone::MARKER);
            let at = tone_bank_offset(slot);
            record[at..at + 3].copy_from_slice(&[msb, lsb, pc]);
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
            vec![scene("First", &[0]), scene("Second", &[1, 2])],
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
            ToneRef::new(87, 0, 0, Some("Tone B".into()))
        );
        assert_eq!(
            scenes[0].zones[1].tone,
            ToneRef::new(87, 0, 1, Some("Tone C".into()))
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
            vec![scene("Target", &[0])],
            vec![shared.clone()],
            b"target system",
        );
        let source = bank(
            vec![scene("Source", &[0, 1])],
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
            ToneRef::new(87, 0, 0, Some("Shared".into()))
        );
        assert_eq!(
            scenes[1].zones[0].tone,
            ToneRef::new(87, 0, 1, Some("Other".into()))
        );
        assert_eq!(
            scenes[1].zones[1].tone,
            ToneRef::new(87, 0, 0, Some("Shared".into()))
        );

        let pat =
            crate::container::PatArea::from_svd(&merged, &Svd::parse(&merged).unwrap()).unwrap();
        assert_eq!(pat.tones().len(), 2);
        assert_eq!(area_bytes(&merged, b"SYSa"), b"target system");
    }

    #[test]
    fn canary_marks_scene_and_tone_names_without_changing_tone_payloads() {
        let raw = bank(
            vec![scene("First", &[0]), scene("Sledgehammer", &[1, 2])],
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
            vec![scene("Broken", &[0, 1])],
            vec![tone("Only one", 0x11)],
            b"",
        );
        let error = extract_scenes(&raw, &[1]).unwrap_err().to_string();
        assert!(error.contains("not a self-contained scene export"));
    }

    #[test]
    fn merge_adds_source_only_engine_areas_and_rewrites_each_family() {
        let target = bank(
            vec![scene("Target", &[0])],
            vec![tone("Target Tone", 0x11)],
            b"target system",
        );
        let source_scene = scene_with_banks("Source", &[(87, 0, 0), (89, 0, 0), (105, 1, 0)]);
        let source = build_svd(&[
            (b"PRFa", record_area(&[source_scene], SCENE_SIZE)),
            (
                b"PATa",
                record_area(&[tone("Source Tone", 0x22)], TONE_SIZE),
            ),
            (b"SNAa", record_area(&[tone("Source SNA", 0x33)], TONE_SIZE)),
            (b"ZEPa", record_area(&[tone("Source ZEP", 0x44)], TONE_SIZE)),
            (b"SYSa", b"source system".to_vec()),
        ]);

        let merged = merge_scenes(&target, &source).unwrap();
        let svd = Svd::parse(&merged).unwrap();
        assert!(svd.area(b"SNAa").is_some());
        assert!(svd.area(b"ZEPa").is_some());
        assert_eq!(area_bytes(&merged, b"SYSa"), b"target system");

        let scenes = read_scenes(&merged).unwrap();
        assert_eq!(scenes[1].zones[0].tone.name(), Some("Source Tone"));
        let prfa = area_bytes(&merged, b"PRFa");
        let source_record = &prfa[HEADER_LEN + SCENE_SIZE..];
        let banks: Vec<_> = (0..3)
            .map(|slot| {
                let at = tone_bank_offset(slot);
                &source_record[at..at + 3]
            })
            .collect();
        assert_eq!(banks, [&[87, 0, 1][..], &[89, 0, 0], &[105, 1, 0]]);
    }

    #[test]
    fn merge_preserves_identical_opaque_dependency_areas() {
        let target = build_svd(&[
            (
                b"PRFa",
                record_area(&[scene_with_banks("Target", &[(107, 0, 0)])], SCENE_SIZE),
            ),
            (b"ACBa", record_area(&[tone("A", 0x11)], TONE_SIZE)),
        ]);
        let source = build_svd(&[
            (
                b"PRFa",
                record_area(&[scene_with_banks("Source", &[(107, 0, 0)])], SCENE_SIZE),
            ),
            (b"ACBa", record_area(&[tone("A", 0x11)], TONE_SIZE)),
        ]);

        let merged = merge_scenes(&target, &source).unwrap();
        assert_eq!(area_bytes(&merged, b"ACBa")[..4], [1, 0, 0, 0]);
        assert_eq!(read_scenes(&merged).unwrap().len(), 2);
    }

    #[test]
    fn merge_rebases_different_opaque_dependency_records() {
        let target = build_svd(&[
            (
                b"PRFa",
                record_area(&[scene_with_banks("Target", &[(107, 0, 0)])], SCENE_SIZE),
            ),
            (b"ACBa", record_area(&[tone("A", 0x11)], TONE_SIZE)),
        ]);
        let source = build_svd(&[
            (
                b"PRFa",
                record_area(&[scene_with_banks("Source", &[(107, 0, 0)])], SCENE_SIZE),
            ),
            (b"ACBa", record_area(&[tone("B", 0x44)], TONE_SIZE)),
        ]);

        let merged = merge_scenes(&target, &source).unwrap();
        let area = area_bytes(&merged, b"ACBa");
        assert_eq!(read_u32(&area, 0, "ACBa").unwrap(), 2);
        let scenes = read_scenes(&merged).unwrap();
        assert_eq!(scenes[0].zones[0].tone.address.pc, 0);
        assert_eq!(scenes[1].zones[0].tone.address.pc, 1);
    }

    #[test]
    fn merge_copies_source_only_opaque_dependency_areas() {
        let target = build_svd(&[
            (
                b"PRFa",
                record_area(&[scene_with_banks("Target", &[(87, 0, 0)])], SCENE_SIZE),
            ),
            (b"PATa", record_area(&[tone("Target", 0x11)], TONE_SIZE)),
        ]);
        let source = build_svd(&[
            (
                b"PRFa",
                record_area(&[scene_with_banks("Source", &[(107, 0, 0)])], SCENE_SIZE),
            ),
            (b"ACBa", record_area(&[tone("A", 0x44)], TONE_SIZE)),
        ]);

        let merged = merge_scenes(&target, &source).unwrap();
        assert_eq!(
            read_u32(&area_bytes(&merged, b"ACBa"), 0, "ACBa").unwrap(),
            1
        );
    }

    #[test]
    fn rhythm_dependencies_keep_rhya_and_insa_records_paired() {
        let raw = build_svd(&[
            (
                b"PRFa",
                record_area(&[scene_with_banks("Drums", &[(86, 0, 0)])], SCENE_SIZE),
            ),
            (b"RHYa", record_area(&[tone("User Kit", 0x55)], TONE_SIZE)),
            (
                b"INSa",
                record_area(&[tone("Kit Instruments", 0x66)], TONE_SIZE),
            ),
            (b"SYSa", b"system".to_vec()),
        ]);

        let extracted = extract_scenes(&raw, &[1]).unwrap();
        assert_eq!(
            &area_bytes(&extracted, b"RHYa")[HEADER_LEN..HEADER_LEN + NAME_LEN],
            &tone("User Kit", 0x55)[..NAME_LEN]
        );
        assert_eq!(
            &area_bytes(&extracted, b"INSa")[HEADER_LEN..HEADER_LEN + NAME_LEN],
            &tone("Kit Instruments", 0x66)[..NAME_LEN]
        );
    }

    #[test]
    fn zen_core_indices_roll_to_the_next_lsb_after_128_records() {
        let scenes: Vec<_> = (0..130)
            .collect::<Vec<_>>()
            .chunks(16)
            .enumerate()
            .map(|(number, ids)| scene(&format!("Scene {number}"), ids))
            .collect();
        let tones: Vec<_> = (0..130)
            .map(|index| tone(&format!("Tone {index}"), index as u8))
            .collect();
        let raw = bank(scenes, tones, b"system");
        let scene_numbers: Vec<_> = (1..=9).collect();

        let extracted = extract_scenes(&raw, &scene_numbers).unwrap();
        let prfa = area_bytes(&extracted, b"PRFa");
        let ninth_scene = &prfa[HEADER_LEN + 8 * SCENE_SIZE..];
        let at = tone_bank_offset(0);
        assert_eq!(&ninth_scene[at..at + 3], &[87, 1, 0]);
        assert_eq!(
            read_scenes(&extracted).unwrap()[8].zones[0].tone.name(),
            Some("Tone 128")
        );
    }
}
