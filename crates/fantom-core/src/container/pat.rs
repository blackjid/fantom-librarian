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
    /// User **multisample** slots this tone plays, 1-based. A separate dependency from `samples`
    /// and one this tool cannot yet carry: the `MLSa` records that define a multisample are not
    /// decoded. Naming them is the least that can be done — see [`multisample_slots`].
    pub multisamples: Vec<u16>,
    /// Wave group ids of any installed expansion this tone plays from, distinct and in order.
    ///
    /// Like a factory ROM reference, this cannot travel: the destination needs the same expansion
    /// installed. Unlike one, it is easy to forget, so it is worth naming. See [`expansion_banks`].
    pub expansions: Vec<u16>,
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

/// A partial's wave selection, within the 124-byte partial that starts at `0xc8 + 124 * partial`.
///
/// The four fields are consecutive members of the partial — `WAV_GTYPE`, `WAV_GID`, `WAV_NUM_L`,
/// `WAV_NUM_R` in Roland's naming — sitting 23, 24, 26 and 28 bytes in. They are expressed here
/// relative to `BASE = 0xde` because that keeps the arithmetic in one place; the absolute offsets
/// for partial 0 are the familiar `0xdf` group and `0xe2` wave number, plus `0xe4` for the second.
///
/// What a byte-level survey could not show is that there are **two** wave numbers. For a ROM wave
/// they are a stereo pair of waves; for a user sample the panel exposes only the left one, but the
/// right still has to be read, because the instrument's own dependency scan reads it.
/// (A drum kit's `INSa` instrument stores the same four fields as a real 28-byte `WMT` array at
/// `+0x1c`; a tone keeps them inline.)
mod wave {
    /// Where partial 0's wave fields begin, chosen so the offsets below are small.
    pub const BASE: usize = 0xde;
    pub const GROUP_TYPE: usize = 0x01;
    pub const GROUP_ID: usize = 0x02;
    pub const NUMBER_L: usize = 0x04;
    pub const NUMBER_R: usize = 0x06;

    /// The four wave group types, read off a FANTOM-6 panel against tones whose bytes were known.
    ///
    /// | value | panel | what the numbers mean |
    /// |-------|-------|-----------------------|
    /// | 0 | — | an internal ROM wave |
    /// | 1 | `EXP` | a wave in an installed expansion; the group id picks the bank |
    /// | 2 | `SAMP` | a 1-based user sample slot |
    /// | 3 | `MSAMP` | a 1-based user **multisample** slot |
    pub const GROUP_EXPANSION: u8 = 1;
    pub const GROUP_SAMPLE: u8 = 2;
    pub const GROUP_MULTISAMPLE: u8 = 3;
}

/// The user sample slots a `PATa` tone record plays, 1-based, in partial order.
///
/// Each of the four partials selects a wave group and up to two wave numbers. Group 2 means those
/// numbers are `SMPa` slots rather than ROM waves. Confirmed on a FANTOM-6 backup: all 93 group-2
/// partials across its 2048 tones resolve to a populated slot in 1..50, with names matching
/// (`IML Whoa 1` → slot 3 `3 IML Whoa 1`, `Relax Bass` → slots 7 and 8, and so on).
///
/// **Both numbers count, as dependencies.** 25 of that backup's 93 sampled partials name a right
/// slot as well as a left one — `Beat It Gong` holds 1 `1 Beat It - C2` and 22 `doh duh 2`. The
/// panel offers no `Wave No. R` field for a sampled partial, so that second number is not a stereo
/// half a player chose (a sample slot already holds both channels); it is most likely left over
/// from when the partial selected a ROM wave, where `L`/`R` are two waves.
///
/// It still has to be followed, and that is confirmed rather than inferred: a tone this tool rebased
/// to `L=2001, R=2002` was imported to a FANTOM-6 and exported back by the instrument, which carried
/// **both** samples and renumbered them to `L=1, R=2`. Roland's dependency scan reads the right
/// number even though its editor hides it. Reading only the left one drops a sample the FANTOM
/// carries, and renumbering only the left one leaves the right pointing at whatever takes over the
/// old slot. A zero means "none".
///
/// These two numbers, plus the multisample a group-3 partial reaches, are the *whole* set of sample
/// references a tone can hold. A PCM-Sync partial names its wave in `SYNC_WAV_NUM` instead, but that
/// field offers only waves specific to PCM Sync — not user samples, not even ordinary internal ones
/// — so it can never carry a dependency and is deliberately not read.
///
/// This is what makes user samples a *dependency*: the reference is a slot number, so a tone
/// carries no audio with it. The instrument's own scene exports behave the same way — NARF holds
/// 68 such references and no sample areas at all — so samples must be transferred separately.
pub fn sample_slots(record: &[u8]) -> Vec<u16> {
    numbers_for_group(record, wave::GROUP_SAMPLE)
}

