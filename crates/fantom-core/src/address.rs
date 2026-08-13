//! Which SVD area a zone's tone address points into, and at which record.
//!
//! A zone stores its tone as a MIDI bank/program tuple — `MSB`, `LSB`, `PC`. The MSB selects the
//! sound engine, and for user sounds the LSB/PC pair is a plain index into that engine's area:
//!
//! ```text
//! index = (LSB - first_lsb) * 128 + PC        for LSB < 64
//! ```
//!
//! **`LSB >= 64` is a factory ROM bank** — those sounds live in the instrument, never in the file,
//! and resolve through [`crate::presets`] instead (ZEN-Core) or not at all.
//!
//! The same rule holds in scene exports *and* full backups. An export bundles only the tones its
//! scenes reference and renumbers them densely; a backup holds the entire USER bank at fixed slots
//! (`PATa` 2048, `SNAa` 256, `RHYa`/`VTWa`/`ZAPa`/`ZEPa`/`DCWa` 128 each, `MDLa` 1024). Both are
//! indexed identically — verified by matching scenes by name across three export/backup pairs and
//! comparing the resolved tone name on both sides: 582 of 582 references agree. See `docs/FORMAT.md`.
//!
//! This table is the single source of truth for that mapping. Reading ([`crate::codec`]) and
//! repackaging ([`crate::repackage`]) both consult it, so they cannot drift apart.

use crate::model::ToneType;
use crate::{Error, Result};

/// Program-change values per LSB page.
pub const PC_PER_PAGE: usize = 128;

/// The first LSB of the factory ROM banks. User banks are below it.
pub const FIRST_PRESET_LSB: u8 = 64;

/// One engine's user bank: the area that stores it and how its records are addressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaSpec {
    /// The area holding the records.
    pub tag: [u8; 4],
    /// Areas indexed in lockstep with `tag`, `tag` first. A drum kit is `RHYa[i]` *plus* the 88
    /// instruments in `INSa[i]`, so the pair can only ever be copied and renumbered together.
    pub paired: &'static [&'static [u8; 4]],
    /// Engine this bank belongs to.
    pub tone_type: ToneType,
    /// Bank Select MSB selecting this engine.
    pub msb: u8,
    /// Lowest LSB of the user bank (1 for `ZEPa`, which shares MSB 105 with `ZAPa`).
    pub lsb_first: u8,
    /// Highest LSB of the user bank.
    pub lsb_last: u8,
    /// Offset of the 16-byte name within a record.
    pub name_offset: usize,
    /// Whether the name is stored as four byte-reversed 4-byte words (`ACBa`).
    pub word_swapped: bool,
    /// Record internals are undecoded: copy verbatim, never interpret beyond the name.
    pub opaque: bool,
    /// Whether we can tell which user samples a record of this engine plays.
    ///
    /// Two engines qualify. A `PATa` tone carries a confirmed wave group and slot number on each of
    /// its four partials; a drum kit carries the same fields in its paired `INSa`, four wave blocks
    /// per instrument, both marked by group value `2` (see [`crate::container::sample_slots_of`]).
    /// Every other engine stores its waves somewhere still undecoded, so repackaging must not
    /// *select* samples for them — see [`crate::tonebank`], which carries all of them instead.
    pub sample_refs_decoded: bool,
}

/// Length of a record's name field.
pub const NAME_LEN: usize = 16;

impl AreaSpec {
    /// The area tag as a string, for messages.
    pub fn tag_str(&self) -> String {
        String::from_utf8_lossy(&self.tag).into_owned()
    }

    /// Decode a record's 16-byte name field.
    pub fn decode_name(&self, field: &[u8]) -> String {
        crate::container::ascii_trim(&self.name_bytes(field))
    }

    /// Encode `name` into a record's 16-byte name field, space-padded.
    pub fn encode_name(&self, name: &str) -> [u8; NAME_LEN] {
        let mut field = [b' '; NAME_LEN];
        let src = name.as_bytes();
        let n = src.len().min(NAME_LEN);
        field[..n].copy_from_slice(&src[..n]);
        self.name_bytes(&field)
    }

    /// Apply the area's name byte order. `ACBa` stores each 4-byte word reversed, so the same
    /// transform both decodes and encodes it.
    fn name_bytes(&self, field: &[u8]) -> [u8; NAME_LEN] {
        let mut out = [b' '; NAME_LEN];
        let n = field.len().min(NAME_LEN);
        out[..n].copy_from_slice(&field[..n]);
        if self.word_swapped {
            for word in out.chunks_exact_mut(4) {
                word.reverse();
            }
        }
        out
    }

    /// How many records this bank can address before it would collide with the next engine.
    pub fn capacity(&self) -> usize {
        (self.lsb_last - self.lsb_first + 1) as usize * PC_PER_PAGE
    }

