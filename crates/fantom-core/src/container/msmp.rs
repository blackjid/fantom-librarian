//! User multisamples — `MLSa` in a backup, `MSPa` in an SVZ export.
//!
//! A multisample maps each of the 128 MIDI keys to a user sample, which is how one sound spans a
//! keyboard from several recordings. The record is 1040 bytes: a 16-byte name, then **128 entries
//! of 8 bytes**, one per key, in key order.
//!
//! | Off | Size | Field |
//! |-----|------|-------|
//! | `0x00` | 2 | user sample slot, 1-based; `0` means the key plays nothing |
//! | `0x02` | 2 | level (`127` throughout every record seen) |
//! | `0x04` | 2 | pan (`128` = centre) |
//! | `0x06` | 2 | unknown, `0` everywhere |
//!
//! Confirmed by capture. A FANTOM-6 multisample built from three samples across three key ranges
//! reads back as exactly that:
//!
//! ```text
//! T8_MSAMP   keys   0..45  -> slot 2003
//!            keys  46..76  -> slot 2005
//!            keys  77..127 -> slot 2018
//! ```
//!
//! and the tone export of a tone playing it carries an `MSPa` holding the same record with those
//! slots renumbered densely to 3, 4 and 5 — the same treatment samples get. The factory
//! `INITIAL MSMPL` default is this structure with slot `0` on every key, which is why it reads as
//! "no sample" rather than as a sample reference.
//!
//! This closes the transitive dependency a tone can have: a `PATa` partial names a multisample by
//! number (wave group 3, `MSAMP` on the panel), and the multisample names samples of its own.

use std::collections::BTreeMap;

/// A multisample record's name field, then one entry per MIDI key.
const NAME_LEN: usize = 16;
const KEYS: usize = 128;
const ENTRY_LEN: usize = 8;
/// Within an entry.
const SLOT: usize = 0x00;

/// What one key of a multisample plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyMap {
    /// MIDI key, 0..128.
    pub key: u8,
    /// 1-based user sample slot. Never zero — silent keys are omitted.
    pub slot: u16,
    pub level: u16,
    /// 128 is centre.
    pub pan: u16,
}

/// Every key of a multisample that plays something, in key order.
pub fn key_map(record: &[u8]) -> Vec<KeyMap> {
    let mut out = Vec::new();
    for key in 0..KEYS {
        let at = NAME_LEN + key * ENTRY_LEN;
        let Some(entry) = record.get(at..at + ENTRY_LEN) else {
            break;
        };
        let slot = le_u16(entry, SLOT);
        if slot == 0 {
            continue;
        }
        out.push(KeyMap {
            key: key as u8,
            slot,
            level: le_u16(entry, 0x02),
            pan: le_u16(entry, 0x04),
        });
    }
    out
}

/// The user sample slots a multisample plays, 1-based, deduplicated in key order.
pub fn sample_slots(record: &[u8]) -> Vec<u16> {
    let mut slots = Vec::new();
    for entry in key_map(record) {
        if !slots.contains(&entry.slot) {
            slots.push(entry.slot);
        }
    }
    slots
}

/// Rewrite a multisample's sample references through `remap` (old slot -> new slot).
pub fn remap_sample_slots(record: &mut [u8], remap: &BTreeMap<u16, u16>) {
    for key in 0..KEYS {
        let at = NAME_LEN + key * ENTRY_LEN + SLOT;
        let Some(slot) = record.get(at..at + 2).map(|b| le_u16(b, 0)) else {
            break;
        };
        if let Some(&new) = remap.get(&slot) {
            record[at..at + 2].copy_from_slice(&new.to_le_bytes());
        }
    }
}

fn le_u16(bytes: &[u8], at: usize) -> u16 {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A multisample mapping `ranges` of keys onto sample slots, the rest silent.
    fn multisample(name: &str, ranges: &[(u8, u8, u16)]) -> Vec<u8> {
        let mut record = vec![0u8; NAME_LEN + KEYS * ENTRY_LEN];
        record[..NAME_LEN].fill(b' ');
        record[..name.len()].copy_from_slice(name.as_bytes());
        for key in 0..KEYS {
            let at = NAME_LEN + key * ENTRY_LEN;
            let slot = ranges
                .iter()
                .find(|(lo, hi, _)| (*lo as usize..=*hi as usize).contains(&key))
                .map(|(_, _, slot)| *slot)
                .unwrap_or(0);
            record[at..at + 2].copy_from_slice(&slot.to_le_bytes());
            record[at + 2..at + 4].copy_from_slice(&127u16.to_le_bytes());
            record[at + 4..at + 6].copy_from_slice(&128u16.to_le_bytes());
        }
        record
    }

    /// The captured `T8_MSAMP`: three samples across three key ranges.
    #[test]
    fn reads_the_key_ranges_of_a_multisample() {
        let record = multisample("T8_MSAMP", &[(0, 45, 2003), (46, 76, 2005), (77, 127, 2018)]);
        assert_eq!(sample_slots(&record), [2003, 2005, 2018]);

        let map = key_map(&record);
        assert_eq!(map.len(), 128, "every key plays something here");
        assert_eq!(map[0].slot, 2003);
        assert_eq!(map[46].slot, 2005);
        assert_eq!(map[127].slot, 2018);
        assert_eq!(map[0].level, 127);
        assert_eq!(map[0].pan, 128, "centre");
    }

    /// The factory default is this structure with slot 0 everywhere — no sample, not sample zero.
    #[test]
    fn a_factory_default_multisample_names_no_samples() {
        let record = multisample("INITIAL MSMPL", &[]);
        assert!(sample_slots(&record).is_empty());
        assert!(key_map(&record).is_empty());
    }

    #[test]
    fn remapping_moves_every_key_that_names_a_moved_slot() {
        let mut record = multisample("T8_MSAMP", &[(0, 45, 2003), (46, 127, 2005)]);
        remap_sample_slots(&mut record, &BTreeMap::from([(2003, 3), (2005, 4)]));

        assert_eq!(sample_slots(&record), [3, 4]);
        assert_eq!(key_map(&record)[0].slot, 3);
        assert_eq!(key_map(&record)[46].slot, 4);
    }

    #[test]
    fn a_record_too_short_yields_nothing() {
        assert!(sample_slots(&[0u8; 8]).is_empty());
    }
}
