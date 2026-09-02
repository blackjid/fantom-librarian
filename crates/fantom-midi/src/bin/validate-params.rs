//! Check the generated parameter table against real hardware.
//!
//! For each user tone that a backup and the connected instrument agree on by name, this asks the
//! instrument for every block of that tone and compares it against the table's prediction from the
//! file bytes. A field only counts as confirmed once it has been seen holding a non-default value —
//! agreeing on a byte that is zero on both sides proves nothing.
//!
//!     cargo run -p fantom-midi --bin validate-params -- path/to/FANTOM.SVD [tone-count]

use fantom_midi::{file_value, offset_addr, params, temp_tone, wire_bytes, Session};
use std::collections::HashMap;
use std::time::Duration;

/// A block the instrument declines to answer for costs this much per tone, so keep it tight.
const REPLY: Duration = Duration::from_millis(400);

struct Stat {
    agree: usize,
    differ: usize,
    /// Agreements where the file value was something other than the block's default.
    meaningful: usize,
    example: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: validate-params <FANTOM.SVD>")?;
    let b = std::fs::read(&path)?;
    let tones: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);

    let mut pata = None;
    let header_size = u16::from_le_bytes([b[0], b[1]]) as usize;
    for i in 0..(header_size - 14) / 16 {
        let off = 0x10 + i * 16;
        if &b[off..off + 4] == b"PATa" {
            pata = Some(u32::from_le_bytes(b[off + 8..off + 12].try_into()?) as usize);
        }
    }
    let pata = pata.ok_or("no PATa area in that file")?;
    let count = u32::from_le_bytes(b[pata..pata + 4].try_into()?) as usize;
    let rs = u32::from_le_bytes(b[pata + 4..pata + 8].try_into()?) as usize;

    let mut fantom = Session::open(None)?.with_timeout(REPLY);

    let mut stats: HashMap<(&str, &str), Stat> = HashMap::new();
    let mut misses: HashMap<(&str, &str), usize> = HashMap::new();
    let mut first_short: Option<String> = None;
    let mut matched = 0;
    let mut skipped = 0;

    for k in 0..count.min(tones) {
        let rec = &b[pata + 16 + k * rs..pata + 16 + (k + 1) * rs];
        let fname = String::from_utf8_lossy(&rec[0..16]).trim_end().to_string();
        if fname.is_empty() || fname.starts_with("INIT") {
            continue;
        }

        fantom.send(&[0xB0, 0x00, 87])?;
        fantom.send(&[0xB0, 0x20, (k / 128) as u8])?;
        fantom.send(&[0xC0, (k % 128) as u8])?;
        std::thread::sleep(Duration::from_millis(250));

        // Confirm the instrument really loaded the tone this file record describes.
        let Ok(wname) = fantom.read_name(temp_tone(0)) else {
            continue;
        };
        if wname != fname {
            skipped += 1;
            continue;
        }
        matched += 1;
        eprint!("\r  {matched} tones…");

        for inst in params::tone::TONE {
            let addr = offset_addr(temp_tone(0), inst.sysex_offset);
            // Whatever came back, not what the map predicts: the length difference is the finding.
            let Some(wire) = fantom.read_available(addr, inst.block.sysex_len as u32) else {
                *misses.entry((inst.block.name, "no reply")).or_insert(0) += 1;
                continue;
            };
            // The instrument is the authority on block length: PCMS_PTL is documented as 30
            // bytes but a FANTOM-6 returns 29. Only require room for the fields themselves.
            let needed = inst
                .block
                .params
                .iter()
                .map(|p| p.sysex_offset as usize + p.len_sysex as usize)
                .max()
                .unwrap_or(0);
            if wire.len() < needed {
                *misses.entry((inst.block.name, "short")).or_insert(0) += 1;
                if first_short.is_none() {
                    first_short = Some(format!(
                        "{} needed {} got {}",
                        inst.block.name,
                        needed,
                        wire.len()
                    ));
                }
                continue;
            }
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
                let actual = &wire[at..at + expect.len()];
                let e = stats.entry((inst.block.name, p.id)).or_insert(Stat {
                    agree: 0,
                    differ: 0,
                    meaningful: 0,
                    example: None,
                });
                if actual == expect.as_slice() {
                    e.agree += 1;
                    if file_value(frec, p) != 0 {
                        e.meaningful += 1;
                    }
                } else {
                    e.differ += 1;
                    if e.example.is_none() {
                        e.example = Some(format!(
                            "{fname}: file {:?} -> expected {:02X?}, got {:02X?}",
                            file_value(frec, p),
                            expect,
                            actual
                        ));
                    }
                }
            }
        }
    }

    eprintln!();
    println!("compared {matched} tones ({skipped} skipped: instrument's USER bank differs)\n");

    let mut by_block: HashMap<&str, (usize, usize, usize)> = HashMap::new();
    for ((block, _), s) in &stats {
        let e = by_block.entry(block).or_default();
        if s.differ > 0 {
            e.1 += 1
        } else if s.meaningful > 0 {
            e.0 += 1
        } else {
            e.2 += 1
        }
    }
    let mut names: Vec<_> = by_block.keys().copied().collect();
    names.sort();
    println!(
        "{:<10} {:>9} {:>9} {:>9}",
        "block", "confirmed", "MISMATCH", "untested"
    );
    for n in names {
        let (ok, bad, un) = by_block[n];
        println!("{n:<10} {ok:>9} {bad:>9} {un:>9}");
    }

    if !misses.is_empty() {
        let mut m: Vec<_> = misses.iter().collect();
        m.sort();
        println!("\nblocks never compared:");
        for ((b, why), n) in m {
            println!("  {b:<10} {why} x{n}");
        }
        if let Some(s) = &first_short {
            println!("  e.g. {s}");
        }
    }

    let mut bad: Vec<_> = stats.iter().filter(|(_, s)| s.differ > 0).collect();
    bad.sort_by_key(|((b, i), _)| (*b, *i));
    if bad.is_empty() {
        println!("\nno mismatches.");
    } else {
        println!("\nmismatching fields:");
        for ((block, id), s) in bad.iter().take(25) {
            println!("  {block}.{id}  {} agree / {} differ", s.agree, s.differ);
            if let Some(x) = &s.example {
                println!("      {x}");
            }
        }
        println!("  ({} mismatching fields total)", bad.len());
    }
    Ok(())
}
