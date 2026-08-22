//! The dependency closure, read off real FANTOM files.
//!
//! The unit tests in `requirements` assert against hand-built values; these assert against files an
//! instrument wrote and against a commercial pack whose requirements are independently known — the
//! NARF pack's instructions tell its buyers to load its samples into slots 1–50, and nothing in
//! this crate was told that.

use std::collections::HashMap;

mod support;
use support::{private, public};

use fantom_core::model::Scene;
use fantom_core::requirements::{self, Reader, Verdict};

const NARF_EXPORT: &str = "backup/ROLAND/SOUND/NARF/FANTOM.SVD";
const NARF_BACKUP: &str = "backup/ROLAND/FANTOM/BACKUP/Black NARFSOUNDS/FANTOM.SVD";

/// Every scene-export bank paired with the full backup it was exported from.
const EXPORT_BACKUP_PAIRS: [(&str, &str); 3] = [
    (NARF_EXPORT, NARF_BACKUP),
    (
        "backup/ROLAND/SOUND/TOP80/FANTOM.SVD",
        "backup/ROLAND/FANTOM/BACKUP/2023.4.8+topandprisma/FANTOM.SVD",
    ),
    (
        "backup/ROLAND/SOUND/PRISMA/FANTOM.SVD",
        "backup/ROLAND/FANTOM/BACKUP/2023.4.8+topandprisma/FANTOM.SVD",
    ),
];

fn slots(requirements: &[requirements::SlotRequirement]) -> Vec<u16> {
    requirements.iter().map(|slot| slot.slot).collect()
}

/// A committed fixture, so this one always runs: a scene playing one factory preset and nothing
/// else needs no user content at all, and says so rather than staying quiet.
#[test]
fn a_scene_playing_only_a_preset_asks_for_nothing_installable() {
    let needs = requirements::requirements(&public("tests/TEST 1/FANTOM.SVD")).unwrap();

    assert!(needs.samples.is_empty());
    assert!(needs.multisamples.is_empty());
    assert!(needs.wave_expansions.is_empty());
    assert_eq!(needs.missing_tones().count(), 0);
    assert!(!needs.carries_audio);

    // The preset it plays is still a dependency — just not one anybody has to act on.
    assert!(!needs.banks.is_empty());
    assert!(needs.banks.iter().all(|bank| bank.is_factory()));
    assert!(!needs.needs_installed_content());
}

/// The requirement a commercial pack states in prose, derived from its bytes alone.
///
/// NARF ships with instructions telling the buyer to clear user sample slots 1–50 and load its
/// audio there. That is exactly the closure this computes — every referenced slot, including the
/// ones a tone reaches only through a drum kit or a multisample, and not one slot more.
#[test]
fn a_pack_states_the_slots_it_demands_and_the_bytes_agree() {
    let Some(pack) = private(NARF_EXPORT) else {
        return;
    };
    let needs = requirements::requirements(&pack).unwrap();

    assert_eq!(slots(&needs.samples), (1..=50).collect::<Vec<u16>>());
    // A scene bank carries no audio, which is why the slots are a requirement rather than content.
    assert!(!needs.carries_audio);
    assert!(needs.samples.iter().all(|sample| !sample.carried));
    // Every sample names the tone that goes silent without it.
    assert!(needs
        .samples
        .iter()
        .all(|sample| !sample.played_by.is_empty()));

    // The pack bundles every user tone its scenes play; nothing is left dangling.
    assert_eq!(needs.missing_tones().count(), 0);

    // And it needs installed content its instructions do not mention at all.
    assert!(!needs.wave_expansions.is_empty());
    assert!(needs.needs_installed_content());
}

