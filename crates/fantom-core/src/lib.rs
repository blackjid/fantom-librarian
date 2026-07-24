//! `fantom-core` — reading and modeling Roland Fantom data files.
//!
//! The crate is deliberately **pure**: it turns bytes into typed values and holds no opinion
//! about I/O, logging, or presentation, so it can back a CLI, a GUI, or a WASM web UI equally.
//!
//! Layering:
//! - [`container`] — SVD/SVZ *framing* (size prefix, memory-area table, zone/tone tables).
//!   Knows about the file envelope, not the musical meaning of its contents.
//! - [`model`] — the domain: [`model::Scene`], [`model::Zone`], [`model::ToneRef`], metadata.
//! - [`codec`] — maps container bytes onto [`model`] types (read now, write later).
//! - [`presets`] — factory ZEN-Core preset tone name lookup (bundled sound list).
//!
//! The layout was reverse-engineered from Fantom-0 files and validated against panel ground truth;
//! see `docs/FORMAT.md`. [`container::Raw`] plus the CLI `inspect` command are the microscope used
//! to learn the parts still unknown.

pub mod codec;
pub mod container;
pub mod model;
pub mod presets;

mod error;

pub use error::{Error, Result};
