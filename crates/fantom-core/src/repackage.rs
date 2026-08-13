//! Extract and merge scenes from FANTOM SVD files.
//!
//! A scene's user sounds live in engine areas alongside it — `PATa` for ZEN-Core tones, `RHYa`
//! plus `INSa` for drum kits, `ACBa`/`DCWa`/`MDLa` for the modelled engines, and so on. Repackaging
//! copies the records the chosen scenes actually reference, de-duplicates them, renumbers them
//! densely, and rewrites each zone's address to match. Factory ROM references are left untouched:
//! they name sounds inside the instrument, so they travel by themselves.
//!
//! Both input shapes work. A **scene export** bundles just its own tones; a **full backup** carries
//! the entire USER bank, of which the selected scenes reference a handful. Because both index their
//! areas the same way (see [`crate::address`]), the only difference is how much gets left behind.
//! The output is always a self-contained scene-export bank.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::address;
use crate::container::{Kind, Raw, RawZone, RecordTable, Svd, ZoneSettings, PANEL_SLOTS};
use crate::{Error, Result};

const HEADER_LEN: usize = 0x10;
const COUNT_OFFSET: usize = 0;
const RECORD_SIZE_OFFSET: usize = 4;
const NAME_LEN: usize = address::NAME_LEN;

/// Areas carried into the output unchanged.
const PRESERVED: [&[u8; 4]; 2] = [b"SYSa", b"DIFa"];

/// Areas we recognise but deliberately leave behind.
///
/// These hold the user sample bank: `SMPa` the slot directory, `MLSa` multisamples, `USDa` the
/// waveform payload (23 MB in a full backup). Dropping them is not a limitation but a match for
/// what the instrument does — its own scene exports carry no sample area either, and nothing in the
/// scene-import path reads one. A scene bank's sample references are absolute panel slots precisely
/// *because* there is no table here for an index to point into.
///
/// The audio still travels, in the container built for it: [`crate::samplebank`] writes a companion
/// `.svz`, and [`rebase_sample_slots`] repoints this bank at wherever that companion is imported.
const DROPPED: [&[u8; 4]; 3] = [b"SMPa", b"MLSa", b"USDa"];

struct RecordArea {
    header: [u8; HEADER_LEN],
    format: [u8; 4],
    record_size: usize,
    records: Vec<Vec<u8>>,
}

/// One engine's user bank as loaded from a file.
///
/// `entries[i]` is record `i` across every area indexed in lockstep — a drum kit is `RHYa[i]`
/// *and* `INSa[i]`, so they can only be copied, de-duplicated, and renumbered as a unit.
struct Bundle {
    areas: Vec<RecordArea>,
    entries: Vec<Vec<Vec<u8>>>,
}

/// The same bank being rebuilt for the output, de-duplicated as records are added.
struct OutputBundle {
    headers: Vec<[u8; HEADER_LEN]>,
    formats: Vec<[u8; 4]>,
    record_sizes: Vec<usize>,
    entries: Vec<Vec<Vec<u8>>>,
    indexes: HashMap<Vec<Vec<u8>>, usize>,
}

impl OutputBundle {
    fn new(bundle: &Bundle) -> Self {
        Self {
            headers: bundle.areas.iter().map(|area| area.header).collect(),
            formats: bundle.areas.iter().map(|area| area.format).collect(),
            record_sizes: bundle.areas.iter().map(|area| area.record_size).collect(),
            entries: Vec::new(),
            indexes: HashMap::new(),
        }
    }

