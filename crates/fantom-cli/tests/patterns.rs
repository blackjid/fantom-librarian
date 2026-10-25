//! Scene commands carry each scene's sequencer patterns, checked against the instrument's own export.
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

fn files_under(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in std::fs::read_dir(next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "SeqSet.SQS") {
                let bytes = std::fs::read(&path).unwrap();
                out.push((path.strip_prefix(dir).unwrap().to_path_buf(), bytes));
            }
        }
    }
    out.sort();
    out
}

/// PRISMA was exported from the backup's C100–C115 (scenes 356–371). Extracting those scenes from
/// the backup must lay their patterns out exactly as EXPORT SCENE did: renumbered from `A/001`.
#[test]
fn extracting_scenes_lays_out_their_patterns_like_export_scene() {
    let (Some(backup), Some(export)) = (
        private("backup/ROLAND/FANTOM/BACKUP/2023.4.8+topandprisma/FANTOM.SVD"),
        private("backup/ROLAND/SOUND/PRISMA/SEQ"),
    ) else {
        return;
    };
    let out = std::env::temp_dir().join(format!("fantom-cli-prisma-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fantom"))
        .args(["scenes", "extract"])
        .arg(&backup)
        .args((356..=371).map(|n| n.to_string()))
        .arg("-o")
        .arg(out.join("FANTOM.SVD"))
        .status()
        .unwrap();
    assert!(status.success());

    let ours = files_under(&out.join("SEQ"));
    assert_eq!(ours.len(), 16);
    assert!(
        ours == files_under(&export),
        "patterns differ from the instrument's export"
    );
}

/// `scenes show` lists the patterns recorded beside the scene.
#[test]
fn showing_a_scene_lists_its_patterns() {
    let Some(export) = private("backup/ROLAND/SOUND/PRISMA/FANTOM.SVD") else {
        return;
    };
    let output = Command::new(env!("CARGO_BIN_EXE_fantom"))
        .args(["scenes", "show"])
        .arg(&export)
        .arg("1")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("T07-2 INIT PTN         8 bars"), "{text}");
    assert_eq!(
        text.lines().filter(|l| l.ends_with(" bars")).count(),
        5,
        "{text}"
    );
}

fn fantom(args: &[&str], file: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_fantom"))
        .args(args)
        .arg(file)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

/// `scenes show` lists a scene's pattern groups and song steps; `scenes pattern` dumps one pattern.
#[test]
fn showing_a_scene_lists_its_song_and_a_pattern_dumps() {
    let Some(narf) = private("backup/NARF/FANTOM.SVD") else {
        return;
    };
    let list = fantom(&["scenes", "list"], &narf);
    let number = list
        .lines()
        .find(|l| l.contains("Blue Monday"))
        .and_then(|l| l.split_whitespace().next())
        .unwrap()
        .to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_fantom"))
        .args(["scenes", "show"])
        .arg(&narf)
        .arg(&number)
        .output()
        .unwrap();
    let text = String::from_utf8(output.stdout).unwrap();
    let (_, tail) = text.split_once("groups:\n").expect(&text);
    let (groups, song) = tail.split_once("song:\n").expect(&text);
    assert_eq!(groups.lines().count(), 5, "{text}");
    assert!(groups.contains("Intro 2   4 bars  T02-3 T10-2"), "{text}");
    let song: Vec<_> = song
        .lines()
        .take(7)
        .map(|l| l.split_whitespace().last().unwrap())
        .collect();
    assert_eq!(song, ["x2", "x1", "x1", "x10", "x1", "x4", "x1"], "{text}");

    let out = std::env::temp_dir().join(format!("fantom-cli-pattern-{}.mid", std::process::id()));
    let dump = Command::new(env!("CARGO_BIN_EXE_fantom"))
        .args(["scenes", "pattern"])
        .arg(&narf)
        .args([&number, "10", "2", "--events", "--mid"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(dump.status.success());
    let dump = String::from_utf8(dump.stdout).unwrap();
    assert!(dump.starts_with("T10-2 Verse 2 bars note "), "{dump}");
    assert!(dump.contains(" note    ch=9 note="), "{dump}");
    assert!(std::fs::read(&out).unwrap().starts_with(b"MThd"));
    let _ = std::fs::remove_file(&out);
}
