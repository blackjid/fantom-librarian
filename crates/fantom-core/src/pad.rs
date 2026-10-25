//! Pad sampler: `PAD_SMPL/SMPL_SET001/` beside `FANTOM.SVD`. Read-only.
//!
//! `PADCONF.BIN` (`RFPD`) holds 64 pad records and names; `BANK<b>-<pp>.SMP` (`RFWV`) holds each
//! pad's audio as 16-bit big-endian PCM behind a 0x200-byte header. Layout in `docs/FORMAT.md`.

use std::path::Path;

use md5::{Digest, Md5};

use crate::{Error, Result};

/// Folder holding the pad files, relative to the folder holding `FANTOM.SVD`.
pub const DIR: &str = "PAD_SMPL/SMPL_SET001";

/// `.SMP` header length; PCM starts here.
pub const SMP_HEADER: usize = 0x200;

const CONF: &str = "PADCONF.BIN";
const PADS: usize = 64;
const RECORDS: usize = 0x10;
const RECORD_LEN: usize = 76;
const NAMES: usize = 0x1310;
const NAME_LEN: usize = 24;
const MD5_AT: usize = 0x20;

/// The fixed fields of an `.SMP` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmpHeader {
    pub rate: u32,
    /// 1 or 2.
    pub channels: u32,
    pub bits: u32,
    /// Sample frames in the file.
    pub frames: usize,
    /// Whole file, header included.
    pub file_size: usize,
}

impl SmpHeader {
    pub fn seconds(&self) -> f64 {
        self.frames as f64 / f64::from(self.rate)
    }

    fn frame_bytes(&self) -> usize {
        self.channels as usize * 2
    }
}

/// One pad: its `PADCONF.BIN` record, name and the header of its `.SMP`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pad {
    /// 1..=4.
    pub bank: u8,
    /// 1..=16.
    pub pad: u8,
    pub name: String,
    /// `.SMP` file size the record expects.
    pub size: u32,
    /// Start of playback, byte offset in the `.SMP` (default `SMP_HEADER`).
    pub start: u32,
    /// End of playback, byte offset in the `.SMP` (default `size`).
    pub end: u32,
    /// 0..=127.
    pub level: u32,
    /// Inferred: the record's `+0x10` word, 0/1, probably GATE.
    pub gate: bool,
    pub looped: bool,
    /// Inferred: the record's `+0x18` word, always 0 so far, probably FX SW.
    pub fx: bool,
    /// `None` when the pad's `.SMP` is missing.
    pub audio: Option<SmpHeader>,
}

impl Pad {
    /// Start or end differs from the whole file.
    pub fn trimmed(&self) -> bool {
        self.start as usize != SMP_HEADER || self.end != self.size
    }

    /// `BANK<b>-<pp>.SMP`.
    pub fn file_name(&self) -> String {
        file_name(self.bank, self.pad)
    }
}

/// A backup's pad sampler set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadSet {
    /// Inferred: the u32 after the records, 0..=127, probably PAD SAMPLER LEV. Absent in factory sets.
    pub sampler_level: Option<u32>,
    /// Bank-major: BANK1-01 .. BANK4-16.
    pub pads: Vec<Pad>,
}

impl PadSet {
    pub fn pad(&self, bank: u8, pad: u8) -> Option<&Pad> {
        self.pads.iter().find(|p| p.bank == bank && p.pad == pad)
    }
}

fn file_name(bank: u8, pad: u8) -> String {
    format!("BANK{bank}-{pad:02}.SMP")
}

fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Parses an `.SMP` header; `file` may be just its first `SMP_HEADER` bytes when `file_size` says
/// how long the whole file is. Rejects a bad magic, MD5, size, rate, channel count or bit depth.
pub fn parse_smp_header(head: &[u8], file_size: usize) -> Result<SmpHeader> {
    let bad = |why: &str| Error::Unrecognized(format!("SMP: {why}"));
    if head.len() < SMP_HEADER || file_size < SMP_HEADER || &head[..4] != b"RFWV" {
        return Err(bad("not RFWV"));
    }
    if Md5::digest(&head[4..0x14])[..] != head[MD5_AT..MD5_AT + 16] {
        return Err(bad("header MD5 mismatch"));
    }
    if be32(head, 4) as usize != file_size - 8 {
        return Err(bad("size field disagrees with file length"));
    }
    let (rate, channels, bits) = (be32(head, 8), be32(head, 12), be32(head, 16));
    if rate == 0 || !(1..=2).contains(&channels) || bits != 16 {
        return Err(bad(&format!(
            "unsupported {rate} Hz {channels}ch {bits}-bit"
        )));
    }
    let frame = channels as usize * 2;
    Ok(SmpHeader {
        rate,
        channels,
        bits,
        frames: (file_size - SMP_HEADER) / frame,
        file_size,
    })
}

