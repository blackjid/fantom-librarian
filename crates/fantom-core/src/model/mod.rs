//! The Fantom domain model.
//!
//! Roland's ZEN-Core hierarchy is **Tones → Zones → Scenes**: a [`Scene`] wires up to 16 [`Zone`]s,
//! and each zone points at a [`Tone`] plus its performance settings. These types are what the
//! librarian browses, renames, and packages; they are intentionally decoupled from the on-disk
//! byte layout, which lives in [`crate::container`] and is mapped by [`crate::codec`].

/// A named performance: up to 16 zones plus scene-level metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub name: String,
    pub zones: Vec<Zone>,
}

/// One of a scene's 16 zone slots: whether it plays, over what key range, and how loud.
///
/// The per-zone **tone reference** (which sound the zone plays) is deferred — it is stored encoded
/// in the scene record and not yet decoded (see `docs/FORMAT.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    /// 0-based index within the scene (0..16).
    pub number: u8,
    /// Whether the zone is switched on.
    pub enabled: bool,
    /// Key-range lower bound (MIDI note, 0..127).
    pub key_low: u8,
    /// Key-range upper bound (MIDI note, 0..127).
    pub key_high: u8,
    /// Zone level (0..127).
    pub level: u8,
}

/// A reference to a tone from within a zone — either a stored tone or a factory/preset location.
#[derive(Debug, Clone, PartialEq)]
pub struct ToneRef {
    pub name: String,
}

/// A ZEN-Core tone (the `.svz` unit), independent of any scene that uses it.
#[derive(Debug, Clone, PartialEq)]
pub struct Tone {
    pub name: String,
}
