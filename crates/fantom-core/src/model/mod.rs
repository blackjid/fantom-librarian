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

/// Which tone a zone plays.
///
/// A scene stores the reference as a 16-bit id. User tones are bundled into the file's `PATa`
/// area and referenced by index (resolved to a [`name`](ToneRef::User) when the file is a scene
/// export); factory presets keep a fixed ROM id and are not stored in the file.
#[derive(Debug, Clone, PartialEq)]
pub enum ToneRef {
    /// A user tone stored in this file's tone area. `name` is `Some` once resolved from `PATa`.
    User { id: u16, name: Option<String> },
    /// A factory ROM preset tone (not stored in the file).
    Preset { id: u16 },
}

impl ToneRef {
    /// The tone's display name when known: user tones resolved from `PATa`, preset tones from the
    /// bundled factory sound list ([`crate::presets`]).
    pub fn name(&self) -> Option<&str> {
        match self {
            ToneRef::User { name, .. } => name.as_deref(),
            ToneRef::Preset { id } => crate::presets::lookup(*id).map(|p| p.name),
        }
    }

    /// Factory preset details (bank / number / name / category), when this is a known preset.
    pub fn preset(&self) -> Option<&'static crate::presets::PresetTone> {
        match self {
            ToneRef::Preset { id } => crate::presets::lookup(*id),
            ToneRef::User { .. } => None,
        }
    }
}