/// WAV bytes (16-bit LE RIFF) for a whole `.SMP` file. With `trim`, only the bytes from `start` to
/// `end` (byte offsets in the `.SMP`, rounded down to whole frames), else all the audio.
pub fn smp_to_wav(smp: &[u8], trim: Option<(u32, u32)>) -> Result<Vec<u8>> {
    let header = parse_smp_header(smp, smp.len())?;
    let (start, end) = match trim {
        Some((s, e)) => (s as usize, e as usize),
        None => (SMP_HEADER, smp.len()),
    };
    if start < SMP_HEADER || start > end || end > smp.len() {
        return Err(Error::Unrecognized(format!(
            "SMP: trim {start:#x}..{end:#x} outside the audio {SMP_HEADER:#x}..{:#x}",
            smp.len()
        )));
    }
    let frame = header.frame_bytes();
    let start = SMP_HEADER + (start - SMP_HEADER) / frame * frame;
    let end = start + (end - start) / frame * frame;
    let pcm = &smp[start..end];
    let mut wav = Vec::with_capacity(44 + pcm.len());
    let data_len =
        u32::try_from(pcm.len()).map_err(|_| Error::Unrecognized("SMP: too long".into()))?;
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&data_len.saturating_add(36).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&(header.channels as u16).to_le_bytes());
    wav.extend_from_slice(&header.rate.to_le_bytes());
    wav.extend_from_slice(&(header.rate * frame as u32).to_le_bytes());
    wav.extend_from_slice(&(frame as u16).to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend(pcm.chunks(2).flat_map(|s| [s[1], s[0]]));
    Ok(wav)
}

/// Parses `PADCONF.BIN`. The pads carry no `audio`; `read_set` fills it in.
pub fn parse_padconf(bytes: &[u8]) -> Result<PadSet> {
    let bad = |why: &str| Error::Unrecognized(format!("{CONF}: {why}"));
    if bytes.len() < NAMES + PADS * NAME_LEN || &bytes[..4] != b"RFPD" {
        return Err(bad("not RFPD or truncated"));
    }
    if be32(bytes, 4) as usize != PADS {
        return Err(bad("pad count is not 64"));
    }
    let payload = be32(bytes, 12) as usize;
    let pads = (0..PADS)
        .map(|i| {
            let r = &bytes[RECORDS + RECORD_LEN * i..];
            let n = &bytes[NAMES + NAME_LEN * i..][..16];
            let n = &n[..n.iter().position(|&c| c == 0).unwrap_or(16)];
            Pad {
                bank: (i / 16 + 1) as u8,
                pad: (i % 16 + 1) as u8,
                name: n
                    .iter()
                    .map(|&c| char::from(c))
                    .collect::<String>()
                    .trim_end()
                    .to_string(),
                size: be32(r, 0),
                start: be32(r, 4),
                end: be32(r, 8),
                level: be32(r, 12),
                gate: be32(r, 16) != 0,
                looped: be32(r, 20) != 0,
                fx: be32(r, 24) != 0,
                audio: None,
            }
        })
        .collect();
    let sampler_level = bytes
        .get(RECORDS + payload..RECORDS + payload + 4)
        .map(|t| be32(t, 0));
    Ok(PadSet {
        sampler_level,
        pads,
    })
}

fn file_err(action: &'static str, path: &Path, source: std::io::Error) -> Error {
    Error::File {
        action,
        path: path.display().to_string(),
        source,
    }
}

/// The pad set under `backup/PAD_SMPL/SMPL_SET001`, each pad's `.SMP` header read and checked.
/// A backup without the folder has none; a pad without its `.SMP` has no `audio`.
pub fn read_set(backup: &Path) -> Result<Option<PadSet>> {
    use std::io::Read;
    let dir = backup.join(DIR);
    if !dir.is_dir() {
        return Ok(None);
    }
    let conf = dir.join(CONF);
    let mut set = parse_padconf(&std::fs::read(&conf).map_err(|e| file_err("read", &conf, e))?)?;
    for pad in &mut set.pads {
        let path = dir.join(pad.file_name());
        if !path.is_file() {
            continue;
        }
        let mut head = Vec::with_capacity(SMP_HEADER);
        let f = std::fs::File::open(&path).map_err(|e| file_err("read", &path, e))?;
        let size = f.metadata().map_err(|e| file_err("read", &path, e))?.len() as usize;
        f.take(SMP_HEADER as u64)
            .read_to_end(&mut head)
            .map_err(|e| file_err("read", &path, e))?;
        pad.audio = Some(
            parse_smp_header(&head, size)
                .map_err(|e| Error::Unrecognized(format!("{}: {e}", pad.file_name())))?,
        );
    }
    Ok(Some(set))
}

