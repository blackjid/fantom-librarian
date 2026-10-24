//! `DIFa`: the stamp naming the device that wrote a file.
//!
//! One 32-byte record. Byte 0 is a key (1–5, or 0 for a blank stamp), byte 1 is zero, and bytes
//! 2–31 are fifteen little-endian u16 words whose bits are shuffled by one of five permutations,
//! chosen by `(key - 1) % 5`. Unshuffled they read: product code (7 bytes), serial number (7),
//! registration ID (16). It covers none of the file's content, so the instrument cannot check it against
//! anything — which is why a stale or zeroed stamp loads.
//!
//! Decoding round-trips every
//! FANTOM-6 backup in the corpus. See `docs/FORMAT.md`.

use crate::checksum::crc32;
use crate::container::{Area, Raw, RecordTable, Svd};
use crate::Result;

/// Bytes in a stamp.
pub const LEN: usize = 32;

/// First scrambled byte; everything before it is stored in the clear.
const SCRAMBLED_FROM: usize = 2;

/// `PERMUTATIONS[row][i]` is where bit `i` of a plain word lands in the stored word.
const PERMUTATIONS: [[u8; 16]; 5] = [
    [4, 9, 15, 14, 0, 6, 5, 13, 12, 1, 11, 10, 8, 7, 3, 2],
    [1, 3, 10, 13, 5, 4, 0, 8, 14, 6, 2, 7, 9, 15, 11, 12],
    [10, 0, 1, 8, 2, 14, 3, 9, 5, 15, 6, 12, 7, 11, 13, 4],
    [3, 10, 14, 7, 12, 1, 2, 15, 4, 13, 5, 6, 11, 8, 0, 9],
    [6, 12, 5, 4, 11, 7, 14, 0, 10, 3, 1, 8, 2, 9, 15, 13],
];

/// What a `DIFa` says about the device that wrote the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// All zero: what every SVZ export from the instrument carries.
    Blank,
    Stamped(Stamp),
}

/// A decoded, non-blank stamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    /// Picks the permutation; the writer chooses it at random.
    pub key: u8,
    /// `420031` on a FANTOM-6, `423521A` from ZENOLOGY.
    pub product: [u8; 7],
    /// The instrument's serial number; spaces from ZENOLOGY.
    pub serial: [u8; 7],
    /// The instrument's registration ID. Zero from ZENOLOGY.
    pub token: [u8; 16],
}

impl Stamp {
    /// The product code as text.
    pub fn product_str(&self) -> String {
        crate::container::ascii_trim(&self.product)
    }

    /// The serial number as text; empty when the writer left it blank.
    pub fn serial_str(&self) -> String {
        crate::container::ascii_trim(&self.serial)
    }

    /// The stored 32 bytes.
    pub fn encode(&self) -> [u8; LEN] {
        let mut plain = [0u8; LEN];
        plain[0] = self.key;
        plain[2..9].copy_from_slice(&self.product);
        plain[9..16].copy_from_slice(&self.serial);
        plain[16..].copy_from_slice(&self.token);
        shuffle(&plain, scatter)
    }
}

/// Decode a stored stamp.
pub fn decode(stored: &[u8; LEN]) -> Origin {
    if stored[0] == 0 {
        return Origin::Blank;
    }
    let plain = shuffle(stored, gather);
    Origin::Stamped(Stamp {
        key: plain[0],
        product: plain[2..9].try_into().unwrap(),
        serial: plain[9..16].try_into().unwrap(),
        token: plain[16..].try_into().unwrap(),
    })
}

/// Read and decode a container's stamp; `None` when it has no `DIFa` record.
pub fn read(raw: &Raw) -> Result<Option<Origin>> {
    let svd = Svd::parse(raw)?;
    let Some(area) = svd.area(b"DIFa") else {
        return Ok(None);
    };
    let Ok(table) = RecordTable::parse(area, svd.area_bytes(raw, area)?) else {
        return Ok(None);
    };
    Ok(table
        .record(0)
        .and_then(|r| <&[u8; LEN]>::try_from(r).ok())
        .map(decode))
}

