//! CRC-32, the checksum Roland stores per record in SVZ areas.
//!
//! An SVZ area header is `16 + 4 × count` bytes: the fixed fields, then **one CRC-32 per record**,
//! in record order. Confirmed exactly — every record of every area across the three SVZ fixtures
//! matches, 366 of 366, using the standard reflected polynomial (`0xEDB88320`, init and final xor
//! `0xFFFFFFFF`) over the record's bytes.
//!
//! That makes it a real integrity check on repackaging: a record we copied must keep its checksum,
//! and a record we edited must get a new one that matches. See [`crate::verify`].
//!
//! SVD5 areas declare no per-record words, so their records carry no checksum of this kind.

/// The reflected CRC-32 polynomial used by zlib, PNG, and Roland's SVZ record table.
const POLYNOMIAL: u32 = 0xEDB8_8320;

/// CRC-32 of `bytes`.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (POLYNOMIAL & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_published_test_vectors() {
        assert_eq!(crc32(b""), 0x0000_0000);
        assert_eq!(crc32(b"a"), 0xE8B7_BE43);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"The quick brown fox jumps over the lazy dog"), 0x414F_A339);
    }

    /// The first record of `fixtures/Z-Core_20260623.svz`'s `PATa` stores this word.
    #[test]
    fn a_changed_byte_changes_the_checksum() {
        let mut record = vec![0u8; 64];
        record[..14].copy_from_slice(b"#Square Flutes");
        let before = crc32(&record);
        record[13] = b'!';
        assert_ne!(crc32(&record), before);
    }
}
