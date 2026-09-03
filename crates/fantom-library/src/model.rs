//! What the catalog stores and the UI shows. Serialisable because the desktop app hands these
//! straight to its front end.

use fantom_core::expansions::Family;
use fantom_core::requirements::{Holding, Requirements};
use serde::{Deserialize, Serialize};

/// The two things the main library browses. Samples are first-class internally but live in their
/// own view, so they are not an asset kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Scene,
    Tone,
}

impl AssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scene => "scene",
            Self::Tone => "tone",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "scene" => Some(Self::Scene),
            "tone" => Some(Self::Tone),
            _ => None,
        }
    }
}

/// One import, browsable as a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub vendor: String,
    pub url: String,
    pub licence_note: String,
    pub note: String,
    pub origin_path: String,
    /// Unix seconds; the front end decides how to show a date.
    pub imported_at: i64,
    pub archived_at: Option<i64>,
    pub file_count: i64,
    pub asset_count: i64,
    /// The files this source contributed. An import is a handful of files, so the sidebar gets
    /// them inline rather than fetching per source.
    pub files: Vec<LibraryFile>,
}

/// Provenance supplied at import. Every field is optional — an incomplete note never blocks an
/// import.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub vendor: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub licence_note: String,
    #[serde(default)]
    pub note: String,
}

/// A managed copy of an imported file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryFile {
    pub id: i64,
    pub source_id: i64,
    pub file_name: String,
    pub origin_path: String,
    pub content_hash: String,
    pub size: i64,
    pub stored_path: String,
    /// The file extension, lowercased.
    pub kind: String,
    /// What the file is for — a backup and a scene export are both `.svd`.
    pub role: fantom_core::role::Role,
    pub status: FileStatus,
    /// Why a file is `Invalid`, or what was odd about a valid one.
    pub problems: Vec<String>,
    pub asset_count: i64,
    pub sample_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Ok,
    Invalid,
}

impl FileStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Invalid => "invalid",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "invalid" => Self::Invalid,
            _ => Self::Ok,
        }
    }
}

/// A row in the library list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: i64,
    pub kind: AssetKind,
    /// The editable, device-facing name — what an export would write.
    pub fantom_name: String,
    /// The name as first imported, kept as history.
    pub imported_name: String,
    pub note: String,
    /// The FANTOM scene memo. Preserved, not edited, in v1.
    pub memo: String,
    /// Sound engine label, for tones and for a scene's dominant engine.
    pub engine: String,
    pub detail: AssetDetail,
    /// Whether this came out of one of the user's files, or ships with the instrument.
    #[serde(default = "user_origin")]
    pub origin: Origin,
    pub created_at: i64,
    pub archived_at: Option<i64>,
    pub tags: Vec<String>,
    /// Every source this asset was seen in, in import order.
    pub sources: Vec<AssetSource>,
}

fn user_origin() -> Origin {
    Origin::User
}

/// Where one asset came from. The same asset can appear in several sources at once.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSource {
    pub source_id: i64,
    pub source_name: String,
    pub file_id: i64,
    pub file_name: String,
    pub slot: i64,
    pub area: String,
    pub name_at_import: String,
}

/// Kind-specific payload, stored as JSON so the shape can grow without a migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AssetDetail {
    Scene(SceneDetail),
    Tone(ToneDetail),
}