/// WAV bytes for one pad of `backup`: its whole audio, or with `trim` only `start..end`.
pub fn read_wav(backup: &Path, pad: &Pad, trim: bool) -> Result<Vec<u8>> {
    let path = backup.join(DIR).join(pad.file_name());
    let smp = std::fs::read(&path).map_err(|e| file_err("read", &path, e))?;
    smp_to_wav(&smp, trim.then_some((pad.start, pad.end)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A valid `.SMP` of `frames` stereo frames whose bytes count up.
    fn smp(frames: usize) -> Vec<u8> {
        let mut f = vec![0u8; SMP_HEADER + frames * 4];
        f[..4].copy_from_slice(b"RFWV");
        let size = (f.len() - 8) as u32;
        f[4..8].copy_from_slice(&size.to_be_bytes());
        f[8..12].copy_from_slice(&48000u32.to_be_bytes());
        f[12..16].copy_from_slice(&2u32.to_be_bytes());
        f[16..20].copy_from_slice(&16u32.to_be_bytes());
        let sum = Md5::digest(&f[4..0x14]);
        f[MD5_AT..MD5_AT + 16].copy_from_slice(&sum);
        for (i, b) in f[SMP_HEADER..].iter_mut().enumerate() {
            *b = i as u8;
        }
        f
    }

    #[test]
    fn parses_the_header() {
        let f = smp(10);
        let h = parse_smp_header(&f, f.len()).unwrap();
        assert_eq!((h.rate, h.channels, h.bits, h.frames), (48000, 2, 16, 10));
    }

    #[test]
    fn rejects_bad_md5_magic_and_size() {
        let mut f = smp(10);
        f[9] ^= 1;
        assert!(parse_smp_header(&f, f.len()).is_err());
        let mut f = smp(10);
        f[0] = b'X';
        assert!(parse_smp_header(&f, f.len()).is_err());
        let f = smp(10);
        assert!(parse_smp_header(&f, f.len() + 4).is_err());
        assert!(smp_to_wav(&f[..100], None).is_err());
    }

    #[test]
    fn wav_swaps_to_little_endian() {
        let f = smp(2);
        let w = smp_to_wav(&f, None).unwrap();
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(w[4..8].try_into().unwrap()), 36 + 8);
        assert_eq!(u16::from_le_bytes(w[22..24].try_into().unwrap()), 2);
        assert_eq!(&w[44..], &[1, 0, 3, 2, 5, 4, 7, 6]);
    }

    #[test]
    fn trim_cuts_whole_frames() {
        let f = smp(10);
        let (s, e) = (SMP_HEADER as u32 + 4 + 1, SMP_HEADER as u32 + 20 + 3);
        let w = smp_to_wav(&f, Some((s, e))).unwrap();
        // start rounds down to frame 1, end to frame 5: frames 1..5 = 4 frames
        assert_eq!(w.len(), 44 + 16);
        assert_eq!(&w[44..48], &[5, 4, 7, 6]);
        assert!(smp_to_wav(&f, Some((SMP_HEADER as u32, f.len() as u32 + 1))).is_err());
        assert!(smp_to_wav(&f, Some((SMP_HEADER as u32 - 1, 0x210))).is_err());
        assert!(smp_to_wav(&f, Some((0x210, 0x208))).is_err());
    }

    #[test]
    fn padconf_records_names_and_tail() {
        let mut p = vec![0u8; NAMES + PADS * NAME_LEN + 4];
        p[..4].copy_from_slice(b"RFPD");
        p[4..8].copy_from_slice(&64u32.to_be_bytes());
        p[12..16].copy_from_slice(&0x1900u32.to_be_bytes());
        let r = RECORDS + RECORD_LEN * 17;
        for (k, v) in [1000u32, 0x200, 900, 99, 1, 0x7fff_ffff, 0]
            .iter()
            .enumerate()
        {
            p[r + 4 * k..r + 4 * k + 4].copy_from_slice(&v.to_be_bytes());
        }
        p[NAMES + NAME_LEN * 17..][..7].copy_from_slice(b"Kick   ");
        let t = RECORDS + 0x1900;
        p[t..t + 4].copy_from_slice(&127u32.to_be_bytes());
        let set = parse_padconf(&p).unwrap();
        let pad = set.pad(2, 2).unwrap();
        assert_eq!(pad.name, "Kick");
        assert_eq!(
            (pad.size, pad.start, pad.end, pad.level),
            (1000, 0x200, 900, 99)
        );
        assert!(pad.gate && pad.looped && !pad.fx && pad.trimmed());
        assert_eq!(pad.file_name(), "BANK2-02.SMP");
        assert_eq!(set.sampler_level, Some(127));
        p[0] = b'X';
        assert!(parse_padconf(&p).is_err());
    }
}
