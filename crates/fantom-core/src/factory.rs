//! Every sound the instrument itself ships with.
//!
//! These live in ROM, not in any file: a scene can only ever *point* at them, which is why a
//! library built from files alone shows a JUNO-106 bank it can never list. The lists Roland
//! publishes are the missing half, and this is them — the ZEN-Core presets from
//! [`crate::presets`], and every other built-in bank from `factory_sounds.tsv`, extracted from the
//! *FANTOM Sound List* by `tools/gen_sound_list.py`.
//!
//! Engine and bank are not stored: [`ToneRef`] derives both from the address, so one taxonomy
//! serves a zone's reference and a catalogue entry alike.
//!
//! What is here is the base instrument. A model or wave expansion publishes its own sound list;
//! until one is added, its banks are named by the scenes that reach for them and nothing more.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::model::{ToneAddress, ToneRef, ToneType};
use crate::presets;

/// One built-in sound, at the address a zone would select it by.
///
/// Borrowed from whichever list it came from: the bundled tables live for the program, one read
/// off disk lives as long as its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactorySound<'a> {
    pub address: ToneAddress,
    /// Number within its bank, as the panel shows it.
    pub number: u16,
    pub name: &'a str,
    /// Roland's category, e.g. `35:Synth Brass`. Empty where the list gives none.
    pub category: &'a str,
}

impl FactorySound<'_> {
    /// The engine that plays it.
    pub fn engine(&self) -> ToneType {
        self.tone_ref().tone_type()
    }

    /// The bank it sits in — `PR-A`, `CMN`, `PRST`, `JP8`.
    pub fn bank(&self) -> Option<&'static str> {
        self.tone_ref().bank()
    }

    fn tone_ref(&self) -> ToneRef {
        ToneRef::new(self.address.msb, self.address.lsb, self.address.pc, None)
    }
}

const TABLE_TSV: &str = include_str!("factory_sounds.tsv");
/// Every built-in sound this build carries: ZEN-Core presets first, then the other banks.
pub fn all() -> impl Iterator<Item = FactorySound<'static>> {
    presets::all().chain(parse(TABLE_TSV))
}

/// The built-in sound at an address, when one of the bundled lists names it.
///
/// Keyed by the whole address rather than by the 16-bit id a ZEN-Core zone stores, which is what
/// lets the drum kits in — MSB 86 and MSB 87 share that id space, so [`crate::presets`] leaves
/// kits out to avoid naming a tone as a kit. Nothing shares an MSB/LSB/PC.
pub fn lookup(address: ToneAddress) -> Option<FactorySound<'static>> {
    static INDEX: OnceLock<HashMap<(u8, u8, u8), FactorySound<'static>>> = OnceLock::new();
    INDEX
        .get_or_init(|| {
            all()
                .map(|sound| {
                    (
                        (sound.address.msb, sound.address.lsb, sound.address.pc),
                        sound,
                    )
                })
                .collect()
        })
        .get(&(address.msb, address.lsb, address.pc))
        .copied()
}

/// Read a sound list in the shape `tools/gen_sound_list.py` and `dump-sounds` both write:
/// `msb`, `lsb`, `pc`, `number`, `name`, `category`, tab separated, one header line.
///
/// This is how an expansion's sounds reach a library. The bundled tables are the base instrument;
/// what a particular FANTOM has installed on top of it is a fact about that instrument, so it
/// arrives as a file rather than being compiled in.
pub fn parse(text: &str) -> impl Iterator<Item = FactorySound<'_>> {
    text.lines().skip(1).filter_map(|line| {
        let mut field = line.split('\t');
        let msb = field.next()?.parse().ok()?;
        let lsb = field.next()?.parse().ok()?;
        // Both lists print `PC` counting from one; a zone stores it counting from zero.
        let pc = field.next()?.parse::<u8>().ok()?.checked_sub(1)?;
        let number = field.next()?.parse().unwrap_or(0);
        let name = field.next()?.trim();
        // A blank slot is not a sound.
        (!name.is_empty()).then(|| FactorySound {
            address: ToneAddress { msb, lsb, pc },
            number,
            name,
            category: field.next().unwrap_or("").trim(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn every_built_in_bank_is_one_this_version_can_name() {
        // A row whose address no engine claims would show up in a library as `Unknown` with no
        // bank — the one outcome worth failing the build over.
        for sound in all() {
            assert_ne!(sound.engine(), ToneType::Unknown, "{}", sound.name);
            assert!(sound.bank().is_some(), "{}", sound.name);
        }
    }

    #[test]
    fn the_banks_the_sound_list_covers() {
        let mut banks: Vec<(ToneType, &str)> = Vec::new();
        for sound in parse(TABLE_TSV) {
            let entry = (sound.engine(), sound.bank().unwrap());
            if !banks.contains(&entry) {
                banks.push(entry);
            }
        }
        assert_eq!(
            banks,
            [
                (ToneType::Drum, "PR-A"),
                (ToneType::Drum, "CMN"),
                (ToneType::SnA, "PRST"),
                (ToneType::VPiano, "PRST"),
                // Roland prints no VTW list; these came off a FANTOM-6 over SysEx.
                (ToneType::Vtw, "PRST"),
                (ToneType::Acb, "JP8"),
            ]
        );
    }

    /// A bank prints as a run of consecutive programs, so a hole in one is a row the sound-list
    /// extraction dropped rather than a slot Roland left empty — the way `TR-808` went missing from
    /// `CMN` and turned a scene's drum zone unnameable. The expansion catalogs are deliberately not
    /// checked this way: those are swept from an instrument and may genuinely be part-captured.
    #[test]
    fn no_bank_is_missing_a_program_in_the_middle() {
        let mut banks: BTreeMap<(u8, u8), Vec<u8>> = BTreeMap::new();
        for sound in all() {
            banks
                .entry((sound.address.msb, sound.address.lsb))
                .or_default()
                .push(sound.address.pc);
        }
        for (bank, mut programs) in banks {
            programs.sort_unstable();
            let span = *programs.last().unwrap() - programs[0] + 1;
            assert_eq!(
                programs.len(),
                span as usize,
                "{}/{} skips a program",
                bank.0,
                bank.1
            );
        }
    }

    /// `Soft & Subtle` is the JUPITER-8 tone every ACB record in the fixtures is a copy of, and the
    /// evidence behind `model_label(Acb, 4102)`.
    #[test]
    fn the_jupiter_8_bank_opens_where_the_acb_records_point() {
        let first = parse(TABLE_TSV)
            .find(|sound| sound.address.msb == 107)
            .expect("an ACB bank");
        assert_eq!(first.name, "Soft & Subtle");
        assert_eq!(first.bank(), Some("JP8"));
    }
}
