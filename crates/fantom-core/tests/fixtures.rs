//! Tests against real FANTOM-6 files.
//!
//! `fixtures/` is gitignored — it holds a user's own backups, sound-list PDFs, and hardware
//! captures, none of which can be committed. Every test here therefore **skips when its fixture is
//! missing**, so a clone without fixtures still runs green, while a developer who has them gets the
//! strongest checks in the suite: the unit tests assert against hand-built byte arrays, these
//! assert against files the instrument actually wrote.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fantom_core::container::Raw;
use fantom_core::model::Scene;

/// The repository's `fixtures/` directory, if it exists.
fn fixtures() -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    dir.is_dir().then_some(dir)
}

/// Open a fixture, or return `None` so the caller can skip.
fn open(relative: &str) -> Option<Raw> {
    let path = fixtures()?.join(relative);
    if !path.is_file() {
        eprintln!("skipping: {} not present", path.display());
        return None;
    }
    Some(Raw::open(&path).expect("fixture is readable"))
}

/// Every scene-export bank paired with the full backup it was exported from.
const EXPORT_BACKUP_PAIRS: [(&str, &str); 3] = [
    (
        "backup/ROLAND/SOUND/NARF/FANTOM.SVD",
        "backup/ROLAND/FANTOM/BACKUP/Black NARFSOUNDS/FANTOM.SVD",
    ),
    (
        "backup/ROLAND/SOUND/TOP80/FANTOM.SVD",
        "backup/ROLAND/FANTOM/BACKUP/2023.4.8+topandprisma/FANTOM.SVD",
    ),
    (
        "backup/ROLAND/SOUND/PRISMA/FANTOM.SVD",
        "backup/ROLAND/FANTOM/BACKUP/2023.4.8+topandprisma/FANTOM.SVD",
    ),
];

fn by_name(scenes: &[Scene]) -> HashMap<&str, &Scene> {
    scenes.iter().map(|s| (s.name.as_str(), s)).collect()
}

/// The oracle for the whole reader: a scene exported to a bank and the same scene sitting in a full
/// backup must resolve to the same tone names.
///
/// This is a real test rather than a tautology because the two files store *different addresses*
/// for the same sound — an export renumbers the tones it bundles densely, while a backup keeps them
/// at their USER-bank slots (`Africa Brass` is `PATa[207]` in the export and `PATa[443]` in the
/// backup). Agreement means the one indexing rule in `fantom_core::address` reads both correctly.
///
/// A previous reading of the format concluded a backup's mapping was unrecoverable and left its
/// user tones unnamed; this test exists so that cannot silently return.
#[test]
fn exports_and_backups_resolve_the_same_tone_names() {
    let mut checked = 0;
    for (export_path, backup_path) in EXPORT_BACKUP_PAIRS {
        let (Some(export), Some(backup)) = (open(export_path), open(backup_path)) else {
            continue;
        };
        let export_scenes = fantom_core::codec::read_scenes(&export).unwrap();
        let backup_scenes = fantom_core::codec::read_scenes(&backup).unwrap();
        let backup_by_name = by_name(&backup_scenes);

        let mut compared = 0;
        for scene in &export_scenes {
            let Some(other) = backup_by_name.get(scene.name.as_str()) else {
                continue;
            };
            assert_eq!(
                scene.zones.len(),
                other.zones.len(),
                "{export_path}: scene {:?} has a different zone count in the backup",
                scene.name
            );
            for (zone, backup_zone) in scene.zones.iter().zip(&other.zones) {
                assert_eq!(
                    zone.tone.name(),
                    backup_zone.tone.name(),
                    "{export_path}: scene {:?} zone {} resolves to {:?} in the export but {:?} in \
                     the backup (export {:?}, backup {:?})",
                    scene.name,
                    zone.number + 1,
                    zone.tone.name(),
                    backup_zone.tone.name(),
                    zone.tone.address,
                    backup_zone.tone.address,
                );
                compared += 1;
            }
        }
        assert!(
            compared > 0,
            "{export_path}: no scenes matched the backup by name — the oracle checked nothing"
        );
        checked += compared;
    }
    if checked == 0 {
        eprintln!("skipped: no export/backup fixture pairs present");
    }
}

