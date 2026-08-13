//! Repackaging SVZ tone banks: pick tones out of one, or join two together.
//!
//! An SVZ is the tone-level counterpart to a scene export. It has no `PRFa` — just an engine area
//! (`PATa` ZEN-Core tones, or `RHYa` drum kits with their paired `INSa` instrument sets), a `DIFa`
//! checksum, and, when a tone plays user samples, the sample slots in `USPa` and their audio in
//! `USDa`.
//!
//! That last part is the reason this module exists rather than deferring to [`crate::repackage`]:
//! **an SVZ carries sample audio and a scene export does not.** A sampled tone in a scene bank is
//! only a slot reference, so it falls silent on an instrument that lacks the sample; the same tone
//! in an SVZ brings its waveform along. When tones are selected here, the samples they reference
//! are carried and renumbered with them.
//!
//! That selection needs a decoded tone→sample link, and only `PATa` has one
//! ([`AreaSpec::sample_refs_decoded`]). A drum kit stores its waves in the paired `INSa`, whose
//! wave blocks are located but whose group field has never been observed set — no fixture has a
//! kit that plays a user sample, so the value meaning "user sample" is unconfirmed. For those
//! engines this module carries **every** sample the source holds, at its original slot number:
//! selecting none would silently strip a sampled kit's audio, and renumbering would break
//! references there is no way to find and rewrite.

use std::collections::BTreeMap;

use crate::address::{self, AreaSpec};
use crate::checksum::crc32;
use crate::container::{Area, Kind, Raw, RecordTable, Svd, PREAMBLE_LEN};
use crate::{Error, Result};

const HEADER_LEN: usize = RecordTable::HEADER_LEN;
/// `{u32 slot, u32 offset, u32 size, u32 word}` per waveform section in `USDa`.
const DIRECTORY_ENTRY: usize = 16;
/// Byte at `0x04` of the preamble: how many areas the file has.
const AREA_COUNT_BYTE: usize = 0x04;
/// A four-byte per-record info word is that record's CRC-32.
const CHECKSUM_LEN: usize = 4;

/// One area loaded whole: its header, the per-record info words, and its records.
struct LoadedArea {
    tag: [u8; 4],
    format: [u8; 4],
    header: [u8; HEADER_LEN],
    record_size: usize,
    info_stride: usize,
    records: Vec<Vec<u8>>,
    info: Vec<Vec<u8>>,
}

impl LoadedArea {
    fn load(raw: &Raw, svd: &Svd, area: &Area) -> Result<Self> {
        let table = RecordTable::parse(area, svd.area_bytes(raw, area)?)?;
        let header: [u8; HEADER_LEN] = table.header().try_into().map_err(|_| {
            Error::Unrecognized(format!("{} area is shorter than its header", area.tag_str()))
        })?;
        let records = table.records().map(<[u8]>::to_vec).collect::<Vec<_>>();
        let info = (0..records.len())
            .map(|i| table.record_info(i).unwrap_or_default().to_vec())
            .collect();
        Ok(Self {
            tag: area.tag,
            format: area.format,
            header,
            record_size: table.record_size,
            info_stride: table.info_stride(),
            records,
            info,
        })
    }

    /// Rebuild the area body from a chosen subset of its records, in the given order.
    fn build(&self, keep: &[usize]) -> Result<Vec<u8>> {
        let records: Vec<&[u8]> = keep
            .iter()
            .map(|&index| {
                self.records
                    .get(index)
                    .map(Vec::as_slice)
                    .ok_or_else(|| out_of_range(self, index))
            })
            .collect::<Result<_>>()?;
        let info: Vec<&[u8]> = keep
            .iter()
            .map(|&index| {
                self.info
                    .get(index)
                    .map(Vec::as_slice)
                    .ok_or_else(|| out_of_range(self, index))
            })
            .collect::<Result<_>>()?;
        Ok(self.assemble(&records, &info))
    }

    /// Lay out a body from records and their info words, refreshing any checksum.
    fn assemble(&self, records: &[&[u8]], info: &[&[u8]]) -> Vec<u8> {
        let mut header = self.header;
        let info_len = HEADER_LEN + records.len() * self.info_stride;
        header[0..4].copy_from_slice(&(records.len() as u32).to_le_bytes());
        header[4..8].copy_from_slice(&(self.record_size as u32).to_le_bytes());
        header[8..12].copy_from_slice(&(info_len as u32).to_le_bytes());

        let mut body = Vec::with_capacity(info_len + records.len() * self.record_size);
        body.extend_from_slice(&header);
        for (index, record) in records.iter().enumerate() {
            // A four-byte info word is that record's CRC-32. Recomputing rather than copying keeps
            // it right when a record was edited on the way through — carrying the original would
            // silently hand the instrument a record that fails its own integrity check.
            if self.info_stride == CHECKSUM_LEN {
                body.extend_from_slice(&crc32(record).to_le_bytes());
            } else {
                body.extend_from_slice(info.get(index).copied().unwrap_or_default());
            }
        }
        for record in records {
            body.extend_from_slice(record);
        }
        body
    }
}

