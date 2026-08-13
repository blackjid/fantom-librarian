//! `INSa` — the 88 key instruments of a drum kit, and the user samples they play.
//!
//! An `INSa` record is the payload of the `RHYa` kit at the same index: **88 sub-records of 216
//! bytes**, one per key. Each holds four wave blocks at stride 28 from `+0x1c` — the structure
//! Roland's editor schema calls `INST_CMN.WMT[4]`, the same four fields a `PATa` partial keeps
//! inline (see [`crate::container::sample_slots`]).
//!
//! For a long time the group byte was `0` in all 45,056 blocks across four files, so what value
//! meant "user sample" was unknown and a drum bank had to carry *every* sample it held rather than
//! selecting. A capture settled it: a FANTOM-6 drum kit was exported, one key's instrument pointed
//! at a user sample, and the kit exported again. The whole difference was five bytes.
//!
//! ```text
//! fantom diff DRUM_BEFORE.svz DRUM_AFTER.svz --area INSa
//!   INSa[0]+0x0cc5   00 08 00 4c 02 -> 02 14 27 01 00
//! ```
//!
//! That is instrument 15's first wave block: group type `0` → **`2`**, group id `8` → `10004`, and
//! wave number `588` → `1`. The group value is the same `2` a tone uses, and the export carried the
//! sample with it — `USPa` and `USDa` appeared in the file, holding `doh duh 2`.

use std::collections::BTreeMap;

/// One key's instrument within an `INSa` record.
const INSTRUMENT_LEN: usize = 216;

/// Where an instrument's four wave blocks start, and how far apart they sit.
const WAVE_BASE: usize = 0x1c;
const WAVE_STRIDE: usize = 28;
const WAVE_COUNT: usize = 4;
/// Within a wave block: the group type, then the two wave numbers.
const GROUP_TYPE: usize = 0x01;
const NUMBER_L: usize = 0x04;
const NUMBER_R: usize = 0x06;
/// "These numbers are user sample slots" — the same value a `PATa` partial uses.
const GROUP_SAMPLE: u8 = 2;

/// Byte offsets of every wave number in an `INSa` record, paired with its block's group type.
fn wave_fields(record: &[u8]) -> impl Iterator<Item = (usize, usize)> + '_ {
    let instruments = record.len() / INSTRUMENT_LEN;
    (0..instruments).flat_map(move |instrument| {
        let instrument = instrument * INSTRUMENT_LEN;
        (0..WAVE_COUNT).flat_map(move |block| {
            let base = instrument + WAVE_BASE + block * WAVE_STRIDE;
            [NUMBER_L, NUMBER_R].map(move |at| (base + GROUP_TYPE, base + at))
        })
    })
}

/// The user sample slots a drum kit's instruments play, 1-based, deduplicated in key order.
///
/// `record` is one whole `INSa` record — all 88 instruments. Empty for a kit that plays only ROM
/// waves, which is every kit in every fixture predating the capture described above.
pub fn sample_slots(record: &[u8]) -> Vec<u16> {
    let mut slots = Vec::new();
    for (group, number) in wave_fields(record) {
        if record.get(group) != Some(&GROUP_SAMPLE) {
            continue;
        }
        let Some(slot) = read_u16(record, number) else {
            continue;
        };
        if slot != 0 && !slots.contains(&slot) {
            slots.push(slot);
        }
    }
    slots
}

/// Rewrite a drum kit's user-sample references through `remap` (old slot -> new slot).
pub fn remap_sample_slots(record: &mut [u8], remap: &BTreeMap<u16, u16>) {
    let fields: Vec<(usize, usize)> = wave_fields(record).collect();
    for (group, number) in fields {
        if record.get(group) != Some(&GROUP_SAMPLE) {
            continue;
        }
        let Some(slot) = read_u16(record, number) else {
            continue;
        };
        if let Some(&new) = remap.get(&slot) {
            record[number..number + 2].copy_from_slice(&new.to_le_bytes());
        }
    }
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An `INSa` record whose instrument `key` plays `slot` on wave block 1.
    fn kit(entries: &[(usize, u8, u16)]) -> Vec<u8> {
        let mut record = vec![0u8; 88 * INSTRUMENT_LEN];
        for &(key, group, slot) in entries {
            let base = key * INSTRUMENT_LEN + WAVE_BASE;
            record[base + GROUP_TYPE] = group;
            record[base + NUMBER_L..base + NUMBER_L + 2].copy_from_slice(&slot.to_le_bytes());
        }
        record
    }

    #[test]
    fn reads_the_samples_a_kit_plays() {
        // Instrument 15 pointing at slot 1 is exactly what the capture produced.
        assert_eq!(sample_slots(&kit(&[(15, 2, 1)])), [1]);
        // Several keys, deduplicated, in key order.
        assert_eq!(
            sample_slots(&kit(&[(40, 2, 9), (2, 2, 4), (60, 2, 9)])),
            [4, 9]
        );
    }

    #[test]
    fn a_rom_wave_is_not_a_sample() {
        // Group 0 with wave 588 is the ROM wave the captured instrument played before the edit.
        assert!(sample_slots(&kit(&[(15, 0, 588)])).is_empty());
        // Nor is a group-2 block that selects no wave.
        assert!(sample_slots(&kit(&[(15, 2, 0)])).is_empty());
    }

    #[test]
    fn remapping_moves_only_sample_references() {
        let mut record = kit(&[(15, 2, 1), (16, 0, 1)]);
        remap_sample_slots(&mut record, &BTreeMap::from([(1, 2001)]));

        assert_eq!(sample_slots(&record), [2001]);
        // The ROM wave next door names wave 1 and must not move.
        let rom = 16 * INSTRUMENT_LEN + WAVE_BASE + NUMBER_L;
        assert_eq!(read_u16(&record, rom), Some(1));
    }

    #[test]
    fn a_record_too_short_for_an_instrument_yields_nothing() {
        assert!(sample_slots(&[0u8; 32]).is_empty());
    }
}
