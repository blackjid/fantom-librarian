//! Where a scene's sequencer patterns live beside its bank.
//!
//! Patterns are not in the SVD. Each scene slot has its own `SeqSet.SQS` under `SEQ/SEQ_SET/`,
//! named by the slot's position, and the file holds nothing that names the slot — so moving a
//! scene means copying its file to the new slot's path. A slot without one has empty patterns.

use std::path::PathBuf;

use crate::{Error, Result};

/// Scene slots a bank can hold: four banks, A–D, of 128.
pub const SLOTS: usize = 512;

const PER_BANK: usize = 128;

/// The pattern file for 0-based scene `index`, relative to the folder holding `FANTOM.SVD`:
/// `SEQ/SEQ_SET/<A–D>/<001–128>/SeqSet.SQS`, as the instrument builds it.
pub fn slot_path(index: usize) -> Option<PathBuf> {
    if index >= SLOTS {
        return None;
    }
    let bank = char::from(b'A' + (index / PER_BANK) as u8);
    Some(
        [
            "SEQ",
            "SEQ_SET",
            &bank.to_string(),
            &format!("{:03}", index % PER_BANK + 1),
        ]
        .iter()
        .collect::<PathBuf>()
        .join("SeqSet.SQS"),
    )
}

const PATTERN_TABLE: usize = 0xe10;
const PATTERN_LEN: usize = 0x30;
const EVENTS: usize = 0x2620;
const EVENT_LEN: usize = 16;
const VARIATIONS: usize = 8;
const FILL: u8 = 0x80;

/// One recorded pattern of a `SeqSet.SQS`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// 1-based track, 1–16.
    pub track: usize,
    /// 1-based variation, 1–8.
    pub variation: usize,
    pub name: String,
    pub bars: u32,
}

/// The patterns of a `SeqSet.SQS` that hold any event; see `docs/FORMAT.md`.
pub fn patterns(file: &[u8]) -> Result<Vec<Pattern>> {
    let bad = |why: &str| Error::Unrecognized(format!("not a SeqSet.SQS: {why}"));
    if !file.starts_with(b"CSQS") {
        return Err(bad("no CSQS magic"));
    }
    let u32_at = |at: usize| {
        file.get(at..at + 4)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
            .ok_or_else(|| bad("truncated"))
    };
    let mut out = Vec::new();
    let mut event = EVENTS;
    for slot in 0..16 * VARIATIONS {
        let entry = PATTERN_TABLE + slot * PATTERN_LEN;
        let count = u32_at(entry)? as usize;
        let events = file
            .get(event..event + count * EVENT_LEN)
            .ok_or_else(|| bad("truncated"))?;
        event += count * EVENT_LEN;
        if events.chunks(EVENT_LEN).all(|e| e[2] == FILL) {
            continue;
        }
        let name = &file[entry + 0x14..entry + 0x24];
        let name = name.split(|&b| b == 0).next().unwrap_or_default();
        out.push(Pattern {
            track: slot / VARIATIONS + 1,
            variation: slot % VARIATIONS + 1,
            name: String::from_utf8_lossy(name).trim_end().to_string(),
            bars: u32_at(entry + 0xc)?,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_fill_bank_a_before_bank_b() {
        let path = |i| slot_path(i).unwrap().to_string_lossy().replace('\\', "/");
        assert_eq!(path(0), "SEQ/SEQ_SET/A/001/SeqSet.SQS");
        assert_eq!(path(127), "SEQ/SEQ_SET/A/128/SeqSet.SQS");
        assert_eq!(path(128), "SEQ/SEQ_SET/B/001/SeqSet.SQS");
        assert_eq!(path(493), "SEQ/SEQ_SET/D/110/SeqSet.SQS");
        assert_eq!(slot_path(SLOTS), None);
    }

    #[test]
    fn patterns_skip_empty_slots_and_fill_events() {
        let mut file = vec![0u8; EVENTS];
        file[..4].copy_from_slice(b"CSQS");
        let mut entry = |slot: usize, count: u32, bars: u32, name: &[u8]| {
            let at = PATTERN_TABLE + slot * PATTERN_LEN;
            file[at..at + 4].copy_from_slice(&count.to_be_bytes());
            file[at + 0xc..at + 0x10].copy_from_slice(&bars.to_be_bytes());
            file[at + 0x14..at + 0x14 + name.len()].copy_from_slice(name);
        };
        entry(0, 1, 4, b"FILL ONLY");
        entry(9, 2, 2, b"Verse   ");
        let mut events = [0u8; 3 * EVENT_LEN];
        events[2] = FILL; // slot 0's only event
        events[EVENT_LEN + 2] = 60; // a note in slot 9
        file.extend_from_slice(&events);
        assert_eq!(
            patterns(&file).unwrap(),
            [Pattern {
                track: 2,
                variation: 2,
                name: "Verse".into(),
                bars: 2,
            }]
        );
        assert!(patterns(b"RIFF").is_err());
    }
}