    /// Add `entry`, returning its index in the output — the existing one if an identical record is
    /// already there. Byte equality is the only safe identity test for records whose internals we
    /// do not fully decode.
    fn add(&mut self, bundle: &Bundle, entry: &[Vec<u8>]) -> Result<usize> {
        let compatible = self
            .formats
            .iter()
            .eq(bundle.areas.iter().map(|area| &area.format))
            && self
                .record_sizes
                .iter()
                .eq(bundle.areas.iter().map(|area| &area.record_size));
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
    bundles: HashMap<[u8; 4], Bundle>,
    scene_header: [u8; HEADER_LEN],
    scene_format: [u8; 4],
    scene_record_size: usize,
}

impl ExportBank {
    fn parse(raw: &Raw) -> Result<Self> {
        let svd = Svd::parse(raw)?;
        let prfa = svd
            .area(b"PRFa")
            .ok_or_else(|| Error::Unrecognized("no PRFa (performance) area in file".into()))?;
        let (scene_header, scene_record_size, scenes) =
            parse_records(svd.area_bytes(raw, prfa)?, "PRFa", true)?;
        let references = dependency_references(&scenes);

        let mut bundles = HashMap::new();
        for spec in &address::AREAS {
            let expected = references.get(&spec.tag).cloned().unwrap_or_default();
            if svd.area(&spec.tag).is_none() {
                if expected.is_empty() {
                    continue;
                }
                return Err(Error::Unrecognized(format!(
                    "scene references missing {} area",
                    spec.tag_str()
                )));
            }

            let mut areas = Vec::new();
            for tag in spec.paired {
                let area = svd.area(tag).ok_or_else(|| {
                    Error::Unrecognized(format!(
                        "{} requires paired {} area",
                        spec.tag_str(),
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
                    spec.tag_str()
                )));
            }
            // Unreferenced records are normal — a full backup is mostly unreferenced — but a
            // reference past the end of the area means we have misread the file.
            if let Some(&highest) = expected.iter().next_back() {
                if highest >= count {
                    return Err(Error::Unrecognized(format!(
                        "scene references {} record {}, but the area holds 0..{}",
                        spec.tag_str(),
                        highest,
                        count.saturating_sub(1)
                    )));
                }
            }
            let entries = (0..count)
                .map(|index| {
                    areas
                        .iter()
                        .map(|area| area.records[index].clone())
                        .collect()
                })
                .collect();
            bundles.insert(spec.tag, Bundle { areas, entries });
        }

        Ok(Self {
            scenes,
            bundles,
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
    let mut outputs: HashMap<[u8; 4], OutputBundle> = HashMap::new();
    let mut scenes = Vec::with_capacity(records.len());

    for (bank, original) in records {
        let mut scene = original.clone();
        let slots: Vec<_> = valid_zone_slots(&scene).collect();
        for slot in slots {
            let at = tone_bank_offset(slot);
            // Factory ROM and engines we cannot place keep their address untouched.
            let Some((spec, old_index)) = address::resolve(scene[at], scene[at + 1], scene[at + 2])
            else {
                continue;
            };
            let bundle = bank.bundles.get(&spec.tag).ok_or_else(|| {
                Error::Unrecognized(format!(
                    "scene references missing {} dependency",
                    spec.tag_str()
                ))
            })?;
            let entry = bundle.entries.get(old_index).ok_or_else(|| {
                Error::Unrecognized(format!(
                    "{} record {old_index} is out of range",
                    spec.tag_str()
                ))
            })?;
            let output = outputs
                .entry(spec.tag)
                .or_insert_with(|| OutputBundle::new(bundle));
            let new_index = output.add(bundle, entry)?;
            let (lsb, pc) = spec.encode(new_index)?;
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
    for spec in &address::AREAS {
        let Some(output) = outputs.remove(&spec.tag) else {
            continue;
        };
        for (area_index, tag) in spec.paired.iter().enumerate() {
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

/// Prefix every bundled record's name with `CNY01`, `CNY02`, … leaving the sound itself untouched.
///
/// Seeing those names on the instrument proves it read the rebuilt bundle rather than resolving a
/// tone it already had, which is the only way to tell a correct repackage from a lucky one.
fn mark_bundled_tone_names(raw: &mut Raw) -> Result<()> {
    let svd = Svd::parse(raw)?;
    let mut names = Vec::new();
    for spec in &address::AREAS {
        for tag in spec.paired {
            let Some(area_info) = svd.area(tag) else {
                continue;
            };
            let area = svd.area_bytes(raw, area_info)?;
            let count = read_u32(area, COUNT_OFFSET, &area_info.tag_str())? as usize;
            let record_size = read_u32(area, RECORD_SIZE_OFFSET, &area_info.tag_str())? as usize;
            if record_size < spec.name_offset + NAME_LEN {
                return Err(Error::Unrecognized(format!(
                    "{} record is shorter than its name field",
                    area_info.tag_str()
                )));
            }
            for index in 0..count {
                let start = HEADER_LEN + index * record_size + spec.name_offset;
                let field = area.get(start..start + NAME_LEN).ok_or_else(|| {
                    Error::Unrecognized(format!(
                        "{} record {} is truncated",
                        area_info.tag_str(),
                        index + 1
                    ))
                })?;
                let marked = format!("CNY{:02} {}", index + 1, spec.decode_name(field));
                names.push((area_info.offset as usize + start, spec.encode_name(&marked)));
            }
        }
    }
    for (offset, name) in names {
        raw.patch_bytes(offset, &name);
    }
    Ok(())
}

fn rebuild_container(
    raw: &Raw,
    mut replacements: HashMap<[u8; 4], ([u8; 4], Vec<u8>)>,
) -> Result<Raw> {
    let svd = Svd::parse(raw)?;
    // Output order matches what the instrument writes: scenes, then each engine's user bank in
    // area order, then the system and checksum areas.
    let order: Vec<[u8; 4]> = std::iter::once(*b"PRFa")
        .chain(address::dependency_tags())
        .chain(PRESERVED.into_iter().copied())
        .collect();
    for area in &svd.areas {
        if !order.contains(&area.tag) && !DROPPED.contains(&&area.tag) {
            return Err(Error::Unrecognized(format!(
                "cannot preserve unknown area {} while rebuilding the area table",
                area.tag_str()
            )));
        }
    }

    let dependency_tags: BTreeSet<[u8; 4]> = address::dependency_tags().collect();
    let mut areas = Vec::new();
    for tag in order {
        if let Some((format, body)) = replacements.remove(&tag) {
            areas.push((tag, format, body));
        } else if !dependency_tags.contains(&tag) && &tag != b"PRFa" {
            // A dependency area with nothing referencing it is left out; everything else that
            // survived the check above is copied verbatim.
            if let Some(area) = svd.area(&tag) {
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

    // Check the result before handing it back: a rebuilt bank whose areas disagree with their own
    // geometry is a bug here, and it should not reach the instrument.
    let raw = Raw::from_bytes(bytes);
    let report = crate::verify::check(&raw)?;
    if !report.is_ok() {
        return Err(Error::Unrecognized(format!(
            "repackaging produced an inconsistent file: {}",
            report
                .problems
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }
    Ok(raw)
}

fn valid_zone_slots(record: &[u8]) -> impl Iterator<Item = usize> + '_ {
    (0..16).filter(|slot| {
        let marker = RawZone::TABLE_OFFSET + slot * RawZone::LEN + 0x3e;
        let tone = tone_bank_offset(*slot);
        record.get(marker..marker + 2) == Some(&RawZone::MARKER) && tone + 3 <= record.len()
    })
}

/// Which record of which engine area every scene zone points at.
fn dependency_references(scenes: &[Vec<u8>]) -> HashMap<[u8; 4], BTreeSet<usize>> {
    let mut references: HashMap<[u8; 4], BTreeSet<usize>> = HashMap::new();
    for record in scenes {
        for slot in valid_zone_slots(record) {
            let at = tone_bank_offset(slot);
            if let Some((spec, index)) = address::resolve(record[at], record[at + 1], record[at + 2])
            {
                references.entry(spec.tag).or_default().insert(index);
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

/// Areas of a scene bank whose records name user samples: ZEN-Core tones, and the instrument sets
/// of the drum kits bundled beside them.
const SAMPLE_REF_AREAS: &[&[u8; 4]] = &[b"PATa", b"INSa"];

/// The user-sample slots this bank's bundled sounds play: 1-based, sorted, deduplicated.
///
/// These are **panel slot numbers**, and they are what makes a sampled scene bank incomplete
/// elsewhere: the destination has to hold this audio at these numbers.
pub fn referenced_sample_slots(raw: &Raw) -> Result<Vec<u16>> {
    let svd = Svd::parse(raw)?;
    let mut slots = BTreeSet::new();
    for tag in SAMPLE_REF_AREAS {
        let Some(table) = RecordTable::from_svd(raw, &svd, tag)? else {
            continue;
        };
        for record in table.records() {
            slots.extend(crate::container::sample_slots_of(tag, record));
        }
    }
    Ok(slots.into_iter().collect())
}

/// Repoint every bundled tone's user-sample references through `remap` (old slot -> new slot).
///
/// The numbers a scene bank stores are absolute panel slots, so a sampled bank only plays correctly
/// where the destination happens to hold that audio at those exact slots — which is why commercial
/// packs tell you to wipe slots 1-50 and load theirs there. Rewriting the references instead lets
/// the audio land anywhere free: pair this with a [`crate::samplebank`] companion file and the
/// numbers agree by construction.
///
/// Slots with no entry in `remap` are left alone. Only `PATa` is rewritten, because only a
/// ZEN-Core tone's sample references are decoded — see [`crate::address::AreaSpec`]; a caller
/// holding a bank with drum kits should say so.
pub fn rebase_sample_slots(raw: &Raw, remap: &BTreeMap<u16, u16>) -> Result<Raw> {
    let svd = Svd::parse(raw)?;
    if svd.kind == Kind::Svz {
        return Err(Error::Unrecognized(
            "an SVZ addresses samples by position within its own USPa, so there is nothing to \
             rebase — extract already carries and renumbers them"
                .into(),
        ));
    }

    let mut bytes = raw.bytes().to_vec();
    for tag in SAMPLE_REF_AREAS {
        let Some(table) = RecordTable::from_svd(raw, &svd, tag)? else {
            continue;
        };
        if table.info_stride() != 0 {
            // No SVD5 area stores per-record checksums, so this cannot happen — but rewriting
            // records without refreshing them would hand the instrument a file failing its own check.
            return Err(Error::Unrecognized(format!(
                "{} carries per-record checksums; rebasing would invalidate them",
                String::from_utf8_lossy(tag.as_slice())
            )));
        }
        let record_size = table.record_size;
        let offsets: Vec<usize> = (0..table.len()).map(|i| table.record_offset(i)).collect();

        for at in offsets {
            let Some(record) = bytes.get_mut(at..at + record_size) else {
                break;
            };
            crate::container::remap_sample_slots_of(tag, record, remap);
        }
    }

    let out = Raw::from_bytes(bytes);
    let report = crate::verify::check(&out)?;
    if !report.is_ok() {
        return Err(Error::Unrecognized(format!(
            "rebasing produced an inconsistent file: {}",
            report
                .problems
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }
    Ok(out)
}

/// Map each slot in `slots` onto a contiguous run starting at `base`, in slot order.
///
/// This is the mapping a companion sample file implies: it numbers its samples densely, the
/// instrument imports them as one run from whichever slot the user picks, so the *n*th slot this
/// bank referenced becomes `base + n`.
///
/// The run has to land inside the panel's `1..=8000` slots. A base of 0 is not merely off by one:
/// a wave number of zero means *no wave*, so it would silence the partials it was meant to fix.
pub fn contiguous_remap(slots: &[u16], base: u16) -> Result<BTreeMap<u16, u16>> {
    if base == 0 {
        return Err(Error::Unrecognized(
            "sample slots are numbered from 1; slot 0 means \"no wave\"".into(),
        ));
    }
    let last = base as usize + slots.len().saturating_sub(1);
    if last > PANEL_SLOTS as usize {
        return Err(Error::Unrecognized(format!(
            "{} sample{} starting at slot {base} would run to {last}, past the panel's {PANEL_SLOTS}",
            slots.len(),
            if slots.len() == 1 { "" } else { "s" },
        )));
    }
    Ok(slots
        .iter()
        .enumerate()
        .map(|(index, &slot)| (slot, base + index as u16))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::read_scenes;
    use crate::model::ToneRef;

    const SCENE_SIZE: usize = 3572;
    const TONE_SIZE: usize = 32;
    const PC_VALUES_PER_BANK: usize = address::PC_PER_PAGE;

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

    /// Wrap opaque bytes in a valid one-record area. `SYSa` in a real file is a proper record
    /// table (count 1, 904-byte record), so tests must build one too — a bare blob is a shape the
    /// instrument never writes, and the output self-check rightly rejects it.
    fn system_area(bytes: &[u8]) -> Vec<u8> {
        record_area(&[bytes.to_vec()], bytes.len())
    }

    fn bank(scenes: Vec<Vec<u8>>, tones: Vec<Vec<u8>>, system: &[u8]) -> Raw {
        build_svd(&[
            (b"PRFa", record_area(&scenes, SCENE_SIZE)),
            (b"SYSa", system_area(system)),
            (b"PATa", record_area(&tones, TONE_SIZE)),
        ])
    }

    /// A tone record long enough to hold four real partials, unlike the 32-byte stub above.
    const SAMPLED_TONE_SIZE: usize = 0xdc + 4 * 124 + 8;

    /// A tone whose `partial` plays user sample `slot`, at the confirmed group/number offsets.
    fn sampled_tone(name: &str, partials: &[(usize, u16)]) -> Vec<u8> {
        let mut record = vec![0u8; SAMPLED_TONE_SIZE];
        record[..NAME_LEN].fill(b' ');
        record[..name.len()].copy_from_slice(name.as_bytes());
        for &(partial, slot) in partials {
            let base = 0xdc + partial * 124;
            record[base + 3] = 2;
            record[base + 6..base + 8].copy_from_slice(&slot.to_le_bytes());
        }
        record
    }

    fn sampled_bank(tones: &[Vec<u8>]) -> Raw {
        build_svd(&[
            (b"PRFa", record_area(&[scene("One", &[0])], SCENE_SIZE)),
            (b"SYSa", system_area(&[0u8; 8])),
            (b"PATa", record_area(tones, SAMPLED_TONE_SIZE)),
        ])
    }

    #[test]
    fn reports_the_sample_slots_the_bundled_tones_play() {
        let raw = sampled_bank(&[
            sampled_tone("Plain", &[]),
            sampled_tone("Two", &[(0, 29), (1, 7)]),
            sampled_tone("Dup", &[(0, 7)]),
        ]);
        assert_eq!(referenced_sample_slots(&raw).unwrap(), [7, 29]);
    }

    /// The point of rebasing: a bank whose tones name panel slots 7 and 29 can be repointed at a
    /// contiguous run the destination actually has free, so the audio need not land on top of
    /// whatever the user already keeps in those slots.
    #[test]
    fn rebasing_repoints_every_reference_onto_a_contiguous_run() {
        let raw = sampled_bank(&[
            sampled_tone("Plain", &[]),
            sampled_tone("Two", &[(0, 29), (1, 7)]),
        ]);
        let slots = referenced_sample_slots(&raw).unwrap();
        let remap = contiguous_remap(&slots, 101).unwrap();

        let rebased = rebase_sample_slots(&raw, &remap).unwrap();
        assert_eq!(referenced_sample_slots(&rebased).unwrap(), [101, 102]);

        // Slot 7 was the first referenced, so it becomes the first of the run.
        let svd = Svd::parse(&rebased).unwrap();
        let table = RecordTable::from_svd(&rebased, &svd, b"PATa").unwrap().unwrap();
        assert_eq!(crate::container::sample_slots(table.record(1).unwrap()), [102, 101]);
    }

    /// The run has to land on real panel slots. Slot 0 is the trap: a wave number of zero means
    /// "no wave", so an off-by-one base would silence the very partials it was meant to repoint.
    #[test]
    fn a_run_that_would_leave_the_panel_is_refused() {
        let zero = contiguous_remap(&[7, 29], 0).unwrap_err().to_string();
        assert!(zero.contains("numbered from 1"), "{zero}");

        let past = contiguous_remap(&[7, 29], PANEL_SLOTS).unwrap_err().to_string();
        assert!(past.contains("past the panel's"), "{past}");

        // The last slot that still fits is fine, and so is a base with no samples to place.
        assert!(contiguous_remap(&[7, 29], PANEL_SLOTS - 1).is_ok());
        assert!(contiguous_remap(&[], PANEL_SLOTS).is_ok());
    }

    #[test]
    fn rebasing_leaves_a_bank_without_samples_untouched() {
        let raw = sampled_bank(&[sampled_tone("Plain", &[])]);
        let rebased = rebase_sample_slots(&raw, &contiguous_remap(&[], 101).unwrap()).unwrap();
        assert_eq!(rebased.bytes(), raw.bytes());
    }

    /// An SVZ numbers its samples by position in its own USPa, so "rebasing" one is meaningless.
    #[test]
    fn rebasing_refuses_a_tone_bank() {
        let raw = Raw::from_bytes({
            let mut b = b"SVZa".to_vec();
            b.push(0);
            b.push(3);
            b.extend_from_slice(b"KY019$");
            b.extend_from_slice(&[0u8; 4]);
            b
        });
        let error = rebase_sample_slots(&raw, &BTreeMap::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("position within its own USPa"), "{error}");
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
        assert_eq!(area_bytes(&extracted, b"SYSa"), system_area(b"opaque system bytes"));
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
        assert_eq!(area_bytes(&merged, b"SYSa"), system_area(b"target system"));
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
        assert_eq!(area_bytes(&canary, b"SYSa"), system_area(b"opaque system bytes"));
    }

    #[test]
    fn rejects_a_reference_past_the_end_of_its_area() {
        let raw = bank(
            vec![scene("Broken", &[0, 1])],
            vec![tone("Only one", 0x11)],
            b"",
        );
        let error = extract_scenes(&raw, &[1]).unwrap_err().to_string();
        assert!(
            error.contains("PATa record 1") && error.contains("0..0"),
            "unexpected error: {error}"
        );
    }

    /// A full backup carries the whole USER bank, so almost every record is unreferenced. That is
    /// the normal case for a backup source, not a malformed file — only the referenced records are
    /// carried into the output.
    #[test]
    fn extracts_from_a_bank_whose_tones_are_mostly_unreferenced() {
        let tones: Vec<_> = (0..200)
            .map(|index| tone(&format!("Tone {index}"), index as u8))
            .collect();
        let raw = bank(
            vec![scene("First", &[0]), scene("Second", &[7, 150])],
            tones,
            b"system",
        );

        let extracted = extract_scenes(&raw, &[2]).unwrap();
        let scenes = read_scenes(&extracted).unwrap();
        assert_eq!(scenes.len(), 1);
        assert_eq!(scenes[0].zones[0].tone.name(), Some("Tone 7"));
        assert_eq!(scenes[0].zones[1].tone.name(), Some("Tone 150"));

        // Only the two referenced tones travel, renumbered densely from zero.
        let pat = crate::container::PatArea::from_svd(&extracted, &Svd::parse(&extracted).unwrap())
            .unwrap();
        assert_eq!(pat.tones().len(), 2);
        assert_eq!(scenes[0].zones[0].tone.address.pc, 0);
        assert_eq!(scenes[0].zones[1].tone.address.pc, 1);
    }

    /// The modelled engines used to be renumbered by rewriting PC alone, which capped them at 128
    /// records and made a backup's 1024-record `MDLa` unrepresentable.
    #[test]
    fn opaque_engine_references_are_rebased_across_lsb_pages() {
        let tones: Vec<_> = (0..200)
            .map(|index| tone(&format!("Model {index}"), index as u8))
            .collect();
        let raw = build_svd(&[
            (
                b"PRFa",
                record_area(
                    &[scene_with_banks("Modelled", &[(97, 1, 22), (97, 0, 5)])],
                    SCENE_SIZE,
                ),
            ),
            (b"MDLa", record_area(&tones, TONE_SIZE)),
        ]);

        // lsb 1 / pc 22 addresses MDLa[150]; lsb 0 / pc 5 addresses MDLa[5].
        let extracted = extract_scenes(&raw, &[1]).unwrap();
        assert_eq!(read_u32(&area_bytes(&extracted, b"MDLa"), 0, "MDLa").unwrap(), 2);
        let prfa = area_bytes(&extracted, b"PRFa");
        let banks: Vec<_> = (0..2)
            .map(|slot| {
                let at = HEADER_LEN + tone_bank_offset(slot);
                prfa[at..at + 3].to_vec()
            })
            .collect();
        assert_eq!(banks, [vec![97, 0, 0], vec![97, 0, 1]]);
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
            (b"SYSa", system_area(b"source system")),
        ]);

        let merged = merge_scenes(&target, &source).unwrap();
        let svd = Svd::parse(&merged).unwrap();
        assert!(svd.area(b"SNAa").is_some());
        assert!(svd.area(b"ZEPa").is_some());
        assert_eq!(area_bytes(&merged, b"SYSa"), system_area(b"target system"));

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
            (b"SYSa", system_area(b"system")),
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
