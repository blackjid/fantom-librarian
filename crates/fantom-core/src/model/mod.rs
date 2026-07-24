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

/// One slot within a scene, referencing a tone and how it is played.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    /// 0-based index within the scene (0..16).
    pub index: u8,
    pub tone: ToneRef,
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
