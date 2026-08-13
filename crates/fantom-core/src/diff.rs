//! Byte-level comparison of two SVD files, resolved to `AREA[record]+offset`.
//!
//! Every confirmed offset in `docs/FORMAT.md` came from exporting two files that differ by one
//! deliberate change and finding the bytes that moved. This module is that workflow: it aligns the
//! two files area by area and record by record, so a differing byte is reported where it means
//! something (`DCWa[0]+0x0015`) rather than as a bare file offset.
//!
//! Aligning per record — instead of comparing the files as flat byte strings — is what makes a
//! capture pair readable when the two files are different sizes, which happens as soon as a
//! dependency area gains or loses a record.

use crate::container::{ascii_trim, Raw, RecordTable, Svd};
use crate::Result;

/// The leading `count` and `record_size` fields of an area header, reported semantically instead.
const STRUCTURAL_HEADER_LEN: usize = 8;

/// Which of the two compared files a one-sided finding belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// A maximal run of consecutive differing bytes within one aligned block.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteRun {
    /// Offset of the run within its record (or within the area header).
    pub offset: usize,
    /// Absolute file offset in the left file.
    pub left_at: usize,
    /// Absolute file offset in the right file.
    pub right_at: usize,
    pub left: Vec<u8>,
    pub right: Vec<u8>,
}

/// One way in which two SVD files differ.
#[derive(Debug, Clone, PartialEq)]
pub enum Finding {
    /// An area present in one file and absent from the other.
    AreaOnlyIn {
        tag: String,
        side: Side,
        size: u32,
        records: usize,
    },
    /// Both files have the area, but its records are a different size — not comparable.
    RecordSizeDiffers {
        tag: String,
        left: usize,
        right: usize,
    },
    /// Both files have the area with different record counts.
    RecordCountDiffers {
        tag: String,
        left: usize,
        right: usize,
    },
    /// A record present in one file only (beyond the other's record count).
    RecordOnlyIn {
        tag: String,
        side: Side,
        record: usize,
        name: String,
    },
    /// Differing bytes inside an area's 16-byte header.
    AreaHeader { tag: String, runs: Vec<ByteRun> },
    /// Differing bytes in an area that is not a record table, compared whole.
    AreaBytes { tag: String, runs: Vec<ByteRun> },
    /// Differing bytes inside a record shared by both files.
    Record {
        tag: String,
        record: usize,
        runs: Vec<ByteRun>,
    },
}

impl Finding {
    /// The area tag this finding concerns.
    pub fn tag(&self) -> &str {
        match self {
            Self::AreaOnlyIn { tag, .. }
            | Self::RecordSizeDiffers { tag, .. }
            | Self::RecordCountDiffers { tag, .. }
            | Self::RecordOnlyIn { tag, .. }
            | Self::AreaHeader { tag, .. }
            | Self::AreaBytes { tag, .. }
            | Self::Record { tag, .. } => tag,
        }
    }

    /// How many differing bytes this finding accounts for.
    pub fn changed_bytes(&self) -> usize {
        match self {
            Self::AreaHeader { runs, .. }
            | Self::AreaBytes { runs, .. }
            | Self::Record { runs, .. } => runs.iter().map(|r| r.left.len()).sum(),
            _ => 0,
        }
    }
}

