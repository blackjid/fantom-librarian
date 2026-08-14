//! What a file is for, which its envelope alone does not say.
//!
//! [`crate::container::Kind`] distinguishes the two envelopes, `SVD5` and `SVZa`, and that is the
//! right distinction for framing bytes. It is not enough to describe a file: a whole-instrument
//! backup and a three-scene export are both `SVD5`.
//!
//! # What actually separates them
//!
//! A backup is a dump of *every* memory area the instrument has. An export writes only the areas
//! its content needs. So the tell is not size or scene count — it is the presence of areas that
//! have nothing to do with the exported material.
//!
//! Two candidate signals were checked against a corpus of backups, commercial packs, and hardware
//! test exports. Only one survived:
//!
//! - **System settings do not work.** [`container::COMMON_AREAS`] travel with every `SVD5`, a
//!   one-scene export included, which is why [`crate::repackage`] copies them through untouched.
//! - **The sample bank does.** [`container::SAMPLE_BANK_AREAS`] appear in backups and in no
//!   export. The decisive case is a backup of an instrument holding *zero* user samples that still
//!   writes all three: they are there because the backup dumps the whole memory, not because there
//!   is anything in them.
//!
//! The full 512-slot performance bank corroborates it — a backup declares every slot, an export
//! only what was exported — and is kept as a second signal so a backup whose sample areas are
//! named differently on another model is still recognised.

use crate::container::{self, Kind, Raw, RecordTable, Svd};

/// Scene slots a FANTOM-6/7/8 performance bank holds. A file declaring this many carries the whole
/// bank rather than a selection from it.
const FULL_SCENE_BANK: usize = 512;

/// Areas that hold tone records. An `SVZa` carrying any of them is a tone export rather than a
/// sample companion.
const TONE_AREAS: [&[u8; 4]; 8] = [
    b"PATa", b"RHYa", b"INSa", b"ZAPa", b"ZEPa", b"MDLa", b"ACBa", b"DCWa",
];

/// Areas that carry user audio in an `SVZa` companion, which names its tables differently from a
/// backup's: `USPa` for the slot directory and `MSPa` for multisamples.
const COMPANION_SAMPLE_AREAS: [&[u8; 4]; 2] = [b"USPa", b"MSPa"];

/// What a readable Fantom file is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum Role {
    /// A whole-instrument backup: the full scene bank, its tone banks, and its user sampling.
    Backup,
    /// A self-contained scene export — scenes plus the user tones they need.
    SceneBank,
    /// An `SVZa` tone export, which may carry its own samples.
    ToneBank,
    /// An `SVZa` holding user sampling and nothing else.
    SampleBank,
    /// Readable, but not a shape this version recognises.
    Unknown,
}

impl Role {
    /// The stable spelling, for storing and for the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::SceneBank => "scene-bank",
            Self::ToneBank => "tone-bank",
            Self::SampleBank => "sample-bank",
            Self::Unknown => "unknown",
        }
    }

    /// The inverse of [`Role::as_str`]; anything unrecognised reads as [`Role::Unknown`].
    pub fn parse(s: &str) -> Self {
        match s {
            "backup" => Self::Backup,
            "scene-bank" => Self::SceneBank,
            "tone-bank" => Self::ToneBank,
            "sample-bank" => Self::SampleBank,
            _ => Self::Unknown,
        }
    }

    /// A short word for a label or a table column.
    pub fn label(self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::SceneBank => "scenes",
            Self::ToneBank => "tones",
            Self::SampleBank => "samples",
            Self::Unknown => "?",
        }
    }
}

/// Classify a file from its area table.
pub fn of(raw: &Raw) -> Role {
    let Ok(svd) = Svd::parse(raw) else {
        return Role::Unknown;
    };
    let has = |tag: &[u8; 4]| svd.area(tag).is_some();

    if svd.kind == Kind::Svz {
        // A sample companion carries audio and no tone records; a tone export may carry both.
        return if TONE_AREAS.iter().any(|tag| has(tag)) {
            Role::ToneBank
        } else if COMPANION_SAMPLE_AREAS
            .iter()
            .chain(container::SAMPLE_BANK_AREAS.iter())
            .any(|tag| has(tag))
        {
            Role::SampleBank
        } else {
            Role::Unknown
        };
    }

    // An `SVD5` with no performance area is not a shape this version knows.
    if !has(b"PRFa") {
        return Role::Unknown;
    }

    let dumps_whole_memory = container::SAMPLE_BANK_AREAS.iter().any(|tag| has(tag));
    let whole_scene_bank = scene_capacity(raw, &svd).is_some_and(|n| n >= FULL_SCENE_BANK);

    if dumps_whole_memory || whole_scene_bank {
        Role::Backup
    } else {
        Role::SceneBank
    }
}