fn out_of_range(area: &LoadedArea, index: usize) -> Error {
    Error::Unrecognized(format!(
        "{} record {index} is out of range (area has {})",
        String::from_utf8_lossy(&area.tag),
        area.records.len()
    ))
}

/// One waveform section of `USDa`, kept as bytes because its interior is not decoded.
struct Waveform {
    /// The `USPa` slot this audio belongs to in the bank it was read from.
    slot: u32,
    word: u32,
    bytes: Vec<u8>,
}

impl Waveform {
    /// Whether this is the same audio as `other`, ignoring which slot each sits in.
    fn same_audio(&self, other: &Waveform) -> bool {
        self.bytes == other.bytes
    }
}

/// A parsed SVZ tone bank.
struct ToneBank {
    preamble: [u8; PREAMBLE_LEN],
    /// The engine area, plus any area indexed in lockstep with it (`INSa` follows `RHYa`).
    family: Vec<LoadedArea>,
    spec: &'static AreaSpec,
    slots: Option<LoadedArea>,
    waveforms: Vec<Waveform>,
    /// The `format` stamp of the source's `USDa`, so a rebuilt one keeps it.
    waveform_format: [u8; 4],
    other: Vec<(Area, Vec<u8>)>,
    /// Area tags in the order the source file laid them out, which the output preserves.
    order: Vec<[u8; 4]>,
}

impl ToneBank {
    fn parse(raw: &Raw) -> Result<Self> {
        let svd = Svd::parse(raw)?;
        if svd.kind != Kind::Svz {
            return Err(Error::Unrecognized(
                "not an SVZ tone bank — use extract/merge for scene banks".into(),
            ));
        }
        let preamble: [u8; PREAMBLE_LEN] = svd.preamble(raw)?.try_into().unwrap();

        // The engine area decides everything else; a tone bank holds exactly one.
        let spec = address::AREAS
            .iter()
            .find(|spec| svd.area(&spec.tag).is_some())
            .ok_or_else(|| {
                Error::Unrecognized("no tone area in this SVZ (expected PATa or RHYa)".into())
            })?;

        let mut family = Vec::new();
        for tag in spec.paired {
            let area = svd.area(tag).ok_or_else(|| {
                Error::Unrecognized(format!(
                    "{} requires paired {} area",
                    spec.tag_str(),
                    String::from_utf8_lossy(*tag)
                ))
            })?;
            family.push(LoadedArea::load(raw, &svd, area)?);
        }
        let count = family[0].records.len();
        if family.iter().any(|a| a.records.len() != count) {
            return Err(Error::Unrecognized(format!(
                "{} paired area counts differ",
                spec.tag_str()
            )));
        }

        let slots = match svd.area(b"USPa") {
            Some(area) => Some(LoadedArea::load(raw, &svd, area)?),
            None => None,
        };
        let waveforms = read_waveforms(raw, &svd)?;
        let waveform_format = svd
            .area(b"USDa")
            .map(|area| area.format)
            .unwrap_or(family[0].format);

        // Anything else (DIFa) travels untouched.
        let handled: Vec<[u8; 4]> = family
            .iter()
            .map(|a| a.tag)
            .chain([*b"USPa", *b"USDa"])
            .collect();
        let mut other = Vec::new();
        for area in &svd.areas {
            if !handled.contains(&area.tag) {
                other.push((area.clone(), svd.area_bytes(raw, area)?.to_vec()));
            }
        }

        Ok(Self {
            preamble,
            family,
            spec,
            slots,
            waveforms,
            waveform_format,
            other,
            order: svd.areas.iter().map(|area| area.tag).collect(),
        })
    }

    /// The 1-based sample slots a tone record plays.
    ///
    /// Empty for every engine but `PATa` — not because those tones play no samples, but because
    /// nothing decoded says which. Callers must treat an empty result from an engine with
    /// `sample_refs_decoded == false` as "unknown", never as "none": see [`ToneBank::carries_samples`].
    fn samples_of(&self, index: usize) -> Vec<u16> {
        if !self.spec.sample_refs_decoded {
            return Vec::new();
        }
        self.family[0]
            .records
            .get(index)
            .map(|record| crate::container::sample_slots(record))
            .unwrap_or_default()
    }

    /// Whether this bank holds user samples at all, whichever engine it is for.
    fn carries_samples(&self) -> bool {
        self.slots.is_some() || !self.waveforms.is_empty()
    }

    /// Whether repackaging must carry every sample rather than selecting the referenced ones.
    fn must_carry_all_samples(&self) -> bool {
        !self.spec.sample_refs_decoded && self.carries_samples()
    }

    /// Every sample slot, in slot order — the selection used when none can be made.
    fn all_slots(&self) -> Vec<usize> {
        self.slots
            .as_ref()
            .map(|slots| (0..slots.records.len()).collect())
            .unwrap_or_default()
    }
}

/// The engine area a tone bank is built around, for callers reporting on it.
pub fn engine(raw: &Raw) -> Result<&'static AreaSpec> {
    Ok(ToneBank::parse(raw)?.spec)
}

