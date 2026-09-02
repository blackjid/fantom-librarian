//! Send one tone from a file into the instrument's temporary memory, then verify it.
//!
//! Every block of the tone is rebuilt from the file record through the parameter table and
//! written with DT1, then read back and compared field by field. Nothing is stored: the tone
//! lives in the zone's edit buffer until you select another sound.
//!
//!     cargo run -p fantom-midi --bin send-tone -- path/to/FANTOM.SVD [tone-index] [zone]

use fantom_midi::{dt1, offset_addr, params, temp_tone, wire_bytes, Session};
use std::time::Duration;

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
    let path = args
        .next()
        .ok_or("usage: send-tone <FANTOM.SVD> [tone-index] [zone]")?;
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
    println!(
        "sending {name:?} (PATa[{index}] of {path}) to zone {}",
        zone + 1
    );

    let mut fantom = Session::open(None)?;

    // The temporary Z-Core address only applies while the zone holds a Z-Core tone, so point the
    // zone at USER ZEN-Core slot 1 first. Addressing the zone's own block rather than sending
    // bank-select on channel `zone` keeps this right when a scene remaps its receive channels.
    let zone_block = [0x02, 0x00, 0x10 + zone, 0x00];
    fantom.send(&dt1(zone_block, &[87, 0, 0]))?;
    std::thread::sleep(Duration::from_millis(300));

    // Play on whatever channel the zone actually listens to.
    let channel = fantom
        .read(zone_block, 0x49)
        .ok()
        .and_then(|zone| zone.get(3).copied())
        .filter(|channel| *channel < 16)
        .unwrap_or(zone);
    println!("zone {} receives on MIDI channel {}", zone + 1, channel + 1);

    let base = temp_tone(zone);
    for inst in params::tone::TONE {
        let frec = &rec[inst.byte_offset as usize..];
        let data = block_bytes(frec, inst.block);
        fantom.send(&dt1(offset_addr(base, inst.sysex_offset), &data))?;
        std::thread::sleep(Duration::from_millis(20)); // Roland's inter-packet interval
    }
    println!("wrote {} blocks", params::tone::TONE.len());

    // The panel caches the tone name and a temporary-memory write does not invalidate it, so the
    // screen keeps showing the old name until something makes it redraw. Selecting another zone
    // does, and Current Zone is just a parameter — so do it from here and land back on `zone`.
    const CURRENT_ZONE: [u8; 4] = [0x02, 0x00, 0x00, 0x12];
    fantom.send(&dt1(CURRENT_ZONE, &[if zone == 0 { 1 } else { 0 }]))?;
    std::thread::sleep(Duration::from_millis(150));
    fantom.send(&dt1(CURRENT_ZONE, &[zone]))?;
    std::thread::sleep(Duration::from_millis(450));

    let (mut ok, mut bad, mut unread) = (0, 0, 0);
    let mut first: Option<String> = None;
    for inst in params::tone::TONE {
        let Ok(wire) = fantom.read(
            offset_addr(base, inst.sysex_offset),
            inst.block.sysex_len as u32,
        ) else {
            unread += 1;
            continue;
        };
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
                        inst.block.name,
                        p.id,
                        expect,
                        &wire[at..at + expect.len()]
                    ));
                }
            }
        }
    }

    let back = fantom.read_name(base).unwrap_or_default();
    println!("instrument now reports: {back:?}");
    println!("{ok} fields match, {bad} differ, {unread} blocks unread");
    if let Some(f) = first {
        println!("first difference: {f}");
    }

    // Play it, so the result is audible and not just a byte count.
    println!("playing…");
    for n in [48u8, 55, 60, 64] {
        fantom.send(&[0x90 | channel, n, 90])?;
        std::thread::sleep(Duration::from_millis(120));
    }
    std::thread::sleep(Duration::from_millis(1400));
    for n in [48u8, 55, 60, 64] {
        fantom.send(&[0x80 | channel, n, 0])?;
    }
    Ok(())
}
