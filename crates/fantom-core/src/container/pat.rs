use crate::container::{ascii_trim, Area, Raw};
use crate::{Error, Result};

/// The `PATa` (patch/tone) area: a header plus fixed-stride tone records.
///
/// Same envelope as `PRFa`: a header of `count`, `record_size`, `data_start`, then `count` records.
/// Each record's first 16 bytes are the tone name; byte `+0x10` is the tone category. A scene's
/// per-zone `tone_id` indexes this list for user tones (see `docs/FORMAT.md`).
///
/// **Records start at the declared `data_start`, not at a fixed 16.** Every SVD5 area declares 16,
/// which is why treating it as constant worked for years; an SVZ declares `16 + 4 × count`, because
/// it stores a CRC-32 per record between the header and the records. Reading an SVZ's `PATa` at a
/// fixed offset lands four bytes short per info word and scrambles every field.
pub struct PatArea {
    tones: Vec<Tone>,
}

/// One tone stored in a `PATa` area.
#[derive(Debug, Clone, PartialEq)]
pub struct Tone {
    pub name: String,
    /// Tone category byte (e.g. `0x23` = brass); meaning of most values still TBD.
    pub category: u8,
    /// User sample slots this tone plays, 1-based as stored. Empty for the vast majority of
    /// tones, which play ROM waves. See [`sample_slots`].
    pub samples: Vec<u16>,
}

const HEADER_LEN: usize = 0x10;
const COUNT_OFFSET: usize = 0x00;
const RECORD_SIZE_OFFSET: usize = 0x04;
/// Where the records begin, measured from the area body.
const DATA_START_OFFSET: usize = 0x08;
const NAME_LEN: usize = 16;
const CATEGORY_OFFSET: usize = 0x10;

/// A ZEN-Core tone has four partials laid out at this stride within its record.
const PARTIAL_STRIDE: usize = 124;
const PARTIAL_COUNT: usize = 4;
/// Selects where a partial's wave comes from: 0 = internal ROM wave, 2 = user sample.
const WAVE_GROUP_OFFSET: usize = 0xdf;
const WAVE_NUMBER_OFFSET: usize = 0xe2;
/// The wave-group value meaning "this partial plays a user sample".
const WAVE_GROUP_SAMPLE: u8 = 2;

/// The user sample slots a `PATa` tone record plays, 1-based, in partial order.
///
/// Each of the four partials selects a wave group and a wave number. Group 2 means the number is
/// an `SMPa` slot rather than a ROM wave. Confirmed on a FANTOM-6 backup: all 93 group-2 partials
/// across its 2048 tones resolve to a populated slot in 1..50, with names matching
/// (`IML Whoa 1` → slot 3 `3 IML Whoa 1`, `Relax Bass` → slots 7 and 8, and so on).
///
/// This is what makes user samples a *dependency*: the reference is a slot number, so a tone
/// carries no audio with it. The instrument's own scene exports behave the same way — NARF holds
/// 68 such references and no sample areas at all — so samples must be transferred separately.
pub fn sample_slots(record: &[u8]) -> Vec<u16> {
    let mut slots = Vec::new();
    for partial in 0..PARTIAL_COUNT {
        let base = partial * PARTIAL_STRIDE;
        let (Some(&group), Some(number)) = (
            record.get(WAVE_GROUP_OFFSET + base),
            record
                .get(WAVE_NUMBER_OFFSET + base..WAVE_NUMBER_OFFSET + base + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]])),
        ) else {
            break;
        };
        if group == WAVE_GROUP_SAMPLE && number != 0 && !slots.contains(&number) {
            slots.push(number);
        }
    }
    slots
}

impl PatArea {
    /// Parse the tone list from a `PATa` area's bytes.
    pub fn parse(area: &[u8]) -> Result<Self> {
        let count = read_u32(area, COUNT_OFFSET)? as usize;
        let record_size = read_u32(area, RECORD_SIZE_OFFSET)? as usize;
        if record_size == 0 {
            return Err(Error::Unrecognized("PATa record size is zero".into()));
        }

        // Trust the declared start, but never let a bad value push records out of the area or
        // overlap the header just read — the same rule [`crate::container::RecordTable`] applies.
        let declared = read_u32(area, DATA_START_OFFSET)? as usize;
        let data_start = if (HEADER_LEN..=area.len()).contains(&declared) {
            declared
        } else {
            HEADER_LEN
        };

        let mut tones = Vec::with_capacity(count);
        for i in 0..count {
            let start = data_start + i * record_size;
            let Some(record) = area.get(start..start + record_size) else {
                break; // truncated area — keep what parsed
            };
            tones.push(Tone {
                name: ascii_trim(&record[..NAME_LEN]),
                category: record[CATEGORY_OFFSET],
                samples: sample_slots(record),
            });
        }
        Ok(Self { tones })
    }

    /// Convenience: locate and parse the `PATa` area of an SVD file.
    pub fn from_svd(raw: &Raw, svd: &crate::container::Svd) -> Result<Self> {
        let area = svd
            .area(b"PATa")
            .ok_or_else(|| Error::Unrecognized("no PATa (tone) area in file".into()))?;
        Self::parse(svd.area_bytes(raw, area)?)
    }

    /// All tones, in storage order.
    pub fn tones(&self) -> &[Tone] {
        &self.tones
    }

    /// The tone at `index`, if present.
    pub fn get(&self, index: usize) -> Option<&Tone> {
        self.tones.get(index)
    }
}