/// `bytes` of a `DIFa` area with its stamp zeroed, and the record checksum refreshed when the area
/// carries one. Anything that is not a one-stamp record table comes back unchanged.
///
/// What leaves the tool should not carry the source instrument's serial number or registration ID. Zero is what the
/// instrument writes in every SVZ export.
pub fn blank(area: &Area, bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let Ok(table) = RecordTable::parse(area, bytes) else {
        return out;
    };
    if table.record(0).map(<[u8]>::len) != Some(LEN) {
        return out;
    }
    out[table.info_len..table.info_len + LEN].fill(0);
    if table.info_stride() == 4 {
        let at = RecordTable::HEADER_LEN;
        out[at..at + 4].copy_from_slice(&crc32(&[0; LEN]).to_le_bytes());
    }
    out
}

fn shuffle(from: &[u8; LEN], map: impl Fn(u16, &[u8; 16]) -> u16) -> [u8; LEN] {
    let mut out = *from;
    let permutation = &PERMUTATIONS[(from[0] as usize + 4) % 5];
    for at in (SCRAMBLED_FROM..LEN).step_by(2) {
        let word = u16::from_le_bytes([from[at], from[at + 1]]);
        out[at..at + 2].copy_from_slice(&map(word, permutation).to_le_bytes());
    }
    out
}

/// Plain → stored: bit `i` moves to `p[i]`.
fn scatter(word: u16, p: &[u8; 16]) -> u16 {
    (0..16).fold(0, |acc, i| acc | ((word >> i) & 1) << p[i])
}

/// Stored → plain: bit `i` comes from `p[i]`.
fn gather(word: u16, p: &[u8; 16]) -> u16 {
    (0..16).fold(0, |acc, i| acc | ((word >> p[i]) & 1) << i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zenology(key: u8) -> Stamp {
        Stamp {
            key,
            product: *b"423521A",
            serial: *b"       ",
            token: [0; 16],
        }
    }

    #[test]
    fn every_key_round_trips() {
        for key in 1..=5 {
            let stamp = zenology(key);
            assert_eq!(decode(&stamp.encode()), Origin::Stamped(stamp));
        }
    }

    #[test]
    fn the_stored_form_is_scrambled() {
        let stored = zenology(5).encode();
        assert_eq!(&stored[..2], &[5, 0]);
        assert!(!stored.windows(7).any(|w| w == b"423521A"));
    }

    #[test]
    fn a_zero_stamp_is_blank() {
        assert_eq!(decode(&[0; LEN]), Origin::Blank);
    }

    #[test]
    fn text_fields_trim() {
        let stamp = zenology(1);
        assert_eq!(stamp.product_str(), "423521A");
        assert_eq!(stamp.serial_str(), "");
    }

    fn area(body: &[u8]) -> Area {
        Area {
            tag: *b"DIFa",
            format: *b"ZCOR",
            offset: 0,
            size: body.len() as u32,
        }
    }

    /// One record of 32 bytes, with or without the SVZ checksum word.
    fn stamped_area(with_checksum: bool) -> Vec<u8> {
        let stamp = zenology(3).encode();
        let info_len: u32 = if with_checksum { 20 } else { 16 };
        let mut body = Vec::new();
        for word in [1, LEN as u32, info_len, 0] {
            body.extend_from_slice(&word.to_le_bytes());
        }
        if with_checksum {
            body.extend_from_slice(&crc32(&stamp).to_le_bytes());
        }
        body.extend_from_slice(&stamp);
        body
    }

    #[test]
    fn blanking_an_svd_stamp_zeroes_the_record() {
        let body = stamped_area(false);
        let blanked = blank(&area(&body), &body);
        assert_eq!(&blanked[..16], &body[..16]);
        assert_eq!(&blanked[16..], &[0; LEN]);
    }

    #[test]
    fn blanking_an_svz_stamp_refreshes_its_checksum() {
        let body = stamped_area(true);
        let blanked = blank(&area(&body), &body);
        assert_eq!(&blanked[16..20], &crc32(&[0; LEN]).to_le_bytes());
        assert_eq!(&blanked[20..], &[0; LEN]);
    }

    #[test]
    fn an_unparseable_area_is_left_alone() {
        let body = vec![0u8; 48];
        assert_eq!(blank(&area(&body), &body), body);
    }

    /// Each row is a permutation, or decoding would lose bits.
    #[test]
    fn every_row_is_a_permutation() {
        for row in PERMUTATIONS {
            let mut sorted = row;
            sorted.sort();
            assert_eq!(sorted, core::array::from_fn(|i| i as u8));
        }
    }
}
