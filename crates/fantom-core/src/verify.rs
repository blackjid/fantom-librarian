//! Structural checks on a container: does it hold together, and do its checksums agree?
//!
//! Repackaging rebuilds a file from parts of another. Tests cover the cases we thought of; this
//! checks the file itself, so a real 35 MB backup with 512 scenes gets the same scrutiny as a
//! synthetic fixture. It is cheap enough to run on every output.
//!
//! What it can prove: every area's declared geometry matches its actual size, and every record
//! whose area carries a CRC-32 still matches it. What it cannot: whether the *contents* mean what
//! we think — only the instrument can answer that, which is what `canary` is for.

use crate::checksum::crc32;
use crate::container::{Raw, RecordTable, Svd};
use crate::Result;

/// Something wrong with a file's structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// An area's `info_length + count * record_size` disagrees with its size in the area table.
    AreaSize {
        tag: String,
        declared: usize,
        actual: usize,
    },
    /// A record's stored CRC-32 does not match its bytes.
    Checksum {
        tag: String,
        record: usize,
        stored: u32,
        computed: u32,
    },
    /// An area declares more records than its bytes can hold.
    Truncated {
        tag: String,
        declared: usize,
        present: usize,
    },
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AreaSize {
                tag,
                declared,
                actual,
            } => write!(
                f,
                "{tag}: geometry says {declared} bytes but the area table says {actual}"
            ),
            Self::Checksum {
                tag,
                record,
                stored,
                computed,
            } => write!(
                f,
                "{tag}[{record}]: checksum {stored:08x} does not match its bytes ({computed:08x})"
            ),
            Self::Truncated {
                tag,
                declared,
                present,
            } => write!(
                f,
                "{tag}: declares {declared} records but only {present} are present"
            ),
        }
    }
}

/// What a check found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Records whose checksum was verified.
    pub checked: usize,
    /// Areas that carry per-record checksums.
    pub areas_with_checksums: usize,
    pub problems: Vec<Problem>,
}

impl Report {
    /// Whether the file is structurally sound.
    pub fn is_ok(&self) -> bool {
        self.problems.is_empty()
    }
}

/// Check a container's geometry and every record checksum it carries.
pub fn check(raw: &Raw) -> Result<Report> {
    let svd = Svd::parse(raw)?;
    let mut report = Report::default();

    for area in &svd.areas {
        let bytes = svd.area_bytes(raw, area)?;
        let Ok(table) = RecordTable::parse(area, bytes) else {
            continue; // not a record table (a bare DIFa blob, say) — nothing to check
        };
        let tag = area.tag_str();

        if table.len() < table.declared_count {
            report.problems.push(Problem::Truncated {
                tag: tag.clone(),
                declared: table.declared_count,
                present: table.len(),
            });
        }

        // Variable-size areas (an SVZ `USDa`) declare record_size 0 and are sized by their
        // directory instead, so the geometry check does not apply to them.
        if table.record_size > 0 && table.declared_count == table.len() {
            let expected = table.info_len + table.declared_count * table.record_size;
            if expected != bytes.len() {
                report.problems.push(Problem::AreaSize {
                    tag: tag.clone(),
                    declared: expected,
                    actual: bytes.len(),
                });
            }
        }

        // A four-byte info word per record is a CRC-32 of that record.
        if table.info_stride() != 4 {
            continue;
        }
        report.areas_with_checksums += 1;
        for index in 0..table.len() {
            let (Some(record), Some(info)) = (table.record(index), table.record_info(index)) else {
                break;
            };
            let stored = u32::from_le_bytes(info.try_into().unwrap());
            let computed = crc32(record);
            report.checked += 1;
            if stored != computed {
                report.problems.push(Problem::Checksum {
                    tag: tag.clone(),
                    record: index,
                    stored,
                    computed,
                });
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::PREAMBLE_LEN;

    /// An SVZ with one area whose records carry CRC-32 info words.
    fn svz(records: &[Vec<u8>], record_size: usize, corrupt: bool) -> Raw {
        let info_len = 16 + records.len() * 4;
        let mut body = Vec::new();
        body.extend_from_slice(&(records.len() as u32).to_le_bytes());
        body.extend_from_slice(&(record_size as u32).to_le_bytes());
        body.extend_from_slice(&(info_len as u32).to_le_bytes());
        body.extend_from_slice(&[0u8; 4]);
        for (i, record) in records.iter().enumerate() {
            let mut sum = crc32(record);
            if corrupt && i == 1 {
                sum ^= 0xFF;
            }
            body.extend_from_slice(&sum.to_le_bytes());
        }
        for record in records {
            body.extend_from_slice(record);
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"SVZa");
        bytes.push(1);
        bytes.push(3);
        bytes.extend_from_slice(b"KY019$");
        bytes.extend_from_slice(&[0u8; 4]);
        bytes.extend_from_slice(b"PATa");
        bytes.extend_from_slice(b"ZCOR");
        bytes.extend_from_slice(&((PREAMBLE_LEN + 16) as u32).to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&body);
        Raw::from_bytes(bytes)
    }

    fn records() -> Vec<Vec<u8>> {
        vec![vec![0xa1; 32], vec![0xb2; 32], vec![0xc3; 32]]
    }

    #[test]
    fn a_sound_file_reports_no_problems() {
        let report = check(&svz(&records(), 32, false)).unwrap();
        assert!(report.is_ok(), "{:?}", report.problems);
        assert_eq!(report.checked, 3);
        assert_eq!(report.areas_with_checksums, 1);
    }

    #[test]
    fn a_record_that_does_not_match_its_checksum_is_reported() {
        let report = check(&svz(&records(), 32, true)).unwrap();
        assert_eq!(report.problems.len(), 1);
        assert!(
            matches!(&report.problems[0], Problem::Checksum { record: 1, tag, .. } if tag == "PATa"),
            "{:?}",
            report.problems
        );
        // The other two still verified.
        assert_eq!(report.checked, 3);
    }

    /// An SVD5 file carries no per-record checksums; checking it must not invent problems.
    #[test]
    fn a_file_without_checksums_checks_clean() {
        let mut body = Vec::new();
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&32u32.to_le_bytes());
        body.extend_from_slice(&16u32.to_le_bytes());
        body.extend_from_slice(&[0u8; 4]);
        body.extend_from_slice(&[7u8; 32]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&30u16.to_le_bytes());
        bytes.extend_from_slice(b"SVD5");
        bytes.extend_from_slice(&[0u8; 10]);
        bytes.extend_from_slice(b"PRFa");
        bytes.extend_from_slice(b"KY19");
        bytes.extend_from_slice(&0x20u32.to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&body);

        let report = check(&Raw::from_bytes(bytes)).unwrap();
        assert!(report.is_ok(), "{:?}", report.problems);
        assert_eq!(report.areas_with_checksums, 0);
        assert_eq!(report.checked, 0);
    }
}