/// The oracle: a scene exported to a bank and the same scene sitting in the backup it came from
/// must ask for the same user samples.
///
/// This is a real test rather than a tautology because the two files store the reference
/// differently — an export renumbers the tones it bundles densely while a backup keeps them at
/// their USER-bank slots, so reaching the same sample means resolving a different address in a
/// different area on each side. It also crosses container scopes: the backup carries the audio and
/// the export does not, and the *requirement* is the same either way.
#[test]
fn an_export_and_its_backup_agree_on_what_each_scene_needs() {
    let mut checked = 0;
    for (export_path, backup_path) in EXPORT_BACKUP_PAIRS {
        let (Some(export), Some(backup)) = (private(export_path), private(backup_path)) else {
            continue;
        };
        let mut compared = 0;
        let export_scenes = fantom_core::codec::read_scenes(&export).unwrap();
        let backup_scenes = fantom_core::codec::read_scenes(&backup).unwrap();
        let by_name: HashMap<&str, &Scene> = backup_scenes
            .iter()
            .map(|scene| (scene.name.as_str(), scene))
            .collect();

        let from_export = Reader::open(&export).unwrap();
        let from_backup = Reader::open(&backup).unwrap();
        for scene in &export_scenes {
            let Some(same) = by_name.get(scene.name.as_str()) else {
                continue;
            };
            assert_eq!(
                slots(&from_export.scene(scene).samples),
                slots(&from_backup.scene(same).samples),
                "scene {:?} needs different samples read from {export_path} and {backup_path}",
                scene.name
            );
            compared += 1;
        }
        // Matching no scene at all would pass every assertion above without testing anything.
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

/// A bank weighed against the instrument it was exported from should find everything it needs
/// there — and should still refuse to claim more than a file can prove.
#[test]
fn a_bank_checked_against_its_own_instrument_finds_its_samples() {
    let (Some(pack), Some(instrument)) = (private(NARF_EXPORT), private(NARF_BACKUP)) else {
        return;
    };
    let held = Reader::open(&instrument).unwrap().inventory();
    // The bank knows the numbers it needs but not what was in them; the backup names them.
    let needs = requirements::requirements(&pack).unwrap().named_from(&held);
    assert_eq!(needs.samples[21].name.as_deref(), Some("doh duh 2"));

    let findings = requirements::compare(&needs, &held);
    let met = |kind: &str| {
        findings
            .iter()
            .filter(|f| f.requirement.starts_with(kind) && f.verdict == Verdict::Met)
            .count()
    };
    assert_eq!(met("sample slot"), 50);
    // Nothing may be reported as the wrong audio when it is the same audio.
    assert!(!findings.iter().any(|f| f.verdict == Verdict::Differs));
    // The expansions the pack plays cannot be confirmed from any file, and are not pretended away.
    assert!(findings
        .iter()
        .any(|f| f.requirement.starts_with("wave expansion") && f.verdict == Verdict::Unknown));
}

/// What an extracted bank needs is what the scenes in it needed, no more.
///
/// Repackaging renumbers the tones it bundles, so the closure has to survive being rebuilt: the
/// samples the output asks for are the ones those scenes asked for in the source, and every tone
/// it points at came with it.
#[test]
fn a_rebuilt_bank_asks_for_exactly_what_its_scenes_asked_for() {
    let Some(source) = private(NARF_EXPORT) else {
        return;
    };
    let scenes = [12, 13];
    let reader = Reader::open(&source).unwrap();
    let decoded = fantom_core::codec::read_scenes(&source).unwrap();
    let mut wanted: Vec<u16> = scenes
        .iter()
        .flat_map(|&number| slots(&reader.scene(&decoded[number - 1]).samples))
        .collect();
    wanted.sort_unstable();
    wanted.dedup();

    let rebuilt = fantom_core::repackage::extract_scenes(&source, &scenes).unwrap();
    let needs = requirements::requirements(&rebuilt).unwrap();

    assert_eq!(slots(&needs.samples), wanted);
    assert_eq!(needs.missing_tones().count(), 0);
    assert!(!needs.user_tones.is_empty());
}

/// A tone bank carries the audio its tones play, so it asks its destination for none of it.
#[test]
fn a_tone_bank_carrying_its_audio_needs_none_of_it() {
    let Some(bank) = private("EXPORT_Z-Core2.svz") else {
        return;
    };
    let needs = requirements::requirements(&bank).unwrap();

    assert!(needs.carries_audio);
    assert!(!needs.samples.is_empty());
    assert_eq!(needs.missing_samples().count(), 0);
    assert!(requirements::compare(&needs, &requirements::Inventory::default()).is_empty());
}
