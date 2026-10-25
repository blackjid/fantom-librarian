//! The analog models' ZEN-Core mirror against every recorded MODEL tone.
//!
//! Skips without the private corpus; `FANTOM_FIXTURES=require` makes an empty corpus fail. The
//! corpus holds JP-8 tones (`type 7`) and one JUNO-106 tone (`type 9`); JX-8P (8) and SH-101 (10)
//! have none, so those two are checked against the instrument's init tones in `mdl_init.rs`.

use std::collections::BTreeSet;

use fantom_core::container::{Raw, RecordTable, Svd};
use fantom_core::mdl::{mirror_consistent, mirror_mismatches};
use fantom_core::params::mdl::JP8_PINNED;

#[allow(dead_code)]
mod support;

const JP8: u32 = 7;
const JUNO: u32 = 9;

/// Every distinct `MDLa` record of `kind` in every backup of the corpus.
fn records(kind: u32) -> Vec<Vec<u8>> {
    let mut seen = BTreeSet::new();
    for path in support::private_files(&["svd", "svz"]) {
        let Ok(raw) = Raw::open(&path) else { continue };
        let Ok(svd) = Svd::parse(&raw) else { continue };
        let Ok(Some(table)) = RecordTable::from_svd(&raw, &svd, b"MDLa") else {
            continue;
        };
        for r in table.records() {
            if r.len() == 2048 && r[8..12] == kind.to_le_bytes() {
                seen.insert(r.to_vec());
            }
        }
    }
    seen.into_iter().collect()
}

fn corpus(kind: u32) -> Option<Vec<Vec<u8>>> {
    let records = records(kind);
    if records.is_empty() {
        assert!(
            std::env::var("FANTOM_FIXTURES").map_or(true, |v| v != "require"),
            "no type {kind} record in the private corpus"
        );
        eprintln!("skipping: no type {kind} records in the private corpus");
        return None;
    }
    Some(records)
}

/// Loading each record's engine block through the instrument's setters reproduces every tone field
/// they write, in all recorded JP-8 tones.
#[test]
fn every_recorded_jp8_tone_agrees_with_its_engine_block() {
    let Some(records) = corpus(JP8) else { return };
    assert!(records.len() >= 10, "{} JP-8 records", records.len());
    for (i, r) in records.iter().enumerate() {
        let m = mirror_mismatches(r).unwrap();
        assert!(m.is_empty(), "record {i}: {m:?}");
        assert!(mirror_consistent(r));
    }
}

/// The same for the recorded JUNO-106 tone, through the JUNO-106 setters (a different curve set
/// and four partials).
#[test]
fn every_recorded_juno_tone_agrees_with_its_engine_block() {
    let Some(records) = corpus(JUNO) else { return };
    for (i, r) in records.iter().enumerate() {
        let m = mirror_mismatches(r).unwrap();
        assert!(m.is_empty(), "record {i}: {m:?}");
        assert!(mirror_consistent(r));
    }
}

/// A changed engine value without its mirror is caught.
#[test]
fn a_stale_mirror_is_reported() {
    for kind in [JP8, JUNO] {
        let Some(records) = corpus(kind) else {
            continue;
        };
        let mut r = records[0].clone();
        r[0x100] ^= 1;
        assert_eq!(mirror_mismatches(&r).unwrap().len(), 1, "type {kind}");
        assert!(!mirror_consistent(&r));
    }
}

/// The fields `JP8_PINNED` calls constant are constant in the corpus.
#[test]
fn pinned_fields_hold_one_value_in_the_corpus() {
    let Some(records) = corpus(JP8) else { return };
    for &(index, value) in JP8_PINNED {
        for r in &records {
            assert_eq!(field_value(r, index), value, "field {index}");
        }
    }
}

fn field_value(record: &[u8], index: usize) -> i32 {
    let p = &fantom_core::params::mdl::ANALOG[index];
    let at = p.offset as usize;
    match (p.size, p.min < 0) {
        (1, false) => i32::from(record[at]),
        (1, true) => i32::from(record[at] as i8),
        (_, false) => i32::from(u16::from_le_bytes([record[at], record[at + 1]])),
        (_, true) => i32::from(i16::from_le_bytes([record[at], record[at + 1]])),
    }
}

/// Whatever `set_mdl_param` accepts leaves the tone agreeing with its engine block, for every
/// analog model in the corpus. Only JP-8 edits are accepted; every other model's are refused and
/// leave the file alone.
#[test]
fn every_accepted_edit_keeps_the_tone_consistent() {
    use fantom_core::mdl::{read_mdl_params, set_mdl_param, MdlEdit};
    let mut seen = BTreeSet::new();
    let mut accepted = [0; 4];
    for path in support::private_files(&["svd", "svz"]) {
        let Ok(raw) = Raw::open(&path) else { continue };
        let Ok(svd) = Svd::parse(&raw) else { continue };
        let Ok(Some(table)) = RecordTable::from_svd(&raw, &svd, b"MDLa") else {
            continue;
        };
        let firsts: Vec<(usize, u32)> = table
            .records()
            .enumerate()
            .filter(|(_, r)| r.len() == 2048)
            .map(|(i, r)| (i, u32::from_le_bytes(r[8..12].try_into().unwrap())))
            .filter(|(_, kind)| (7..=10).contains(kind))
            .filter(|&(i, _)| seen.insert(table.record(i).unwrap().to_vec()))
            .collect();
        for (index, kind) in firsts {
            let (_, fields) = read_mdl_params(&raw, index).unwrap();
            if kind != JP8 {
                assert!(fields.iter().all(|f| f.edit == MdlEdit::Locked));
            }
            for field in &fields {
                let (lo, hi) = (field.param.min, field.param.max);
                // a refusal is the same whatever the value
                let values = if kind == JP8 {
                    vec![lo, hi, (lo + hi) / 2, lo + 1, hi - 1, field.value]
                } else {
                    vec![lo, hi]
                };
                for value in values {
                    let mut edited = Raw::from_bytes(raw.bytes().to_vec());
                    let v = value.clamp(lo, hi).to_string();
                    if set_mdl_param(&mut edited, index, field.param.id, &v).is_err() {
                        assert_eq!(edited.bytes(), raw.bytes(), "a refusal changed the file");
                        continue;
                    }
                    assert_eq!(kind, JP8, "{} = {v} accepted", field.param.id);
                    accepted[(kind - 7) as usize] += 1;
                    let svd = Svd::parse(&edited).unwrap();
                    let t = RecordTable::from_svd(&edited, &svd, b"MDLa")
                        .unwrap()
                        .unwrap();
                    let m = mirror_mismatches(t.record(index).unwrap()).unwrap();
                    assert!(m.is_empty(), "{} = {v}: {m:?}", field.param.id);
                }
            }
        }
    }
    if !seen.is_empty() {
        assert!(accepted[0] > 100, "only {} edits accepted", accepted[0]);
        assert_eq!(accepted[1..], [0, 0, 0]);
    }
}
