//! `pads list` and `pads wav` on a backup's pad set.
//!
//! Private fixtures only, so this skips without the corpus; `FANTOM_FIXTURES=require` makes a
//! missing file fail, as in `fantom-core`'s fixture tests.

use std::path::{Path, PathBuf};
use std::process::Command;

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

fn run(args: &[&std::ffi::OsStr]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_fantom"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn lists_pads_and_writes_a_wav() {
    let Some(dir) = private("backup/ROLAND/FANTOM/BACKUP/Black NARFSOUNDS") else {
        return;
    };
    let text = run(&["pads".as_ref(), "list".as_ref(), dir.as_ref()]);
    assert!(text.contains("1-01  A_Drums_80"), "{text}");
    assert!(text.contains("stereo"), "{text}");
    assert_eq!(text.lines().count(), 66, "{text}");

    let wav = std::env::temp_dir().join(format!("fantom-pads-{}.wav", std::process::id()));
    let svd = dir.join("FANTOM.SVD");
    run(&[
        "pads".as_ref(),
        "wav".as_ref(),
        svd.as_ref(),
        "1".as_ref(),
        "1".as_ref(),
        wav.as_ref(),
    ]);
    let bytes = std::fs::read(&wav).unwrap();
    std::fs::remove_file(&wav).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(bytes.len(), 44 + 576_001 * 4);
}
