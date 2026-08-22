//! SVD → SVZ, against the instrument's own exports.
//!
//! The oracle here is as strong as this project gets: `hwtest_back/` holds tone exports a FANTOM-6
//! wrote *and* the backup taken from the same instrument, so a rebuild can be compared byte for
//! byte with what Roland's own code produced from the same material.

// Only the private corpus can answer these; `public` goes unused in this binary.
#[allow(dead_code)]
mod support;
use support::private;

use fantom_core::container::Raw;
use fantom_core::convert::export_tones;

const BACKUP: &str = "hwtest_back/T8_MSMP_BACKUP/FANTOM.SVD";

/// The last byte of the preamble stamp is an OS-era marker the instrument copies out of `SYSa` —
/// `KY019%` on the machine that wrote these, `KY019$` on every older fixture and on everything this
/// tool writes. Files carrying `$` import and play (see `docs/FORMAT.md`), so it is excluded from
/// the comparison rather than chased.
const STAMP_BYTE: usize = 0x0b;

fn assert_same_but_the_stamp(ours: &Raw, theirs: &Raw, what: &str) {
    assert_eq!(
        ours.len(),
        theirs.len(),
        "{what}: rebuilt file is a different size"
    );
    let differing: Vec<usize> = ours
        .bytes()
        .iter()
        .zip(theirs.bytes())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(at, _)| at)
        .collect();
    assert_eq!(
        differing,
        vec![STAMP_BYTE],
        "{what}: rebuilt file differs from the instrument's in {} bytes",
        differing.len()
    );
}

/// Two tones, a multisample, and five samples — the whole machinery at once.
///
/// The instrument exported `Beat It Gong` (two samples, reached directly) and `T8_MSAMP` (three
/// samples, reached only through a multisample) as one file. Rebuilding it from the backup
/// reproduces every part: the renumbered tone records, the `MSPa` whose key map now points at
/// positions instead of panel slots, the `USPa` slot table converted out of `SMPa`, the `USDa`
/// directory with its carried per-section words, all 7 MB of audio, every CRC-32, and the preamble
/// down to its shape revision.
#[test]
fn rebuilding_an_instrument_written_tone_export_is_byte_identical() {
    let (Some(backup), Some(theirs)) = (private(BACKUP), private("hwtest_back/T8_MSMP_TONE.svz"))
    else {
        return;
    };
    let ours = export_tones(&backup, b"PATa", &[556, 954]).unwrap();
    assert_same_but_the_stamp(&ours, &theirs, "T8_MSMP_TONE.svz");
}

/// The same for a single multisampled tone, which the instrument exported on its own.
#[test]
fn rebuilding_a_multisampled_tone_export_is_byte_identical() {
    let (Some(backup), Some(theirs)) = (private(BACKUP), private("hwtest_back/T9_BACK.svz")) else {
        return;
    };
    let ours = export_tones(&backup, b"PATa", &[954]).unwrap();
    assert_same_but_the_stamp(&ours, &theirs, "T9_BACK.svz");
}

/// A drum kit: the paired `RHYa`+`INSa` shape, and a file with no sample areas at all.
///
/// This kit plays no user samples, so the export carries none — `DIFa,RHYa,INSa` and nothing else,
/// which is exactly what the instrument wrote. It also pins the shape revision, which differs from
/// the sampled shapes'.
#[test]
fn rebuilding_an_unsampled_drum_kit_export_is_byte_identical() {
    let (Some(backup), Some(theirs)) = (private(BACKUP), private("hwtest_back/DRUM_BEFORE.svz"))
    else {
        return;
    };
    let ours = export_tones(&backup, b"RHYa", &[1]).unwrap();
    assert_same_but_the_stamp(&ours, &theirs, "DRUM_BEFORE.svz");
}

/// And a drum kit that *does* play a user sample: the paired shape with its audio.
///
/// The kit is `#TR-MIX`, which the backup holds three times over — one copy playing a sample and
/// one not. Only the sampled copy rebuilds into the sampled export, which is a check on the
/// `INSa` sample references as much as on the writer: reading them wrong would pick the wrong
/// record, or carry no audio at all.
#[test]
fn rebuilding_a_sampled_drum_kit_export_is_byte_identical() {
    let (Some(backup), Some(theirs)) = (private(BACKUP), private("hwtest_back/DRUM_AFTER.svz"))
    else {
        return;
    };
    let ours = export_tones(&backup, b"RHYa", &[4]).unwrap();
    assert_same_but_the_stamp(&ours, &theirs, "DRUM_AFTER.svz");
}

/// What comes out is self-contained: it carries the audio, so it asks its destination for none.
#[test]
fn an_exported_tone_needs_nothing_from_the_destination() {
    let Some(backup) = private(BACKUP) else {
        return;
    };
    let source = fantom_core::requirements::requirements(&backup).unwrap();
    assert!(
        source.samples.iter().any(|slot| slot.slot == 2001),
        "the backup should hold the audio this test exports"
    );

    let exported = export_tones(&backup, b"PATa", &[556]).unwrap();
    let needs = fantom_core::requirements::requirements(&exported).unwrap();
    assert!(needs.carries_audio);
    assert_eq!(needs.missing_samples().count(), 0);
    // Renumbered densely from 1, as an instrument-written export does.
    assert_eq!(
        needs.samples.iter().map(|s| s.slot).collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        fantom_core::codec::read_bundled_tones(&exported).unwrap()[0].name,
        "Beat It Gong"
    );
}

/// A scene bank can still give up a tone — but only one whose sound it holds all of.
///
/// A tone record crosses envelopes unchanged, so an unsampled tone converts out of a scene export
/// perfectly well. A sampled one cannot: the bank stores slot numbers and no audio, and there is
/// nothing to carry. Refusing beats writing a file that looks self-contained and is silent.
#[test]
fn a_scene_bank_gives_up_only_the_tones_it_holds_whole() {
    let Some(pack) = private("backup/ROLAND/SOUND/NARF/FANTOM.SVD") else {
        return;
    };
    let reader = fantom_core::requirements::Reader::open(&pack).unwrap();
    let tones = fantom_core::codec::read_bundled_tones(&pack).unwrap();
    let sampled = tones
        .iter()
        .find(|tone| {
            tone.area == *b"PATa"
                && !reader
                    .tone(&tone.area, tone.index)
                    .unwrap()
                    .samples
                    .is_empty()
        })
        .expect("this pack plays user samples");
    let plain = tones
        .iter()
        .find(|tone| {
            tone.area == *b"PATa"
                && reader
                    .tone(&tone.area, tone.index)
                    .unwrap()
                    .samples
                    .is_empty()
        })
        .expect("this pack also has tones that play none");

    let error = export_tones(&pack, b"PATa", &[sampled.index])
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("SMPa"),
        "the error should name what is missing: {error}"
    );

    let exported = export_tones(&pack, b"PATa", &[plain.index]).unwrap();
    assert_eq!(
        fantom_core::codec::read_bundled_tones(&exported).unwrap()[0].name,
        plain.name
    );
    assert!(fantom_core::requirements::requirements(&exported)
        .unwrap()
        .samples
        .is_empty());
}