impl Area {
    /// The `PATa` tag, exposed for callers building tone lookups.
    pub const PATA: &'static [u8; 4] = b"PATa";
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32> {
    let slice = bytes
        .get(at..at + 4)
        .ok_or_else(|| Error::Unrecognized(format!("PATa area truncated at offset {at}")))?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

/// Rewrite a tone record's user-sample references through `remap` (old slot -> new slot).
///
/// Used when repackaging carries a tone's samples along and renumbers them densely.
pub fn remap_sample_slots(record: &mut [u8], remap: &std::collections::BTreeMap<u16, u16>) {
    for partial in 0..PARTIAL_COUNT {
        let base = partial * PARTIAL_STRIDE;
        let (Some(&group), Some(number)) = (
            record.get(WAVE_GROUP_OFFSET + base),
            record
                .get(WAVE_NUMBER_OFFSET + base..WAVE_NUMBER_OFFSET + base + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]])),
        ) else {
            break;
        };
        if group != WAVE_GROUP_SAMPLE {
            continue;
        }
        if let Some(&new) = remap.get(&number) {
            record[WAVE_NUMBER_OFFSET + base..WAVE_NUMBER_OFFSET + base + 2]
                .copy_from_slice(&new.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area_with(names: &[(&str, u8)], record_size: usize) -> Vec<u8> {
        let mut a = Vec::new();
        a.extend_from_slice(&(names.len() as u32).to_le_bytes());
        a.extend_from_slice(&(record_size as u32).to_le_bytes());
        a.extend_from_slice(&[0u8; 8]);
        for (name, cat) in names {
            let mut rec = vec![0u8; record_size];
            let b = name.as_bytes();
            rec[..b.len().min(NAME_LEN)].copy_from_slice(&b[..b.len().min(NAME_LEN)]);
            rec[CATEGORY_OFFSET] = *cat;
            a.extend_from_slice(&rec);
        }
        a
    }

    #[test]
    fn parses_tone_names_and_categories() {
        let area = area_with(&[("Africa Brass", 0x23), ("Africa Kalimba", 0x0f)], 64);
        let pat = PatArea::parse(&area).unwrap();
        assert_eq!(pat.tones().len(), 2);
        assert_eq!(pat.get(0).unwrap().name, "Africa Brass");
        assert_eq!(pat.get(0).unwrap().category, 0x23);
        assert_eq!(pat.get(1).unwrap().name, "Africa Kalimba");
        assert!(pat.get(2).is_none());
    }

    /// An SVZ's `PATa` puts a CRC-32 per record between the header and the records, so its records
    /// start at `16 + 4 × count`. Reading at a fixed 16 lands mid-header and scrambles every field —
    /// which is what happened to the one tone of an instrument-written `.svz` export.
    #[test]
    fn records_start_at_the_declared_offset_not_a_fixed_sixteen() {
        const RECORD_SIZE: usize = 64;
        let names = [("Sledgehammer Sha", 0x11u8), ("Second Tone", 0x22)];
        let data_start = HEADER_LEN + names.len() * 4;

        let mut area = Vec::new();
        area.extend_from_slice(&(names.len() as u32).to_le_bytes());
        area.extend_from_slice(&(RECORD_SIZE as u32).to_le_bytes());
        area.extend_from_slice(&(data_start as u32).to_le_bytes());
        area.extend_from_slice(&[0u8; 4]);
        area.extend_from_slice(&[0xAA; 8]); // the per-record info words
        for (name, category) in names {
            let mut record = vec![0u8; RECORD_SIZE];
            record[..name.len()].copy_from_slice(name.as_bytes());
            record[CATEGORY_OFFSET] = category;
            area.extend_from_slice(&record);
        }

        let pat = PatArea::parse(&area).unwrap();
        assert_eq!(pat.get(0).unwrap().name, "Sledgehammer Sha");
        assert_eq!(pat.get(0).unwrap().category, 0x11);
        assert_eq!(pat.get(1).unwrap().name, "Second Tone");
    }

    /// A declared start that cannot be right must not send the reader out of the area.
    #[test]
    fn an_impossible_data_start_falls_back_to_the_fixed_header() {
        let mut area = area_with(&[("Africa Brass", 0x23)], 64);
        area[8..12].copy_from_slice(&0x5555_5555u32.to_le_bytes());
        assert_eq!(PatArea::parse(&area).unwrap().get(0).unwrap().name, "Africa Brass");
    }

    /// Build a tone record whose partials use the given `(group, wave number)` pairs.
    fn tone_with_partials(partials: &[(u8, u16)]) -> Vec<u8> {
        let mut record = vec![0u8; PARTIAL_COUNT * PARTIAL_STRIDE + WAVE_NUMBER_OFFSET + 2];
        for (partial, &(group, number)) in partials.iter().enumerate() {
            let base = partial * PARTIAL_STRIDE;
            record[WAVE_GROUP_OFFSET + base] = group;
            record[WAVE_NUMBER_OFFSET + base..WAVE_NUMBER_OFFSET + base + 2]
                .copy_from_slice(&number.to_le_bytes());
        }
        record
    }

    #[test]
    fn reads_the_user_samples_a_tone_plays() {
        // "Relax Bass": partials 1 and 2 play user samples 7 and 8, the rest are ROM waves.
        let record = tone_with_partials(&[(2, 7), (2, 8), (0, 383), (0, 0)]);
        assert_eq!(sample_slots(&record), [7, 8]);

        // A wave number only means a sample when its group says so.
        assert_eq!(sample_slots(&tone_with_partials(&[(0, 7)])), Vec::<u16>::new());
        // Repeats collapse: four partials layering one sample is still one dependency.
        assert_eq!(sample_slots(&tone_with_partials(&[(2, 5), (2, 5)])), [5]);
        // A record too short to hold the partials yields nothing rather than panicking.
        assert!(sample_slots(&[0u8; 32]).is_empty());
    }
}
