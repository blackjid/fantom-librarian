//! Per-model quirks, isolated behind a trait.
//!
//! The SVD envelope differs between Roland models (Fantom-0 series vs Fantom 6/7/8, and cousins
//! like the Jupiter-X). Rather than sprinkle `if model == ...` across the codec, model-specific
//! offsets and counts live behind [`Device`]. Adding a new instrument is then a new `impl`, not a
//! rewrite of the parser.

/// Model-specific constants and layout knowledge for a Roland instrument family.
pub trait Device {
    /// Human-readable model name, e.g. `"Fantom-08"`.
    fn name(&self) -> &'static str;

    /// Maximum number of zones a scene can hold on this device (typically 16).
    fn zones_per_scene(&self) -> u8 {
        16
    }
}

/// The Fantom-0 series (FANTOM-06 / 07 / 08).
#[derive(Debug, Clone, Copy, Default)]
pub struct Fantom0;

impl Device for Fantom0 {
    fn name(&self) -> &'static str {
        "Fantom-0"
    }
}
