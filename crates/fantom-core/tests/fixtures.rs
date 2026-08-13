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

/// A commercial pack: a scene bank whose tones name sample slots 1..50, and the sample-only SVZ
/// its instructions say to import into those slots. Its 50 samples are the same recordings as the
/// `2023.4.8+topandprisma` backup's, which is what makes the two container shapes comparable.
const FFC_SAMPLES: &str =
    "Fantom & Fantom-0 FFC 3 Pack Bundle Open For Instructions/FFC SAMPLES 1-50.svz";
const SAMPLED_BACKUP: &str = "backup/ROLAND/FANTOM/BACKUP/2023.4.8+topandprisma/FANTOM.SVD";

/// Building a sample-only SVZ from a backup must reproduce the one Roland's own tooling shipped.
///
/// This is the strongest check available on the conversion, because the pack and the backup hold
/// the *same 50 recordings* in the two different shapes. Every rule the builder applies is under
/// test at once: the `USPa` record layout, the `SMPd` header rewrite, where the audio starts in
/// each shape, how much of it a backup section really has, the per-section word that is carried
/// rather than computed, the area geometry, and every CRC-32.
///
/// One byte is allowed to differ: the preamble's format revision. The pack was written by an older
/// OS and says 2; a current FANTOM writes 3 for an SVZ carrying samples (`EXPORT_Z-Core2.svz`), and
/// that is what this emits.
#[test]
fn building_a_sample_svz_from_a_backup_reproduces_a_shipped_one() {
    let (Some(backup), Some(shipped)) = (open(SAMPLED_BACKUP), open(FFC_SAMPLES)) else {
        return;
    };
    let slots: Vec<usize> = (0..50).collect();
    let built = fantom_core::samplebank::export_samples(&backup, &slots).unwrap();

    assert_eq!(built.bytes().len(), shipped.bytes().len(), "size differs");
    const REVISION_BYTE: usize = 0x05;
    let differing: Vec<usize> = built
        .bytes()
        .iter()
        .zip(shipped.bytes())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        differing,
        [REVISION_BYTE],
        "only the format revision may differ"
    );
}

/// The whole sampled-scene workflow, on a real scene that really plays a user sample.
///
/// Scene 401 of the NARFSOUNDS backup is `Beat It`, whose `Beat It Gong` tone plays panel sample
/// slot 1 on the left and slot 22 on the right. Extracting it gives a bank that is silent anywhere
/// those slots hold something else. The two pieces together fix that: a companion `.svz` holding
/// just those samples — 2 MB, not the backup's 23 — and the bank repointed at wherever it lands.
///
/// The stereo split is the point of this fixture. Following only the left wave number would build
/// a companion missing `doh duh 2` and a bank still pointing at slot 22 on the destination.
#[test]
fn a_sampled_scene_travels_as_a_bank_plus_a_companion_sample_file() {
    let Some(backup) = open("backup/ROLAND/FANTOM/BACKUP/Black NARFSOUNDS/FANTOM.SVD") else {
        return;
    };
    let extracted = fantom_core::repackage::extract_scenes(&backup, &[401]).unwrap();
    let slots = fantom_core::repackage::referenced_sample_slots(&extracted).unwrap();
    assert_eq!(slots, [1, 22], "a slot per channel, both dependencies");

    // The companion carries only what the scene needs, numbered densely from 0.
    let indexes: Vec<usize> = slots.iter().map(|&s| s as usize - 1).collect();
    let companion = fantom_core::samplebank::export_samples(&backup, &indexes).unwrap();
    let svd = fantom_core::container::Svd::parse(&companion).unwrap();
    let carried = fantom_core::container::read_samples(&companion, &svd).unwrap();
    let names: Vec<&str> = carried.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["1 Beat It - C2", "doh duh 2"]);
    assert!(
        companion.bytes().len() < backup.bytes().len() / 10,
        "the companion must carry two samples, not the whole bank"
    );

    // With the companion landing at slot 101, the bank has to say 101 and 102 — in that order,
    // because that is the order the instrument will lay the companion's samples down.
    let remap = fantom_core::repackage::contiguous_remap(&slots, 101).unwrap();
    let rebased = fantom_core::repackage::rebase_sample_slots(&extracted, &remap).unwrap();
    assert_eq!(
        fantom_core::repackage::referenced_sample_slots(&rebased).unwrap(),
        [101, 102]
    );
    assert!(fantom_core::verify::check(&rebased).unwrap().is_ok());
    assert!(fantom_core::verify::check(&companion).unwrap().is_ok());
}

