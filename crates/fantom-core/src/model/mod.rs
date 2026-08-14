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
    /// Scene tempo in hundredths of a BPM, as stored: `12000` is 120.00.
    pub tempo: u16,
    /// Scene level (0..127).
    pub level: u8,
    pub zones: Vec<Zone>,
    /// Keyboard switch groups the scene actually configures.
    ///
    /// Empty when it leaves them at the factory default, where group *n* holds only zone *n* and
    /// so says nothing. See [`KeyboardGroup`].
    pub groups: Vec<KeyboardGroup>,
}

impl Scene {
    /// Scene tempo in BPM.
    pub fn bpm(&self) -> f32 {
        self.tempo as f32 / 100.0
    }

    /// The configured groups that switch `zone_number` (1-based) on.
    pub fn groups_containing(&self, zone_number: u8) -> Vec<u8> {
        self.groups
            .iter()
            .filter(|group| group.zones.contains(&zone_number))
            .map(|group| group.number)
            .collect()
    }

    /// How a zone stands in this scene: playing, silent by choice, or never used.
    ///
    /// The distinction matters for packaging. A zone that is off *now* but belongs to a keyboard
    /// group is one pad press from sounding, so the tone it plays is still a dependency of the
    /// scene; a zone that was never configured is not.
    pub fn zone_state(&self, zone: &Zone) -> ZoneState {
        if zone.enabled {
            return if zone.muted {
                ZoneState::Muted
            } else {
                ZoneState::On
            };
        }
        if !self.groups_containing(zone.number + 1).is_empty() {
            return ZoneState::Grouped;
        }
        if zone.is_at_factory_default() {
            ZoneState::Unused
        } else {
            ZoneState::Off
        }
    }
}

/// A saved set of zone keyboard switches, recalled from a pad.
///
/// The FANTOM stores sixteen of them, each a 16-bit mask over the scene's zones (`Pad_Mode` has a
/// `KBD SW GROUP` setting that maps the pads to them). Selecting a group sets every zone's KBD
/// switch to that group's mask, which is how one scene holds several playable arrangements — and
/// why a zone can be switched off in the saved state yet still be part of the performance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardGroup {
    /// 1-based group number, as the panel shows it.
    pub number: u8,
    /// 1-based zone numbers this group switches on.
    pub zones: Vec<u8>,
}

/// Why a zone sounds, or does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneState {
    /// Switched on and audible.
    On,
    /// Switched on, but muted.
    Muted,
    /// Switched off, and a keyboard group switches it on — part of the performance.
    Grouped,
    /// Switched off with settings of its own: configured, then deliberately silenced.
    Off,
    /// Switched off and never configured — still exactly as the factory left it.
    Unused,
}

impl ZoneState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Muted => "muted",
            Self::Grouped => "grouped",
            Self::Off => "off",
            Self::Unused => "unused",
        }
    }

    /// Whether the tone this zone plays is a dependency of the scene.
    ///
    /// Everything but an untouched zone counts. A muted or switched-off zone is one control away
    /// from sounding, and a package that dropped its tone would be quietly broken; an unused zone
    /// still points at the factory default it was born with, which is not a dependency at all.
    pub fn is_played(self) -> bool {
        self != Self::Unused
    }
}

/// One of a scene's 16 zone slots: the tone it plays, its key range, and how it is set up.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    /// 0-based index within the scene (0..16).
    pub number: u8,
    /// Whether the zone is switched on — the panel's KBD switch.
    pub enabled: bool,
    /// Whether the zone is muted. Distinct from [`Zone::enabled`]: a muted zone still receives.
    pub muted: bool,
    /// The tone this zone plays.
    pub tone: ToneRef,
    /// Key-range lower bound (MIDI note, 0..127).
    pub key_low: u8,
    /// Key-range upper bound (MIDI note, 0..127).
    pub key_high: u8,
    /// Velocity-range lower bound (1..127).
    pub velocity_low: u8,
    /// Velocity-range upper bound (1..127).
    pub velocity_high: u8,
    /// Zone level (0..127).
    pub level: u8,
    /// Zone pan, zero-centred: −64 is hard left, +63 hard right.
    pub pan: i8,
    /// Transpose in semitones (−48..48).
    pub transpose: i8,
    /// Octave shift (−3..3).
    pub octave: i8,
    /// MIDI receive channel, 0-based.
    pub midi_channel: u8,
    /// Whether the arpeggiator is on for this zone.
    pub arpeggio: bool,
}

