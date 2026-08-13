//! The scene parameter table, against files a FANTOM-6 wrote.
//!
//! `params::scene` is generated from Roland's MIDI Implementation, which describes the *wire*.
//! Its file offsets are derived by the packing rule in `docs/FORMAT.md`, so the derivation is
//! what needs checking, and only real records can check it. Like `fixtures.rs`, every test here
//! skips when its fixture is missing.

use fantom_core::codec;
use fantom_core::container::{Raw, RecordTable, Svd};
use fantom_core::params::{scene, Instance};

mod support;
use support::{private, public};

/// Every scene record in a file, as raw bytes.
fn records(raw: &Raw) -> Vec<Vec<u8>> {
    let svd = Svd::parse(raw).expect("parses");
    let table = RecordTable::from_svd(raw, &svd, b"PRFa")
        .expect("readable")
        .expect("has a PRFa area");
    table.records().map(|r| r.to_vec()).collect()
}

/// The `n`th instance of a named block within the scene record.
fn block(name: &str, n: usize) -> &'static Instance {
    scene::SCENE
        .iter()
        .filter(|i| i.block.name == name)
        .nth(n)
        .unwrap_or_else(|| panic!("scene has no {name} instance {n}"))
}

fn zone(n: usize) -> &'static Instance {
    block("Scene Zone", n)
}

fn control(n: usize) -> &'static Instance {
    block("Zone Control", n)
}

/// The controlled single-variable edits that first established these offsets. `TEST 1..3` differ
/// by one deliberate panel change each, so they pin the fields the table now claims to hold.
#[test]
fn the_controlled_edits_read_back_through_the_table() {
    let recs = records(&public("tests/TEST 1/FANTOM.SVD"));
    let r = &recs[0];
    let common = block("Scene Common", 0);

    assert_eq!(common.read(r, "Scene_Level"), Some(100));
    // Tempo is the four-nibble field whose packing puts the memo at file +0x40, wire +0x42.
    assert_eq!(common.read(r, "Scene_Tempo"), Some(12000), "120.00 BPM");
    assert_eq!(common.read(r, "Current_Zone"), Some(0));

    // TEST 3 set zone 1's key range to C4-C5 and its level to 50.
    let recs3 = records(&public("tests/TEST 3/FANTOM.SVD"));
    let r3 = &recs3[0];
    assert_eq!(zone(0).read(r3, "Zone_Level"), Some(50));
    assert_eq!(
        control(0).read(r3, "Keyboard_Control_Range_Lower"),
        Some(60),
        "C4"
    );
    assert_eq!(
        control(0).read(r3, "Keyboard_Control_Range_Upper"),
        Some(72),
        "C5"
    );

    // TEST 2 is the same scene before the level edit.
    let recs2 = records(&public("tests/TEST 2/FANTOM.SVD"));
    assert_eq!(zone(0).read(&recs2[0], "Zone_Level"), Some(100));
}