    /// Encode a record index back into the `(LSB, PC)` a zone stores.
    pub fn encode(&self, index: usize) -> Result<(u8, u8)> {
        if index >= self.capacity() {
            return Err(Error::Unrecognized(format!(
                "too many {} records to encode ({} maximum)",
                self.tag_str(),
                self.capacity()
            )));
        }
        Ok((
            self.lsb_first + (index / PC_PER_PAGE) as u8,
            (index % PC_PER_PAGE) as u8,
        ))
    }
}

/// The common case: name at the start of the record, plain ASCII, records we understand, and a
/// user bank spanning the whole `LSB < 64` range. Each entry below overrides what differs.
const DEFAULT: AreaSpec = AreaSpec {
    tag: *b"    ",
    paired: &[],
    tone_type: ToneType::Unknown,
    msb: 0,
    lsb_first: 0,
    lsb_last: 63,
    name_offset: 0,
    word_swapped: false,
    opaque: false,
    sample_refs_decoded: false,
};

/// Every user bank we can locate in a file, in the order areas appear on disk.
///
/// `MDLa` and `ACBa` keep their names at a non-zero offset, and `ACBa` stores them word-reversed —
/// both confirmed by single-byte rename captures (`TONEMAP9_*` pairs). The three modelled engines
/// are `opaque`: their records are copied and renumbered but never interpreted beyond the name.
pub const AREAS: [AreaSpec; 9] = [
    AreaSpec {
        tag: *b"PATa",
        paired: &[b"PATa"],
        tone_type: ToneType::ZenCore,
        msb: 87,
        sample_refs_decoded: true,
        ..DEFAULT
    },
    AreaSpec {
        tag: *b"RHYa",
        // A drum kit is the RHYa record plus its 88 instruments in INSa, indexed together.
        paired: &[b"RHYa", b"INSa"],
        tone_type: ToneType::Drum,
        msb: 86,
        // The references live in the paired INSa, not in the kit record itself.
        sample_refs_decoded: true,
        ..DEFAULT
    },
    AreaSpec {
        tag: *b"VTWa",
        paired: &[b"VTWa"],
        tone_type: ToneType::Vtw,
        msb: 91,
        ..DEFAULT
    },
    AreaSpec {
        tag: *b"SNAa",
        paired: &[b"SNAa"],
        tone_type: ToneType::SnA,
        msb: 89,
        ..DEFAULT
    },
    // MSB 105 holds two engines side by side, one LSB page each.
    AreaSpec {
        tag: *b"ZAPa",
        paired: &[b"ZAPa"],
        tone_type: ToneType::SnAp,
        msb: 105,
        lsb_first: 0,
        lsb_last: 0,
        ..DEFAULT
    },
    AreaSpec {
        tag: *b"ZEPa",
        paired: &[b"ZEPa"],
        tone_type: ToneType::SnEp,
        msb: 105,
        lsb_first: 1,
        lsb_last: 1,
        ..DEFAULT
    },
    AreaSpec {
        tag: *b"ACBa",
        paired: &[b"ACBa"],
        tone_type: ToneType::Acb,
        msb: 107,
        name_offset: 0x1c44,
        word_swapped: true,
        opaque: true,
        ..DEFAULT
    },
    AreaSpec {
        tag: *b"DCWa",
        paired: &[b"DCWa"],
        tone_type: ToneType::VPiano,
        msb: 90,
        opaque: true,
        ..DEFAULT
    },
    AreaSpec {
        tag: *b"MDLa",
        paired: &[b"MDLa"],
        tone_type: ToneType::Model,
        msb: 97,
        name_offset: 0x10,
        opaque: true,
        ..DEFAULT
    },
];

/// The bank a `(MSB, LSB)` pair selects, or `None` for factory ROM and unknown engines.
pub fn spec(msb: u8, lsb: u8) -> Option<&'static AreaSpec> {
    if lsb >= FIRST_PRESET_LSB {
        return None;
    }
    AREAS
        .iter()
        .find(|spec| spec.msb == msb && lsb >= spec.lsb_first && lsb <= spec.lsb_last)
}

/// Resolve a zone's tone address to the area and record index it points at.
///
/// `None` means the address is not a user record in this file: a factory ROM bank, or an engine
/// whose storage we have not confirmed. Callers must show such addresses raw rather than guess.
pub fn resolve(msb: u8, lsb: u8, pc: u8) -> Option<(&'static AreaSpec, usize)> {
    let spec = spec(msb, lsb)?;
    let index = (lsb - spec.lsb_first) as usize * PC_PER_PAGE + pc as usize;
    Some((spec, index))
}

/// The bank stored in a given area, if we know that area.
pub fn spec_for_tag(tag: &[u8; 4]) -> Option<&'static AreaSpec> {
    AREAS.iter().find(|spec| &spec.tag == tag)
}

