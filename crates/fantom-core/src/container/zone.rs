use binrw::binread;

/// One entry of the scene record's **zone table** (record-relative `0x6d0`, 16 × 96 bytes).
///
/// Offsets confirmed by controlled single-variable edits (`fixtures/tests/TEST 1..3`); see
/// `docs/FORMAT.md`. This carries the zone's switch and key range; its `level` lives in a separate
/// settings table (see [`ZoneSettings`]).
#[binread]
#[derive(Debug, Clone, PartialEq)]
#[br(little)]
pub struct RawZone {
    _a: [u8; 4],
    /// +0x04 — zone on/off.
    pub enable: u8,
    _b: [u8; 3],
    /// +0x08 — key-range lower (MIDI note).
    pub key_low: u8,
    /// +0x09 — key-range upper (MIDI note).
    pub key_high: u8,
    _c: [u8; 0x34],
    /// +0x3e — constant `cf cd` marker on real FANTOM-6 zones; used to tell a decodable zone from
    /// an uninitialized/foreign one rather than as a hard parse assertion (see [`RawZone::MARKER`]).
    pub marker: [u8; 2],
    _d: [u8; 0x20],
}

impl RawZone {
    /// On-disk length of one zone entry.
    pub const LEN: usize = 0x60;
    /// Record-relative offset of the zone table.
    pub const TABLE_OFFSET: usize = 0x6d0;
    /// The `cf cd` alignment marker a valid FANTOM-6 zone carries at `+0x3e`.
    pub const MARKER: [u8; 2] = [0xcf, 0xcd];

    /// Whether this looks like a real, decodable zone (correct alignment marker).
    pub fn is_valid(&self) -> bool {
        self.marker == Self::MARKER
    }
}

/// One entry of the scene record's **zone settings table** (record-relative `0x194`, 16 × 72 bytes).
///
/// Only `level` is decoded so far; the encoded tone reference also lives in this table (deferred).
/// The leading byte hovers around `0x57` but is not a stable magic (seen `0x56`/`0x5a`), so it is
/// not asserted — the reliable alignment anchor is [`RawZone`]'s `cf cd` marker.
#[binread]
#[derive(Debug, Clone, PartialEq)]
#[br(little)]
pub struct ZoneSettings {
    _a: [u8; 7],
    /// +0x07 — zone level (0..127).
    pub level: u8,
    _b: [u8; 0x40],
}

impl ZoneSettings {
    /// On-disk length of one settings entry.
    pub const LEN: usize = 0x48;
    /// Record-relative offset of the settings table.
    pub const TABLE_OFFSET: usize = 0x194;
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::BinRead;
    use std::io::Cursor;

    #[test]
    fn raw_zone_reads_switch_and_key_range() {
        let mut b = vec![0u8; RawZone::LEN];
        b[0x04] = 1; // enable
        b[0x08] = 60; // C4
        b[0x09] = 72; // C5
        b[0x3e] = 0xcf;
        b[0x3f] = 0xcd;
        let z = RawZone::read(&mut Cursor::new(&b)).unwrap();
        assert_eq!((z.enable, z.key_low, z.key_high), (1, 60, 72));
        assert!(z.is_valid());
    }

    #[test]
    fn raw_zone_without_marker_parses_but_is_invalid() {
        // A zone with no cf cd marker still parses (no hard assert) but reports invalid.
        let b = vec![0u8; RawZone::LEN];
        let z = RawZone::read(&mut Cursor::new(&b)).unwrap();
        assert!(!z.is_valid());
    }

    #[test]
    fn zone_settings_reads_level() {
        let mut b = vec![0u8; ZoneSettings::LEN];
        b[0x00] = 0x57;
        b[0x07] = 50;
        let s = ZoneSettings::read(&mut Cursor::new(&b)).unwrap();
        assert_eq!(s.level, 50);
    }
}
