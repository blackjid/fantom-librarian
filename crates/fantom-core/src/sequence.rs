//! Where a scene's sequencer patterns live beside its bank.
//!
//! Patterns are not in the SVD. Each scene slot has its own `SeqSet.SQS` under `SEQ/SEQ_SET/`,
//! named by the slot's position, and the file holds nothing that names the slot — so moving a
//! scene means copying its file to the new slot's path. A slot without one has empty patterns.

use std::path::PathBuf;

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
}
