//! Talking to a Fantom over SysEx.
//!
//! Roland addresses every parameter the instrument holds, and the same map describes both the
//! wire and the file: a record is the parameter blocks laid end to end, with multi-nibble wire
//! fields packed into little-endian bytes and signed fields stored zero-centred. That
//! correspondence is [`fantom_core::params`], because it is as much a fact about the file as
//! about the wire and the librarian reads it without a MIDI port in sight. This crate is only
//! the transport, and re-exports the map for callers that want both.

pub use fantom_core::params;
pub use fantom_core::params::file_value;

/// Model ID of the FANTOM-6/7/8, its EX revision, and the FANTOM-06/07/08.
///
/// Confirmed on a FANTOM-6 by Identity Request: family code `5B 03`, family number `00 00`.
pub const MODEL: [u8; 4] = [0x00, 0x00, 0x00, 0x5B];

/// Device ID a Fantom answers on out of the box.
pub const DEVICE: u8 = 0x10;

/// Base address of the temporary (currently sounding) scene.
pub const TEMP_SCENE: [u8; 4] = [0x02, 0x00, 0x00, 0x00];

/// Base address of the temporary ZEN-Core tone for `zone`, 0-based.
pub fn temp_tone(zone: u8) -> [u8; 4] {
    [0x02, 0x10 + zone, 0x00, 0x00]
}

/// Roland's checksum: the address and data summed, then the low 7 bits inverted.
pub fn checksum(body: &[u8]) -> u8 {
    let sum: u32 = body.iter().map(|&b| b as u32).sum();
    ((128 - (sum % 128)) % 128) as u8
}

fn message(command: u8, addr: [u8; 4], tail: &[u8]) -> Vec<u8> {
    let mut body = addr.to_vec();
    body.extend_from_slice(tail);
    let mut m = vec![0xF0, 0x41, DEVICE];
    m.extend_from_slice(&MODEL);
    m.push(command);
    m.extend_from_slice(&body);
    m.push(checksum(&body));
    m.push(0xF7);
    m
}

/// Data Request 1 — ask the instrument for `size` bytes at `addr`.
///
/// Sizes are carried 7 bits per byte, like addresses: 144 goes out as `00 00 01 10`, not `00 00
/// 00 90`. Masking the bytes of a plain big-endian integer silently asks for the wrong length.
pub fn rq1(addr: [u8; 4], size: u32) -> Vec<u8> {
    message(
        0x11,
        addr,
        &[
            ((size >> 21) & 0x7F) as u8,
            ((size >> 14) & 0x7F) as u8,
            ((size >> 7) & 0x7F) as u8,
            (size & 0x7F) as u8,
        ],
    )
}

/// Data Set 1 — write `data` at `addr`.
pub fn dt1(addr: [u8; 4], data: &[u8]) -> Vec<u8> {
    message(0x12, addr, data)
}

/// Add `base` to a block-relative address, carrying in Roland's 7-bit-per-byte space.
pub fn offset_addr(base: [u8; 4], off: [u8; 3]) -> [u8; 4] {
    let a = ((base[1] as u32) << 14) | ((base[2] as u32) << 7) | base[3] as u32;
    let b = ((off[0] as u32) << 14) | ((off[1] as u32) << 7) | off[2] as u32;
    let s = a + b;
    [
        base[0],
        ((s >> 14) & 0x7F) as u8,
        ((s >> 7) & 0x7F) as u8,
        (s & 0x7F) as u8,
    ]
}

/// Render a parameter's file value as the bytes the wire expects.
///
/// A one-byte wire field is a plain 7-bit value; wider ones are nibbles, most significant first.
pub fn wire_bytes(rec: &[u8], p: &params::Param) -> Vec<u8> {
    let v = (file_value(rec, p) as i64 + p.bias as i64) as u64;
    if p.len_sysex == 1 {
        return vec![(v & 0x7F) as u8];
    }
    (0..p.len_sysex)
        .rev()
        .map(|i| ((v >> (4 * i as u64)) & 0x0F) as u8)
        .collect()
}