impl Zone {
    /// Whether every setting still holds the value the panel gives a fresh zone.
    ///
    /// A scene always stores all sixteen zones, so most of them in most scenes were never touched.
    /// This is what tells those apart from a zone somebody set up and then switched off — measured
    /// across a corpus of backups and commercial packs, no *switched-on* zone matches this shape,
    /// while it accounts for the majority of switched-off ones.
    ///
    /// The tone address is deliberately not part of the test: the factory default differs by slot
    /// (zone 10 gets a drum kit, the rest a piano) and by instrument revision, so the settings are
    /// the stable signal.
    pub fn is_at_factory_default(&self) -> bool {
        self.key_low == 0
            && self.key_high == 127
            && self.velocity_low == 1
            && self.velocity_high == 127
            && self.level == 100
            && self.pan == 0
            && self.transpose == 0
            && self.octave == 0
    }
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
            (ToneType::Model, 0 | 64) => Some("USER"),
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
mod zone_state_tests {
    use super::*;

    fn zone(number: u8, enabled: bool) -> Zone {
        Zone {
            number,
            enabled,
            muted: false,
            tone: ToneRef::new(87, 93, 60, None),
            key_low: 0,
            key_high: 127,
            velocity_low: 1,
            velocity_high: 127,
            level: 100,
            pan: 0,
            transpose: 0,
            octave: 0,
            midi_channel: 0,
            arpeggio: false,
        }
    }

    fn scene(zones: Vec<Zone>, groups: Vec<KeyboardGroup>) -> Scene {
        Scene {
            name: "Test".into(),
            comment: String::new(),
            tempo: 12000,
            level: 100,
            zones,
            groups,
        }
    }

    #[test]
    fn an_untouched_zone_is_told_from_one_that_was_silenced() {
        let untouched = zone(0, false);
        assert!(untouched.is_at_factory_default());

        // One setting away from the default is enough: somebody configured this and switched it off.
        let mut silenced = zone(1, false);
        silenced.key_high = 71;
        assert!(!silenced.is_at_factory_default());

        let scene = scene(vec![untouched, silenced], Vec::new());
        assert_eq!(scene.zone_state(&scene.zones[0]), ZoneState::Unused);
        assert_eq!(scene.zone_state(&scene.zones[1]), ZoneState::Off);
    }

    #[test]
    fn a_grouped_zone_is_part_of_the_performance_though_switched_off() {
        let mut split = zone(2, false);
        split.key_low = 60;
        let scene = scene(
            vec![zone(0, true), zone(1, false), split],
            vec![KeyboardGroup {
                number: 2,
                zones: vec![1, 3],
            }],
        );

        assert_eq!(scene.zone_state(&scene.zones[0]), ZoneState::On);
        // Zone 2 is in no group and never configured.
        assert_eq!(scene.zone_state(&scene.zones[1]), ZoneState::Unused);
        // Zone 3 is off now, but group 2 switches it on.
        assert_eq!(scene.zone_state(&scene.zones[2]), ZoneState::Grouped);
        assert_eq!(scene.groups_containing(3), vec![2]);
        assert!(scene.groups_containing(2).is_empty());
    }

    #[test]
    fn only_an_unused_zone_is_left_out_of_a_package() {
        // Everything a control can bring back has to travel with the scene.
        assert!(ZoneState::On.is_played());
        assert!(ZoneState::Muted.is_played());
        assert!(ZoneState::Grouped.is_played());
        assert!(ZoneState::Off.is_played());
        assert!(!ZoneState::Unused.is_played());
    }

    #[test]
    fn a_muted_zone_is_still_switched_on() {
        let mut muted = zone(0, true);
        muted.muted = true;
        let scene = scene(vec![muted], Vec::new());
        assert_eq!(scene.zone_state(&scene.zones[0]), ZoneState::Muted);
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
