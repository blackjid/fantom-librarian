//! Talking to a Fantom over SysEx.
//!
//! Roland addresses every parameter the instrument holds, and the same map describes both the
//! wire and the file: a record is the parameter blocks laid end to end, with multi-nibble wire
//! fields packed into little-endian bytes and signed fields stored zero-centred. That
//! correspondence is [`fantom_core::params`], because it is as much a fact about the file as
//! about the wire and the librarian reads it without a MIDI port in sight. This crate is only
//! the transport, and re-exports the map for callers that want both.

mod session;

pub use fantom_core::params;
pub use fantom_core::params::file_value;
pub use session::{Session, Unanswered, PORT};

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

/// Address of a zone's own block in the temporary scene, where its MSB, LSB and PC live at +0,
/// +1 and +2.
///
/// Selecting a sound by writing here, rather than by sending bank-select on channel `zone`, is
/// what makes the choice independent of the receive-channel mapping a scene is free to remap.
pub fn zone_block(zone: u8) -> [u8; 4] {
    [0x02, 0x00, 0x10 + zone, 0x00]
}

/// Length of a tone name, in every engine's Common block.
pub const NAME_LEN: u32 = 16;

/// The temporary area a sounding tone occupies, which is per engine rather than per zone.
///
/// Each area's Common block opens with the 16-byte name — the addresses are the *FANTOM EX MIDI
/// Implementation* parameter address map, and every one below is hardware-confirmed on a FANTOM-6
/// except where its doc comment says otherwise.
///
/// The map's own layout is not quite what the instrument does. A MODEL or n/zyme tone answers at
/// the **Z-Core** address, not at the Model area the map gives it: selecting `97/79/0` and reading
/// `02 10 00 00` returns `Geometric Wave`, while `05 20 0A 00` — Model Tone Common by the map —
/// answers nothing at all. Drum kits, SN-A, V-Piano and EXSN do each answer at their own area, and
/// the Z-Core one goes stale for them, so this is a real per-engine split rather than one address
/// for everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempArea {
    ZCore,
    DrumKit,
    SnA,
    VPiano,
    Vtw,
    /// MSB 105 — SN-Acoustic, SN-EPiano and the EXSN expansions share one area.
    Exsn,
    /// The Model Expansions, `EXM001-`. Named at the Z-Core address, whatever the map says.
    Model,
    /// `EXM007`, its own engine rather than a Model Expansion — and named there too.
    Nzyme,
    /// `EXM005`. Untested: no JD-800 to select on the instrument this was confirmed against.
    Jd800,
}

impl TempArea {
    /// Where a bank's sounds land when a zone selects them, or `None` for a bank whose engine this
    /// version cannot name.
    pub fn for_bank(msb: u8, lsb: u8) -> Option<Self> {
        use fantom_core::model::{ToneRef, ToneType};
        // n/zyme is addressed as a MODEL bank but sounds on an engine of its own.
        if msb == 97 && lsb == 72 {
            return Some(Self::Nzyme);
        }
        Some(match ToneRef::new(msb, lsb, 0, None).tone_type() {
            ToneType::Drum => Self::DrumKit,
            // A wave expansion is ZEN-Core playing expansion waves, and sounds in the Z-Core area
            // like any other: confirmed by dumping `93/7`-`93/14`, `93/23` and `93/26`.
            ToneType::ZenCore | ToneType::Exz => Self::ZCore,
            ToneType::SnA => Self::SnA,
            ToneType::VPiano => Self::VPiano,
            ToneType::Vtw => Self::Vtw,
            ToneType::SnAp | ToneType::SnEp | ToneType::Exsn => Self::Exsn,
            ToneType::Model => Self::Model,
            // ACB has no temporary area in the map, and the Z-Core one stays blank while a zone
            // holds a JUPITER-8 tone: its names have to come from the sound list.
            ToneType::Acb | ToneType::Unknown => return None,
        })
    }

    /// Where to read this engine's tone name for `zone`, 0-based: [`NAME_LEN`] ASCII bytes.
    pub fn name_addr(self, zone: u8) -> [u8; 4] {
        match self {
            // The modelled engines answer here along with ZEN-Core itself; see the type's note.
            Self::ZCore | Self::Model | Self::Nzyme => [0x02, 0x10 + zone, 0x00, 0x00],
            Self::DrumKit => [0x02, 0x30 + 2 * zone, 0x00, 0x00],
            Self::SnA => [0x04, zone, 0x00, 0x00],
            // One V-Piano and one VTW between all sixteen zones.
            Self::VPiano => [0x04, 0x20, 0x00, 0x00],
            Self::Vtw => [0x04, 0x40, 0x00, 0x00],
            Self::Exsn => [0x05, zone, 0x00, 0x00],
            Self::Jd800 => [0x05, 0x60 + zone, 0x00, 0x00],
        }
    }

    /// Named on a command line, for a bank whose engine is not mapped yet.
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "zcore" | "zen-core" | "zencore" => Self::ZCore,
            "drum" | "drumkit" => Self::DrumKit,
            "sn-a" | "sna" => Self::SnA,
            "vpiano" | "v-piano" => Self::VPiano,
            "vtw" => Self::Vtw,
            "exsn" | "sn-ap" | "sn-ep" => Self::Exsn,
            "model" => Self::Model,
            "nzyme" | "n/zyme" => Self::Nzyme,
            "jd800" | "jd-800" => Self::Jd800,
            _ => return None,
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_engine_reads_the_area_that_answered_for_it() {
        // Hardware-confirmed on a FANTOM-6: select the bank, read here, get that tone's name.
        assert_eq!(TempArea::ZCore.name_addr(0), [0x02, 0x10, 0x00, 0x00]);
        assert_eq!(TempArea::DrumKit.name_addr(1), [0x02, 0x32, 0x00, 0x00]);
        assert_eq!(TempArea::SnA.name_addr(2), [0x04, 0x02, 0x00, 0x00]);
        assert_eq!(TempArea::VPiano.name_addr(9), [0x04, 0x20, 0x00, 0x00]);
        assert_eq!(TempArea::Exsn.name_addr(1), [0x05, 0x01, 0x00, 0x00]);
        // A modelled tone is named at the Z-Core address, not at the Model area.
        assert_eq!(TempArea::Model.name_addr(0), [0x02, 0x10, 0x00, 0x00]);
        assert_eq!(TempArea::Nzyme.name_addr(3), [0x02, 0x13, 0x00, 0x00]);
    }

    #[test]
    fn a_bank_knows_which_area_its_sounds_sound_in() {
        assert_eq!(TempArea::for_bank(87, 64), Some(TempArea::ZCore));
        assert_eq!(TempArea::for_bank(86, 64), Some(TempArea::DrumKit));
        assert_eq!(TempArea::for_bank(105, 64), Some(TempArea::Exsn));
        assert_eq!(TempArea::for_bank(97, 66), Some(TempArea::Model));
        // A MODEL bank by address, its own engine by sound.
        assert_eq!(TempArea::for_bank(97, 72), Some(TempArea::Nzyme));
        // ACB has no temporary area in the map, so it cannot be read back this way.
        assert_eq!(TempArea::for_bank(107, 64), None);
    }
}
