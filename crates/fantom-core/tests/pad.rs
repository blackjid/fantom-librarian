//! Pad sampler sets against the private corpus, the WAV export against `fixtures-local/tools/pad.py`, and the
//! factory pads against the factory `LIBRARY` WAVs.
//!
//! Private fixtures only, so these skip without the corpus; `FANTOM_FIXTURES=require` makes
//! absence (or a missing `python3`) a failure.

use std::path::{Path, PathBuf};
use std::process::Command;

use fantom_core::pad::{self, PadSet};

#[allow(dead_code)]
mod support;

fn required() -> bool {
    std::env::var("FANTOM_FIXTURES").is_ok_and(|v| v == "require")
}

/// Backup folders holding a pad set (the folder above `PAD_SMPL`).
fn sets() -> Vec<PathBuf> {
    let sets: Vec<_> = support::private_files(&["bin"])
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|n| n == "PADCONF.BIN"))
        .map(|p| p.ancestors().nth(3).unwrap().to_path_buf())
        .collect();
    if sets.is_empty() {
        assert!(!required(), "no PADCONF.BIN in the private corpus");
        eprintln!("skipping: no PADCONF.BIN in the private corpus");
    }
    sets
}

fn read(dir: &Path) -> PadSet {
    pad::read_set(dir).unwrap().expect("pad set")
}

#[test]
fn every_corpus_set_parses() {
    let sets = sets();
    for dir in &sets {
        let set = read(dir);
        assert_eq!(set.pads.len(), 64, "{}", dir.display());
        for p in &set.pads {
            let a = p.audio.expect("every corpus pad has its .SMP");
            assert_eq!(a.file_size, p.size as usize, "{} {}", dir.display(), p.name);
            assert!(p.level <= 127);
            assert!(p.start as usize >= pad::SMP_HEADER && p.end <= p.size);
        }
    }
    if required() {
        assert_eq!(sets.len(), 5);
    }
}

#[test]
fn a_folder_without_pads_has_none() {
    assert!(pad::read_set(&std::env::temp_dir()).unwrap().is_none());
}

#[test]
fn wav_matches_tools_pad_py() {
    let sets = sets();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures-local/tools/pad.py");
    for (n, dir) in sets.iter().enumerate() {
        let out = std::env::temp_dir().join(format!("fantom-pad-{}-{n}", std::process::id()));
        let run = Command::new("python3")
            .arg("-I")
            .arg(&script)
            .arg("wav")
            .arg(dir.join(pad::DIR))
            .arg(&out)
            .output();
        match run {
            Ok(o) => assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr)),
            Err(e) => {
                assert!(!required(), "python3 unavailable: {e}");
                eprintln!("skipping: python3 unavailable");
                return;
            }
        }
        let set = read(dir);
        for p in &set.pads {
            let prefix = format!("BANK{}-{:02}_", p.bank, p.pad);
            let py = std::fs::read_dir(&out)
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|f| {
                    f.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with(&prefix)
                })
                .expect("python wrote this pad");
            let rust = pad::read_wav(dir, p, false).unwrap();
            assert!(rust == std::fs::read(&py).unwrap(), "{}", py.display());
        }
        std::fs::remove_dir_all(&out).unwrap();
    }
}

#[test]
fn trimmed_wav_is_the_start_end_slice() {
    for dir in sets() {
        for p in read(&dir).pads.iter().filter(|p| p.trimmed()) {
            let all = pad::read_wav(&dir, p, false).unwrap();
            let cut = pad::read_wav(&dir, p, true).unwrap();
            let frame = p.audio.unwrap().channels as usize * 2;
            let (s, e) = (
                p.start as usize - pad::SMP_HEADER,
                p.end as usize - pad::SMP_HEADER,
            );
            assert_eq!(
                cut[44..],
                all[44 + s / frame * frame..][..(e - s) / frame * frame]
            );
        }
    }
}

/// The `data` chunk of a RIFF WAV.
fn wav_pcm(wav: &[u8]) -> &[u8] {
    let mut at = 12;
    while at + 8 <= wav.len() {
        let len = u32::from_le_bytes(wav[at + 4..at + 8].try_into().unwrap()) as usize;
        if &wav[at..at + 4] == b"data" {
            return &wav[at + 8..(at + 8 + len).min(wav.len())];
        }
        at += 8 + len + (len & 1);
    }
    panic!("no data chunk");
}

fn find(dir: &Path, file: &str) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find(&p, file) {
                return Some(f);
            }
        } else if p.file_name().is_some_and(|n| n == file) {
            return Some(p);
        }
    }
    None
}

#[test]
fn factory_pads_are_the_library_wavs() {
    let Some(factory) = sets()
        .into_iter()
        .find(|d| d.ends_with("factory_ex/_DATA/FACTORY"))
    else {
        assert!(!required(), "factory pad set missing");
        return;
    };
    let library = factory.join("LIBRARY");
    let set = read(&factory);
    for p in &set.pads {
        let wav = find(&library, &format!("{}.wav", p.name))
            .unwrap_or_else(|| panic!("no LIBRARY wav for {}", p.name));
        let lib = std::fs::read(wav).unwrap();
        let ours = pad::read_wav(&factory, p, false).unwrap();
        assert!(wav_pcm(&lib) == wav_pcm(&ours), "{}", p.name);
    }
}