/// Compare two SVD files, aligning them by area and record.
///
/// Areas are matched by tag and records by index — the alignment the format itself implies. A
/// capture pair that differs by one edit therefore reports one run, wherever the edit landed and
/// whatever else changed size around it.
pub fn compare(left: &Raw, right: &Raw) -> Result<Vec<Finding>> {
    let left_svd = Svd::parse(left)?;
    let right_svd = Svd::parse(right)?;

    let mut findings = Vec::new();
    let mut seen = Vec::new();

    for area in &left_svd.areas {
        seen.push(area.tag);
        let tag = area.tag_str();
        let left_bytes = left_svd.area_bytes(left, area)?;

        // Not every area is a record table — `DIFa` is a bare checksum blob, `USDa` a waveform
        // directory. One area we cannot parse must not abort the whole comparison, so fall back to
        // comparing its bytes.
        let Ok(left_table) = RecordTable::parse(area, left_bytes) else {
            if let Some(right_area) = right_svd.area(&area.tag) {
                let right_bytes = right_svd.area_bytes(right, right_area)?;
                let runs = runs_between(
                    left_bytes,
                    right_bytes,
                    area.offset as usize,
                    right_area.offset as usize,
                );
                if !runs.is_empty() || left_bytes.len() != right_bytes.len() {
                    findings.push(Finding::AreaBytes { tag, runs });
                }
            }
            continue;
        };

        let Some(right_area) = right_svd.area(&area.tag) else {
            findings.push(Finding::AreaOnlyIn {
                tag,
                side: Side::Left,
                size: area.size,
                records: left_table.len(),
            });
            continue;
        };
        let right_bytes = right_svd.area_bytes(right, right_area)?;
        let Ok(right_table) = RecordTable::parse(right_area, right_bytes) else {
            // Parsed on the left but not the right: compare the bytes rather than give up.
            let runs = runs_between(
                left_bytes,
                right_bytes,
                area.offset as usize,
                right_area.offset as usize,
            );
            if !runs.is_empty() || left_bytes.len() != right_bytes.len() {
                findings.push(Finding::AreaBytes { tag, runs });
            }
            continue;
        };

        if left_table.record_size != right_table.record_size {
            findings.push(Finding::RecordSizeDiffers {
                tag,
                left: left_table.record_size,
                right: right_table.record_size,
            });
            continue;
        }

        // Skip the header's `count` and `record_size`: a differing record count is reported below
        // as what it means, not as four changed bytes the reader has to decode.
        let runs = runs_between_from(
            left_table.header(),
            right_table.header(),
            left_table.area_offset,
            right_table.area_offset,
            STRUCTURAL_HEADER_LEN,
        );
        if !runs.is_empty() {
            findings.push(Finding::AreaHeader {
                tag: tag.clone(),
                runs,
            });
        }

        if left_table.len() != right_table.len() {
            findings.push(Finding::RecordCountDiffers {
                tag: tag.clone(),
                left: left_table.len(),
                right: right_table.len(),
            });
        }

        for index in 0..left_table.len().min(right_table.len()) {
            let (Some(l), Some(r)) = (left_table.record(index), right_table.record(index)) else {
                continue;
            };
            let runs = runs_between(
                l,
                r,
                left_table.record_offset(index),
                right_table.record_offset(index),
            );
            if !runs.is_empty() {
                findings.push(Finding::Record {
                    tag: tag.clone(),
                    record: index,
                    runs,
                });
            }
        }

        for (side, table) in [(Side::Left, &left_table), (Side::Right, &right_table)] {
            let shared = left_table.len().min(right_table.len());
            for index in shared..table.len() {
                findings.push(Finding::RecordOnlyIn {
                    tag: tag.clone(),
                    side,
                    record: index,
                    name: record_name(table, index),
                });
            }
        }
    }

    for area in &right_svd.areas {
        if seen.contains(&area.tag) {
            continue;
        }
        // An area present only on the right may be one that is not a record table at all — a
        // `USDa` declares `record_size = 0`, which is exactly what appears when a drum kit gains a
        // user sample. Report it as a whole area rather than failing the entire comparison, which
        // is the one thing a diff must never do when the files genuinely differ.
        let records = RecordTable::parse(area, right_svd.area_bytes(right, area)?)
            .map(|table| table.len())
            .unwrap_or(0);
        findings.push(Finding::AreaOnlyIn {
            tag: area.tag_str(),
            side: Side::Right,
            size: area.size,
            records,
        });
    }

    Ok(findings)
}

/// A record's leading 16 bytes as text, for labelling one-sided records.
///
/// Most areas start a record with its name; the ones that do not (`MDLa`, `ACBa`) simply produce
/// an unhelpful-but-harmless label here rather than a wrong one elsewhere.
fn record_name(table: &RecordTable<'_>, index: usize) -> String {
    table
        .record(index)
        .map(|r| ascii_trim(&r[..16.min(r.len())]))
        .unwrap_or_default()
}

