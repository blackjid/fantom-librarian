//! `SVD` → `SVZ`: lifting a user tone out of a backup with the audio it plays.
//!
//! An `.svz` is the only envelope that carries user sample audio. A scene bank stores slot
//! *references*, which is Roland's own behaviour and not a limitation of this tool — so a sampled
//! sound that lives in somebody's backup has, until now, been unshareable without going back to the
//! instrument and re-exporting it by hand. This builds that file instead.
//!
//! # Why it is mostly assembly
//!
//! Both halves already existed. [`crate::samplebank`] converts a backup's `SMPa`/`SMPd` sampling
//! into the `USPa`/`USDa` an SVZ wants, and [`crate::tonebank`] lays an SVZ out and renumbers
//! references while repackaging one. What was missing is the join, and one fact that had to be
//! measured rather than assumed:
//!
//! **A tone record crosses envelopes unchanged.** `T8_MSMP_TONE.svz`, exported by a FANTOM-6, holds
//! two `PATa` records; the backup taken from the same instrument holds both byte for byte, with a
//! single byte differing — the right wave number of one partial, `22` in the backup and `2` in the
//! export, which is that sample's new position. So the conversion is renumbering plus envelope
//! construction, and nothing about a tone's 1632 bytes needs decoding to move it.
//!
//! The same holds for a multisample: the export's `MSPa` record equals the backup's `MLSa` record
//! except in the per-key sample numbers, which are renumbered the same way.
//!
//! # What travels
//!
//! Everything the selected tones reach — the samples their partials play, the samples a
//! multisample maps across the keyboard, and the multisample itself — as decided by
//! [`crate::requirements`], numbered densely from 1 in slot order. What cannot travel is what never
//! can: factory and expansion content, which lives in the instrument.

use std::collections::{BTreeMap, BTreeSet};

use crate::address::{self, AreaSpec};
use crate::container::{self, Kind, Raw, RecordTable, Svd};
use crate::requirements::Reader;
use crate::tonebank::BuiltArea;
use crate::{Error, Result};

/// Areas of an SVZ tone export are stamped `ZCOR`, whatever the source file stamped them.
const FORMAT: [u8; 4] = *b"ZCOR";

/// Where each area sits in an instrument-written tone export.
///
/// `MSPa` between the slot table and the audio is the order `T8_MSMP_TONE.svz` uses; the drum
/// shape (`RHYa`, `INSa`) is `DRUM_AFTER.svz`.
const AREA_ORDER: [[u8; 4]; 7] = [
    *b"DIFa", *b"PATa", *b"RHYa", *b"INSa", *b"USPa", *b"MSPa", *b"USDa",
];

/// Build an `.svz` tone export from `source`, carrying the `indexes` of its `area` bank and the
/// user audio those tones play.
///
/// `indexes` are the record numbers `fantom tones list` prints, in the order they should appear.
/// `source` is an SVD — a full backup, or a scene export for tones that play no user samples.
///
/// Only engines whose sample references are decoded can be exported this way: `PATa` (ZEN-Core) and
/// `RHYa` with its paired `INSa` (drum kits). For any other engine the tool cannot tell which
/// samples a record plays, so it could neither carry them nor honestly say it had left them
/// behind — see [`AreaSpec::sample_refs_decoded`].
pub fn export_tones(source: &Raw, area: &[u8; 4], indexes: &[usize]) -> Result<Raw> {
    if indexes.is_empty() {
        return Err(Error::Unrecognized(
            "at least one tone index is required".into(),
        ));
    }
    let spec = engine(area)?;
    let svd = Svd::parse(source)?;
    if svd.kind == Kind::Svz {
        return Err(Error::Unrecognized(
            "this is already an SVZ — `tonebank::extract_tones` repackages one".into(),
        ));
    }

    let (samples, multisamples) = closure(source, area, indexes)?;
    let sample_remap = dense(&samples);
    let multisample_remap = dense(&multisamples);

    let mut areas = vec![crate::samplebank::dif_area()];
    for tag in spec.paired {
        areas.push(tone_area(
            source,
            &svd,
            tag,
            indexes,
            &sample_remap,
            &multisample_remap,
        )?);
    }
    if !samples.is_empty() {
        // 0-based for the sample builder, which counts slots the way `SMPa` records them.
        let slots: Vec<usize> = samples.iter().map(|&slot| slot as usize - 1).collect();
        areas.extend(crate::samplebank::sample_areas(source, &svd, &slots)?);
    }
    if !multisamples.is_empty() {
        areas.push(multisample_area(
            source,
            &svd,
            &multisamples,
            &sample_remap,
        )?);
    }

    let order: Vec<[u8; 4]> = AREA_ORDER
        .into_iter()
        .filter(|tag| areas.iter().any(|(present, _, _)| present == tag))
        .collect();
    crate::tonebank::assemble(&crate::tonebank::preamble(&order), &AREA_ORDER, areas)
}

