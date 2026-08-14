//! What the catalog stores and the UI shows. Serialisable because the desktop app hands these
//! straight to its front end.

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
    pub created_at: i64,
    pub archived_at: Option<i64>,
    pub tags: Vec<String>,
    /// Every source this asset was seen in, in import order.
    pub sources: Vec<AssetSource>,
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
    /// Distinct engines the scene's enabled zones call for.
    pub engines: Vec<String>,
    /// Bundled user tones this scene needs, by resolved name.
    pub user_tones: Vec<String>,
    /// Zones pointing at factory, expansion, or model content the app never substitutes.
    pub external_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDetail {
    pub number: u8,
    pub enabled: bool,
    pub muted: bool,
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