fn read_waveforms(raw: &Raw, svd: &Svd) -> Result<Vec<Waveform>> {
    let Some(area) = svd.area(b"USDa") else {
        return Ok(Vec::new());
    };
    let bytes = svd.area_bytes(raw, area)?;
    let count = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;

    let mut out = Vec::new();
    for index in 0..count {
        let at = HEADER_LEN + index * DIRECTORY_ENTRY;
        let entry = bytes.get(at..at + DIRECTORY_ENTRY).ok_or_else(|| {
            Error::Unrecognized(format!("USDa directory entry {index} is truncated"))
        })?;
        let slot = u32::from_le_bytes(entry[0..4].try_into().unwrap());
        let offset = u32::from_le_bytes(entry[4..8].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
        let word = u32::from_le_bytes(entry[12..16].try_into().unwrap());
        let section = bytes.get(offset..offset + size).ok_or_else(|| {
            Error::Unrecognized(format!("USDa section {index} exceeds the area"))
        })?;
        out.push(Waveform {
            slot,
            word,
            bytes: section.to_vec(),
        });
    }
    Ok(out)
}

/// Build a tone bank holding only `indexes`, in the order given, with the samples they play.
///
/// Indexes are the record numbers `fantom tones` prints.
pub fn extract_tones(raw: &Raw, indexes: &[usize]) -> Result<Raw> {
    if indexes.is_empty() {
        return Err(Error::Unrecognized(
            "at least one tone index is required".into(),
        ));
    }
    let bank = ToneBank::parse(raw)?;
    let count = bank.family[0].records.len();
    if let Some(&bad) = indexes.iter().find(|&&i| i >= count) {
        return Err(Error::Unrecognized(format!(
            "tone {bad} is out of range (the bank has {count}, numbered 0..{})",
            count.saturating_sub(1)
        )));
    }
    rebuild(&bank, indexes)
}

/// Append every tone of `source` to `target`, de-duplicating identical records.
pub fn merge_tones(target: &Raw, source: &Raw) -> Result<Raw> {
    let a = ToneBank::parse(target)?;
    let b = ToneBank::parse(source)?;
    if a.spec.tag != b.spec.tag {
        return Err(Error::Unrecognized(format!(
            "cannot merge a {} bank into a {} bank",
            b.spec.tag_str(),
            a.spec.tag_str()
        )));
    }
    if a.family[0].record_size != b.family[0].record_size {
        return Err(Error::Unrecognized(
            "tone record sizes differ between the two banks".into(),
        ));
    }
    merge(&a, &b)
}

/// Emit a bank containing `keep` from `bank`.
fn rebuild(bank: &ToneBank, keep: &[usize]) -> Result<Raw> {
    // Which sample slots the chosen tones need, in first-seen order. Stays empty for an engine
    // whose sample references are not decoded, which is why `carry_all` exists rather than this
    // being read as "the selection plays no samples".
    let mut slot_map: BTreeMap<u16, u16> = BTreeMap::new();
    for &index in keep {
        for slot in bank.samples_of(index) {
            let next = slot_map.len() as u16 + 1;
            slot_map.entry(slot).or_insert(next);
        }
    }
    let carry_all = bank.must_carry_all_samples();

    let mut areas: Vec<([u8; 4], [u8; 4], Vec<u8>)> = Vec::new();
    for (position, area) in bank.family.iter().enumerate() {
        // Rewrite each tone's sample references *before* the body is laid out, so the checksum
        // written alongside it covers the record we actually emit.
        let mut records = Vec::with_capacity(keep.len());
        for &index in keep {
            let mut record = area
                .records
                .get(index)
                .cloned()
                .ok_or_else(|| out_of_range(area, index))?;
            if position == 0 && !slot_map.is_empty() {
                crate::container::remap_sample_slots(&mut record, &slot_map);
            }
            records.push(record);
        }
        let info: Vec<&[u8]> = keep
            .iter()
            .map(|&i| area.info.get(i).map(Vec::as_slice).unwrap_or_default())
            .collect();
        let rows: Vec<&[u8]> = records.iter().map(Vec::as_slice).collect();
        areas.push((area.tag, area.format, area.assemble(&rows, &info)));
    }

    if carry_all {
        // Every slot keeps its original number, so whatever the records reference still resolves.
        if let Some(slots) = bank.slots.as_ref() {
            areas.push((slots.tag, slots.format, slots.build(&bank.all_slots())?));
        }
        if !bank.waveforms.is_empty() {
            let sections = bank.waveforms.iter().map(|w| (w.slot, w.word, w.bytes.as_slice()));
            areas.push((
                *b"USDa",
                bank.waveform_format,
                build_waveform_area(sections)?,
            ));
        }
    } else if !slot_map.is_empty() {
        let slots = bank.slots.as_ref().ok_or_else(|| {
            Error::Unrecognized("tones reference samples but the bank has no USPa area".into())
        })?;
        let keep_slots: Vec<usize> = slot_map.keys().map(|&s| s as usize - 1).collect();
        areas.push((slots.tag, slots.format, slots.build(&keep_slots)?));

        // The slots were renumbered densely above; the directory must say the same.
        let sections: Vec<(u32, u32, &[u8])> = keep_slots
            .iter()
            .enumerate()
            .map(|(new, &slot)| {
                bank.waveforms
                    .iter()
                    .find(|w| w.slot as usize == slot)
                    .map(|w| (new as u32, w.word, w.bytes.as_slice()))
                    .ok_or_else(|| {
                        Error::Unrecognized(format!("no waveform data for sample slot {slot}"))
                    })
            })
            .collect::<Result<_>>()?;
        areas.push((
            *b"USDa",
            bank.waveform_format,
            build_waveform_area(sections)?,
        ));
    }

    for (area, body) in &bank.other {
        areas.push((area.tag, area.format, body.clone()));
    }
    assemble(&bank.preamble, &bank.order, areas)
}

/// Emit a bank holding every tone of `a` then every new tone of `b`.
fn merge(a: &ToneBank, b: &ToneBank) -> Result<Raw> {
    let mut records: Vec<Vec<Vec<u8>>> = Vec::new();
    let mut info: Vec<Vec<Vec<u8>>> = Vec::new();
    let mut samples: Vec<Vec<u16>> = Vec::new();
    let mut waveforms: Vec<Waveform> = Vec::new();
    let mut slot_records: Vec<Vec<u8>> = Vec::new();
    let mut slot_info: Vec<Vec<u8>> = Vec::new();

    for bank in [a, b] {
        for index in 0..bank.family[0].records.len() {
            let entry: Vec<Vec<u8>> = bank
                .family
                .iter()
                .map(|area| area.records[index].clone())
                .collect();
            if records.contains(&entry) {
                continue;
            }
            // Carry this tone's samples across, renumbering as they land in the output.
            let mut remap = BTreeMap::new();
            for slot in bank.samples_of(index) {
                let source_slot = slot as usize - 1;
                let Some(source) = bank
                    .waveforms
                    .iter()
                    .find(|w| w.slot as usize == source_slot)
                else {
                    return Err(Error::Unrecognized(format!(
                        "no waveform data for sample slot {source_slot}"
                    )));
                };
                let existing = waveforms.iter().position(|w| w.bytes == source.bytes);
                let new_index = match existing {
                    Some(i) => i,
                    None => {
                        let slots = bank.slots.as_ref().ok_or_else(|| {
                            Error::Unrecognized("sampled tone without a USPa area".into())
                        })?;
                        slot_records.push(slots.records[source_slot].clone());
                        slot_info.push(slots.info[source_slot].clone());
                        waveforms.push(Waveform {
                            slot: waveforms.len() as u32,
                            word: source.word,
                            bytes: source.bytes.clone(),
                        });
                        waveforms.len() - 1
                    }
                };
                remap.insert(slot, new_index as u16 + 1);
            }
            let mut entry = entry;
            if !remap.is_empty() {
                crate::container::remap_sample_slots(&mut entry[0], &remap);
            }
            records.push(entry);
            info.push(
                bank.family
                    .iter()
                    .map(|area| area.info[index].clone())
                    .collect(),
            );
            samples.push(Vec::new());
        }
    }

    let mut areas: Vec<([u8; 4], [u8; 4], Vec<u8>)> = Vec::new();
    for (position, area) in a.family.iter().enumerate() {
        let rows: Vec<&[u8]> = records.iter().map(|e| e[position].as_slice()).collect();
        let words: Vec<&[u8]> = info.iter().map(|e| e[position].as_slice()).collect();
        areas.push((area.tag, area.format, area.assemble(&rows, &words)));
    }

    if !a.spec.sample_refs_decoded {
        // Nothing said which record plays what, so the two sample banks could not be interleaved
        // above. Carry one through whole instead — see `shared_sample_bank` for when that is safe.
        if let Some(source) = shared_sample_bank(a, b)? {
            if let Some(slots) = source.slots.as_ref() {
                areas.push((slots.tag, slots.format, slots.build(&source.all_slots())?));
            }
            if !source.waveforms.is_empty() {
                let sections =
                    source.waveforms.iter().map(|w| (w.slot, w.word, w.bytes.as_slice()));
                areas.push((
                    *b"USDa",
                    source.waveform_format,
                    build_waveform_area(sections)?,
                ));
            }
        }
    } else if !waveforms.is_empty() {
        let slots = a
            .slots
            .as_ref()
            .or(b.slots.as_ref())
            .ok_or_else(|| Error::Unrecognized("sampled tones without a USPa area".into()))?;
        let rows: Vec<&[u8]> = slot_records.iter().map(Vec::as_slice).collect();
        let words: Vec<&[u8]> = slot_info.iter().map(Vec::as_slice).collect();
        areas.push((slots.tag, slots.format, slots.assemble(&rows, &words)));
        let sections = waveforms
            .iter()
            .enumerate()
            .map(|(i, w)| (i as u32, w.word, w.bytes.as_slice()));
        areas.push((*b"USDa", a.waveform_format, build_waveform_area(sections)?));
    }

    for (area, body) in &a.other {
        areas.push((area.tag, area.format, body.clone()));
    }
    assemble(&a.preamble, &a.order, areas)
}

/// The sample bank a merge of two undecoded-engine banks carries, if either has one.
///
/// Their slots cannot be interleaved the way `PATa`'s are: nothing decoded says which record plays
/// which sample, so a collision could only be resolved by rewriting references there is no way to
/// find. One side carrying samples is fine — its slot numbers survive untouched. Both is only fine
/// when they carry the same samples in the same slots, which is the common case, since banks
/// usually come from extracts of one source. Anything else stops rather than emit a bank whose
/// audio is quietly wrong.
fn shared_sample_bank<'a>(a: &'a ToneBank, b: &'a ToneBank) -> Result<Option<&'a ToneBank>> {
    match (a.carries_samples(), b.carries_samples()) {
        (false, false) => Ok(None),
        (true, false) => Ok(Some(a)),
        (false, true) => Ok(Some(b)),
        (true, true) => {
            let same = a.waveforms.len() == b.waveforms.len()
                && a.waveforms
                    .iter()
                    .zip(&b.waveforms)
                    .all(|(x, y)| x.slot == y.slot && x.same_audio(y))
                && match (a.slots.as_ref(), b.slots.as_ref()) {
                    (Some(x), Some(y)) => x.records == y.records,
                    (None, None) => true,
                    _ => false,
                };
            if !same {
                return Err(Error::Unrecognized(format!(
                    "cannot merge two {} banks that carry different user samples — a {} record's \
                     sample references are not decoded, so colliding slots cannot be renumbered \
                     without silently breaking them",
                    a.spec.tag_str(),
                    a.spec.tag_str(),
                )));
            }
            Ok(Some(a))
        }
    }
}

/// Lay out a `USDa`: header, one directory entry per section, then the sections back to back.
///
/// Each section is passed as `(slot, word, bytes)`, so callers decide whether samples keep their
/// original numbers or are renumbered densely, and supply the per-section word — which is carried,
/// never computed. [`crate::samplebank`] builds its sections from a backup and uses this too.
pub(crate) fn build_waveform_area<'a>(
    sections: impl IntoIterator<Item = (u32, u32, &'a [u8])>,
) -> Result<Vec<u8>> {
    let sections: Vec<(u32, u32, &[u8])> = sections.into_iter().collect();
    let info_len = HEADER_LEN + sections.len() * DIRECTORY_ENTRY;
    let mut header = [0u8; HEADER_LEN];
    header[0..4].copy_from_slice(&(sections.len() as u32).to_le_bytes());
    header[8..12].copy_from_slice(&(info_len as u32).to_le_bytes());

    let mut directory = Vec::new();
    let mut bodies = Vec::new();
    let mut offset = info_len;
    for (slot, word, bytes) in &sections {
        directory.extend_from_slice(&slot.to_le_bytes());
        directory.extend_from_slice(&(offset as u32).to_le_bytes());
        directory.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        directory.extend_from_slice(&word.to_le_bytes());
        offset += bytes.len();
        bodies.extend_from_slice(bytes);
    }

    let mut body = Vec::with_capacity(offset);
    body.extend_from_slice(&header);
    body.extend_from_slice(&directory);
    body.extend_from_slice(&bodies);
    Ok(body)
}

/// Write the preamble, area table, and bodies of a new SVZ, keeping the source's area order.
///
/// The result is checked before it is returned: every record must match the checksum written
/// beside it, and every area's geometry must match its size. A file that fails is a bug here, and
/// it is better to fail loudly than to hand the instrument something it will reject — or worse,
/// accept.
pub(crate) fn assemble(
    preamble: &[u8; PREAMBLE_LEN],
    order: &[[u8; 4]],
    mut areas: Vec<([u8; 4], [u8; 4], Vec<u8>)>,
) -> Result<Raw> {
    let rank = |tag: &[u8; 4]| order.iter().position(|t| t == tag).unwrap_or(usize::MAX);
    areas.sort_by_key(|(tag, _, _)| rank(tag));

    let count = u8::try_from(areas.len())
        .map_err(|_| Error::Unrecognized("too many areas for an SVZ".into()))?;
    let mut bytes = preamble.to_vec();
    bytes[AREA_COUNT_BYTE] = count;

    let mut offset = PREAMBLE_LEN + areas.len() * Area::LEN;
    for (tag, format, body) in &areas {
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(format);
        bytes.extend_from_slice(&(offset as u32).to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        offset += body.len();
    }
    for (_, _, body) in &areas {
        bytes.extend_from_slice(body);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::sample_slots;

    const TONE_LEN: usize = 700;

    /// A record of `len` bytes carrying nothing but a name.
    fn named(name: &str, len: usize) -> Vec<u8> {
        let mut record = vec![0u8; len];
        record[..16].fill(b' ');
        record[..name.len()].copy_from_slice(name.as_bytes());
        record
    }

    /// A tone record whose first partial optionally plays user sample `slot`.
    fn tone(name: &str, slot: Option<u16>) -> Vec<u8> {
        let mut record = named(name, TONE_LEN);
        if let Some(slot) = slot {
            record[0xdf] = 2;
            record[0xe2..0xe4].copy_from_slice(&slot.to_le_bytes());
        }
        record
    }

    fn area_body(records: &[Vec<u8>], record_size: usize, info_stride: usize) -> Vec<u8> {
        let info_len = HEADER_LEN + records.len() * info_stride;
        let mut body = Vec::new();
        body.extend_from_slice(&(records.len() as u32).to_le_bytes());
        body.extend_from_slice(&(record_size as u32).to_le_bytes());
        body.extend_from_slice(&(info_len as u32).to_le_bytes());
        body.extend_from_slice(&[0u8; 4]);
        for record in records {
            // Real files store each record's CRC-32 here; a fixture must too, or the output
            // self-check will (rightly) reject the file built from it.
            let word = if info_stride == CHECKSUM_LEN {
                crc32(record)
            } else {
                0
            };
            body.extend_from_slice(&word.to_le_bytes()[..info_stride]);
        }
        for record in records {
            body.extend_from_slice(record);
        }
        body
    }

    /// A USDa holding one section per sample, each just a tagged blob.
    fn waveform_body(sections: &[Vec<u8>]) -> Vec<u8> {
        let refs: Vec<Waveform> = sections
            .iter()
            .enumerate()
            .map(|(i, bytes)| Waveform {
                slot: i as u32,
                word: 0x1234_0000 + i as u32,
                bytes: bytes.clone(),
            })
            .collect();
        build_waveform_area(refs.iter().map(|w| (w.slot, w.word, w.bytes.as_slice()))).unwrap()
    }

    fn svz(areas: &[([u8; 4], Vec<u8>)]) -> Raw {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"SVZa");
        bytes.push(areas.len() as u8);
        bytes.push(3);
        bytes.extend_from_slice(b"KY019$");
        bytes.extend_from_slice(&[0u8; 4]);
        let mut offset = PREAMBLE_LEN + areas.len() * Area::LEN;
        for (tag, body) in areas {
            bytes.extend_from_slice(tag);
            bytes.extend_from_slice(b"ZCOR");
            bytes.extend_from_slice(&(offset as u32).to_le_bytes());
            bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
            offset += body.len();
        }
        for (_, body) in areas {
            bytes.extend_from_slice(body);
        }
        Raw::from_bytes(bytes)
    }

    fn sampled_bank() -> Raw {
        let tones = vec![
            tone("Plain", None),
            tone("Sampled A", Some(1)),
            tone("Sampled B", Some(2)),
        ];
        let slots = vec![
            tone("SlotOne", None)[..64].to_vec(),
            tone("SlotTwo", None)[..64].to_vec(),
        ];
        svz(&[
            (*b"DIFa", area_body(&[vec![7u8; 32]], 32, 4)),
            (*b"PATa", area_body(&tones, TONE_LEN, 4)),
            (*b"USPa", area_body(&slots, 64, 4)),
            (
                *b"USDa",
                waveform_body(&[b"SMPd-one".to_vec(), b"SMPd-two".to_vec()]),
            ),
        ])
    }

    const KIT_LEN: usize = 256;
    const INSTRUMENTS_LEN: usize = 512;

    /// A drum bank: `RHYa` kits with their paired `INSa` instrument sets, carrying `samples` that
    /// nothing decoded links to any particular kit.
    fn drum_bank(samples: &[&str]) -> Raw {
        let kits = vec![named("Kit A", KIT_LEN), named("Kit B", KIT_LEN)];
        let instruments = vec![
            named("Instruments A", INSTRUMENTS_LEN),
            named("Instruments B", INSTRUMENTS_LEN),
        ];
        let mut areas = vec![
            (*b"DIFa", area_body(&[vec![7u8; 32]], 32, 4)),
            (*b"RHYa", area_body(&kits, KIT_LEN, 4)),
            (*b"INSa", area_body(&instruments, INSTRUMENTS_LEN, 4)),
        ];
        if !samples.is_empty() {
            let slots: Vec<Vec<u8>> = samples.iter().map(|name| named(name, 64)).collect();
            let audio: Vec<Vec<u8>> = samples
                .iter()
                .map(|name| format!("SMPd-{name}").into_bytes())
                .collect();
            areas.push((*b"USPa", area_body(&slots, 64, 4)));
            areas.push((*b"USDa", waveform_body(&audio)));
        }
        svz(&areas)
    }

    /// The slot each `USDa` directory entry names, in order.
    fn waveform_slots(raw: &Raw) -> Vec<u32> {
        let svd = Svd::parse(raw).unwrap();
        let Some(area) = svd.area(b"USDa") else {
            return Vec::new();
        };
        let bytes = svd.area_bytes(raw, area).unwrap();
        let count = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
        (0..count)
            .map(|index| {
                let at = HEADER_LEN + index * DIRECTORY_ENTRY;
                u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
            })
            .collect()
    }

    fn contains(raw: &Raw, needle: &[u8]) -> bool {
        let svd = Svd::parse(raw).unwrap();
        let bytes = svd
            .area_bytes(raw, svd.area(b"USDa").unwrap())
            .unwrap()
            .to_vec();
        bytes.windows(needle.len()).any(|w| w == needle)
    }

    fn slot_count(raw: &Raw) -> usize {
        let svd = Svd::parse(raw).unwrap();
        svd.area(b"USPa")
            .map(|area| {
                RecordTable::parse(area, svd.area_bytes(raw, area).unwrap())
                    .unwrap()
                    .len()
            })
            .unwrap_or(0)
    }

    fn tone_names(raw: &Raw) -> Vec<String> {
        crate::codec::read_bundled_tones(raw)
            .unwrap()
            .into_iter()
            .map(|t| t.name)
            .collect()
    }

    fn tone_records(raw: &Raw) -> Vec<Vec<u8>> {
        let svd = Svd::parse(raw).unwrap();
        let area = svd.area(b"PATa").unwrap();
        let table = RecordTable::parse(area, svd.area_bytes(raw, area).unwrap()).unwrap();
        table.records().map(<[u8]>::to_vec).collect()
    }

    #[test]
    fn extracts_the_requested_tones_in_order() {
        let extracted = extract_tones(&sampled_bank(), &[2, 0]).unwrap();
        assert_eq!(tone_names(&extracted), ["Sampled B", "Plain"]);
    }

    /// The point of an SVZ: a sampled tone brings its waveform along, renumbered to match.
    #[test]
    fn a_sampled_tone_carries_its_sample_and_is_renumbered() {
        let extracted = extract_tones(&sampled_bank(), &[2]).unwrap();

        // The tone referenced slot 2; alone in the output it must now reference slot 1.
        assert_eq!(sample_slots(&tone_records(&extracted)[0]), [1]);

        let svd = Svd::parse(&extracted).unwrap();
        let slots = RecordTable::parse(
            svd.area(b"USPa").unwrap(),
            svd.area_bytes(&extracted, svd.area(b"USPa").unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(slots.len(), 1, "only the referenced slot travels");
        assert_eq!(&slots.record(0).unwrap()[..7], b"SlotTwo");

        // And its audio, verbatim.
        let usda = svd.area_bytes(&extracted, svd.area(b"USDa").unwrap()).unwrap();
        assert!(
            usda.windows(8).any(|w| w == b"SMPd-two"),
            "the referenced waveform is missing"
        );
        assert!(
            !usda.windows(8).any(|w| w == b"SMPd-one"),
            "an unreferenced waveform came along"
        );
    }

    #[test]
    fn a_tone_without_samples_produces_no_sample_areas() {
        let extracted = extract_tones(&sampled_bank(), &[0]).unwrap();
        let svd = Svd::parse(&extracted).unwrap();
        assert!(svd.area(b"USPa").is_none());
        assert!(svd.area(b"USDa").is_none());
    }

    #[test]
    fn areas_keep_the_order_the_source_used() {
        let extracted = extract_tones(&sampled_bank(), &[1]).unwrap();
        let tags: Vec<String> = Svd::parse(&extracted)
            .unwrap()
            .areas
            .iter()
            .map(|a| a.tag_str())
            .collect();
        assert_eq!(tags, ["DIFa", "PATa", "USPa", "USDa"]);
    }

    #[test]
    fn merging_appends_new_tones_and_drops_duplicates() {
        let bank = sampled_bank();
        let a = extract_tones(&bank, &[0, 1]).unwrap();
        let b = extract_tones(&bank, &[1, 2]).unwrap();

        let merged = merge_tones(&a, &b).unwrap();
        assert_eq!(tone_names(&merged), ["Plain", "Sampled A", "Sampled B"]);

        // Both sampled tones kept distinct samples, renumbered into one dense bank.
        assert_eq!(sample_slots(&tone_records(&merged)[1]), [1]);
        assert_eq!(sample_slots(&tone_records(&merged)[2]), [2]);
    }

    #[test]
    fn merging_a_bank_with_itself_changes_nothing() {
        let a = extract_tones(&sampled_bank(), &[0, 2]).unwrap();
        let merged = merge_tones(&a, &a).unwrap();
        assert_eq!(tone_names(&merged), tone_names(&a));
    }

    #[test]
    fn refuses_a_scene_bank() {
        let raw = Raw::from_bytes({
            let mut b = vec![];
            b.extend_from_slice(&30u16.to_le_bytes());
            b.extend_from_slice(b"SVD5");
            b.extend_from_slice(&[0u8; 26]);
            b
        });
        let error = extract_tones(&raw, &[0]).unwrap_err().to_string();
        assert!(error.contains("not an SVZ tone bank"), "{error}");
    }

    /// The checksum must cover the record we actually emit. Rewriting a tone's sample reference
    /// after computing its CRC once produced a record the instrument's own check would reject.
    #[test]
    fn a_rewritten_record_gets_a_checksum_that_matches_it() {
        let extracted = extract_tones(&sampled_bank(), &[2]).unwrap();
        let report = crate::verify::check(&extracted).unwrap();
        assert!(report.is_ok(), "{:?}", report.problems);
        assert!(report.checked > 0, "nothing was actually checked");

        // The rewritten record's checksum must differ from the source's, since its bytes did.
        let source = sampled_bank();
        let svd = Svd::parse(&source).unwrap();
        let area = svd.area(b"PATa").unwrap();
        let table = RecordTable::parse(area, svd.area_bytes(&source, area).unwrap()).unwrap();
        let original = u32::from_le_bytes(table.record_info(2).unwrap().try_into().unwrap());

        let svd = Svd::parse(&extracted).unwrap();
        let area = svd.area(b"PATa").unwrap();
        let table = RecordTable::parse(area, svd.area_bytes(&extracted, area).unwrap()).unwrap();
        let emitted = u32::from_le_bytes(table.record_info(0).unwrap().try_into().unwrap());
        assert_ne!(emitted, original, "checksum was carried over, not recomputed");
    }

    /// A record copied through untouched must keep its original checksum exactly.
    #[test]
    fn an_untouched_record_keeps_its_checksum() {
        let source = sampled_bank();
        let extracted = extract_tones(&source, &[0]).unwrap();

        let svd = Svd::parse(&source).unwrap();
        let area = svd.area(b"PATa").unwrap();
        let table = RecordTable::parse(area, svd.area_bytes(&source, area).unwrap()).unwrap();
        let original = table.record_info(0).unwrap().to_vec();

        let svd = Svd::parse(&extracted).unwrap();
        let area = svd.area(b"PATa").unwrap();
        let table = RecordTable::parse(area, svd.area_bytes(&extracted, area).unwrap()).unwrap();
        assert_eq!(table.record_info(0).unwrap(), original.as_slice());
    }

    /// A drum kit's tone→sample link is not decoded, so no sample can be shown to be unreferenced.
    /// Selecting none — which is what asking `samples_of` for a `RHYa` record used to produce —
    /// silently stripped the audio out of a sampled kit. Carry every sample instead.
    #[test]
    fn an_undecoded_engine_carries_every_sample_it_has() {
        let extracted = extract_tones(&drum_bank(&["one", "two"]), &[1]).unwrap();

        assert_eq!(tone_names(&extracted), ["Kit B"]);
        assert_eq!(slot_count(&extracted), 2, "both slots must travel");
        assert!(contains(&extracted, b"SMPd-one"));
        assert!(contains(&extracted, b"SMPd-two"));
    }

    /// Carrying them is only safe if their numbers do not move: an undecoded record's reference to
    /// slot 2 can never be rewritten, so slot 2 has to still be slot 2.
    #[test]
    fn carried_samples_keep_their_original_slot_numbers() {
        let extracted = extract_tones(&drum_bank(&["one", "two", "three"]), &[0]).unwrap();
        assert_eq!(waveform_slots(&extracted), [0, 1, 2]);
    }

    #[test]
    fn an_undecoded_engine_without_samples_gains_no_sample_areas() {
        let extracted = extract_tones(&drum_bank(&[]), &[0]).unwrap();
        let svd = Svd::parse(&extracted).unwrap();
        assert!(svd.area(b"USPa").is_none());
        assert!(svd.area(b"USDa").is_none());
    }

    #[test]
    fn merging_undecoded_banks_carries_the_samples_through() {
        let bank = drum_bank(&["one", "two"]);
        let a = extract_tones(&bank, &[0]).unwrap();
        let b = extract_tones(&bank, &[1]).unwrap();

        let merged = merge_tones(&a, &b).unwrap();
        assert_eq!(tone_names(&merged), ["Kit A", "Kit B"]);
        assert_eq!(slot_count(&merged), 2);
        assert_eq!(waveform_slots(&merged), [0, 1]);
    }

    #[test]
    fn merging_an_undecoded_bank_that_has_samples_into_one_that_does_not() {
        let a = extract_tones(&drum_bank(&[]), &[0]).unwrap();
        let b = extract_tones(&drum_bank(&["one"]), &[1]).unwrap();

        let merged = merge_tones(&a, &b).unwrap();
        assert_eq!(slot_count(&merged), 1);
        assert!(contains(&merged, b"SMPd-one"));
    }

    /// Two drum banks holding *different* samples cannot be joined: slot 1 means one thing in each,
    /// and no decoded field says which kit to repoint. Refusing beats emitting a plausible file
    /// whose kits play the wrong audio.
    #[test]
    fn merging_undecoded_banks_with_different_samples_is_refused() {
        let a = extract_tones(&drum_bank(&["one"]), &[0]).unwrap();
        let b = extract_tones(&drum_bank(&["other"]), &[1]).unwrap();

        let error = merge_tones(&a, &b).unwrap_err().to_string();
        assert!(error.contains("different user samples"), "{error}");
    }

    #[test]
    fn rejects_a_tone_index_past_the_end() {
        let error = extract_tones(&sampled_bank(), &[9]).unwrap_err().to_string();
        assert!(error.contains("out of range"), "{error}");
    }
}
