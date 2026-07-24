use std::fmt::{self, Write as _};
use std::path::Path;

use crate::Result;

/// A Fantom file loaded verbatim into memory, with generic inspection helpers.
///
/// This is the reverse-engineering microscope: it makes no assumptions about the layout, so it
/// works on any `.svd` / `.svz` / `.sdz` blob. Confirmed structure graduates into typed parsers
/// elsewhere in [`crate::container`].
#[derive(Clone)]
pub struct Raw {
    bytes: Vec<u8>,
}

impl Raw {
    /// Read a file from disk verbatim.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::from_bytes(std::fs::read(path)?))
    }

    /// Wrap already-loaded bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// The raw file contents.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Total length in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the file is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The leading ASCII magic, if the first `n` bytes are printable — Roland containers tend to
    /// start with an identifier such as `SVD ` or `PSVD`. Returns `None` when not printable.
    pub fn ascii_magic(&self, n: usize) -> Option<String> {
        let head = self.bytes.get(..n)?;
        if head.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
            Some(String::from_utf8_lossy(head).into_owned())
        } else {
            None
        }
    }

    /// A classic `hexdump -C` style rendering of a byte range, for eyeballing structure.
    ///
    /// `offset` and `len` are clamped to the file bounds. Each line shows 16 bytes as hex plus an
    /// ASCII gutter.
    pub fn hexdump(&self, offset: usize, len: usize) -> String {
        let start = offset.min(self.bytes.len());
        let end = start.saturating_add(len).min(self.bytes.len());
        let slice = &self.bytes[start..end];

        let mut out = String::new();
        for (row, chunk) in slice.chunks(16).enumerate() {
            let addr = start + row * 16;
            let _ = write!(out, "{addr:08x}  ");

            for (i, b) in chunk.iter().enumerate() {
                let _ = write!(out, "{b:02x} ");
                if i == 7 {
                    out.push(' ');
                }
            }
            // Pad a short final row so the ASCII gutter stays aligned.
            for i in chunk.len()..16 {
                out.push_str("   ");
                if i == 7 {
                    out.push(' ');
                }
            }

            out.push_str(" |");
            for b in chunk {
                out.push(if b.is_ascii_graphic() || *b == b' ' {
                    *b as char
                } else {
                    '.'
                });
            }
            out.push_str("|\n");
        }
        out
    }
}

impl fmt::Debug for Raw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Raw")
            .field("len", &self.bytes.len())
            .field("magic", &self.ascii_magic(4))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexdump_renders_addresses_and_ascii() {
        let raw = Raw::from_bytes(b"SVD \x00\x01ABCDEF".to_vec());
        let dump = raw.hexdump(0, raw.len());
        assert!(dump.starts_with("00000000  53 56 44 20"));
        assert!(dump.contains("|SVD ..ABCDEF|"));
    }

    #[test]
    fn ascii_magic_detects_printable_header() {
        let raw = Raw::from_bytes(b"SVD \x00\x00".to_vec());
        assert_eq!(raw.ascii_magic(4).as_deref(), Some("SVD "));
        assert_eq!(raw.ascii_magic(6), None); // trailing NULs are not printable
    }
}
