//! `fantom-core` — reading and modeling Roland Fantom data files.
//!
//! The crate is deliberately **pure**: it turns bytes into typed values and holds no opinion
//! about I/O, logging, or presentation, so it can back a CLI, a GUI, or a WASM web UI equally.
//!
//! Layering:
//! - [`container`] — SVD/SVZ *framing* (size prefix, memory-area table, zone headers).
//!   Knows about the file envelope, not the musical meaning of its contents.
//! - [`model`] — the domain: [`model::Scene`], [`model::Zone`], [`model::Tone`], metadata.
//! - [`codec`] — maps container bytes onto [`model`] types (read now, write later).
//! - [`device`] — isolates per-model quirks (Fantom-0 vs 6/7/8) behind [`device::Device`].
//!
//! The SVD byte layout is only partially understood (community reverse-engineering), so the
//! typed parsers grow against real fixture files. Until then [`container::Raw`] plus the CLI
//! `inspect` command are the microscope used to learn the format.

pub mod codec;
pub mod container;
pub mod device;
pub mod model;
pub mod presets;

mod error;

pub use error::{Error, Result};
