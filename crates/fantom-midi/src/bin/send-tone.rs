//! Send one tone from a file into the instrument's temporary memory, then verify it.
//!
//! Every block of the tone is rebuilt from the file record through the parameter table and
//! written with DT1, then read back and compared field by field. Nothing is stored: the tone
//! lives in the zone's edit buffer until you select another sound.
//!
//!     cargo run -p fantom-midi --bin send-tone -- path/to/FANTOM.SVD [tone-index] [zone]

use fantom_midi::{dt1, offset_addr, params, rq1, temp_tone, wire_bytes};
use midir::{MidiInput, MidiOutput};
use std::sync::mpsc;
use std::time::Duration;

const PORT: &str = "FANTOM-6 7 8";
const REPLY: Duration = Duration::from_millis(600);

/// Build one block's wire form from the file record. Gaps between fields, and the addresses
/// the instrument reserves, go out as zero.
fn block_bytes(frec: &[u8], block: &params::Block) -> Vec<u8> {
    let mut buf = vec![0u8; block.sysex_len as usize];
    for p in block.params {
        if p.reserved {
            continue;
        }
        let at = p.sysex_offset as usize;
        for (i, b) in wire_bytes(frec, p).into_iter().enumerate() {
            if at + i < buf.len() {
                buf[at + i] = b;
            }
        }
    }
    buf
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: send-tone <FANTOM.SVD> [tone-index] [zone]")?;
    let index: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let zone: u8 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let b = std::fs::read(&path)?;

    let header_size = u16::from_le_bytes([b[0], b[1]]) as usize;
    let mut pata = None;
    for i in 0..(header_size - 14) / 16 {
        let off = 0x10 + i * 16;
        if &b[off..off + 4] == b"PATa" {
            pata = Some(u32::from_le_bytes(b[off + 8..off + 12].try_into()?) as usize);
        }
    }
    let pata = pata.ok_or("no PATa area in that file")?;
    let rs = u32::from_le_bytes(b[pata + 4..pata + 8].try_into()?) as usize;
    let rec = &b[pata + 16 + index * rs..pata + 16 + (index + 1) * rs];
    let name = String::from_utf8_lossy(&rec[0..16]).trim_end().to_string();
    println!("sending {name:?} (PATa[{index}] of {path}) to zone {}", zone + 1);

    let out = MidiOutput::new("send-tone-out")?;
    let inp = MidiInput::new("send-tone-in")?;
    let dest = out.ports().into_iter()
        .find(|p| out.port_name(p).as_deref() == Ok(PORT)).ok_or("Fantom not found")?;
    let src = inp.ports().into_iter()
        .find(|p| inp.port_name(p).as_deref() == Ok(PORT)).ok_or("Fantom not found")?;
    let (tx, rx) = mpsc::channel();
    let mut pending: Vec<u8> = Vec::new();
    let _ci = inp.connect(&src, "s", move |_, m, _| {
        if m.first() == Some(&0xF0) {
            pending.clear();
        } else if pending.is_empty() {
            return;
        }
        pending.extend_from_slice(m);
        if pending.last() == Some(&0xF7) {
            let _ = tx.send(std::mem::take(&mut pending));
        }
    }, ())?;
    let mut co = out.connect(&dest, "s")?;

    // The temporary Z-Core address only applies while the zone holds a Z-Core tone.
    co.send(&[0xB0 | zone, 0x00, 87])?;
    co.send(&[0xB0 | zone, 0x20, 0])?;
    co.send(&[0xC0 | zone, 0])?;
    std::thread::sleep(Duration::from_millis(300));

    let base = temp_tone(zone);
    for inst in params::TONE {
        let frec = &rec[inst.byte_offset as usize..];
        let data = block_bytes(frec, inst.block);
        co.send(&dt1(offset_addr(base, inst.sysex_offset), &data))?;
        std::thread::sleep(Duration::from_millis(20)); // Roland's inter-packet interval
    }
    println!("wrote {} blocks", params::TONE.len());

    std::thread::sleep(Duration::from_millis(300));
    while rx.try_recv().is_ok() {}

    let (mut ok, mut bad, mut unread) = (0, 0, 0);
    let mut first: Option<String> = None;
    for inst in params::TONE {
        co.send(&rq1(offset_addr(base, inst.sysex_offset), inst.block.sysex_len as u32))?;
        let Ok(r) = rx.recv_timeout(REPLY) else { unread += 1; continue };
        let wire = &r[12..r.len() - 2];
        let frec = &rec[inst.byte_offset as usize..];
        for p in inst.block.params {
            if p.reserved {
                continue;
            }
            let expect = wire_bytes(frec, p);
            let at = p.sysex_offset as usize;
            if at + expect.len() > wire.len() {
                continue;
            }
            if wire[at..at + expect.len()] == expect[..] {
                ok += 1;
            } else {
                bad += 1;
                if first.is_none() {
                    first = Some(format!(
                        "{}.{} expected {:02X?}, got {:02X?}",
                        inst.block.name, p.id, expect, &wire[at..at + expect.len()]));
                }
            }
        }
    }

    let back = {
        co.send(&rq1(base, 0x36))?;
        rx.recv_timeout(REPLY).ok()
            .map(|r| String::from_utf8_lossy(&r[12..28]).trim_end().to_string())
    };
    println!("instrument now reports: {:?}", back.unwrap_or_default());
    println!("{ok} fields match, {bad} differ, {unread} blocks unread");
    if let Some(f) = first {
        println!("first difference: {f}");
    }

    // Play it, so the result is audible and not just a byte count.
    println!("playing…");
    for n in [48u8, 55, 60, 64] {
        co.send(&[0x90 | zone, n, 90])?;
        std::thread::sleep(Duration::from_millis(120));
    }
    std::thread::sleep(Duration::from_millis(1400));
    for n in [48u8, 55, 60, 64] {
        co.send(&[0x80 | zone, n, 0])?;
    }
    Ok(())
}