/// The engine a tag names, if this is one whose samples can be followed.
fn engine(area: &[u8; 4]) -> Result<&'static AreaSpec> {
    let spec = address::spec_for_tag(area).ok_or_else(|| {
        Error::Unrecognized(format!(
            "{} is not a user tone area",
            String::from_utf8_lossy(area)
        ))
    })?;
    if !spec.sample_refs_decoded {
        return Err(Error::Unrecognized(format!(
            "a {} record's sample references are not decoded, so an export could not tell which \
             audio to carry — and a tone bank missing its samples plays the wrong sound silently",
            spec.tag_str(),
        )));
    }
    Ok(spec)
}

/// Everything the selected tones need, as 1-based slot numbers.
fn closure(source: &Raw, area: &[u8; 4], indexes: &[usize]) -> Result<(Vec<u16>, Vec<u16>)> {
    let reader = Reader::open(source)?;
    let mut samples = BTreeSet::new();
    let mut multisamples = BTreeSet::new();
    for &index in indexes {
        let needs = reader.tone(area, index)?;
        samples.extend(needs.samples.iter().map(|slot| slot.slot));
        multisamples.extend(needs.multisamples.iter().map(|slot| slot.slot));
    }
    Ok((
        samples.into_iter().collect(),
        multisamples.into_iter().collect(),
    ))
}

/// Number what travels from 1, in slot order — the mapping an instrument-written export uses.
fn dense(slots: &[u16]) -> BTreeMap<u16, u16> {
    slots
        .iter()
        .enumerate()
        .map(|(position, &slot)| (slot, position as u16 + 1))
        .collect()
}

/// One engine area of the output: the chosen records, repointed at what travels with them.
fn tone_area(
    source: &Raw,
    svd: &Svd,
    tag: &[u8; 4],
    indexes: &[usize],
    samples: &BTreeMap<u16, u16>,
    multisamples: &BTreeMap<u16, u16>,
) -> Result<BuiltArea> {
    let table = area(source, svd, tag)?;
    let mut records = Vec::with_capacity(indexes.len());
    for &index in indexes {
        let mut record = table
            .record(index)
            .ok_or_else(|| {
                Error::Unrecognized(format!(
                    "tone {index} is out of range ({} holds {}, numbered 0..{})",
                    String::from_utf8_lossy(tag),
                    table.len(),
                    table.len().saturating_sub(1)
                ))
            })?
            .to_vec();
        container::remap_sample_slots_of(tag, &mut record, samples);
        // Only a ZEN-Core tone stores multisample references, and only it has the partial layout
        // that rewrite reads. Running it over a drum kit's 19 KB instrument set would rewrite
        // whatever bytes happened to look like a partial.
        if tag == b"PATa" {
            container::remap_multisample_slots(&mut record, multisamples);
        }
        records.push(record);
    }
    Ok((
        *tag,
        FORMAT,
        crate::tonebank::record_area(
            &records.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            table.record_size,
        ),
    ))
}

/// `MSPa`: the backup's `MLSa` records for what travels, with their key maps renumbered.
fn multisample_area(
    source: &Raw,
    svd: &Svd,
    multisamples: &[u16],
    samples: &BTreeMap<u16, u16>,
) -> Result<BuiltArea> {
    let table = area(source, svd, b"MLSa")?;
    let mut records = Vec::with_capacity(multisamples.len());
    for &number in multisamples {
        let mut record = table
            .record(number as usize - 1)
            .ok_or_else(|| {
                Error::Unrecognized(format!(
                    "multisample {number} is out of range (the file defines {})",
                    table.len()
                ))
            })?
            .to_vec();
        container::remap_sample_slots_of(b"MSPa", &mut record, samples);
        records.push(record);
    }
    Ok((
        *b"MSPa",
        FORMAT,
        crate::tonebank::record_area(
            &records.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            table.record_size,
        ),
    ))
}

fn area<'a>(source: &'a Raw, svd: &Svd, tag: &[u8; 4]) -> Result<RecordTable<'a>> {
    RecordTable::from_svd(source, svd, tag)?.ok_or_else(|| {
        Error::Unrecognized(format!(
            "this file has no {} area",
            String::from_utf8_lossy(tag)
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The engines this cannot serve are refused by name, before anything is written.
    ///
    /// Carrying a tone whose sample references we cannot read would produce a file that looks
    /// self-contained and plays silence — the failure this whole module exists to avoid.
    #[test]
    fn an_engine_whose_samples_cannot_be_followed_is_refused() {
        let nothing = Raw::from_bytes(Vec::new());

        let error = export_tones(&nothing, b"ACBa", &[0])
            .unwrap_err()
            .to_string();
        assert!(error.contains("not decoded"), "{error}");

        let error = export_tones(&nothing, b"PRFa", &[0])
            .unwrap_err()
            .to_string();
        assert!(error.contains("not a user tone area"), "{error}");

        assert!(export_tones(&nothing, b"PATa", &[]).is_err());
    }

    /// A destination numbers what arrives from 1, in slot order — position, not panel slot.
    #[test]
    fn what_travels_is_numbered_densely_from_one() {
        assert_eq!(
            dense(&[2001, 2002, 2018]),
            BTreeMap::from([(2001, 1), (2002, 2), (2018, 3)])
        );
        assert!(dense(&[]).is_empty());
    }
}
