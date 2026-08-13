use std::io::Cursor;

use binrw::BinRead;

use crate::container::Raw;
use crate::{Error, Result};

/// Which envelope a file uses. Both put their area table at `0x10`; they differ in the preamble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `SVD5` — scene banks and full backups. A u16 header length, then the magic at `0x02`.
    Svd,
    /// `SVZa` — tone exports. The magic leads at `0x00`, and byte `0x04` is the area count.
    Svz,
}

impl Kind {
    /// The four-character magic that identifies this envelope.
    pub fn magic(self) -> &'static [u8; 4] {
        match self {
            Self::Svd => b"SVD5",
            Self::Svz => b"SVZa",
        }
    }
}

/// Bytes before the area table. The same in both envelopes.
pub const PREAMBLE_LEN: usize = 0x10;

/// A parsed container: the file preamble plus its table of memory areas.
///
/// This is the *envelope* only — it tells you which areas exist and where their bytes live, not
/// what those bytes mean. Layout is documented in `docs/FORMAT.md` and confirmed against FANTOM-6
/// backups and tone exports.
#[derive(Debug, Clone, PartialEq)]
pub struct Svd {
    /// Which envelope this file uses.
    pub kind: Kind,
    /// One entry per memory area (Performances, Patches, System, …).
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
    /// Parse a container from raw file bytes, accepting either envelope.
    pub fn parse(raw: &Raw) -> Result<Self> {
        let bytes = raw.bytes();
        let head = bytes
            .get(..PREAMBLE_LEN)
            .ok_or_else(|| Error::Unrecognized("file is shorter than a container header".into()))?;

        // SVZ leads with its magic; SVD puts a u16 length first and the magic at 0x02.
        let (kind, count) = if &head[..4] == Kind::Svz.magic() {
            (Kind::Svz, head[4] as usize)
        } else if &head[2..6] == Kind::Svd.magic() {
            let header_size = u16::from_le_bytes([head[0], head[1]]) as usize;
            (Kind::Svd, header_size.saturating_sub(14) / Area::LEN)
        } else {
            return Err(Error::Unrecognized(format!(
                "not an SVD5 or SVZa container (starts with {:?})",
                &head[..6]
            )));
        };

        let table = bytes
            .get(PREAMBLE_LEN..PREAMBLE_LEN + count * Area::LEN)
            .ok_or_else(|| {
                Error::Unrecognized(format!("area table of {count} entries exceeds the file"))
            })?;
        let mut cursor = Cursor::new(table);
        let areas = (0..count)
            .map(|_| Area::read(&mut cursor))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(Self { kind, areas })
    }

    /// The bytes before the area table, carried verbatim when rebuilding a file.
    pub fn preamble<'a>(&self, raw: &'a Raw) -> Result<&'a [u8]> {
        raw.bytes()
            .get(..PREAMBLE_LEN)
            .ok_or_else(|| Error::Unrecognized("container header is truncated".into()))
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

    /// A minimal SVZ tone export: the magic leads, and byte 0x04 is the area count.
    fn synthetic_svz() -> Raw {
        let mut b = Vec::new();
        b.extend_from_slice(b"SVZa"); // 0x00
        b.push(1); // 0x04 area count
        b.push(3); // 0x05 format revision
        b.extend_from_slice(b"KY019$"); // 0x06
        b.extend_from_slice(&[0u8; 4]); // 0x0c
                                        // area table (0x10): one PATa entry
        b.extend_from_slice(b"PATa");
        b.extend_from_slice(b"ZCOR");
        b.extend_from_slice(&0x20u32.to_le_bytes()); // offset
        b.extend_from_slice(&0x34u32.to_le_bytes()); // size: info 20 + one 32-byte record
                                                     // area body: count, record_size, info_length = 20 (16 + one 4-byte word)
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0x20u32.to_le_bytes());
        b.extend_from_slice(&20u32.to_le_bytes());
        b.extend_from_slice(&[0u8; 4]);
        b.extend_from_slice(&0xdeadbeefu32.to_le_bytes()); // per-record info word
        b.extend_from_slice(b"ACYL Lead\0\0\0\0\0\0\0");
        b.extend_from_slice(&[0u8; 16]);
        Raw::from_bytes(b)
    }

    #[test]
    fn parses_header_and_single_area() {
        let raw = synthetic_svd();
        let svd = Svd::parse(&raw).unwrap();
        assert_eq!(svd.kind, Kind::Svd);
        assert_eq!(svd.areas.len(), 1);
        assert_eq!(svd.areas[0].tag_str(), "PRFa");
        assert_eq!(svd.areas[0].offset, 0x20);
    }

    /// SVZ tone exports use a different preamble but the same area table.
    #[test]
    fn parses_an_svz_tone_export() {
        let raw = synthetic_svz();
        let svd = Svd::parse(&raw).unwrap();
        assert_eq!(svd.kind, Kind::Svz);
        assert_eq!(svd.areas.len(), 1);
        assert_eq!(svd.areas[0].tag_str(), "PATa");
        assert_eq!(svd.areas[0].format_str(), "ZCOR");
    }

    /// The records sit after `info_length` bytes, not after a fixed 16.
    #[test]
    fn svz_records_start_after_the_declared_info_block() {
        let raw = synthetic_svz();
        let svd = Svd::parse(&raw).unwrap();
        let area = svd.area(b"PATa").unwrap();
        let table = crate::container::RecordTable::parse(area, svd.area_bytes(&raw, area).unwrap())
            .unwrap();

        assert_eq!(table.info_len, 20);
        assert_eq!(table.len(), 1);
        assert_eq!(&table.record(0).unwrap()[..9], b"ACYL Lead");
        assert_eq!(table.info_stride(), 4);
        assert_eq!(table.record_info(0).unwrap(), 0xdeadbeefu32.to_le_bytes());
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
