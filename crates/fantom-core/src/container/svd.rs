use std::io::Cursor;

use binrw::{binread, BinRead};

use crate::container::Raw;
use crate::{Error, Result};

/// A parsed SVD5 container: the file header plus its table of memory areas.
///
/// This is the *envelope* only — it tells you which areas exist and where their bytes live, not
/// what those bytes mean. Layout is documented in `docs/FORMAT.md` and confirmed against FANTOM-6
/// backups.
#[binread]
#[derive(Debug, Clone, PartialEq)]
#[br(little)]
pub struct Svd {
    /// Bytes from offset 0x02 to the first data area; also encodes the area count.
    pub header_size: u16,

    #[br(assert(&magic == b"SVD5", "not an SVD5 container (magic = {:?})", magic))]
    pub magic: [u8; 4],

    #[br(temp, count = 10)]
    _reserved: Vec<u8>,

    /// One entry per memory area (Performances, Patches, System, …).
    #[br(count = (header_size as usize).saturating_sub(14) / Area::LEN)]
    pub areas: Vec<Area>,
}

/// One entry in the SVD area table: a tagged, located span of the file.
#[derive(Debug, Clone, PartialEq, BinRead)]
#[br(little)]
pub struct Area {
    /// Four-character area kind, e.g. `PRFa` (Performances/Scenes) or `PATa` (Patches).
    pub tag: [u8; 4],
    /// Format/version stamp, constant within a file (`KY19` on FANTOM-6, `ZCOR` for ZEN-Core).
    pub format: [u8; 4],
    /// Absolute byte offset of the area within the file.
    pub offset: u32,
    /// Length of the area in bytes.
    pub size: u32,
}

impl Area {
    /// Size of one area-table entry on disk.
    pub const LEN: usize = 16;

    /// The area tag as a trimmed string, e.g. `"PRFa"`.
    pub fn tag_str(&self) -> String {
        ascii_trim(&self.tag)
    }

    /// The format/version stamp as a trimmed string, e.g. `"KY19"`.
    pub fn format_str(&self) -> String {
        ascii_trim(&self.format)
    }

    /// The area's byte range as a `start..end` pair, clamped to nothing here (caller validates).
    pub fn range(&self) -> std::ops::Range<usize> {
        let start = self.offset as usize;
        start..start + self.size as usize
    }
}

impl Svd {
    /// Parse an SVD5 container from raw file bytes.
    pub fn parse(raw: &Raw) -> Result<Self> {
        let mut cursor = Cursor::new(raw.bytes());
        Ok(Self::read(&mut cursor)?)
    }

    /// Find the first area with the given four-character tag.
    pub fn area(&self, tag: &[u8; 4]) -> Option<&Area> {
        self.areas.iter().find(|a| &a.tag == tag)
    }

    /// Borrow the bytes of `area` out of `raw`, validating the range lies within the file.
    pub fn area_bytes<'a>(&self, raw: &'a Raw, area: &Area) -> Result<&'a [u8]> {
        raw.bytes().get(area.range()).ok_or_else(|| {
            Error::Unrecognized(format!(
                "area {} range {:?} exceeds file length {}",
                area.tag_str(),
                area.range(),
                raw.len()
            ))
        })
    }
}

/// Trim a fixed ASCII byte field to a `String`, dropping trailing spaces and NULs.
pub(crate) fn ascii_trim(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.trim_end_matches([' ', '\0']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal but structurally valid SVD5 with a single `PRFa` area (committable fixture).
    fn synthetic_svd() -> Raw {
        let mut b = Vec::new();
        // header_size = 14 (preamble) + 16 (one area) = 30 → first area at 0x02+30 = 0x20.
        b.extend_from_slice(&30u16.to_le_bytes()); // 0x00
        b.extend_from_slice(b"SVD5"); // 0x02
        b.extend_from_slice(&[0u8; 10]); // 0x06 reserved
        // area table (0x10): one PRFa entry
        b.extend_from_slice(b"PRFa"); // tag
        b.extend_from_slice(b"KY19"); // format
        b.extend_from_slice(&0x20u32.to_le_bytes()); // offset
        b.extend_from_slice(&0x20u32.to_le_bytes()); // size (one 32-byte record)
        // PRFa area data (0x20): one scene record, name at +0x10
        b.extend_from_slice(&0x10u32.to_le_bytes()); // name_offset
        b.extend_from_slice(&0x20u32.to_le_bytes()); // record_size (stride = 32)
        b.extend_from_slice(&[0u8; 8]); // unknown
        b.extend_from_slice(b"Test Scene\0\0\0\0\0\0"); // 16-char name
        Raw::from_bytes(b)
    }

    #[test]
    fn parses_header_and_single_area() {
        let raw = synthetic_svd();
        let svd = Svd::parse(&raw).unwrap();
        assert_eq!(&svd.magic, b"SVD5");
        assert_eq!(svd.areas.len(), 1);
        assert_eq!(svd.areas[0].tag_str(), "PRFa");
        assert_eq!(svd.areas[0].offset, 0x20);
    }

    #[test]
    fn area_lookup_and_bytes_are_bounds_checked() {
        let raw = synthetic_svd();
        let svd = Svd::parse(&raw).unwrap();
        let prfa = svd.area(b"PRFa").expect("PRFa present");
        let bytes = svd.area_bytes(&raw, prfa).unwrap();
        assert_eq!(bytes.len(), 0x20);
    }

    #[test]
    fn rejects_wrong_magic() {
        let raw = Raw::from_bytes(b"\x1e\x00XXXX".to_vec());
        assert!(Svd::parse(&raw).is_err());
    }
}
