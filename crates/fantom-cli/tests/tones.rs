//! `tones set` against the instrument's own edit.
//!
//! Private fixtures only, so this skips without the corpus; `FANTOM_FIXTURES=require` makes a
//! missing file fail, as in `fantom-core`'s fixture tests.

use std::path::{Path, PathBuf};
use std::process::Command;

use fantom_core::container::{Raw, RecordTable, Svd};

fn private(relative: &str) -> Option<PathBuf> {
    let root = match std::env::var("FANTOM_FIXTURES_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures-local"),
    };
    let path = root.join(relative);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var("FANTOM_FIXTURES").map_or(true, |v| v != "require"),
        "private fixture missing: {}",
        path.display()
    );
    eprintln!("skipping: {} not present", path.display());
    None
}

fn records(path: &Path, tag: &[u8; 4]) -> Vec<Vec<u8>> {
    let raw = Raw::open(path).unwrap();
    let svd = Svd::parse(&raw).unwrap();
    let table = RecordTable::from_svd(&raw, &svd, tag).unwrap().unwrap();
    table.records().map(<[u8]>::to_vec).collect()
}

fn acb_record(path: &Path) -> Vec<u8> {
    records(path, b"ACBa").swap_remove(0)
}

/// TONEMAP4 -> TONEMAP5 was five panel edits; making them by name reproduces the ACB record.
#[test]
fn setting_the_hardware_edits_reproduces_the_instruments_record() {
    let (Some(before), Some(after)) = (
        private("TONEMAP4_ACB/FANTOM.SVD"),
        private("TONEMAP5_ACB/FANTOM.SVD"),
    ) else {
        return;
    };
    let out = std::env::temp_dir().join(format!("fantom-cli-tones-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let edited = out.join("FANTOM.SVD");

    let status = Command::new(env!("CARGO_BIN_EXE_fantom"))
        .args(["tones", "set"])
        .arg(&before)
        .args([
            "0",
            "LFO_rate=142",
            "OSC1_wave=0",
            "FLT_lpf=144",
            "FLT_sus=176",
            "PAT_NAME_efx_type=3",
        ])
        .arg("-o")
        .arg(&edited)
        .status()
        .unwrap();
    assert!(status.success());

    assert!(
        acb_record(&edited) == acb_record(&after),
        "ACBa record differs from the instrument's"
    );
    let (ours, theirs) = (
        std::fs::read(&edited).unwrap(),
        std::fs::read(&after).unwrap(),
    );
    let differing = ours.iter().zip(&theirs).filter(|(a, b)| a != b).count();
    eprintln!(
        "whole file: {} vs {} bytes, {differing} bytes differ",
        ours.len(),
        theirs.len()
    );
    assert_eq!(ours.len(), theirs.len());
    assert!(
        ours == theirs,
        "{differing} bytes outside the record differ"
    );
}

const MODEL_CAPTURES: [&str; 9] = [
    "TONEMAP4MOD",
    "TONEMAP4MOD_old",
    "TONEMAP4_MOD",
    "TONEMAP5_MOD",
    "TONEMAP6_MOD",
    "TONEMAP8_MOD",
    "TONEMAP9_MOD",
    "TONEMAP9_MOD2",
    "TONEMAP10_MOD",
];

/// Between any two MODEL captures of one tone, setting the engine parameters that differ must
/// give the instrument's record, ZEN-Core mirrors included. Pairs that differ in a locked
/// parameter or outside the engine cannot be reproduced and are reported, not asserted.
#[test]
fn setting_the_engine_edits_reproduces_the_instruments_model_record() {
    use fantom_core::mdl::{read_mdl_params, MdlEdit};
    let mut files = Vec::new();
    for name in MODEL_CAPTURES {
        let Some(path) = private(&format!("{name}/FANTOM.SVD")) else {
            return;
        };
        files.push((name, path));
    }
    let out = std::env::temp_dir().join(format!("fantom-cli-mdl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();

    let (mut reproduced, mut skipped) = (Vec::new(), Vec::new());
    let mut name_only = 0;
    for (from, before) in &files {
        for (to, after) in &files {
            let (a, b) = (records(before, b"MDLa"), records(after, b"MDLa"));
            if from == to || a[0][..16] != b[0][..16] {
                continue;
            }
            let read = |p: &Path| read_mdl_params(&Raw::open(p).unwrap(), 0).unwrap().1;
            let (fa, fb) = (read(before), read(after));
            let changed: Vec<_> = fa
                .iter()
                .zip(&fb)
                .filter(|(x, y)| x.value != y.value)
                .collect();
            if changed.is_empty() {
                continue;
            }
            let label = format!("{from} -> {to}");
            if let Some((x, _)) = changed.iter().find(|(x, _)| x.edit == MdlEdit::Locked) {
                skipped.push(format!("{label}: {} is locked", x.param.id));
                continue;
            }
            let edited = out.join("FANTOM.SVD");
            let status = Command::new(env!("CARGO_BIN_EXE_fantom"))
                .args(["tones", "set", "--area", "MDLa"])
                .arg(before)
                .arg("0")
                .args(
                    changed
                        .iter()
                        .map(|(_, y)| format!("{}={}", y.param.id, y.value)),
                )
                .arg("-o")
                .arg(&edited)
                .status()
                .unwrap();
            assert!(status.success(), "{label}");
            let mine = records(&edited, b"MDLa").swap_remove(0);
            if mine != b[0] {
                // The tone name, category and MFX are not engine parameters; everything else must agree.
                let masked = |r: &[u8]| {
                    let mut r = r.to_vec();
                    r[0x10..0x21].fill(0);
                    r[0x50..0x84].fill(0);
                    r
                };
                let (x, y) = (masked(&mine), masked(&b[0]));
                let at: Vec<String> = (0..x.len())
                    .filter(|&i| x[i] != y[i])
                    .map(|i| format!("{i:#x}"))
                    .collect();
                assert!(
                    at.is_empty(),
                    "{label}: differs outside name and MFX at {at:?}"
                );
                name_only += 1;
                skipped.push(format!("{label}: only the name, category or MFX differs"));
                continue;
            }
            if std::fs::read(&edited).unwrap() == std::fs::read(after).unwrap() {
                reproduced.push(format!("{label} (whole file)"));
            } else {
                reproduced.push(format!("{label} (record only)"));
            }
        }
    }
    eprintln!("reproduced:\n  {}", reproduced.join("\n  "));
    eprintln!(
        "{} reproduced, {name_only} equal but for name/MFX",
        reproduced.len()
    );
    assert!(
        skipped.iter().all(|s| !s.contains("is locked")),
        "{skipped:?}"
    );
    eprintln!("not reproducible:\n  {}", skipped.join("\n  "));
    assert!(
        reproduced
            .iter()
            .any(|r| r.starts_with("TONEMAP4MOD -> TONEMAP5_MOD")),
        "the four-edit pair must reproduce"
    );
    assert!(
        reproduced
            .iter()
            .any(|r| r == "TONEMAP4_MOD -> TONEMAP5_MOD (whole file)"),
        "the same edits reproduce the whole file"
    );
}
