//! `fantom-core` — reading and modeling Roland Fantom data files.
//!
//! The crate is deliberately **pure**: it turns bytes into typed values and holds no opinion
//! about I/O, logging, or presentation, so it can back a CLI, a GUI, or a WASM web UI equally.
//!
//! Layering:
//! - [`container`] — SVD/SVZ *framing* (size prefix, memory-area table, zone/tone tables).
//!   Knows about the file envelope, not the musical meaning of its contents.
//! - [`model`] — the domain: [`model::Scene`], [`model::Zone`], [`model::ToneRef`], metadata.
//! - [`address`] — the one table saying which area a tone address indexes, and at which record.
//! - [`codec`] — maps container bytes onto [`model`] types (read now, write later).
//! - [`params`] — Roland's parameter map, file bytes against SysEx addresses, for tones and scenes.
//! - [`presets`] — factory ZEN-Core preset tone name lookup (bundled sound list).
//! - [`factory`] — every sound the instrument ships with, at the address a zone selects it by.
//! - [`expansions`] — the sounds each expansion adds, keyed by product code.
//! - [`diff`] — compares two files by area and record; the tool that finds new offsets.
//! - [`role`] — what a file is *for*: a backup and a scene export are both `SVD5`.
//! - [`requirements`] — the dependency closure as a value: what material needs from its destination.
//! - [`tonebank`] — repackaging SVZ tone banks, which carry their samples.
//! - [`convert`] — SVD to SVZ: lifting a user tone out of a backup with the audio it plays.
//! - [`samplebank`] — building a sample-only SVZ, the one container that moves user audio.
//! - [`checksum`] / [`verify`] — the CRC-32 Roland stores per record, and checking a file against it.
//!
//! The layout was reverse-engineered from a Roland FANTOM-6 and validated against panel ground
//! truth; see `docs/FORMAT.md` (including a note on Roland's confusingly similar model names).
//! [`container::Raw`] plus the CLI `inspect` command are the microscope used to learn the parts
//! still unknown.

pub mod address;
pub mod checksum;
pub mod codec;
pub mod container;
pub mod convert;
pub mod diff;
pub mod expansions;
pub mod factory;
pub mod model;
pub mod params;
pub mod presets;
pub mod repackage;
pub mod requirements;
pub mod role;
pub mod samplebank;
pub mod tonebank;
pub mod verify;

mod error;

pub use error::{Error, Result};
