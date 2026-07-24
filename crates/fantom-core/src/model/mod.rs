//! The Fantom domain model.
//!
//! Roland's ZEN-Core hierarchy is **Tones → Zones → Scenes**: a [`Scene`] wires up to 16 [`Zone`]s,
//! and each zone plays a tone ([`ToneRef`]) over a key range. These types are what the librarian
//! browses, renames, and packages; they are intentionally decoupled from the on-disk byte layout,
//! which lives in [`crate::container`] and is mapped by [`crate::codec`].

/// A named performance: up to 16 zones plus scene-level metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub name: String,
    /// Free-text scene comment/memo (empty when unset).
    pub comment: String,
    pub zones: Vec<Zone>,
}

/// One of a scene's 16 zone slots: the tone it plays, its key range, and level.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    /// 0-based index within the scene (0..16).
    pub number: u8,
    /// Whether the zone is switched on.
    pub enabled: bool,
    /// The tone this zone plays.
    pub tone: ToneRef,
    /// Key-range lower bound (MIDI note, 0..127).
    pub key_low: u8,
    /// Key-range upper bound (MIDI note, 0..127).
    pub key_high: u8,
    /// Zone level (0..127).
    pub level: u8,
}

/// The raw MIDI bank/program address stored by a scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToneAddress {
    pub msb: u8,
    pub lsb: u8,
    /// Zero-based MIDI program number.
    pub pc: u8,
}

/// Sound-engine type selected by a tone address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneType {
    Drum,
    ZenCore,
    SnA,
    SnAp,
    SnEp,
    Exsn,
    Vtw,
    VPiano,
    Model,
    Exz,
    Acb,
    Unknown,
}

impl ToneType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Drum => "Drum",
            Self::ZenCore => "ZEN-Core",
            Self::SnA => "SN-A",
            Self::SnAp => "SN-AP",
            Self::SnEp => "SN-EP",
            Self::Exsn => "EXSN",
            Self::Vtw => "VTW",
            Self::VPiano => "VPiano",
            Self::Model => "MODEL",
            Self::Exz => "EXZ",
            Self::Acb => "ACB",
            Self::Unknown => "Unknown",
        }
    }
}

/// Which tone a zone plays, retaining its complete on-disk address.
#[derive(Debug, Clone, PartialEq)]
pub struct ToneRef {
    pub address: ToneAddress,
    /// Resolved bundled or factory name, when known.
    pub name: Option<String>,
}

impl ToneRef {
    pub fn new(msb: u8, lsb: u8, pc: u8, name: Option<String>) -> Self {
        Self {
            address: ToneAddress { msb, lsb, pc },
            name,
        }
    }

    pub fn tone_type(&self) -> ToneType {
        match (self.address.msb, self.address.lsb) {
            (86 | 92 | 100, _) => ToneType::Drum,
            (87, _) => ToneType::ZenCore,
            (89, _) => ToneType::SnA,
            (90 | 103, _) => ToneType::VPiano,
            (91, _) => ToneType::Vtw,
            (93 | 101, _) => ToneType::Exz,
            (97, _) => ToneType::Model,
            (105, 0) => ToneType::SnAp,
            (105, 1) => ToneType::SnEp,
            (105, 64 | 66) => ToneType::SnAp,
            (105, 65) => ToneType::SnEp,
            (105, _) => ToneType::Exsn,
            (107, _) => ToneType::Acb,
            _ => ToneType::Unknown,
        }
    }

