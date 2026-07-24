//! Factory ZEN-Core **preset** tone lookup.
//!
//! Scene zones reference factory presets by a 16-bit id of the form `(LSB << 8) | (PC - 1)` (MSB is
//! always 87 for ZEN-Core tones). This module maps that id back to the tone's bank, number, name,
//! and category using a table extracted from Roland's *FANTOM Sound List* (`preset_tones.tsv`).
//!
//! Only ZEN-Core tones (MSB 87) are included; drum kits (MSB 86) share the same 16-bit id space and
//! cannot be told apart from a tone by id alone, so they are omitted to avoid wrong names.

use std::collections::HashMap;
use std::sync::OnceLock;

/// A factory preset tone from the sound list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetTone {
    /// Bank label, e.g. `"PR-A"`.
    pub bank: &'static str,
    /// 1-based number within the bank, e.g. `61`.
    pub number: u16,
    /// Tone name, e.g. `"JX Cream"`.
    pub name: &'static str,
    /// Category, e.g. `"35:Synth Brass"` (may be empty).
    pub category: &'static str,
}

const TABLE_TSV: &str = include_str!("preset_tones.tsv");

fn table() -> &'static HashMap<u16, PresetTone> {
    static TABLE: OnceLock<HashMap<u16, PresetTone>> = OnceLock::new();
    TABLE.get_or_init(|| {
        TABLE_TSV
            .lines()
            .skip(1) // header row
            .filter_map(|line| {
                let mut f = line.split('\t');
                let id: u16 = f.next()?.parse().ok()?;
                let bank = f.next()?;
                let number: u16 = f.next()?.parse().ok()?;
                let name = f.next()?;
                let category = f.next().unwrap_or("");
                Some((id, PresetTone { bank, number, name, category }))
            })
            .collect()
    })
}

/// Look up a factory preset tone by its 16-bit scene reference id.
pub fn lookup(tone_id: u16) -> Option<&'static PresetTone> {
    table().get(&tone_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_presets() {
        // JX Cream = PR-A 0061 = (92<<8)|(61-1).
        let jx = lookup(0x5c3c).unwrap();
        assert_eq!((jx.bank, jx.number, jx.name), ("PR-A", 61, "JX Cream"));
        // PR-B 0001 has the 0x4000 flag set (LSB 64).
        assert_eq!(lookup(0x4000).unwrap().name, "AX Classic Lead");
    }

    #[test]
    fn whole_table_loads() {
        assert!(table().len() > 3000);
    }
}
