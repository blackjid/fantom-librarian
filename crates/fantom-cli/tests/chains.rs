//! `scenes chains` lists a backup's scene chains with the scene each step plays.
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

fn chains(backup: &Path, all: bool) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fantom"));
    cmd.args(["scenes", "chains"]).arg(backup);
    if all {
        cmd.arg("--all");
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn top80_lists_its_numbered_chains_and_hides_working_files() {
    let Some(dir) = private("backup/ROLAND/FANTOM/BACKUP/Fantom-0 TOP80") else {
        return;
    };
    let text = chains(&dir, false);
    assert!(text.starts_with("19 chains:"), "{text}");
    assert!(text.contains("Chain 1 \"QUEEN ANDREA\""), "{text}");
    assert!(!text.contains("Tmp"), "{text}");
    assert!(!text.contains("(no such scene)"), "{text}");

    let all = chains(&dir, true);
    assert!(all.starts_with("21 chains:"), "{all}");
    assert!(all.contains("Chain Tmp \"DGM repay\" (3 steps)"), "{all}");
}

#[test]
fn markers_show_on_their_steps() {
    let Some(dir) = private("TESTv1") else {
        return;
    };
    let text = chains(&dir, false);
    assert!(
        text.contains("[marker 1]") && text.contains("[marker 3]"),
        "{text}"
    );
}
