//! `system show` and `system diff`: the public fixtures, and parity with `fixtures-local/tools/sysa.py` over
//! every file of the private corpus that has a `SYSa` area.
//!
//! The parity part skips without the corpus; `FANTOM_FIXTURES=require` makes an empty one fail.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fantom_core::container::{Raw, RecordTable, Svd};

const SYSA_PY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures-local/tools/sysa.py"
);

fn public(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(relative)
}

fn fantom(args: &[&Path], flags: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fantom"))
        .arg("system")
        .arg(if args.len() == 2 { "diff" } else { "show" })
        .args(args)
        .args(flags)
        .output()
        .unwrap()
}

fn python(args: &[&Path], flags: &[&str]) -> Output {
    Command::new("python3")
        .args(["-I", SYSA_PY])
        .args(args)
        .args(flags)
        .output()
        .unwrap()
}

fn stdout(out: Output) -> String {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn show_hides_reserved_and_init_settings() {
    let file = public("tests/TEST 1/FANTOM.SVD");
    let default = stdout(fantom(&[&file], &[]));
    let all = stdout(fantom(&[&file], &["--all"]));
    assert!(default.contains("0x028 SC:USB_MIDI_DRV                    0 VENDOR\n"));
    assert!(default.contains("0x044 SC:CLOCK_YEAR                      19 (2019)\n"));
    assert!(!default.contains("RSV"));
    assert!(!default.contains("SC:TUNE"));
    assert!(all.contains("SC:TUNE") && all.contains("RSV"));
    assert!(all.lines().count() > default.lines().count());
}

#[test]
fn diff_lists_changed_settings() {
    let src = public("tests/TEST 1/FANTOM.SVD");
    let mut raw = Raw::open(&src).unwrap();
    let start = {
        let svd = Svd::parse(&raw).unwrap();
        RecordTable::from_svd(&raw, &svd, b"SYSa")
            .unwrap()
            .unwrap()
            .record_offset(0)
    };
    // SC:LEVEL
    raw.patch_bytes(start + 3, &[100]);
    let edited = Path::new(env!("CARGO_TARGET_TMPDIR")).join("system_diff.SVD");
    raw.save(&edited).unwrap();

    let text = stdout(fantom(&[&src, &edited], &[]));
    assert_eq!(
        text,
        "0x003 SC:LEVEL                           127 -> 100\n"
    );
    assert_eq!(stdout(fantom(&[&src, &src], &[])), "");
}

fn corpus() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "svd" | "svz"))
            {
                out.push(p);
            }
        }
    }
    let root = match std::env::var("FANTOM_FIXTURES_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures-local"),
    };
    let mut out = Vec::new();
    walk(&root, &mut out);
    out.sort();
    out
}

/// Both tools print the same text for every file, and fail on the same files.
#[test]
fn matches_sysa_py_over_the_corpus() {
    let mut read = Vec::new();
    for file in corpus() {
        let rust = fantom(&[&file], &["--all"]);
        let py = python(&[&file], &["-a"]);
        assert_eq!(
            rust.status.success(),
            py.status.success(),
            "{}",
            file.display()
        );
        if !rust.status.success() {
            continue;
        }
        assert_eq!(rust.stdout, py.stdout, "{} -a", file.display());
        let (rust, py) = (fantom(&[&file], &[]), python(&[&file], &[]));
        assert_eq!(rust.stdout, py.stdout, "{}", file.display());
        read.push(file);
    }
    if read.is_empty() {
        assert!(
            std::env::var("FANTOM_FIXTURES").map_or(true, |v| v != "require"),
            "no SYSa in the private corpus"
        );
        eprintln!("skipping: no SYSa files in the private corpus");
        return;
    }
    // Consecutive pairs, with and without reserved settings.
    for pair in read.windows(2) {
        for (flag, py_flag) in [(&[][..], &[][..]), (&["--all"][..], &["-a"][..])] {
            let rust = fantom(&[&pair[0], &pair[1]], flag);
            let py = python(&[&pair[0], &pair[1]], py_flag);
            assert_eq!(rust.stdout, py.stdout, "{:?} {flag:?}", pair);
        }
    }
    eprintln!("compared {} files", read.len());
}
