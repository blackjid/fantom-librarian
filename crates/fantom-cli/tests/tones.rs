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

fn acb_record(path: &Path) -> Vec<u8> {
    let raw = Raw::open(path).unwrap();
    let svd = Svd::parse(&raw).unwrap();
    let table = RecordTable::from_svd(&raw, &svd, b"ACBa").unwrap().unwrap();
    table.record(0).unwrap().to_vec()
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