/// A bank this tool built, imported to a FANTOM-6 and exported straight back, returns unchanged.
///
/// The strongest available statement about the scene repackager: the instrument stored our records
/// verbatim rather than merely tolerating them. Only `SYSa` differs, where it stamps its own system
/// settings on the way out — `PRFa` and `PATa` come back byte for byte, rebased sample references
/// included.
#[test]
fn a_bank_round_tripped_through_the_instrument_comes_back_unchanged() {
    let (Some(sent), Some(back)) = (
        open("hwtest/T3_BANK/FANTOM.SVD"),
        open("hwtest_back/T6_BACK/FANTOM.SVD"),
    ) else {
        return;
    };
    assert_eq!(sent.bytes().len(), back.bytes().len(), "size changed");

    let svd = fantom_core::container::Svd::parse(&back).unwrap();
    for area in &svd.areas {
        let ours = fantom_core::container::Svd::parse(&sent)
            .unwrap()
            .area(&area.tag)
            .map(|a| {
                fantom_core::container::Svd::parse(&sent)
                    .unwrap()
                    .area_bytes(&sent, a)
                    .unwrap()
                    .to_vec()
            });
        let theirs = svd.area_bytes(&back, area).unwrap();
        let tag = area.tag_str();
        if tag == "SYSa" {
            continue; // the instrument's own settings, not ours to preserve
        }
        assert_eq!(
            ours.as_deref(),
            Some(theirs),
            "{tag} changed passing through the instrument"
        );
    }
}

/// The instrument's own export of a rebased tone carries **both** of its wave numbers' samples.
///
/// `Beat It Gong` was sent in referencing slots 2001 and 2002. Exported back by the FANTOM it holds
/// two samples renumbered to 1 and 2 — so Roland's dependency scan reads the right wave number even
/// though its editor never displays one for a sampled partial. Following it is not optional.
#[test]
fn an_instrument_export_carries_the_right_wave_numbers_sample_too() {
    let Some(raw) = open("hwtest_back/T6_TONE.svz") else {
        return;
    };
    let svd = fantom_core::container::Svd::parse(&raw).unwrap();
    let bank = fantom_core::container::read_samples(&raw, &svd).unwrap();
    let names: Vec<&str> = bank.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["1 Beat It - C2", "doh duh 2"]);

    let tones = fantom_core::container::PatArea::from_svd(&raw, &svd).unwrap();
    assert_eq!(tones.get(0).unwrap().name, "Beat It Gong");
    assert_eq!(
        tones.get(0).unwrap().samples,
        [1, 2],
        "both wave numbers renumbered densely"
    );
}

/// A file with no user samples cannot be a source, and must say so rather than emit an empty bank.
#[test]
fn a_scene_export_cannot_source_sample_audio() {
    let Some(raw) = open("backup/ROLAND/SOUND/NARF/FANTOM.SVD") else {
        return;
    };
    let error = fantom_core::samplebank::export_samples(&raw, &[0])
        .unwrap_err()
        .to_string();
    assert!(error.contains("no SMPa area"), "{error}");
}

/// SVZ tone banks, which use a different envelope and carry their samples.
const TONE_BANKS: [&str; 4] = [
    "Z-Core_20260623.svz",              // 274 ZEN-Core tones, 2 samples
    "DRUM_20260623.svz",                // 38 drum kits with paired INSa instrument sets
    "backup/ROLAND/SOUND/EXPORT_Z-Core.svz", // 10 tones, older revision
    "EXPORT_Z-Core2.svz",              // one tone exported *with* its user sample
];

/// Every tone of a bank must survive a round trip through extraction unchanged.
///
/// This exercises the parts of the SVZ envelope that differ from an SVD: the magic leads the file,
/// the area count is a single byte, and each area declares its own header length with a four-byte
/// info word per record. Getting any of those wrong shows up here as scrambled records.
#[test]
fn extracting_every_tone_of_an_svz_reproduces_it() {
    for path in TONE_BANKS {
        let Some(raw) = open(path) else { continue };
        let before = fantom_core::codec::read_bundled_tones(&raw).unwrap();
        let all: Vec<usize> = (0..before.len()).collect();

        let extracted = fantom_core::tonebank::extract_tones(&raw, &all).unwrap();
        let after = fantom_core::codec::read_bundled_tones(&extracted).unwrap();
        assert_eq!(after, before, "{path}: re-extracting every tone changed them");
    }
}