/// A full backup must name the user tones its scenes reference. Naming *nothing* would pass the
/// agreement test above only if the export were equally blank, so assert coverage directly.
#[test]
fn a_backup_names_the_user_tones_its_scenes_reference() {
    let Some(backup) = open("backup/ROLAND/FANTOM/BACKUP/Black NARFSOUNDS/FANTOM.SVD") else {
        return;
    };
    let scenes = fantom_core::codec::read_scenes(&backup).unwrap();

    let user_zones: Vec<_> = scenes
        .iter()
        .flat_map(|scene| &scene.zones)
        .filter(|zone| zone.tone.address.lsb < 64)
        .collect();
    let unnamed = user_zones
        .iter()
        .filter(|zone| zone.tone.name().is_none())
        .count();

    assert!(user_zones.len() > 100, "expected a bank full of user tones");
    assert_eq!(
        unnamed,
        0,
        "{unnamed} of {} user-tone references in the backup went unnamed",
        user_zones.len()
    );
}

/// Extracting a scene from a full backup must produce the same bank as extracting it from the
/// scene export the instrument itself wrote.
///
/// The two sources store the same scene with *different* tone addresses and bundle wildly different
/// amounts of data (a 35 MB backup with 2048 user tones versus a 839 KB export with 348). If the
/// repackager reads both correctly, the extracted banks decode identically — same scene, same
/// zones, same tone names, same dense renumbering.
#[test]
fn extracting_a_scene_from_a_backup_matches_extracting_it_from_the_export() {
    let mut compared = 0;
    for (export_path, backup_path) in EXPORT_BACKUP_PAIRS {
        let (Some(export), Some(backup)) = (open(export_path), open(backup_path)) else {
            continue;
        };
        let export_scenes = fantom_core::codec::read_scenes(&export).unwrap();
        let backup_scenes = fantom_core::codec::read_scenes(&backup).unwrap();

        for (export_index, scene) in export_scenes.iter().enumerate() {
            let Some(backup_index) = backup_scenes.iter().position(|s| s.name == scene.name) else {
                continue;
            };
            let from_export =
                fantom_core::repackage::extract_scenes(&export, &[export_index + 1]).unwrap();
            let from_backup =
                fantom_core::repackage::extract_scenes(&backup, &[backup_index + 1]).unwrap();

            let a = fantom_core::codec::read_scenes(&from_export).unwrap();
            let b = fantom_core::codec::read_scenes(&from_backup).unwrap();
            assert_eq!(
                a, b,
                "{export_path}: scene {:?} extracts differently from the backup",
                scene.name
            );

            // The bundled records themselves must match too, not just the scene that points at them.
            assert_eq!(
                fantom_core::codec::read_bundled_tones(&from_export).unwrap(),
                fantom_core::codec::read_bundled_tones(&from_backup).unwrap(),
                "{export_path}: scene {:?} bundles different tones from the backup",
                scene.name
            );
            compared += 1;
        }
    }
    if compared == 0 {
        eprintln!("skipped: no export/backup fixture pairs present");
    }
}

/// Hardware captures of the modelled engines, whose record internals stay opaque.
const OPAQUE_ENGINE_EXPORTS: [&str; 6] = [
    "TONEMAP9_ACB/FANTOM.SVD",   // ACBa, one record
    "TONEMAP7V_ACB2/FANTOM.SVD", // ACBa, two records
    "TONEMAP9_VP/FANTOM.SVD",    // DCWa, one record
    "TONEMAP10_VP/FANTOM.SVD",   // DCWa, two records
    "TONEMAP9_MOD/FANTOM.SVD",   // MDLa, one record
    "TONEMAP10_MOD/FANTOM.SVD",  // MDLa, two records
];

/// What a scene actually plays, ignoring where its user tones happen to be stored.
///
/// Repackaging renumbers bundled records by design, so a zone's LSB/PC legitimately changes. A
/// factory reference has no record to renumber, so its address must survive untouched.
fn playable(scene: &Scene) -> Vec<String> {
    scene
        .zones
        .iter()
        .map(|z| {
            let address = if z.tone.address.lsb < 64 {
                "user".to_owned()
            } else {
                format!("{:?}", z.tone.address)
            };
            format!(
                "{} {} {}..{} lvl{} {} {:?} {address}",
                z.number,
                z.enabled,
                z.key_low,
                z.key_high,
                z.level,
                z.tone.tone_type().label(),
                z.tone.name(),
            )
        })
        .collect()
}

