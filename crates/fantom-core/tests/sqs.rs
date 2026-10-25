//! `SeqSet.SQS` against the private corpus, and the SMF export against `fixtures-local/tools/sqs.py`.
//!
//! Private fixtures only, so these skip without the corpus; `FANTOM_FIXTURES=require` makes
//! absence (or a missing `python3`) a failure.

use std::path::{Path, PathBuf};
use std::process::Command;

use fantom_core::sequence::{to_smf, SeqSet};

#[allow(dead_code)]
mod support;

fn sqs_files() -> Vec<PathBuf> {
    let files: Vec<_> = support::private_files(&["sqs"])
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|n| n == "SeqSet.SQS"))
        .collect();
    if files.is_empty() {
        assert!(
            std::env::var("FANTOM_FIXTURES").map_or(true, |v| v != "require"),
            "no SeqSet.SQS in the private corpus"
        );
        eprintln!("skipping: no SeqSet.SQS in the private corpus");
    }
    files
}

#[test]
fn every_corpus_file_parses() {
    let files = sqs_files();
    for path in &files {
        let bytes = std::fs::read(path).unwrap();
        let set = SeqSet::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(set.tpqn, 480);
        assert_eq!(set.patterns.len(), 128);
        assert_eq!(set.groups.len(), 16);
        assert_eq!(set.tracks.len(), 16);
        for step in &set.song.steps {
            assert!(set.groups[step.group].used, "{}", path.display());
        }
    }
    if files.len() >= 1000 {
        assert_eq!(files.len(), 1436);
    }
}

fn python_smf(file: &Path, track: usize, var: usize, out: &Path) -> bool {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures-local/tools/sqs.py");
    match Command::new("python3")
        .args(["-I"])
        .arg(script)
        .arg(file)
        .args(["--mid", &track.to_string(), &var.to_string()])
        .arg(out)
        .output()
    {
        Ok(o) => {
            assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
            true
        }
        Err(e) => {
            assert!(
                std::env::var("FANTOM_FIXTURES").map_or(true, |v| v != "require"),
                "python3 unavailable: {e}"
            );
            eprintln!("skipping: python3 unavailable");
            false
        }
    }
}

#[test]
fn smf_export_matches_tools_sqs_py() {
    let files = sqs_files();
    let out = std::env::temp_dir().join(format!("fantom-sqs-{}.mid", std::process::id()));
    let (mut compared, mut with_lanes) = (0, 0);
    let mut lane_files = 0;
    for (i, path) in files.iter().enumerate() {
        let set = SeqSet::parse(&std::fs::read(path).unwrap()).unwrap();
        let has_lanes = set
            .lanes
            .iter()
            .any(|l| l.iter().any(|l| !l.points.is_empty()));
        // A stride through the corpus, plus the first few files that carry automation.
        let lanes_pick = has_lanes && lane_files < 6;
        lane_files += usize::from(lanes_pick);
        if i % 40 != 0 && !lanes_pick {
            continue;
        }
        let mut picked = 0;
        for (slot, p) in set.patterns.iter().enumerate() {
            let lanes: Vec<_> = set.lanes[slot]
                .iter()
                .filter(|l| !l.points.is_empty())
                .cloned()
                .collect();
            if !p.has_data() || picked >= 4 && lanes.is_empty() {
                continue;
            }
            picked += 1;
            if !python_smf(path, p.track, p.variation, &out) {
                return;
            }
            assert_eq!(
                to_smf(p, set.tpqn, &lanes),
                std::fs::read(&out).unwrap(),
                "{} T{} V{}",
                path.display(),
                p.track,
                p.variation
            );
            compared += 1;
            with_lanes += usize::from(!lanes.is_empty());
        }
    }
    let _ = std::fs::remove_file(&out);
    if files.len() >= 1000 {
        assert!(compared > 100 && with_lanes > 0, "{compared} {with_lanes}");
    }
}