    /// User-facing bank label when its byte mapping is confirmed.
    pub fn bank(&self) -> Option<&str> {
        match (self.tone_type(), self.address.lsb) {
            (ToneType::ZenCore, lsb) if lsb < 64 => Some("USER"),
            (ToneType::Drum | ToneType::SnAp | ToneType::Vtw, 0)
            | (ToneType::SnA, 0..=1)
            | (ToneType::SnEp, 1) => Some("USER"),
            (ToneType::VPiano, 0) if self.address.msb == 90 => Some("USER"),
            (ToneType::Drum, 64) if self.address.msb == 86 => Some("PR-A"),
            (ToneType::Drum, 65) if self.address.msb == 86 => Some("CMN"),
            (ToneType::SnA | ToneType::Vtw, 65) => Some("PRST"),
            (ToneType::VPiano, 64) if self.address.msb == 90 => Some("PRST"),
            (ToneType::VPiano, 64) if self.address.msb == 103 => Some("M09X01"),
            (ToneType::SnAp, 64) => Some("EXSN01"),
            (ToneType::SnEp, 65) => Some("EXSN02"),
            (ToneType::SnAp, 66) => Some("EXSN03"),
            (ToneType::Exz, 1) if self.address.msb == 93 => Some("EXZ013"),
            (ToneType::Exz, 2) if self.address.msb == 93 => Some("EXZ005"),
            (ToneType::Exz, 3) if self.address.msb == 93 => Some("EXZ009"),
            (ToneType::Exz, 7..=10) if self.address.msb == 93 => Some("EXZ007"),
            (ToneType::Exz, 11..=14) if self.address.msb == 93 => Some("EXZ008"),
            (ToneType::Exz, 15..=17) if self.address.msb == 93 => Some("EXZ012"),
            (ToneType::Exz, 19..=22) if self.address.msb == 93 => Some("EXZ006"),
            (ToneType::Exz, 23) if self.address.msb == 93 => Some("EXZ010"),
            (ToneType::Exz, 24) if self.address.msb == 93 => Some("EXZ014"),
            (ToneType::Exz, 26) if self.address.msb == 93 => Some("EXZ011"),
            (ToneType::Exz, 27) if self.address.msb == 93 => Some("EXZ015"),
            (ToneType::Exz, 64) if self.address.msb == 101 => Some("EXZ001"),
            (ToneType::Exz, 65) if self.address.msb == 101 => Some("EXZ002"),
            (ToneType::Drum, 64) if self.address.msb == 100 => Some("EXZ003"),
            (ToneType::Drum, 65) if self.address.msb == 100 => Some("EXZ004"),
            (ToneType::Model, 64) => Some("USER"),
            (ToneType::Model, 66) => Some("JP8"),
            (ToneType::Model, 68) => Some("JU106"),
            (ToneType::Model, 70) => Some("JX8P"),
            (ToneType::Model, 72) => Some("n/zyme"),
            (ToneType::Model, 79) => Some("SH101"),
            (ToneType::Acb, 0) => Some("USER"),
            (ToneType::Acb, 64) => Some("JP8"),
            (ToneType::Acb, 66) => Some("SH101"),
            (ToneType::Acb, 70) => Some("JU106"),
            (ToneType::Acb, 76) => Some("JX3P"),
            _ => self.preset().map(|p| p.bank),
        }
    }

    /// The tone's display name when known.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Factory ZEN-Core preset details, when this is a known entry in the bundled sound list.
    pub fn preset(&self) -> Option<&'static crate::presets::PresetTone> {
        if self.tone_type() == ToneType::ZenCore && self.address.lsb >= 64 {
            crate::presets::lookup(u16::from_be_bytes([self.address.lsb, self.address.pc]))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_documented_engine_and_bank_addresses() {
        let cases = [
            (86, 64, ToneType::Drum, Some("PR-A")),
            (89, 65, ToneType::SnA, Some("PRST")),
            (90, 64, ToneType::VPiano, Some("PRST")),
            (91, 65, ToneType::Vtw, Some("PRST")),
            (97, 7, ToneType::Model, None),
            (103, 9, ToneType::VPiano, None),
            (105, 64, ToneType::SnAp, Some("EXSN01")),
            (105, 65, ToneType::SnEp, Some("EXSN02")),
            (107, 64, ToneType::Acb, Some("JP8")),
            (103, 64, ToneType::VPiano, Some("M09X01")),
            (97, 64, ToneType::Model, Some("USER")),
            (97, 66, ToneType::Model, Some("JP8")),
            (107, 66, ToneType::Acb, Some("SH101")),
            (89, 1, ToneType::SnA, Some("USER")),
            (93, 2, ToneType::Exz, Some("EXZ005")),
            (101, 64, ToneType::Exz, Some("EXZ001")),
            (100, 65, ToneType::Drum, Some("EXZ004")),
        ];
        for (msb, lsb, tone_type, bank) in cases {
            let tone = ToneRef::new(msb, lsb, 0, None);
            assert_eq!(tone.tone_type(), tone_type);
            assert_eq!(tone.bank(), bank);
        }
    }

    #[test]
    fn leaves_undocumented_banks_raw() {
        let tone = ToneRef::new(107, 72, 3, None);
        assert_eq!(tone.tone_type(), ToneType::Acb);
        assert_eq!(tone.bank(), None);
    }
}