/// Every area tag that holds user records, including paired ones such as `INSa`.
pub fn dependency_tags() -> impl Iterator<Item = [u8; 4]> {
    AREAS
        .iter()
        .flat_map(|spec| spec.paired.iter().map(|tag| **tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_references_index_their_area_directly() {
        // The rule that holds in exports and backups alike.
        let (spec, index) = resolve(87, 0, 5).unwrap();
        assert_eq!(spec.tag, *b"PATa");
        assert_eq!(index, 5);

        // Panel-confirmed: NARF "Africa Main" zone 1 is lsb 3 / pc 59 -> PATa[443] "Africa Brass".
        assert_eq!(resolve(87, 3, 59).unwrap().1, 443);
        // NARF "Sledgehammer" zone 1 is lsb 4 / pc 34 -> PATa[546] "Sledgehammer Sha".
        assert_eq!(resolve(87, 4, 34).unwrap().1, 546);
    }

    #[test]
    fn factory_banks_are_not_records_in_the_file() {
        // LSB >= 64 is ROM: PR-A 0061 "JX Cream" lives in the instrument, not the export.
        assert!(resolve(87, 92, 60).is_none());
        assert!(spec(87, FIRST_PRESET_LSB).is_none());
    }

    #[test]
    fn the_two_engines_sharing_msb_105_are_told_apart_by_lsb() {
        assert_eq!(resolve(105, 0, 3).unwrap().0.tag, *b"ZAPa");
        assert_eq!(resolve(105, 0, 3).unwrap().1, 3);
        assert_eq!(resolve(105, 1, 3).unwrap().0.tag, *b"ZEPa");
        // ZEPa's first LSB is 1, so its page starts at index 0 again.
        assert_eq!(resolve(105, 1, 3).unwrap().1, 3);
    }

    #[test]
    fn encoding_round_trips_through_lsb_pages() {
        let pat = spec_for_tag(b"PATa").unwrap();
        assert_eq!(pat.encode(0).unwrap(), (0, 0));
        assert_eq!(pat.encode(127).unwrap(), (0, 127));
        assert_eq!(pat.encode(128).unwrap(), (1, 0));
        assert_eq!(pat.encode(546).unwrap(), (4, 34));
        for index in [0usize, 1, 127, 128, 546, 2047] {
            let (lsb, pc) = pat.encode(index).unwrap();
            assert_eq!(resolve(87, lsb, pc).unwrap().1, index);
        }
    }

    /// ZAPa and ZEPa are adjacent in one MSB, so neither may spill into the other's page.
    #[test]
    fn engines_sharing_an_msb_cannot_overflow_into_each_other() {
        let zap = spec_for_tag(b"ZAPa").unwrap();
        assert_eq!(zap.capacity(), 128);
        assert_eq!(zap.encode(127).unwrap(), (0, 127));
        assert!(zap.encode(128).is_err());
    }

    /// A backup's MDLa holds 1024 records — more than one LSB page.
    #[test]
    fn opaque_banks_address_a_full_backups_worth_of_records() {
        let mdl = spec_for_tag(b"MDLa").unwrap();
        assert!(mdl.opaque);
        assert_eq!(mdl.encode(1023).unwrap(), (7, 127));
        assert_eq!(resolve(97, 7, 127).unwrap().1, 1023);
    }

    /// `ACBa` names are stored as four byte-reversed 4-byte words — confirmed by the
    /// `TONEMAP9_ACB` / `TONEMAP9_ACB2` rename pair, which differ in exactly one byte.
    #[test]
    fn word_swapped_names_round_trip() {
        // The exact 16 bytes at ACBa[0]+0x1c44 in fixtures/TONEMAP9_ACB.
        let acb = spec_for_tag(b"ACBa").unwrap();
        assert_eq!(acb.decode_name(b"tfoSS & ltbu  2e"), "Soft & Subtle2");
        assert_eq!(&acb.encode_name("Soft & Subtle2"), b"tfoSS & ltbu  2e");
        assert_eq!(acb.decode_name(&acb.encode_name("Soft & Subtle3")), "Soft & Subtle3");

        // A plain area is unchanged by both directions.
        let pat = spec_for_tag(b"PATa").unwrap();
        assert_eq!(pat.decode_name(b"Africa Brass    "), "Africa Brass");
        assert_eq!(&pat.encode_name("Africa Brass"), b"Africa Brass    ");
    }

    #[test]
    fn drum_kits_carry_their_instrument_area_along() {
        let rhy = spec_for_tag(b"RHYa").unwrap();
        assert_eq!(rhy.paired, &[b"RHYa", b"INSa"]);
        assert!(dependency_tags().any(|tag| tag == *b"INSa"));
    }
}
