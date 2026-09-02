//! One open conversation with the instrument.
//!
//! Every tool here does the same three things before it can ask the FANTOM anything: find the port
//! by name in both directions, reassemble SysEx replies that arrive split across callbacks, and
//! throw away whatever is still queued from the last question before asking the next one. Getting
//! any of them wrong produces the same symptom — an answer that belongs to the previous request —
//! so they live here once rather than in each `bin`.
//!
//! [`Session::read`] is the operation worth having: ask for an address and get back exactly the
//! bytes that address holds, with the reply's header already off and a short answer told apart
//! from a silent one. [`Session::read_available`] is the same question for a caller that wants
//! whatever the instrument actually returned, however much that is.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

use crate::rq1;

/// The port a FANTOM-6/7/8 presents over USB.
pub const PORT: &str = "FANTOM-6 7 8";

/// How long to wait for a reply before giving up on it.
const REPLY: Duration = Duration::from_millis(800);

/// Where a DT1 reply's data starts: `F0 41 dev <model×4> 12 <address×4>`.
const DATA_AT: usize = 12;

/// An open MIDI connection to one instrument, in both directions.
pub struct Session {
    out: MidiOutputConnection,
    replies: Receiver<Vec<u8>>,
    /// Held only to keep the callback alive; dropping it closes the input.
    _input: MidiInputConnection<()>,
    timeout: Duration,
}

/// Why a request went unanswered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unanswered {
    /// Nothing came back before the timeout — usually a sleeping instrument, `Rx SysEx` switched
    /// off, or too little settling time after selecting a sound.
    Silence,
    /// A reply arrived holding fewer data bytes than were asked for, which is what an address
    /// the instrument does not implement looks like.
    Short(usize),
}

impl std::fmt::Display for Unanswered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Silence => write!(f, "no answer"),
            Self::Short(len) => write!(f, "short reply ({len} bytes)"),
        }
    }
}

impl Session {
    /// Open the FANTOM's usual port, or `port` when it is somewhere else.
    ///
    /// The error names the ports that *are* here, because a missing one is nearly always a wrong
    /// name or an instrument that has not woken up rather than a fault worth debugging.
    pub fn open(port: Option<&str>) -> Result<Self, String> {
        let wanted = port.unwrap_or(PORT);
        let out = MidiOutput::new("fantom-out").map_err(|e| e.to_string())?;
        let inp = MidiInput::new("fantom-in").map_err(|e| e.to_string())?;

        let names = |found: Vec<String>| {
            if found.is_empty() {
                format!("no MIDI ports at all, so no {wanted:?}")
            } else {
                format!(
                    "no MIDI port called {wanted:?}. Ports here: {}",
                    found.join(", ")
                )
            }
        };
        let dest = out
            .ports()
            .into_iter()
            .find(|p| out.port_name(p).as_deref() == Ok(wanted))
            .ok_or_else(|| {
                names(
                    out.ports()
                        .iter()
                        .filter_map(|p| out.port_name(p).ok())
                        .collect(),
                )
            })?;
        let src = inp
            .ports()
            .into_iter()
            .find(|p| inp.port_name(p).as_deref() == Ok(wanted))
            .ok_or_else(|| {
                names(
                    inp.ports()
                        .iter()
                        .filter_map(|p| inp.port_name(p).ok())
                        .collect(),
                )
            })?;

        let (tx, replies) = std::sync::mpsc::channel();
        // A SysEx message longer than the driver's buffer arrives in pieces, so a reply is only
        // whole at `F7`. Anything starting mid-message — the tail of something already dropped —
        // is discarded rather than prepended to the next one.
        let mut pending: Vec<u8> = Vec::new();
        let input = inp
            .connect(
                &src,
                "fantom",
                move |_, message, _| {
                    if message.first() == Some(&0xF0) {
                        pending.clear();
                    } else if pending.is_empty() {
                        return;
                    }
                    pending.extend_from_slice(message);
                    if pending.last() == Some(&0xF7) {
                        let _ = tx.send(std::mem::take(&mut pending));
                    }
                },
                (),
            )
            .map_err(|e| e.to_string())?;

        Ok(Self {
            out: out.connect(&dest, "fantom").map_err(|e| e.to_string())?,
            replies,
            _input: input,
            timeout: REPLY,
        })
    }

    /// Wait this long for each reply instead of the default.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Send one MIDI message.
    pub fn send(&mut self, message: &[u8]) -> Result<(), String> {
        self.out.send(message).map_err(|e| e.to_string())
    }

    /// Throw away replies still queued from an earlier question.
    ///
    /// The instrument answers at its own pace, so a reply that arrives late is indistinguishable
    /// from the next question's — except that it is wrong. Every read starts by dropping them.
    pub fn drain(&mut self) {
        while !matches!(self.replies.try_recv(), Err(TryRecvError::Empty)) {}
    }

    /// The next whole SysEx message, or nothing within the timeout.
    pub fn receive(&mut self) -> Option<Vec<u8>> {
        self.replies.recv_timeout(self.timeout).ok()
    }

    /// Ask for `size` bytes at `addr` and return whatever data came back.
    ///
    /// `size` is what to request, not what to require: **the instrument is the authority on a
    /// block's length**, and it does not always agree with the parameter map — `PCMS_PTL` is
    /// documented as 30 bytes and a FANTOM-6 answers with 29. A caller measuring the map against
    /// the hardware needs that difference, so it is reported rather than rejected.
    ///
    /// Drains first, so the answer belongs to this question. The reply's `F0 41 …` header and its
    /// trailing checksum are off: what comes back is the parameter data itself.
    pub fn read_available(&mut self, addr: [u8; 4], size: u32) -> Option<Vec<u8>> {
        self.drain();
        self.send(&rq1(addr, size)).ok()?;
        let reply = self.receive()?;
        // `F0 41 dev <model×4> 12 <address×4> … <checksum> F7`
        let end = reply.len().checked_sub(2)?;
        reply.get(DATA_AT..end).map(<[u8]>::to_vec)
    }

    /// Ask for `size` bytes at `addr` and insist on getting exactly that many.
    ///
    /// For a caller that knows what the address holds and cannot use a partial answer. See
    /// [`Session::read_available`] for one that can.
    pub fn read(&mut self, addr: [u8; 4], size: u32) -> Result<Vec<u8>, Unanswered> {
        let mut data = self.read_available(addr, size).ok_or(Unanswered::Silence)?;
        if data.len() < size as usize {
            return Err(Unanswered::Short(data.len()));
        }
        data.truncate(size as usize);
        Ok(data)
    }

    /// Read an ASCII name of [`crate::NAME_LEN`] bytes, trimmed as the panel shows it.
    pub fn read_name(&mut self, addr: [u8; 4]) -> Result<String, Unanswered> {
        let bytes = self.read(addr, crate::NAME_LEN)?;
        Ok(String::from_utf8_lossy(&bytes).trim_end().to_string())
    }
}