/// How many scene slots the file's performance area declares, empty ones included.
fn scene_capacity(raw: &Raw, svd: &Svd) -> Option<usize> {
    RecordTable::from_svd(raw, svd, b"PRFa")
        .ok()
        .flatten()
        .map(|table| table.declared_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREAMBLE: usize = 0x10;

    /// A container with the given areas, each empty. Enough for classification, which reads the
    /// area table and — only for the scene count — the `PRFa` header.
    fn container_of(kind: Kind, tags: &[&[u8; 4]]) -> Raw {
        let mut file = Vec::new();
        match kind {
            Kind::Svd => {
                let header_size = (tags.len() * 16 + 14) as u16;
                file.extend_from_slice(&header_size.to_le_bytes());
                file.extend_from_slice(b"SVD5");
                file.resize(PREAMBLE, 0);
            }
            Kind::Svz => {
                file.extend_from_slice(b"SVZa");
                file.push(tags.len() as u8);
                file.resize(PREAMBLE, 0);
            }
        }
        // Every area points at the same empty region past the table; none is read for its content.
        let body = PREAMBLE + tags.len() * 16;
        for tag in tags {
            file.extend_from_slice(*tag);
            file.extend_from_slice(b"KY19");
            file.extend_from_slice(&(body as u32).to_le_bytes());
            file.extend_from_slice(&0u32.to_le_bytes());
        }
        Raw::from_bytes(file)
    }

    #[test]
    fn an_unframeable_file_is_unknown() {
        assert_eq!(of(&Raw::from_bytes(vec![0; 8])), Role::Unknown);
    }

    /// The rule that matters: a backup dumps the sample bank whether or not the instrument holds
    /// any audio, which is what makes its presence a statement about scope rather than content.
    #[test]
    fn the_sample_bank_marks_a_backup_even_when_it_holds_no_audio() {
        let backup = container_of(
            Kind::Svd,
            &[
                b"PRFa", b"PATa", b"RHYa", b"SMPa", b"MLSa", b"USDa", b"SYSa", b"DIFa",
            ],
        );
        assert_eq!(of(&backup), Role::Backup);
    }

    /// System settings travel with every export, so they must not tip the decision.
    #[test]
    fn common_areas_alone_do_not_make_a_backup() {
        let export = container_of(Kind::Svd, &[b"PRFa", b"PATa", b"RHYa", b"SYSa", b"DIFa"]);
        assert_eq!(of(&export), Role::SceneBank);

        // Nor do the engine banks an export legitimately carries for the tones it references.
        let with_engines = container_of(
            Kind::Svd,
            &[b"PRFa", b"DCWa", b"MDLa", b"ACBa", b"SYSa", b"DIFa"],
        );
        assert_eq!(of(&with_engines), Role::SceneBank);
    }

    #[test]
    fn an_svd_without_a_performance_area_is_unknown() {
        assert_eq!(
            of(&container_of(Kind::Svd, &[b"PATa", b"SYSa"])),
            Role::Unknown
        );
    }

    #[test]
    fn an_svz_is_a_tone_bank_when_it_carries_tone_records() {
        // A tone export may bring its own audio; the tone areas are what name it.
        let with_audio = container_of(Kind::Svz, &[b"DIFa", b"PATa", b"USPa", b"USDa"]);
        assert_eq!(of(&with_audio), Role::ToneBank);

        let drums = container_of(Kind::Svz, &[b"DIFa", b"RHYa", b"INSa"]);
        assert_eq!(of(&drums), Role::ToneBank);
    }

    #[test]
    fn an_svz_of_audio_alone_is_a_sample_bank() {
        let samples = container_of(Kind::Svz, &[b"DIFa", b"USPa", b"USDa"]);
        assert_eq!(of(&samples), Role::SampleBank);
    }

    #[test]
    fn the_spelling_round_trips() {
        for role in [
            Role::Backup,
            Role::SceneBank,
            Role::ToneBank,
            Role::SampleBank,
            Role::Unknown,
        ] {
            assert_eq!(Role::parse(role.as_str()), role);
        }
        assert_eq!(Role::parse("something else"), Role::Unknown);
    }
}