/// Group the differing bytes of two blocks into maximal consecutive runs.
fn runs_between(left: &[u8], right: &[u8], left_at: usize, right_at: usize) -> Vec<ByteRun> {
    runs_between_from(left, right, left_at, right_at, 0)
}

/// As [`runs_between`], ignoring everything before `from`.
fn runs_between_from(
    left: &[u8],
    right: &[u8],
    left_at: usize,
    right_at: usize,
    from: usize,
) -> Vec<ByteRun> {
    let mut runs: Vec<ByteRun> = Vec::new();
    let mut start: Option<usize> = None;
    let len = left.len().min(right.len());

    for i in from..=len {
        let differs = i < len && left[i] != right[i];
        match (differs, start) {
            (true, None) => start = Some(i),
            (false, Some(from)) => {
                runs.push(ByteRun {
                    offset: from,
                    left_at: left_at + from,
                    right_at: right_at + from,
                    left: left[from..i].to_vec(),
                    right: right[from..i].to_vec(),
                });
                start = None;
            }
            _ => {}
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An area to build: tag, record size, and its records.
    type AreaSpec<'a> = (&'a [u8; 4], usize, Vec<Vec<u8>>);

    /// Build an SVD whose areas each hold the given records.
    fn svd(areas: &[AreaSpec<'_>]) -> Raw {
        let header_len = 14 + areas.len() * 16;
        let mut table = Vec::new();
        let mut bodies = Vec::new();
        let mut at = 0x10 + areas.len() * 16;

        for (tag, record_size, records) in areas {
            let mut body = Vec::new();
            body.extend_from_slice(&(records.len() as u32).to_le_bytes());
            body.extend_from_slice(&(*record_size as u32).to_le_bytes());
            body.extend_from_slice(&16u32.to_le_bytes());
            body.extend_from_slice(&[0u8; 4]);
            for record in records {
                let mut r = record.clone();
                r.resize(*record_size, 0);
                body.extend_from_slice(&r);
            }
            table.extend_from_slice(*tag);
            table.extend_from_slice(b"KY19");
            table.extend_from_slice(&(at as u32).to_le_bytes());
            table.extend_from_slice(&(body.len() as u32).to_le_bytes());
            at += body.len();
            bodies.extend_from_slice(&body);
        }

        let mut b = Vec::new();
        b.extend_from_slice(&(header_len as u16).to_le_bytes());
        b.extend_from_slice(b"SVD5");
        b.extend_from_slice(&[0u8; 10]);
        b.extend_from_slice(&table);
        b.extend_from_slice(&bodies);
        Raw::from_bytes(b)
    }

    fn named(name: &str) -> Vec<u8> {
        let mut r = vec![b' '; 32];
        r[..name.len()].copy_from_slice(name.as_bytes());
        r
    }

    #[test]
    fn locates_a_single_changed_byte_in_a_record() {
        let a = svd(&[(b"DCWa", 32, vec![named("Stage Grand3")])]);
        let b = svd(&[(b"DCWa", 32, vec![named("Stage Grand4")])]);
        let findings = compare(&a, &b).unwrap();

        assert_eq!(findings.len(), 1);
        let Finding::Record { tag, record, runs } = &findings[0] else {
            panic!("expected a record diff, got {:?}", findings[0]);
        };
        assert_eq!(tag, "DCWa");
        assert_eq!(*record, 0);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].offset, 11);
        assert_eq!(runs[0].left, b"3");
        assert_eq!(runs[0].right, b"4");
    }

    #[test]
    fn splits_non_adjacent_changes_into_separate_runs() {
        let mut left = named("Tone");
        let mut right = named("Tone");
        left[20] = 1;
        right[20] = 2;
        left[21] = 3;
        right[21] = 4;
        left[25] = 5;
        right[25] = 6;

        let findings = compare(
            &svd(&[(b"MDLa", 32, vec![left])]),
            &svd(&[(b"MDLa", 32, vec![right])]),
        )
        .unwrap();

        let Finding::Record { runs, .. } = &findings[0] else {
            panic!("expected a record diff");
        };
        assert_eq!(runs.len(), 2);
        assert_eq!((runs[0].offset, runs[0].left.len()), (20, 2));
        assert_eq!((runs[1].offset, runs[1].left.len()), (25, 1));
    }

    /// The point of aligning per record: a size change must not smear every later byte.
    #[test]
    fn aligns_records_when_one_file_has_more_of_them() {
        let a = svd(&[(b"DCWa", 32, vec![named("Stage Grand4")])]);
        let b = svd(&[(
            b"DCWa",
            32,
            vec![named("Stage Grand4"), named("Stage Grand4 3")],
        )]);
        let findings = compare(&a, &b).unwrap();

        assert!(matches!(
            findings.as_slice(),
            [
                Finding::RecordCountDiffers {
                    left: 1,
                    right: 2,
                    ..
                },
                Finding::RecordOnlyIn {
                    side: Side::Right,
                    record: 1,
                    ..
                },
            ]
        ));
        assert_eq!(findings[1].tag(), "DCWa");
    }

    #[test]
    fn reports_areas_present_in_only_one_file() {
        let a = svd(&[(b"PRFa", 32, vec![named("Scene")])]);
        let b = svd(&[
            (b"PRFa", 32, vec![named("Scene")]),
            (b"ACBa", 32, vec![named("Soft & Subtle2")]),
        ]);
        let findings = compare(&a, &b).unwrap();

        assert_eq!(findings.len(), 1);
        assert!(matches!(
            &findings[0],
            Finding::AreaOnlyIn {
                side: Side::Right,
                records: 1,
                ..
            }
        ));
        assert_eq!(findings[0].tag(), "ACBa");
    }

    /// An area that is not a record table (a zeroed `DIFa`, say) must not abort the comparison of
    /// everything else — it falls back to a byte-level diff of that area.
    #[test]
    fn an_area_that_is_not_a_record_table_falls_back_to_comparing_bytes() {
        fn with_raw_difa(scene: &str, difa: &[u8]) -> Raw {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&(14u16 + 32).to_le_bytes());
            bytes.extend_from_slice(b"SVD5");
            bytes.extend_from_slice(&[0u8; 10]);
            let prfa = {
                let mut b = vec![0u8; 16];
                b[0..4].copy_from_slice(&1u32.to_le_bytes());
                b[4..8].copy_from_slice(&32u32.to_le_bytes());
                b.extend_from_slice(&named(scene)[..32]);
                b
            };
            let first = 0x10 + 32;
            for (tag, body) in [(b"PRFa", &prfa), (b"DIFa", &difa.to_vec())] {
                bytes.extend_from_slice(tag);
                bytes.extend_from_slice(b"KY19");
                let at = if tag == b"PRFa" {
                    first
                } else {
                    first + prfa.len()
                };
                bytes.extend_from_slice(&(at as u32).to_le_bytes());
                bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
            }
            bytes.extend_from_slice(&prfa);
            bytes.extend_from_slice(difa);
            Raw::from_bytes(bytes)
        }

        // A DIFa of all zeros decodes as record_size = 0, which is not a parseable record table.
        let a = with_raw_difa("Scene", &[0u8; 32]);
        let mut zeros = [0u8; 32];
        zeros[20] = 0x99;
        let b = with_raw_difa("Scene", &zeros);

        let findings = compare(&a, &b).unwrap();
        assert_eq!(findings.len(), 1, "{findings:?}");
        let Finding::AreaBytes { tag, runs } = &findings[0] else {
            panic!("expected a byte-level area diff, got {:?}", findings[0]);
        };
        assert_eq!(tag, "DIFa");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].offset, 20);
        assert_eq!(runs[0].right, [0x99]);
    }

    #[test]
    fn identical_files_report_nothing() {
        let a = svd(&[(b"PRFa", 32, vec![named("Scene")])]);
        let b = svd(&[(b"PRFa", 32, vec![named("Scene")])]);
        assert!(compare(&a, &b).unwrap().is_empty());
    }
}
