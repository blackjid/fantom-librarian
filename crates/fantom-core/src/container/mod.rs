//! SVD/SVZ container framing.
//!
//! This layer is about the *envelope* of a Fantom file — where its bytes live — not about the
//! musical meaning of those bytes (that is [`crate::model`]). Because the layout is still being
//! reverse-engineered, the concrete typed parsers ([`SvdHeader`] and friends) are grown against
//! real sample files. Until a field is confirmed, prefer [`Raw`], which loads a file verbatim and
//! exposes generic inspection helpers.

mod pat;
mod raw;
mod records;
mod sample;
mod svd;
mod zone;

pub use pat::{remap_sample_slots, sample_slots, PatArea, Tone};
pub use raw::Raw;
pub use records::RecordTable;
pub use sample::{Multisample, SampleBank, SampleData, SampleSlot};
pub use svd::{Area, Kind, Svd, PREAMBLE_LEN};
pub use zone::{RawZone, ZoneSettings};
pub(crate) use svd::ascii_trim;

/// Read the user sample bank (`SMPa` / `USDa` / `MLSa`) of a file.
pub use sample::read as read_samples;