/// The user **multisample** slots a tone plays, 1-based, in partial order.
///
/// Group 3 reads `MSAMP` on the panel: `Finesse Rise` partial 1 is group 3 number 1, and the
/// instrument shows it as multisample 1. This is a dependency exactly as a sample is, and one this
/// tool cannot carry — the `MLSa` records defining a multisample are undecoded, and a multisample
/// in turn references samples per key. Reporting it beats the alternative, which is a scene that
/// silently claims to need nothing.
///
/// (In the one fixture that has such a reference the multisample does not exist: every `MLSa`
/// record in that backup is still the factory `INITIAL MSMPL`, so the tone points at nothing. That
/// is a property of the fixture, not of the format.)
pub fn multisample_slots(record: &[u8]) -> Vec<u16> {
    numbers_for_group(record, wave::GROUP_MULTISAMPLE)
}

/// The wave group ids of installed expansions this tone plays from, distinct and in partial order.
///
/// Group 1 reads `EXP`, and the group id selects the bank: a FANTOM-6 showed id 1005 as `EXZ005`
/// and id 1008 as `EXZ006`, so the displayed number is *not* simply the id and the mapping is not
/// decoded. The id is reported raw rather than guessed at.
pub fn expansion_banks(record: &[u8]) -> Vec<u16> {
    let mut banks = Vec::new();
    for partial in 0..PARTIAL_COUNT {
        let base = wave::BASE + partial * PARTIAL_STRIDE;
        let Some(&group) = record.get(base + wave::GROUP_TYPE) else {
            break;
        };
        if group != wave::GROUP_EXPANSION {
            continue;
        }
        // A partial with no wave selected names no bank, whatever its group says.
        let plays = [wave::NUMBER_L, wave::NUMBER_R]
            .iter()
            .any(|&at| read_u16(record, base + at).unwrap_or(0) != 0);
        if let (true, Some(id)) = (plays, read_u16(record, base + wave::GROUP_ID)) {
            if !banks.contains(&id) {
                banks.push(id);
            }
        }
    }
    banks
}