/// Re-extracting every scene of an already self-contained export must change nothing that matters.
///
/// These are the captures that established how `ACBa`, `DCWa`, and `MDLa` are indexed, so they are
/// the regression net for the repackager's opaque path — the one that now encodes an index as
/// LSB *and* PC instead of PC alone.
#[test]
fn extracting_every_scene_of_an_export_reproduces_it() {
    for path in OPAQUE_ENGINE_EXPORTS {
        let Some(raw) = open(path) else { continue };
        let scenes = fantom_core::codec::read_scenes(&raw).unwrap();
        let all: Vec<_> = (1..=scenes.len()).collect();

        let extracted = fantom_core::repackage::extract_scenes(&raw, &all).unwrap();
        let after = fantom_core::codec::read_scenes(&extracted).unwrap();

        assert_eq!(after.len(), scenes.len(), "{path}: scene count changed");
        for (before, after) in scenes.iter().zip(&after) {
            assert_eq!(before.name, after.name, "{path}: scene name changed");
            assert_eq!(before.comment, after.comment, "{path}: comment changed");
            assert_eq!(
                playable(before),
                playable(after),
                "{path}: scene {:?} plays something different after extraction",
                before.name
            );
        }

        // Every record the scenes referenced must still be there, byte for byte.
        let before_tones = fantom_core::codec::read_bundled_tones(&raw).unwrap();
        let after_tones = fantom_core::codec::read_bundled_tones(&extracted).unwrap();
        assert_eq!(
            after_tones.len(),
            before_tones.len(),
            "{path}: bundled tone count changed"
        );
    }
}

/// The banks that were built by this tool and then loaded successfully on a FANTOM-6. Whatever
/// else changes, these must keep reading back the way the instrument accepted them.
#[test]
fn hardware_validated_banks_still_decode() {
    for (path, engine, tone) in [
        ("MERGE_TEST_ACB/FANTOM.SVD", "ACB", "Soft & Subtle2"),
        ("MERGE_TEST_MODEL/FANTOM.SVD", "MODEL", "INITIAL TONE"),
        ("MERGE_TEST_VP/FANTOM.SVD", "VPiano", "Stage Grand3"),
    ] {
        let Some(raw) = open(path) else { continue };
        let scenes = fantom_core::codec::read_scenes(&raw).unwrap();
        assert_eq!(scenes.len(), 3, "{path}: expected a three-scene merge result");
        assert_eq!(
            scenes[0].zones[0].tone.tone_type().label(),
            engine,
            "{path}: wrong engine"
        );
        assert_eq!(scenes[0].zones[0].tone.name(), Some(tone), "{path}");
    }
}

/// `SMPa` slots, the `USDa` waveform directory, and `MLSa` read consistently against each other.
#[test]
fn the_sample_bank_agrees_with_its_waveform_directory() {
    let Some(backup) = open("backup/ROLAND/FANTOM/BACKUP/2023.4.8+topandprisma/FANTOM.SVD") else {
        return;
    };
    let svd = fantom_core::container::Svd::parse(&backup).unwrap();
    let bank = fantom_core::container::read_samples(&backup, &svd).unwrap();

    assert_eq!(bank.slots.len(), 50, "named SMPa slots");
    assert_eq!(bank.data.len(), 50, "SMPd sections found via the USDa directory");
    assert!(bank.orphans().is_empty(), "{:?}", bank.orphans());

    // Every multisample in every fixture is still the factory default.
    assert!(bank.multisamples.is_empty(), "unexpected edited multisample");

    let first = &bank.slots[0];
    assert_eq!(first.name, "1 Beat It - C2");
    assert_eq!(first.original_key, 60);
    let audio = &bank.data[0];
    assert_eq!(audio.sample_rate, 48000);
    // The slot's end point is the recorded length: two 16-bit words per frame.
    assert_eq!(audio.frames(), first.end);

    // A scene export carries no sampling at all.
    let Some(export) = open("backup/ROLAND/SOUND/NARF/FANTOM.SVD") else {
        return;
    };
    let svd = fantom_core::container::Svd::parse(&export).unwrap();
    assert!(fantom_core::container::read_samples(&export, &svd)
        .unwrap()
        .is_empty());
}

/// Panel ground truth for one scene, transcribed from the instrument's own display.
#[test]
fn africa_main_decodes_to_the_panel_display() {
    let Some(backup) = open("backup/ROLAND/FANTOM/BACKUP/Black NARFSOUNDS/FANTOM.SVD") else {
        return;
    };
    let scenes = fantom_core::codec::read_scenes(&backup).unwrap();
    let scene = &scenes[384]; // scene 385 on the panel

    assert_eq!(scene.name, "Africa Main");
    let zones: Vec<_> = scene
        .zones
        .iter()
        .filter(|z| z.enabled)
        .map(|z| (z.tone.name(), z.key_low, z.key_high, z.level))
        .collect();
    assert_eq!(
        zones,
        [
            (Some("Africa Brass"), 0, 71, 107),
            (Some("Africa Kalimba"), 73, 127, 107),
            (Some("Africa Kalimba"), 72, 72, 100),
            (Some("JX Cream"), 0, 71, 82),
        ]
    );
}
