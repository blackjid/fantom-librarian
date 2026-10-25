//! The analog models' mirror against the firmware's own init tones.
//!
//! The firmware holds an init tone per model (the ZEN-Core tone and the engine block, adjacent), also
//! for JX-8P and SH-101, of which no recorded tone exists. The reimplemented setters
//! must reproduce every tone field they write in its own init data.

use fantom_core::mdl::{mirror_consistent, mirror_mismatches};
use fantom_core::params::mdl::ENGINE_BASE;

#[path = "mdl_init/data.rs"]
mod data;

/// A record of `kind` holding the engine block and the tone fields the setters write.
fn record(kind: u32, engine: &[u8; 112], tone: &[(usize, usize, u32)]) -> Vec<u8> {
    let mut r = vec![0u8; 2048];
    r[5] = 2;
    r[8..12].copy_from_slice(&kind.to_le_bytes());
    r[ENGINE_BASE..ENGINE_BASE + 112].copy_from_slice(engine);
    for &(at, size, value) in tone {
        r[at..at + size].copy_from_slice(&value.to_le_bytes()[..size]);
    }
    r
}

fn init_tones() -> [(&'static str, Vec<u8>); 4] {
    [
        ("JP-8", record(7, &data::JP8_ENGINE, data::JP8_TONE)),
        ("JX-8P", record(8, &data::JX8P_ENGINE, data::JX8P_TONE)),
        ("JUNO-106", record(9, &data::JUNO_ENGINE, data::JUNO_TONE)),
        ("SH-101", record(10, &data::SH101_ENGINE, data::SH101_TONE)),
    ]
}

#[test]
fn the_checker_accepts_every_models_init_tone() {
    for (name, r) in init_tones() {
        let m = mirror_mismatches(&r).unwrap();
        assert!(m.is_empty(), "{name}: {m:?}");
        assert!(mirror_consistent(&r), "{name}");
    }
}

/// Each tone field of the init data is checked: changing any one is reported.
#[test]
fn every_field_of_an_init_tone_is_checked() {
    let tones = [
        data::JP8_TONE,
        data::JX8P_TONE,
        data::JUNO_TONE,
        data::SH101_TONE,
    ];
    for ((name, r), tone) in init_tones().into_iter().zip(tones) {
        for &(at, _, _) in tone {
            let mut stale = r.clone();
            stale[at] ^= 1;
            let m = mirror_mismatches(&stale).unwrap();
            assert_eq!(m.len(), 1, "{name} {at:#x}");
            assert!(m[0].offset <= at && at < m[0].offset + m[0].size);
        }
    }
}

#[test]
fn a_record_that_is_not_analog_has_no_mirror() {
    let mut r = record(7, &data::JP8_ENGINE, data::JP8_TONE);
    r[8..12].copy_from_slice(&15u32.to_le_bytes());
    assert!(mirror_mismatches(&r).is_err());
    assert!(!mirror_consistent(&r));
}