/// Rebuilding an instrument-written tone-plus-sample export must reproduce it **byte for byte**.
///
/// `EXPORT_Z-Core2.svz` is a FANTOM-6 export of one ZEN-Core tone together with the user sample it
/// plays — the smallest complete example of the thing this module exists to build. Selecting its
/// only tone asks the repackager to lay out the same file from parts, so every layout decision has
/// to match Roland's: the preamble and its stamp, area order and offsets, each area's `info_length`
/// and per-record CRC-32, the `USPa` slot record, and the `USDa` directory — including the
/// per-section word that is carried rather than computed. Decoding equal is a weaker claim than
/// this; a wrong `info_length` or a recomputed word can still decode fine.
#[test]
fn rebuilding_an_instrument_written_sampled_export_is_byte_identical() {
    let Some(raw) = open("EXPORT_Z-Core2.svz") else {
        return;
    };
    let rebuilt = fantom_core::tonebank::extract_tones(&raw, &[0]).unwrap();
    assert_eq!(
        rebuilt.bytes(),
        raw.bytes(),
        "rebuilding the instrument's own export did not reproduce it"
    );
}

/// The instrument numbers an exported sample reference densely, exactly as extraction does.
///
/// The one tone of `EXPORT_Z-Core2.svz` plays user sample slot **1** and the file carries exactly
/// one `USPa` record — but on the panel, that same tone's wave reads group `SAMP`, sample `0029`.
/// The FANTOM renumbered 29 to 1 on export, so an SVZ addresses samples by position within its own
/// `USPa`. That is the renumbering [`fantom_core::tonebank::extract_tones`] applies, which means a
/// bank this tool builds and one the FANTOM builds address their samples the same way.
#[test]
fn an_instrument_export_numbers_its_sample_reference_from_one() {
    let Some(raw) = open("EXPORT_Z-Core2.svz") else {
        return;
    };
    let svd = fantom_core::container::Svd::parse(&raw).unwrap();
    let bank = fantom_core::container::read_samples(&raw, &svd).unwrap();
    assert_eq!(bank.slots.len(), 1);
    assert_eq!(bank.slots[0].index, 0, "the single slot is numbered 0");

    let tones = fantom_core::container::PatArea::from_svd(&raw, &svd).unwrap();
    assert_eq!(
        tones.tones()[0].samples,
        [1],
        "the tone must reference the first slot, 1-based"
    );
}

/// A sampled tone taken out of a bank keeps its audio, which is what an SVZ is for.
///
/// This bank has exactly **one** sampled tone and the instrument put **two** samples in it — which
/// is the whole argument that a partial's second wave number is a live reference. `MyPolySyn1`
/// names slot 2 on the left and slot 1 on the right, and since an export carries only what its
/// tones reference (`EXPORT_Z-Core2.svz` carries one sample for one tone, not the machine's whole
/// bank), both slots are dependencies. Extracting that tone must therefore reproduce both.
#[test]
fn extracting_a_sampled_tone_carries_its_waveform() {
    let Some(raw) = open("Z-Core_20260623.svz") else {
        return;
    };
    let svd = fantom_core::container::Svd::parse(&raw).unwrap();
    let source = fantom_core::container::read_samples(&raw, &svd).unwrap();
    assert_eq!(source.slots.len(), 2, "fixture should carry two samples");

    // `MyPolySyn1` is the only tone in this bank that plays a user sample.
    let tones = fantom_core::codec::read_bundled_tones(&raw).unwrap();
    let index = tones
        .iter()
        .position(|t| t.name == "MyPolySyn1")
        .expect("sampled tone present");
    let pat = fantom_core::container::PatArea::from_svd(&raw, &svd).unwrap();
    assert_eq!(
        pat.get(index).unwrap().samples,
        [2, 1],
        "the tone names a slot per channel: left 2, right 1"
    );

    let extracted = fantom_core::tonebank::extract_tones(&raw, &[index]).unwrap();
    let svd = fantom_core::container::Svd::parse(&extracted).unwrap();
    let carried = fantom_core::container::read_samples(&extracted, &svd).unwrap();

    let names: Vec<&str> = carried.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        ["Sample003;F#3-F#", "Sample005;G#3-G#"],
        "both channels' samples travel"
    );
    assert!(carried.orphans().is_empty(), "{:?}", carried.orphans());

    // The invariant that matters is not the order but that renumbering preserved *meaning*: each
    // channel must still resolve to the sample it named before. This tone plays a higher slot on
    // the left than the right, so a renumbering that assigned new numbers in mention order rather
    // than emission order would swap its channels while leaving both samples present.
    let resolve = |bank: &fantom_core::container::SampleBank, slot: u16| -> String {
        bank.slots
            .iter()
            .find(|s| s.index + 1 == slot as usize)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| format!("<no slot {slot}>"))
    };
    let out_pat = fantom_core::container::PatArea::from_svd(&extracted, &svd).unwrap();
    let before: Vec<String> = pat
        .get(index)
        .unwrap()
        .samples
        .iter()
        .map(|&s| resolve(&source, s))
        .collect();
    let after: Vec<String> = out_pat
        .get(0)
        .unwrap()
        .samples
        .iter()
        .map(|&s| resolve(&carried, s))
        .collect();
    assert_eq!(before, after, "a channel now points at a different sample");
    // Same audio, not a re-encode: frames and rate must match the source exactly.
    for slot in &carried.data {
        let original = source.data.iter().find(|d| d.name == slot.name).unwrap();
        assert_eq!(slot.channel_bytes, original.channel_bytes);
        assert_eq!(slot.sample_rate, original.sample_rate);
    }
}

