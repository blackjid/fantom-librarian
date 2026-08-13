//! SVD/SVZ container framing.
//!
//! This layer is about the *envelope* of a Fantom file — where its bytes live — not about the
//! musical meaning of those bytes (that is [`crate::model`]). Because the layout is still being
//! reverse-engineered, the concrete typed parsers ([`SvdHeader`] and friends) are grown against
//! real sample files. Until a field is confirmed, prefer [`Raw`], which loads a file verbatim and
//! exposes generic inspection helpers.

mod ins;
mod msmp;
mod pat;
mod raw;
mod records;
mod sample;
mod svd;
mod zone;

pub use pat::{
    expansion_banks, multisample_slots, remap_multisample_slots, remap_sample_slots, sample_slots,
    PatArea, Tone,
};

/// The user sample slots one record of `tag` plays, 1-based.
///
/// Two areas hold sample references and they store them differently: a `PATa` tone keeps four
/// partials inline, while a drum kit's live in its paired `INSa`, 88 instruments of four wave
/// blocks each. Everything that has to follow those references — repackaging, rebasing, reporting —
/// goes through here so the two cannot drift apart, and so an area with no decoded link answers
/// with an empty list rather than by being forgotten.
pub fn sample_slots_of(tag: &[u8; 4], record: &[u8]) -> Vec<u16> {
    match tag {
        b"PATa" => pat::sample_slots(record),
        b"INSa" => ins::sample_slots(record),
        // A multisample names samples per key — a tone reaches these only through it.
        b"MLSa" | b"MSPa" => msmp::sample_slots(record),
        _ => Vec::new(),
    }
}

/// Rewrite one record's user-sample references, whichever area it belongs to.
pub fn remap_sample_slots_of(
    tag: &[u8; 4],
    record: &mut [u8],
    remap: &std::collections::BTreeMap<u16, u16>,
) {
    match tag {
        b"PATa" => pat::remap_sample_slots(record, remap),
        b"INSa" => ins::remap_sample_slots(record, remap),
        b"MLSa" | b"MSPa" => msmp::remap_sample_slots(record, remap),
        _ => {}
    }
}
pub use msmp::{key_map as multisample_key_map, KeyMap};
pub use raw::Raw;
pub use records::RecordTable;
pub use sample::{Multisample, SampleBank, SampleData, SampleSlot, PANEL_SLOTS};
pub(crate) use svd::ascii_trim;
pub use svd::{Area, Kind, Svd, PREAMBLE_LEN};
pub use zone::{RawZone, ZoneSettings};

/// Read the user sample bank (`SMPa` / `USDa` / `MLSa`) of a file.
pub use sample::read as read_samples;
