//! Where a scene's sequencer patterns live beside its bank.
//!
//! Patterns are not in the SVD. Each scene slot has its own `SeqSet.SQS` under `SEQ/SEQ_SET/`,
//! named by the slot's position, and the file holds nothing that names the slot — so moving a
//! scene means copying its file to the new slot's path. A slot without one has empty patterns.
//!
//! [`SeqSet::parse`] reads the file (read-only; layout in `docs/FORMAT.md`) and [`to_smf`] exports
//! one pattern as a Standard MIDI File.

use std::path::PathBuf;

use md5::Digest;

use crate::{Error, Result};

/// Scene slots a bank can hold: four banks, A–D, of 128.
pub const SLOTS: usize = 512;

const PER_BANK: usize = 128;

/// The pattern file for 0-based scene `index`, relative to the folder holding `FANTOM.SVD`:
/// `SEQ/SEQ_SET/<A–D>/<001–128>/SeqSet.SQS`, as the firmware builds it.
pub fn slot_path(index: usize) -> Option<PathBuf> {
    if index >= SLOTS {
        return None;
    }
    let bank = char::from(b'A' + (index / PER_BANK) as u8);
    Some(
        [
            "SEQ",
            "SEQ_SET",
            &bank.to_string(),
            &format!("{:03}", index % PER_BANK + 1),
        ]
        .iter()
        .collect::<PathBuf>()
        .join("SeqSet.SQS"),
    )
}

const TRACKS: usize = 16;
const VARIATIONS: usize = 8;
const GROUPS: usize = 16;
const STEPS: usize = 32;
const LANES: usize = 13;
const TPQN_AT: usize = 0x10;
const TIME_SIG_AT: usize = 0x70;
const SONG_LOOP_AT: usize = 0xf0;
const STEPS_AT: usize = 0x110;
const GROUPS_AT: usize = 0x510;
const GROUP_LEN: usize = 0x70;
const TRACK_STATE_AT: usize = 0xc10;
const PATTERN_TABLE: usize = 0xe10;
const PATTERN_LEN: usize = 0x30;
const MD5_AT: usize = 0x2610;
const EVENTS: usize = 0x2620;
const EVENT_LEN: usize = 16;
/// First version (major << 8 | minor) with automation lanes.
const LANES_FROM: u32 = 0x106;
const TRAILER: [u8; 4] = [0xbc, 0xac, 0xae, 0xac]; // !"CSQS"

/// One recorded event of a pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Absolute tick; the file stores the gap after each event.
    pub tick: u32,
    pub kind: Kind,
}

/// What an [`Event`] does. `ch` is 0-based.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Note {
        ch: u8,
        note: u8,
        vel: u8,
        off_vel: u8,
        /// Length in ticks.
        gate: u32,
    },
    PolyAt {
        ch: u8,
        note: u8,
        val: u8,
    },
    Cc {
        ch: u8,
        num: u8,
        val: u8,
    },
    Pc {
        ch: u8,
        prog: u8,
    },
    ChanAt {
        ch: u8,
        val: u8,
    },
    /// 14-bit, centre 8192.
    Bend {
        ch: u8,
        val: u16,
    },
    /// BPM × 100.
    Tempo {
        bpm_x100: u32,
    },
    TimeSig {
        num: u32,
        den: u32,
        clocks: u8,
    },
    /// Head and continuation events folded into one payload.
    SysEx {
        payload: Vec<u8>,
    },
    /// Advances time only.
    Fill,
    /// Kind byte not decoded here (`0x8C` end, a stray `0x96` continuation, unknown).
    Other(u8),
}

impl Kind {
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Note { .. } => "note",
            Kind::PolyAt { .. } => "polyAT",
            Kind::Cc { .. } => "cc",
            Kind::Pc { .. } => "pc",
            Kind::ChanAt { .. } => "chanAT",
            Kind::Bend { .. } => "bend",
            Kind::Tempo { .. } => "tempo",
            Kind::TimeSig { .. } => "timesig",
            Kind::SysEx { .. } => "sysex",
            Kind::Fill => "fill",
            Kind::Other(0x8c) => "end",
            Kind::Other(_) => "other",
        }
    }
}

/// One automation lane of a pattern. `id`: 0 channel AT, `0x79` pitch bend, otherwise CC `id - 1`.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    pub id: u32,
    /// (absolute tick, value)
    pub points: Vec<(u32, u32)>,
}