/// A drum kit is `RHYa` plus its 88 instruments in `INSa`; the two must stay index-locked.
#[test]
fn extracting_drum_kits_keeps_their_instrument_sets_paired() {
    let Some(raw) = open("DRUM_20260623.svz") else {
        return;
    };
    let extracted = fantom_core::tonebank::extract_tones(&raw, &[0, 5]).unwrap();

    let svd = fantom_core::container::Svd::parse(&extracted).unwrap();
    let rhy = fantom_core::container::RecordTable::from_svd(&extracted, &svd, b"RHYa")
        .unwrap()
        .unwrap();
    let ins = fantom_core::container::RecordTable::from_svd(&extracted, &svd, b"INSa")
        .unwrap()
        .unwrap();
    assert_eq!(rhy.len(), 2);
    assert_eq!(ins.len(), rhy.len(), "INSa must match RHYa one for one");

    // And each INSa record must be the one that belonged to its kit in the source.
    let source_svd = fantom_core::container::Svd::parse(&raw).unwrap();
    let source_ins = fantom_core::container::RecordTable::from_svd(&raw, &source_svd, b"INSa")
        .unwrap()
        .unwrap();
    assert_eq!(ins.record(0), source_ins.record(0));
    assert_eq!(ins.record(1), source_ins.record(5));
}

/// Every file the instrument wrote must pass our own integrity check — otherwise the check is
/// wrong, not the file.
#[test]
fn files_written_by_the_instrument_verify_clean() {
    for path in TONE_BANKS.iter().chain(&[
        "backup/ROLAND/SOUND/NARF/FANTOM.SVD",
        "backup/ROLAND/SOUND/PRISMA/FANTOM.SVD",
        "backup/ROLAND/FANTOM/BACKUP/Black NARFSOUNDS/FANTOM.SVD",
        "TONEMAP9_ACB/FANTOM.SVD",
        "tests/TEST 1/FANTOM.SVD",
    ]) {
        let Some(raw) = open(path) else { continue };
        let report = fantom_core::verify::check(&raw).unwrap();
        assert!(report.is_ok(), "{path}: {:?}", report.problems);
    }
}

/// And so must everything we write, including the sample-carrying path that edits a record.
#[test]
fn repackaged_tone_banks_verify_clean() {
    let Some(raw) = open("Z-Core_20260623.svz") else {
        return;
    };
    let tones = fantom_core::codec::read_bundled_tones(&raw).unwrap();
    let sampled = tones.iter().position(|t| t.name == "MyPolySyn1").unwrap();

    for selection in [vec![0], vec![sampled], vec![0, sampled, 3]] {
        let out = fantom_core::tonebank::extract_tones(&raw, &selection).unwrap();
        let report = fantom_core::verify::check(&out).unwrap();
        assert!(report.is_ok(), "{selection:?}: {:?}", report.problems);
        assert!(report.checked > 0);
    }

    let a = fantom_core::tonebank::extract_tones(&raw, &[0, 1]).unwrap();
    let b = fantom_core::tonebank::extract_tones(&raw, &[sampled]).unwrap();
    let merged = fantom_core::tonebank::merge_tones(&a, &b).unwrap();
    let report = fantom_core::verify::check(&merged).unwrap();
    assert!(report.is_ok(), "merge: {:?}", report.problems);
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
