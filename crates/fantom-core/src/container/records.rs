use crate::container::{Area, Raw, Svd};
use crate::{Error, Result};

/// A fixed-stride record table — the shape every named SVD area shares.
///
/// An area body opens with a 16-byte header (`count`, `record_size`, `info_length`, reserved) and
/// is followed by `count` records of `record_size` bytes. `PRFa` holds scenes this way, `PATa`
/// tones, `SMPa` samples, and so on; only the record *contents* differ per area.
///
/// Confirmed on FANTOM-6 backups and scene exports — see `docs/FORMAT.md`.
pub struct RecordTable<'a> {
    /// Four-character area tag, e.g. `PRFa`.
    pub tag: [u8; 4],
    /// Format/version stamp, e.g. `KY19`.
    pub format: [u8; 4],
    /// Absolute file offset of the area (its 16-byte header).
    pub area_offset: usize,
    /// Record count declared in the header. May exceed what the area actually holds.
    pub declared_count: usize,
    /// Bytes per record.
    pub record_size: usize,
    /// The area bytes, header included.
    bytes: &'a [u8],
}

impl<'a> RecordTable<'a> {
    /// Size of the per-area header that precedes the records.
    pub const HEADER_LEN: usize = 0x10;
    const COUNT_OFFSET: usize = 0x00;
    const RECORD_SIZE_OFFSET: usize = 0x04;

    /// Parse an area's record table from its bytes.
    pub fn parse(area: &Area, bytes: &'a [u8]) -> Result<Self> {
        let tag = area.tag_str();
        let count = read_u32(bytes, Self::COUNT_OFFSET, &tag)? as usize;
        let record_size = read_u32(bytes, Self::RECORD_SIZE_OFFSET, &tag)? as usize;
        if record_size == 0 {
            return Err(Error::Unrecognized(format!("{tag} record size is zero")));
        }
        Ok(Self {
            tag: area.tag,
            format: area.format,
            area_offset: area.offset as usize,
            declared_count: count,
            record_size,
            bytes,
        })
    }

    /// Parse the table for `tag`, if the file has that area.
    pub fn from_svd(raw: &'a Raw, svd: &Svd, tag: &[u8; 4]) -> Result<Option<Self>> {
        let Some(area) = svd.area(tag) else {
            return Ok(None);
        };
        Ok(Some(Self::parse(area, svd.area_bytes(raw, area)?)?))
    }

    /// The 16-byte area header.
    pub fn header(&self) -> &'a [u8] {
        &self.bytes[..Self::HEADER_LEN.min(self.bytes.len())]
    }

    /// How many whole records the area actually contains, never more than declared.
    pub fn len(&self) -> usize {
        let body = self.bytes.len().saturating_sub(Self::HEADER_LEN);
        self.declared_count.min(body / self.record_size)
    }

    /// Whether the area holds no records.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bytes of record `index`.
    pub fn record(&self, index: usize) -> Option<&'a [u8]> {
        if index >= self.len() {
            return None;
        }
        let start = Self::HEADER_LEN + index * self.record_size;
        self.bytes.get(start..start + self.record_size)
    }

    /// Every record, in storage order.
    pub fn records(&self) -> impl Iterator<Item = &'a [u8]> + '_ {
        (0..self.len()).filter_map(|i| self.record(i))
    }

    /// Absolute file offset of record `index`, whether or not it is present.
    pub fn record_offset(&self, index: usize) -> usize {
        self.area_offset + Self::HEADER_LEN + index * self.record_size
    }
}

fn read_u32(bytes: &[u8], at: usize, tag: &str) -> Result<u32> {
    let slice = bytes
        .get(at..at + 4)
        .ok_or_else(|| Error::Unrecognized(format!("{tag} area truncated at offset {at}")))?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(offset: u32, size: u32) -> Area {
        Area {
            tag: *b"PATa",
            format: *b"KY19",
            offset,
            size,
        }
    }

    fn body(count: u32, record_size: u32, records: usize) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&count.to_le_bytes());
        b.extend_from_slice(&record_size.to_le_bytes());
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&[0u8; 4]);
        for i in 0..records {
            b.extend(std::iter::repeat_n(i as u8, record_size as usize));
        }
        b
    }

    #[test]
    fn reads_records_at_the_declared_stride() {
        let bytes = body(3, 8, 3);
        let table = RecordTable::parse(&area(0x100, bytes.len() as u32), &bytes).unwrap();
        assert_eq!(table.len(), 3);
        assert_eq!(table.record_size, 8);
        assert_eq!(table.record(1).unwrap(), &[1u8; 8]);
        assert_eq!(table.record_offset(1), 0x100 + 0x10 + 8);
        assert!(table.record(3).is_none());
    }

    #[test]
    fn caps_the_declared_count_at_what_the_area_holds() {
        // Header claims 9 records but only 2 are present — a truncated or mis-sized area.
        let bytes = body(9, 8, 2);
        let table = RecordTable::parse(&area(0, bytes.len() as u32), &bytes).unwrap();
        assert_eq!(table.declared_count, 9);
        assert_eq!(table.len(), 2);
        assert!(table.record(2).is_none());
    }

    #[test]
    fn rejects_a_zero_record_size() {
        let bytes = body(1, 0, 0);
        assert!(RecordTable::parse(&area(0, bytes.len() as u32), &bytes).is_err());
    }
}