impl AssetDetail {
    /// A one-line summary for the list row.
    pub fn summary(&self) -> String {
        match self {
            Self::Scene(s) => format!(
                "{:.2} BPM · {} zone{}",
                s.bpm,
                s.active_zones,
                if s.active_zones == 1 { "" } else { "s" }
            ),
            Self::Tone(t) => {
                if t.area.is_empty() {
                    t.engine.clone()
                } else {
                    format!("{} · {}", t.engine, t.area)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDetail {
    pub bpm: f32,
    pub level: u8,
    /// Zones with their KBD switch on.
    pub active_zones: usize,
    pub zones: Vec<ZoneDetail>,
    /// Distinct engines the scene's playing zones call for.
    pub engines: Vec<String>,
    /// Keyboard switch groups this scene configures; empty when it leaves them at the default.
    pub groups: Vec<KeyboardGroupDetail>,
    /// Bundled user tones this scene needs, by resolved name.
    pub user_tones: Vec<String>,
    /// Zones pointing at factory, expansion, or model content the app never substitutes.
    pub external_refs: Vec<String>,
    /// Everything this scene needs from wherever it is loaded, decided from the bytes.
    ///
    /// Defaulted so a catalog written before requirements existed still reads; those assets get
    /// theirs the next time their source is imported.
    #[serde(default)]
    pub requirements: Requirements,
}

/// A saved set of zone switches the player recalls from a pad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardGroupDetail {
    pub number: u8,
    /// 1-based zone numbers this group switches on.
    pub zones: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDetail {
    pub number: u8,
    pub enabled: bool,
    pub muted: bool,
    /// Why this zone sounds or does not: `on`, `muted`, `grouped`, `off`, or `unused`.
    pub state: String,
    /// Keyboard groups that switch this zone on, if any.
    pub groups: Vec<u8>,
    pub engine: String,
    pub bank: String,
    pub tone: String,
    pub msb: u8,
    pub lsb: u8,
    pub pc: u8,
    pub key_low: u8,
    pub key_high: u8,
    pub velocity_low: u8,
    pub velocity_high: u8,
    pub level: u8,
    pub pan: i8,
    pub transpose: i8,
    pub octave: i8,
    pub midi_channel: u8,
    pub arpeggio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneDetail {
    pub engine: String,
    /// Four-byte SVD area tag the record lives in.
    pub area: String,
    pub index: usize,
    /// The bank it sits in, for a sound that is the instrument's rather than a record in a file.
    #[serde(default)]
    pub bank: Option<String>,
    /// The address a zone selects it by — a built-in sound is only ever reached this way.
    #[serde(default)]
    pub address: Option<fantom_core::model::ToneAddress>,
    /// Roland's category for a built-in sound, e.g. `35:Synth Brass`.
    #[serde(default)]
    pub category: Option<String>,
    /// Which model of its engine family this is — `MODEL` and `ACB` records each carry one.
    ///
    /// Defaulted so a catalog written before the selector was decoded still reads; those tones
    /// get theirs the next time their source is imported.
    #[serde(default)]
    pub model_id: Option<u32>,
    /// The samples, multisamples, and expansions this tone plays. See [`SceneDetail::requirements`].
    #[serde(default)]
    pub requirements: Requirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: i64,
    pub title: String,
    pub artist: String,
    /// Performance key, which is often not the original's.
    pub song_key: String,
    pub notes: String,
    pub created_at: i64,
    pub links: Vec<SongLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongLink {
    pub asset_id: i64,
    pub asset_name: String,
    pub asset_kind: AssetKind,
    pub note: String,
}

/// What the library list was asked for.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Query {
    /// Matched against the current name, the imported name, and the note.
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub kind: Option<AssetKind>,
    #[serde(default)]
    pub source_id: Option<i64>,
    /// Narrower than `source_id`: one file within a source.
    #[serde(default)]
    pub file_id: Option<i64>,
    #[serde(default)]
    pub song_id: Option<i64>,
    /// An asset must carry every tag listed.
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub include_archived: bool,
    #[serde(default)]
    pub limit: Option<i64>,
    /// Sound engines to keep, by their label — `MODEL`, `ZEN-Core`, `ACB`. Empty keeps every one.
    #[serde(default)]
    pub engines: Vec<String>,
    /// Models and expansions to keep, as [`crate::facet`] names them. Empty keeps every one.
    #[serde(default)]
    pub models: Vec<String>,
    /// Keep only the instrument's own sounds, or only what the user's files carry.
    #[serde(default)]
    pub origin: Option<Origin>,
    /// Keep only what plays anywhere, or only what asks for something first.
    #[serde(default)]
    pub plays: Option<Plays>,
    /// Omit scenes and tones that need a bundled expansion not installed on this instrument.
    #[serde(default)]
    pub hide_uninstalled_expansions: bool,
}

impl Query {
    /// Whether any facet is set. Those are decided from the stored detail, not in SQL, so the
    /// catalog takes a different path through the same rows when one is.
    pub fn narrows_by_facet(&self) -> bool {
        !self.engines.is_empty()
            || !self.models.is_empty()
            || self.origin.is_some()
            || self.plays.is_some()
            || self.hide_uninstalled_expansions
    }
}

/// Where an asset came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    /// The instrument ships with it; no file in the library carries it.
    Factory,
    /// It came out of one of the user's imported files.
    User,
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Factory => "factory",
            Self::User => "user",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "factory" => Self::Factory,
            _ => Self::User,
        }
    }
}

/// What an asset asks of the instrument it is loaded onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Plays {
    /// Preset banks alone, and no bundled user tones: it plays anywhere, as it was heard.
    FactoryOnly,
    /// Needs something of the user's, or an expansion that has to be installed.
    NeedsYours,
}

/// One value a facet can take, and how many assets in scope take it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facet {
    pub value: String,
    pub count: i64,
}

/// What the current scope can be narrowed by, and by how much.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Facets {
    pub engines: Vec<Facet>,
    pub models: Vec<Facet>,
    pub origins: Vec<Facet>,
    pub plays: Vec<Facet>,
}

/// What one import did, reported back so nothing is silently dropped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub source_id: i64,
    pub source_name: String,
    pub files_imported: usize,
    /// Files already present in this source, byte for byte.
    pub files_skipped: usize,
    pub files_invalid: usize,
    pub scenes_added: usize,
    pub tones_added: usize,
    /// Assets that consolidated onto an existing library item.
    pub assets_consolidated: usize,
    pub samples_catalogued: usize,
    /// Anything the user should see: an unreadable file, a checksum problem, a skipped record.
    pub warnings: Vec<String>,
}

/// One expansion in the workspace's inventory.
///
/// The list is the bundled catalogs plus whatever else has been recorded, so an expansion nobody
/// owns still appears — that is what makes it selectable in the first place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionEntry {
    /// Product code: `EXZ007`, `EXSN01`, `JP8`.
    pub code: String,
    /// What kind of expansion it is, for a list that groups them.
    pub family: Family,
    /// The engine that plays it, when a catalog says. Empty for a code nothing here can place.
    pub engine: String,
    /// How many sounds the bundled catalog carries for it; zero when there is no catalog.
    pub sounds: usize,
    /// How far it has got towards playing: unowned, owned, or loaded into a slot.
    pub state: Holding,
    /// Whether this build carries a catalog of its sounds. A code recorded by hand does not.
    pub catalogued: bool,
}