/// The table must reproduce, for every zone of every scene, exactly what the existing decoder
/// reports — a decoder whose offsets were confirmed against the FANTOM-6 panel. Agreement over
/// hundreds of scenes is what turns a derived map into a checked one.
#[test]
fn agrees_with_the_decoder_across_a_whole_backup() {
    let Some(raw) = private("backup/ROLAND/SOUND/NARF/FANTOM.SVD") else {
        return;
    };
    let scenes = codec::read_scenes(&raw).expect("decodes");
    let recs = records(&raw);
    assert_eq!(scenes.len(), recs.len());

    let mut checked = 0;
    for (s, r) in scenes.iter().zip(&recs) {
        for z in &s.zones {
            let n = z.number as usize;
            assert_eq!(
                zone(n).read(r, "Tone_Bank_Select_MSB"),
                Some(z.tone.address.msb as u32),
                "scene {:?} zone {} MSB",
                s.name,
                n + 1
            );
            assert_eq!(
                zone(n).read(r, "Tone_Bank_Select_LSB"),
                Some(z.tone.address.lsb as u32)
            );
            assert_eq!(
                zone(n).read(r, "Tone_Program_Change"),
                Some(z.tone.address.pc as u32)
            );
            assert_eq!(zone(n).read(r, "Zone_Level"), Some(z.level as u32));
            assert_eq!(
                control(n).read(r, "Keyboard_Control_Range_Lower"),
                Some(z.key_low as u32)
            );
            assert_eq!(
                control(n).read(r, "Keyboard_Control_Range_Upper"),
                Some(z.key_high as u32)
            );
            assert_eq!(
                control(n).read(r, "Keyboard_Switch"),
                Some(z.enabled as u32)
            );
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} zones checked");
}

/// Blocks the decoder has never read, whose placement is therefore only as good as the layout
/// arithmetic. Every zone of every scene must hold a *plausible* EQ — gains centred, the three
/// band frequencies in range, Q one of six values — which a misplaced block would not.
#[test]
fn the_newly_placed_blocks_hold_values_in_range() {
    let Some(raw) = private("backup/ROLAND/SOUND/NARF/FANTOM.SVD") else {
        return;
    };
    for r in &records(&raw) {
        for n in 0..16 {
            let eq = block("Zone EQ", n);
            for band in ["EQ_Low_Frequency", "EQ_Mid_Frequency", "EQ_High_Frequency"] {
                let v = eq.read(r, band).expect("band is mapped");
                assert!(v <= 29, "{band} = {v}, past the 30-entry frequency list");
            }
            assert!(eq.read(r, "EQ_Mid_Q").unwrap() <= 5);
            assert!(eq.read(r, "EQ_Switch").unwrap() <= 1);
        }

        // Ranges as the MIDI Implementation declares them: cutoff and resonance are the
        // four-nibble 0-1023 fields, and the type is one of OFF/LPF1/LPF2/LPF3/HPF/BPF.
        let af = block("Analog Filter", 0);
        assert!(af.read(r, "Analog_Filter_Cutoff_1").unwrap() <= 1023);
        assert!(af.read(r, "Analog_Filter_Cutoff_2").unwrap() <= 1023);
        assert!(af.read(r, "Analog_Filter_Resonance_1").unwrap() <= 1023);
        assert!(af.read(r, "Analog_Filter_Resonance_2").unwrap() <= 1023);
        assert!(af.read(r, "Analog_Filter_Type").unwrap() <= 5);
        assert!(af.read(r, "Analog_Filter_Amp_Sw").unwrap() <= 1);
        assert!(af.read(r, "Analog_Filter_Drive_Sw").unwrap() <= 1);

        // A controller assign triple is (assign, min, max); the range bytes are 7-bit.
        let ctl = block("Scene Controller", 0);
        assert!(ctl.read(r, "Pedal_1_Range_Min").unwrap() <= 127);
        assert!(ctl.read(r, "Pedal_1_Range_Max").unwrap() <= 127);
    }
}

/// "Africa Main" is the scene `docs/FORMAT.md` records panel ground truth for. The fields the
/// model gained from the parameter table must agree with it, and the signed ones must come back
/// signed — an unbiased read would show pan L16 as 240.
#[test]
fn africa_main_matches_the_panel() {
    let Some(raw) = private("backup/ROLAND/SOUND/NARF/FANTOM.SVD") else {
        return;
    };
    let scenes = codec::read_scenes(&raw).expect("decodes");
    let s = scenes
        .iter()
        .find(|s| s.name == "Africa Main")
        .expect("NARF has Africa Main");

    assert_eq!(s.tempo, 12000, "120.00 BPM");
    assert_eq!(s.bpm(), 120.0);

    let z: Vec<_> = s.zones.iter().filter(|z| z.enabled).collect();
    assert_eq!(z.len(), 4);
    // Levels and key ranges as the panel shows them.
    assert_eq!(
        z.iter().map(|z| z.level).collect::<Vec<_>>(),
        [107, 107, 100, 82]
    );
    assert_eq!(
        z.iter()
            .map(|z| (z.key_low, z.key_high))
            .collect::<Vec<_>>(),
        [(0, 71), (73, 127), (72, 72), (0, 71)]
    );
    // Pan is stored zero-centred; two of these are negative and must survive as such.
    assert_eq!(
        z.iter().map(|z| z.pan).collect::<Vec<_>>(),
        [-16, 17, 0, 25]
    );
    // Zones default to receiving on their own channel.
    assert_eq!(
        z.iter().map(|z| z.midi_channel).collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
}

/// Every signed field must read back inside the range Roland declares for it. Reading one
/// unsigned would put a negative value up near 255 and fail here.
#[test]
fn signed_fields_stay_inside_their_declared_range() {
    let Some(raw) = private("backup/ROLAND/SOUND/NARF/FANTOM.SVD") else {
        return;
    };
    for s in codec::read_scenes(&raw).expect("decodes") {
        for z in &s.zones {
            assert!((-64..=63).contains(&z.pan), "pan {} out of range", z.pan);
            assert!(
                (-48..=48).contains(&z.transpose),
                "transpose {} out of range",
                z.transpose
            );
            assert!((-3..=3).contains(&z.octave), "octave {}", z.octave);
            assert!(z.midi_channel < 16);
            assert!(z.velocity_low >= 1 && z.velocity_high <= 127);
        }
    }
}

/// The scene name and memo are the two fields the librarian already reads and writes. Reading
/// them through the parameter table must give the same bytes.
#[test]
fn name_and_memo_agree_with_the_codec() {
    let Some(raw) = private("backup/ROLAND/SOUND/NARF/FANTOM.SVD") else {
        return;
    };
    let scenes = codec::read_scenes(&raw).expect("decodes");
    let common = block("Scene Common", 0);
    for (s, r) in scenes.iter().zip(&records(&raw)) {
        let name: String = (1..=16)
            .filter_map(|i| common.read(r, &format!("Scene_Name_{i}")))
            .map(|b| b as u8 as char)
            .collect();
        assert_eq!(name.trim_end(), s.name);

        let memo: String = (1..=64)
            .filter_map(|i| common.read(r, &format!("Scene_Memo_{i}")))
            .map(|b| b as u8 as char)
            .collect();
        assert_eq!(memo.trim_end(), s.comment);
    }
}