/// Wave numbers named by every partial whose group type is `group`, deduplicated, in order.
fn numbers_for_group(record: &[u8], group: u8) -> Vec<u16> {
    let mut numbers = Vec::new();
    for partial in 0..PARTIAL_COUNT {
        let base = wave::BASE + partial * PARTIAL_STRIDE;
        let Some(&kind) = record.get(base + wave::GROUP_TYPE) else {
            break;
        };
        if kind != group {
            continue;
        }
        for at in [wave::NUMBER_L, wave::NUMBER_R] {
            let Some(number) = read_u16(record, base + at) else {
                break;
            };
            if number != 0 && !numbers.contains(&number) {
                numbers.push(number);
            }
        }
    }
    numbers
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
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
                multisamples: multisample_slots(record),
                expansions: expansion_banks(record),
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
/// Used when repackaging carries a tone's samples along and renumbers them densely, and when
/// repointing a scene bank at the slots its samples will be imported to. Both wave numbers are
/// rewritten, for the reason [`sample_slots`] reads both: a partial can name a slot per channel,
/// and moving only the left one leaves the right pointing at whatever now occupies the old slot.
pub fn remap_sample_slots(record: &mut [u8], remap: &std::collections::BTreeMap<u16, u16>) {
    remap_numbers_for_group(record, wave::GROUP_SAMPLE, remap)
}

/// Rewrite a tone's **multisample** references through `remap` (old number -> new number).
///
/// Needed for the same reason samples are renumbered: an export numbers its multisamples densely
/// from 1, so a tone that named multisample 7 on the instrument must name 1 in a file carrying only
/// that one.
pub fn remap_multisample_slots(record: &mut [u8], remap: &std::collections::BTreeMap<u16, u16>) {
    remap_numbers_for_group(record, wave::GROUP_MULTISAMPLE, remap)
}

fn remap_numbers_for_group(
    record: &mut [u8],
    group: u8,
    remap: &std::collections::BTreeMap<u16, u16>,
) {
    for partial in 0..PARTIAL_COUNT {
        let base = wave::BASE + partial * PARTIAL_STRIDE;
        let Some(&kind) = record.get(base + wave::GROUP_TYPE) else {
            break;
        };
        if kind != group {
            continue;
        }
        for at in [wave::NUMBER_L, wave::NUMBER_R] {
            let Some(number) = read_u16(record, base + at) else {
                break;
            };
            if let Some(&new) = remap.get(&number) {
                record[base + at..base + at + 2].copy_from_slice(&new.to_le_bytes());
            }
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

    /// Build a tone record whose partials use the given `(group, left, right)` triples.
    fn tone_with_partials(partials: &[(u8, u16, u16)]) -> Vec<u8> {
        let mut record =
            vec![0u8; wave::BASE + PARTIAL_COUNT * PARTIAL_STRIDE + wave::NUMBER_R + 2];
        for (partial, &(group, left, right)) in partials.iter().enumerate() {
            let base = wave::BASE + partial * PARTIAL_STRIDE;
            record[base + wave::GROUP_TYPE] = group;
            record[base + wave::NUMBER_L..base + wave::NUMBER_L + 2]
                .copy_from_slice(&left.to_le_bytes());
            record[base + wave::NUMBER_R..base + wave::NUMBER_R + 2]
                .copy_from_slice(&right.to_le_bytes());
        }
        record
    }

    #[test]
    fn reads_the_user_samples_a_tone_plays() {
        // "Relax Bass": partials 1 and 2 play user samples 7 and 8, the rest are ROM waves.
        let record = tone_with_partials(&[(2, 7, 0), (2, 8, 0), (0, 383, 0), (0, 0, 0)]);
        assert_eq!(sample_slots(&record), [7, 8]);

        // A wave number only means a sample when its group says so.
        assert_eq!(
            sample_slots(&tone_with_partials(&[(0, 7, 0)])),
            Vec::<u16>::new()
        );
        // Repeats collapse: four partials layering one sample is still one dependency.
        assert_eq!(sample_slots(&tone_with_partials(&[(2, 5, 0), (2, 5, 0)])), [5]);
        // A record too short to hold the partials yields nothing rather than panicking.
        assert!(sample_slots(&[0u8; 32]).is_empty());
    }

    /// A sampled partial can hold a second wave number the panel never shows. `Beat It Gong` holds
    /// 1 and 22, and the instrument'''s own export follows both — so reading only the left one drops
    /// a sample the FANTOM would have carried.
    #[test]
    fn a_partial_can_name_a_different_sample_per_channel() {
        assert_eq!(sample_slots(&tone_with_partials(&[(2, 1, 22)])), [1, 22]);
        // Zero is "no wave", not slot zero.
        assert_eq!(sample_slots(&tone_with_partials(&[(2, 1, 0)])), [1]);
        // The same slot on both channels is still one dependency.
        assert_eq!(sample_slots(&tone_with_partials(&[(2, 4, 4)])), [4]);
    }

    /// Group 3 is `MSAMP` on the panel — a user multisample, a dependency of its own that must not
    /// be confused with a sample slot or quietly dropped.
    #[test]
    fn a_multisample_reference_is_read_separately_from_a_sample() {
        let record = tone_with_partials(&[(2, 30, 31), (3, 1, 0), (1, 355, 0), (0, 383, 0)]);
        assert_eq!(sample_slots(&record), [30, 31]);
        assert_eq!(multisample_slots(&record), [1]);
        // The ROM partial names no bank, and only the group-1 partial contributes one.
        assert_eq!(expansion_banks(&record), [0]);
    }

    /// A partial whose group says "expansion" but which selects no wave names no bank.
    #[test]
    fn an_empty_partial_contributes_no_expansion() {
        assert!(expansion_banks(&tone_with_partials(&[(1, 0, 0)])).is_empty());
        assert!(multisample_slots(&tone_with_partials(&[(3, 0, 0)])).is_empty());
    }

    #[test]
    fn remapping_moves_both_channels() {
        let mut record = tone_with_partials(&[(2, 1, 22), (0, 383, 384)]);
        let remap = std::collections::BTreeMap::from([(1, 101), (22, 102)]);
        remap_sample_slots(&mut record, &remap);

        assert_eq!(sample_slots(&record), [101, 102]);
        // A ROM partial's stereo pair is not a sample reference and must not move.
        let base = wave::BASE + PARTIAL_STRIDE;
        assert_eq!(read_u16(&record, base + wave::NUMBER_L), Some(383));
        assert_eq!(read_u16(&record, base + wave::NUMBER_R), Some(384));
    }
}
