//! The JP-8 ZEN-Core mirror against every recorded MODEL tone.
//!
//! Skips without the private corpus; `FANTOM_FIXTURES=require` makes an empty corpus fail.

use std::collections::BTreeSet;

use fantom_core::container::{Raw, RecordTable, Svd};
use fantom_core::mdl::{mirror_consistent, mirror_mismatches};
use fantom_core::params::mdl::JP8_PINNED;

#[allow(dead_code)]
mod support;

/// Every distinct JP-8 (`type 7`) `MDLa` record of every backup in the corpus.
fn jp8_records() -> Vec<Vec<u8>> {
    let mut seen = BTreeSet::new();
    for path in support::private_files(&["svd", "svz"]) {
        let Ok(raw) = Raw::open(&path) else { continue };
        let Ok(svd) = Svd::parse(&raw) else { continue };
        let Ok(Some(table)) = RecordTable::from_svd(&raw, &svd, b"MDLa") else {
            continue;
        };
        for r in table.records() {
            if r.len() == 2048 && r[8..12] == 7u32.to_le_bytes() {
                seen.insert(r.to_vec());
            }
        }
    }
    seen.into_iter().collect()
}

fn corpus() -> Option<Vec<Vec<u8>>> {
    let records = jp8_records();
    if records.is_empty() {
        assert!(
            std::env::var("FANTOM_FIXTURES").map_or(true, |v| v != "require"),
            "no JP-8 record in the private corpus"
        );
        eprintln!("skipping: no JP-8 records in the private corpus");
        return None;
    }
    Some(records)
}

/// Loading each record's engine block through the instrument's setters reproduces every tone field
/// they write, in all recorded JP-8 tones.
#[test]
fn every_recorded_jp8_tone_agrees_with_its_engine_block() {
    let Some(records) = corpus() else { return };
    assert!(records.len() >= 10, "{} JP-8 records", records.len());
    for (i, r) in records.iter().enumerate() {
        let m = mirror_mismatches(r).unwrap();
        assert!(m.is_empty(), "record {i}: {m:?}");
        assert!(mirror_consistent(r));
    }
}

/// A changed engine value without its mirror is caught.
#[test]
fn a_stale_mirror_is_reported() {
    let Some(records) = corpus() else { return };
    let mut r = records[0].clone();
    r[0x100] ^= 1;
    assert_eq!(mirror_mismatches(&r).unwrap().len(), 1);
    assert!(!mirror_consistent(&r));
}

/// The fields `JP8_PINNED` calls constant are constant in the corpus.
#[test]
fn pinned_fields_hold_one_value_in_the_corpus() {
    let Some(records) = corpus() else { return };
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

/// Whatever `set_mdl_param` accepts leaves the tone agreeing with its engine block.
#[test]
fn every_accepted_edit_keeps_the_tone_consistent() {
    use fantom_core::mdl::{read_mdl_params, set_mdl_param, MdlEdit};
    let mut seen = BTreeSet::new();
    let mut accepted = 0;
    for path in support::private_files(&["svd", "svz"]) {
        let Ok(raw) = Raw::open(&path) else { continue };
        let Ok(svd) = Svd::parse(&raw) else { continue };
        let Ok(Some(table)) = RecordTable::from_svd(&raw, &svd, b"MDLa") else {
            continue;
        };
        let firsts: Vec<usize> = table
            .records()
            .enumerate()
            .filter(|(_, r)| r.len() == 2048 && r[8..12] == 7u32.to_le_bytes())
            .filter(|(_, r)| seen.insert(r.to_vec()))
            .map(|(i, _)| i)
            .collect();
        for index in firsts {
            let (_, fields) = read_mdl_params(&raw, index).unwrap();
            for field in fields.iter().filter(|f| f.edit != MdlEdit::Locked) {
                let (lo, hi) = (field.param.min, field.param.max);
                for value in [lo, hi, (lo + hi) / 2, lo + 1, hi - 1, field.value] {
                    let mut edited = Raw::from_bytes(raw.bytes().to_vec());
                    let v = value.clamp(lo, hi).to_string();
                    if set_mdl_param(&mut edited, index, field.param.id, &v).is_err() {
                        continue;
                    }
                    accepted += 1;
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
        assert!(accepted > 100, "only {accepted} edits accepted");
    }
}