/// One recorded pattern of a `SeqSet.SQS`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// 1-based track, 1–16.
    pub track: usize,
    /// 1-based variation, 1–8.
    pub variation: usize,
    pub name: String,
    pub bars: u32,
    pub events: Vec<Event>,
}

impl Pattern {
    /// Whether it holds anything beyond fill and stray continuation events.
    pub fn has_data(&self) -> bool {
        self.events
            .iter()
            .any(|e| !matches!(e.kind, Kind::Fill | Kind::Other(0x96)))
    }
}

/// A pattern group: one variation per track, played together as a song step.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub used: bool,
    /// 0-based variation per track 1–16; `None` = track silent.
    pub variations: [Option<u8>; TRACKS],
    pub bars: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SongStep {
    /// 0-based group.
    pub group: usize,
    /// Plays minus one (inferred).
    pub repeat: u32,
    pub skip: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub steps: Vec<SongStep>,
    pub loop_on: bool,
    /// 0-based steps.
    pub loop_a: Option<usize>,
    pub loop_b: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackState {
    /// 0-based active variation.
    pub variation: Option<u8>,
    pub mute: bool,
    pub solo: bool,
}

/// A parsed `SeqSet.SQS`.
#[derive(Debug, Clone, PartialEq)]
pub struct SeqSet {
    /// File version (major, minor).
    pub version: (u32, u32),
    pub tpqn: u32,
    /// Bar 1 time signature (numerator, denominator).
    pub time_sig: (u32, u32),
    /// 128 patterns, track-major: index `(track - 1) * 8 + variation - 1`.
    pub patterns: Vec<Pattern>,
    /// Per pattern, parallel to `patterns`; empty before v1.06.
    pub lanes: Vec<Vec<Lane>>,
    /// 16 groups.
    pub groups: Vec<Group>,
    pub song: Song,
    /// 16 tracks.
    pub tracks: Vec<TrackState>,
}

fn bad(why: &str) -> Error {
    Error::Unrecognized(format!("not a SeqSet.SQS: {why}"))
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn u32(&self, at: usize) -> Result<u32> {
        self.0
            .get(at..at + 4)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
            .ok_or_else(|| bad("truncated"))
    }

    fn i32(&self, at: usize) -> Result<i32> {
        self.u32(at).map(|v| v as i32)
    }

    fn index(&self, at: usize) -> Result<Option<usize>> {
        self.i32(at).map(|v| usize::try_from(v).ok())
    }

    fn name(&self, at: usize, len: usize) -> Result<String> {
        let b = self.0.get(at..at + len).ok_or_else(|| bad("truncated"))?;
        let b = b.split(|&c| c == 0).next().unwrap_or_default();
        Ok(b.iter()
            .map(|&c| c as char)
            .collect::<String>()
            .trim_end()
            .to_string())
    }
}

impl SeqSet {
    /// Parse a file: magic, size field, header MD5, trailer and layout end are checked.
    pub fn parse(file: &[u8]) -> Result<SeqSet> {
        if file.len() < EVENTS + TRAILER.len() || !file.starts_with(b"CSQS") {
            return Err(bad("no CSQS magic"));
        }
        if file[file.len() - 4..] != TRAILER {
            return Err(bad("bad trailer"));
        }
        let r = Reader(file);
        if r.u32(4)? as usize != file.len() {
            return Err(bad("size field != file length"));
        }
        if md5::Md5::digest(&file[..MD5_AT])[..] != file[MD5_AT..EVENTS] {
            return Err(bad("header MD5 mismatch"));
        }
        let version = (r.u32(8)?, r.u32(0xc)?);
        let mut patterns = Vec::with_capacity(TRACKS * VARIATIONS);
        let mut at = EVENTS;
        for slot in 0..TRACKS * VARIATIONS {
            let entry = PATTERN_TABLE + slot * PATTERN_LEN;
            let count = r.u32(entry)? as usize;
            let end = count
                .checked_mul(EVENT_LEN)
                .and_then(|n| n.checked_add(at))
                .ok_or_else(|| bad("truncated"))?;
            let raw = file.get(at..end).ok_or_else(|| bad("truncated"))?;
            at = end;
            patterns.push(Pattern {
                track: slot / VARIATIONS + 1,
                variation: slot % VARIATIONS + 1,
                name: r.name(entry + 0x14, 8)?,
                bars: r.u32(entry + 0xc)?,
                events: decode_events(raw),
            });
        }
        let mut lanes = Vec::with_capacity(patterns.len());
        if (version.0 << 8 | version.1) >= LANES_FROM {
            for _ in 0..patterns.len() {
                let mut set = Vec::with_capacity(LANES);
                for _ in 0..LANES {
                    let (id, n) = (r.u32(at)?, r.u32(at + 4)? as usize);
                    let body = n
                        .checked_mul(8)
                        .and_then(|n| n.checked_add(at + 16))
                        .and_then(|end| file.get(at + 16..end))
                        .ok_or_else(|| bad("truncated"))?;
                    let le = |b: &[u8]| u32::from_le_bytes(b.try_into().unwrap());
                    let points = body
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .map(|p| (le(&p[..4]), le(&p[4..])))
                        .collect();
                    at += 16 + 8 * n;
                    set.push(Lane { id, points });
                }
                lanes.push(set);
            }
        } else {
            lanes.resize(patterns.len(), Vec::new());
        }
        if at + TRAILER.len() != file.len() {
            return Err(bad(&format!(
                "layout ends at {at:#x}, file at {:#x}",
                file.len() - TRAILER.len()
            )));
        }

        let mut groups = Vec::with_capacity(GROUPS);
        for g in 0..GROUPS {
            let at = GROUPS_AT + g * GROUP_LEN;
            let mut variations = [None; TRACKS];
            for (t, v) in variations.iter_mut().enumerate() {
                *v = r.index(at + 4 + 4 * t)?.map(|v| v as u8);
            }
            groups.push(Group {
                used: file[at] == 0,
                variations,
                bars: r.u32(at + 0x44)?,
                name: r.name(at + 0x48, 8)?,
            });
        }
        let mut steps = Vec::new();
        for i in 0..STEPS {
            let at = STEPS_AT + i * 0x20;
            if let Some(group) = r.index(at)?.filter(|&g| g < GROUPS) {
                steps.push(SongStep {
                    group,
                    repeat: r.u32(at + 4)?,
                    skip: r.u32(at + 8)? != 0,
                });
            }
        }
        let song = Song {
            steps,
            loop_on: r.u32(SONG_LOOP_AT)? != 0,
            loop_a: r.index(SONG_LOOP_AT + 4)?,
            loop_b: r.index(SONG_LOOP_AT + 8)?,
        };
        let mut tracks = Vec::with_capacity(TRACKS);
        for t in 0..TRACKS {
            let at = TRACK_STATE_AT + t * 0x20;
            tracks.push(TrackState {
                variation: r.index(at)?.map(|v| v as u8),
                mute: r.u32(at + 4)? != 0,
                solo: r.u32(at + 8)? != 0,
            });
        }
        Ok(SeqSet {
            version,
            tpqn: r.u32(TPQN_AT)?,
            time_sig: (r.u32(TIME_SIG_AT)?, r.u32(TIME_SIG_AT + 4)?),
            patterns,
            lanes,
            groups,
            song,
            tracks,
        })
    }

    /// The pattern of 1-based `track` (1–16) and `variation` (1–8), with its lanes.
    pub fn pattern(&self, track: usize, variation: usize) -> Option<(&Pattern, &[Lane])> {
        if !(1..=TRACKS).contains(&track) || !(1..=VARIATIONS).contains(&variation) {
            return None;
        }
        let slot = (track - 1) * VARIATIONS + variation - 1;
        Some((&self.patterns[slot], &self.lanes[slot]))
    }

    /// Patterns that hold any event, in file order.
    pub fn used(&self) -> impl Iterator<Item = &Pattern> {
        self.patterns.iter().filter(|p| p.has_data())
    }
}

fn decode_events(raw: &[u8]) -> Vec<Event> {
    let recs = raw.as_chunks::<EVENT_LEN>().0;
    let u32_at = |e: &[u8], at: usize| u32::from_be_bytes(e[at..at + 4].try_into().unwrap());
    let le_at = |e: &[u8], at: usize| u32::from_le_bytes(e[at..at + 4].try_into().unwrap());
    let delta = |e: &[u8]| u32::from(u16::from_be_bytes([e[0], e[1]]));
    let mut out = Vec::new();
    let (mut tick, mut i) = (0u32, 0);
    while i < recs.len() {
        let e = &recs[i];
        let ch = e[3] & 0x0f;
        let kind = match e[2] {
            note @ 0..=0x7f => Kind::Note {
                ch,
                note,
                vel: e[6],
                off_vel: e[7],
                gate: u32_at(e, 12),
            },
            0x80 => Kind::Fill,
            0x8d => Kind::PolyAt {
                ch,
                note: e[6],
                val: e[7],
            },
            0x8e => Kind::Cc {
                ch,
                num: e[6],
                val: e[7],
            },
            0x8f => Kind::Pc { ch, prog: e[6] },
            0x90 => Kind::ChanAt { ch, val: e[6] },
            0x91 => Kind::Bend {
                ch,
                val: u16::from(e[6] & 0x7f) | u16::from(e[7] & 0x7f) << 7,
            },
            0xfd => Kind::Tempo {
                bpm_x100: u32_at(e, 12),
            },
            0xfb => Kind::TimeSig {
                num: le_at(e, 8),
                den: le_at(e, 12),
                clocks: e[6],
            },
            0x95 => {
                // 10 payload bytes per continuation event, which also carry time.
                let len = u32_at(e, 12) as usize;
                let cont = &recs[i + 1..recs.len().min(i + 1 + len.div_ceil(10))];
                let mut payload: Vec<u8> = cont.iter().flat_map(|c| c[6..16].to_vec()).collect();
                payload.truncate(len);
                out.push(Event {
                    tick,
                    kind: Kind::SysEx { payload },
                });
                tick = tick.wrapping_add(delta(e));
                for c in cont {
                    tick = tick.wrapping_add(delta(c));
                }
                i += 1 + cont.len();
                continue;
            }
            other => Kind::Other(other),
        };
        out.push(Event { tick, kind });
        tick = tick.wrapping_add(delta(e));
        i += 1;
    }
    out
}

fn vlq(mut n: u64) -> Vec<u8> {
    let mut out = vec![(n & 0x7f) as u8];
    while n > 0x7f {
        n >>= 7;
        out.insert(0, (n & 0x7f) as u8 | 0x80);
    }
    out
}

/// Export one pattern (and its lanes) as a format-0 SMF on the pattern's track channel.
///
/// The loop is `bars` long: messages starting past it are dropped and note-offs cut at it.
pub fn to_smf(pattern: &Pattern, tpqn: u32, lanes: &[Lane]) -> Vec<u8> {
    // (tick, order, bytes); order puts note-offs before other messages before note-ons.
    let mut msgs: Vec<(u64, u8, Vec<u8>)> = Vec::new();
    for ev in &pattern.events {
        let t = u64::from(ev.tick);
        let m7 = |v: u8| v & 0x7f;
        match &ev.kind {
            Kind::Note {
                ch,
                note,
                vel,
                off_vel,
                gate,
            } => {
                msgs.push((t, 3, vec![0x90 | ch, *note, m7(*vel).max(1)]));
                msgs.push((
                    t + u64::from(*gate),
                    1,
                    vec![0x80 | ch, *note, m7(*off_vel)],
                ));
            }
            Kind::PolyAt { ch, note, val } => {
                msgs.push((t, 2, vec![0xa0 | ch, m7(*note), m7(*val)]))
            }
            Kind::Cc { ch, num, val } => msgs.push((t, 2, vec![0xb0 | ch, m7(*num), m7(*val)])),
            Kind::Pc { ch, prog } => msgs.push((t, 2, vec![0xc0 | ch, m7(*prog)])),
            Kind::ChanAt { ch, val } => msgs.push((t, 2, vec![0xd0 | ch, m7(*val)])),
            Kind::Bend { ch, val } => msgs.push((
                t,
                2,
                vec![0xe0 | ch, (val & 0x7f) as u8, (val >> 7) as u8 & 0x7f],
            )),
            Kind::Tempo { bpm_x100 } if *bpm_x100 != 0 => {
                let us = (6e9 / f64::from(*bpm_x100))
                    .round_ties_even()
                    .min(f64::from(0xff_ffff)) as u32;
                msgs.push((
                    t,
                    0,
                    [&[0xff, 0x51, 3][..], &us.to_be_bytes()[1..]].concat(),
                ));
            }
            Kind::TimeSig { num, den, clocks } => {
                let dd = (32 - den.leading_zeros()).saturating_sub(1) as u8;
                let clocks = if *clocks == 0 { 24 } else { *clocks };
                msgs.push((t, 0, vec![0xff, 0x58, 4, *num as u8, dd, clocks, 8]));
            }
            Kind::SysEx { payload } => {
                let m = match payload.split_first() {
                    Some((0xf0, rest)) => [&[0xf0][..], &vlq(rest.len() as u64), rest].concat(),
                    _ => [&[0xf7][..], &vlq(payload.len() as u64), payload].concat(),
                };
                msgs.push((t, 2, m));
            }
            _ => {}
        }
    }
    let ch = ((pattern.track - 1) & 0x0f) as u8;
    for lane in lanes {
        for &(t, v) in &lane.points {
            let m = match lane.id {
                0 => vec![0xd0 | ch, (v & 0x7f) as u8],
                0x79 => vec![0xe0 | ch, (v & 0x7f) as u8, (v >> 7 & 0x7f) as u8],
                id => vec![
                    0xb0 | ch,
                    (id.wrapping_sub(1) & 0x7f) as u8,
                    (v & 0x7f) as u8,
                ],
            };
            msgs.push((u64::from(t), 2, m));
        }
    }
    let end = u64::from(pattern.bars) * u64::from(tpqn) * 4;
    let mut msgs: Vec<_> = msgs
        .into_iter()
        .filter(|(t, o, _)| *t < end || *o == 1)
        .map(|(t, o, m)| (t.min(end), o, m))
        .collect();
    msgs.sort();

    let name: Vec<u8> = pattern
        .name
        .chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect();
    let mut trk = [&[0, 0xff, 3][..], &vlq(name.len() as u64), &name].concat();
    let mut last = 0;
    for (t, _, m) in &msgs {
        trk.extend(vlq(t - last));
        trk.extend(m);
        last = *t;
    }
    trk.extend(vlq(end.saturating_sub(last)));
    trk.extend([0xff, 0x2f, 0]);

    let mut out = b"MThd\0\0\0\x06\0\0\0\x01".to_vec();
    out.extend((tpqn as u16).to_be_bytes());
    out.extend(b"MTrk");
    out.extend((trk.len() as u32).to_be_bytes());
    out.extend(trk);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(file: &mut [u8], at: usize, v: u32) {
        file[at..at + 4].copy_from_slice(&v.to_be_bytes());
    }

    fn ev(delta: u16, kind: u8, ch: u8, b6: u8, b7: u8, tail: u32) -> [u8; 16] {
        let mut e = [0u8; 16];
        e[..2].copy_from_slice(&delta.to_be_bytes());
        (e[2], e[3], e[6], e[7]) = (kind, ch, b6, b7);
        e[12..].copy_from_slice(&tail.to_be_bytes());
        e
    }

    /// A v1.08 file whose pattern `slot` holds `events` (bars/name given) and lane 0 of slot 0
    /// holds `points`.
    /// Refresh the header MD5 after editing the header.
    fn seal(f: &mut [u8]) {
        let sum = md5::Md5::digest(&f[..MD5_AT]);
        f[MD5_AT..EVENTS].copy_from_slice(&sum);
    }

    fn build(
        slot: usize,
        name: &[u8],
        bars: u32,
        events: &[[u8; 16]],
        points: &[(u32, u32)],
    ) -> Vec<u8> {
        let mut f = vec![0u8; EVENTS];
        f[..4].copy_from_slice(b"CSQS");
        put(&mut f, 8, 1);
        put(&mut f, 0xc, 8);
        put(&mut f, TPQN_AT, 480);
        for g in 0..GROUPS {
            f[GROUPS_AT + g * GROUP_LEN] = 1;
            for t in 0..TRACKS {
                put(&mut f, GROUPS_AT + g * GROUP_LEN + 4 + 4 * t, u32::MAX);
            }
        }
        for i in 0..STEPS {
            put(&mut f, STEPS_AT + i * 0x20, u32::MAX);
        }
        for t in 0..TRACKS {
            put(&mut f, TRACK_STATE_AT + t * 0x20, u32::MAX);
        }
        let at = PATTERN_TABLE + slot * PATTERN_LEN;
        put(&mut f, at, events.len() as u32);
        put(&mut f, at + 0xc, bars);
        f[at + 0x14..at + 0x14 + name.len()].copy_from_slice(name);
        f.extend(events.iter().flatten());
        for lane in 0..TRACKS * VARIATIONS * LANES {
            let pts: &[(u32, u32)] = if lane == 0 { points } else { &[] };
            let mut head = [0u8; 16];
            head[..4].copy_from_slice(&(lane as u32 % 13).to_be_bytes());
            head[4..8].copy_from_slice(&(pts.len() as u32).to_be_bytes());
            f.extend(head);
            for (t, v) in pts {
                f.extend(t.to_le_bytes());
                f.extend(v.to_le_bytes());
            }
        }
        f.extend(TRAILER);
        let len = f.len() as u32;
        put(&mut f, 4, len);
        seal(&mut f);
        f
    }

    #[test]
    fn slots_fill_bank_a_before_bank_b() {
        let path = |i| slot_path(i).unwrap().to_string_lossy().replace('\\', "/");
        assert_eq!(path(0), "SEQ/SEQ_SET/A/001/SeqSet.SQS");
        assert_eq!(path(127), "SEQ/SEQ_SET/A/128/SeqSet.SQS");
        assert_eq!(path(128), "SEQ/SEQ_SET/B/001/SeqSet.SQS");
        assert_eq!(path(493), "SEQ/SEQ_SET/D/110/SeqSet.SQS");
        assert_eq!(slot_path(SLOTS), None);
    }

    #[test]
    fn events_get_absolute_ticks_and_fill_only_patterns_are_unused() {
        let fill = ev(100, 0x80, 0, 0, 0, 0);
        let file = build(
            9,
            b"Verse   ",
            2,
            &[ev(10, 60, 1, 100, 64, 48), fill, ev(0, 0x8e, 1, 7, 99, 0)],
            &[],
        );
        let s = SeqSet::parse(&file).unwrap();
        assert_eq!(s.used().count(), 1);
        let (p, _) = s.pattern(2, 2).unwrap();
        assert_eq!((p.name.as_str(), p.bars), ("Verse", 2));
        assert_eq!(p.events[1].tick, 10);
        assert_eq!(
            p.events[2],
            Event {
                tick: 110,
                kind: Kind::Cc {
                    ch: 1,
                    num: 7,
                    val: 99
                }
            }
        );
        let fill_only = build(0, b"x", 1, &[fill], &[]);
        assert_eq!(SeqSet::parse(&fill_only).unwrap().used().count(), 0);
    }

    #[test]
    fn rejects_bad_files() {
        let good = build(0, b"x", 1, &[], &[]);
        assert!(SeqSet::parse(&good).is_ok());
        assert!(SeqSet::parse(b"RIFF").is_err());
        assert!(SeqSet::parse(&good[..good.len() - 1]).is_err());
        let mut size = good.clone();
        put(&mut size, 4, 1);
        assert!(SeqSet::parse(&size).is_err());
        let mut extra = good.clone();
        extra.splice(good.len() - 4..good.len() - 4, [0u8; 8]);
        let len = extra.len() as u32;
        put(&mut extra, 4, len);
        seal(&mut extra);
        assert!(SeqSet::parse(&extra).is_err());
        let mut tampered = good.clone();
        tampered[TPQN_AT] ^= 1;
        assert!(SeqSet::parse(&tampered).is_err());
    }

    #[test]
    fn header_groups_song_and_track_state() {
        let mut f = build(0, b"x", 1, &[], &[]);
        let g = GROUPS_AT + 2 * GROUP_LEN;
        f[g] = 0;
        put(&mut f, g + 4, 3);
        put(&mut f, g + 0x44, 8);
        f[g + 0x48..g + 0x4f].copy_from_slice(b"Intro 1");
        put(&mut f, STEPS_AT, 2);
        put(&mut f, STEPS_AT + 4, 9);
        put(&mut f, STEPS_AT + 8, 1);
        put(&mut f, TRACK_STATE_AT + 0x20 + 4, 1);
        put(&mut f, SONG_LOOP_AT, 1);
        put(&mut f, SONG_LOOP_AT + 4, 0);
        put(&mut f, SONG_LOOP_AT + 8, u32::MAX);
        seal(&mut f);
        let s = SeqSet::parse(&f).unwrap();
        assert!(s.groups[2].used && !s.groups[0].used);
        assert_eq!(s.groups[2].variations[..2], [Some(3), None]);
        assert_eq!(
            (s.groups[2].bars, s.groups[2].name.as_str()),
            (8, "Intro 1")
        );
        assert_eq!(
            s.song.steps,
            [SongStep {
                group: 2,
                repeat: 9,
                skip: true
            }]
        );
        assert!(s.song.loop_on);
        assert_eq!((s.song.loop_a, s.song.loop_b), (Some(0), None));
        assert!(s.tracks[1].mute && !s.tracks[0].mute);
        assert_eq!(s.tracks[0].variation, None);
    }

    #[test]
    fn sysex_is_folded_and_exported() {
        let mut head = ev(5, 0x95, 0, 0, 0, 12);
        head[3] = 0;
        let mut c1 = ev(5, 0x96, 0, 0, 0, 0);
        c1[6..16].copy_from_slice(&[0xf0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let mut c2 = ev(0, 0x96, 0, 0, 0, 0);
        c2[6..8].copy_from_slice(&[10, 0xf7]);
        let file = build(0, b"S", 1, &[head, c1, c2, ev(0, 62, 0, 90, 0, 10)], &[]);
        let s = SeqSet::parse(&file).unwrap();
        let (p, _) = s.pattern(1, 1).unwrap();
        assert_eq!(p.events.len(), 2);
        let payload = vec![0xf0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0xf7];
        assert_eq!(
            p.events[0].kind,
            Kind::SysEx {
                payload: payload.clone()
            }
        );
        assert_eq!(p.events[1].tick, 10); // continuation deltas count
        let smf = to_smf(p, 480, &[]);
        let want = [&[0xf0, 11][..], &payload[1..]].concat();
        assert!(smf.windows(want.len()).any(|w| w == want));
        // no leading F0: sent as F7 escape
        let bare = Pattern {
            events: vec![Event {
                tick: 0,
                kind: Kind::SysEx {
                    payload: vec![1, 2],
                },
            }],
            ..p.clone()
        };
        assert!(to_smf(&bare, 480, &[])
            .windows(4)
            .any(|w| w == [0xf7, 2, 1, 2]));
    }

    #[test]
    fn smf_clamps_to_the_loop_and_adds_lanes() {
        let end = 480 * 4;
        let file = build(
            0,
            b"Lp",
            1,
            &[
                ev(end as u16 - 10, 0x80, 0, 0, 0, 0),
                ev(20, 60, 0, 100, 64, 100), // off at end + 90, cut to the end
                ev(0, 62, 0, 100, 64, 10),   // note-on starts past the end: dropped
            ],
            &[(0, 5), (7, 99)],
        );
        let s = SeqSet::parse(&file).unwrap();
        let (p, lanes) = s.pattern(1, 1).unwrap();
        // lane id 0 = channel AT
        let smf = to_smf(p, s.tpqn, &lanes[..1]);
        let mut want = b"MThd\0\0\0\x06\0\0\0\x01\x01\xe0MTrk".to_vec();
        let trk = [
            &[0, 0xff, 3, 2, b'L', b'p'][..],
            &[0, 0xd0, 5, 7, 0xd0, 99],
            &[0x8e, 0x6f, 0x90, 60, 100], // 1903 ticks after the lane point
            &[10, 0x80, 60, 64],
            &[0, 0x80, 62, 64], // its note-off is kept
            &[0, 0xff, 0x2f, 0],
        ]
        .concat();
        want.extend((trk.len() as u32).to_be_bytes());
        want.extend(trk);
        assert_eq!(smf, want);
    }

    #[test]
    fn tempo_rounds_half_to_even() {
        // 61.44 BPM = 976562.5 us per quarter
        let p = Pattern {
            track: 1,
            variation: 1,
            name: String::new(),
            bars: 1,
            events: vec![Event {
                tick: 0,
                kind: Kind::Tempo { bpm_x100: 6144 },
            }],
        };
        let smf = to_smf(&p, 480, &[]);
        assert!(smf
            .windows(6)
            .any(|w| w == [0xff, 0x51, 3, 0x0e, 0xe6, 0xb2]));
    }
}
