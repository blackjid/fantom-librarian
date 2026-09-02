//! Ask the instrument which wave group a bank's sounds play from.
//!
//! A tone stores a wave **group id** per partial, and for an expansion wave that id is the product
//! (`1005` is `EXZ005`). This selects each program of a bank in turn and reads the group fields
//! back out of the temporary area, which is how the id-to-product mapping was decoded: every bank
//! of every installed expansion answers with its own product's id, from every page it occupies.
//!
//!     cargo run -p fantom-midi --bin dump-wave-groups -- <msb> <lsb> [options]
//!     cargo run -p fantom-midi --bin dump-wave-groups -- 93 11 --last 4
//!
//! Options:
//!     --zone N       zone to audition in, 1-based (default 1)
//!     --first PC     first program, 0-based (default 0)
//!     --last PC      last program, 0-based (default 4)
//!     --delay MS     how long to let the engine settle before reading (default 150)
//!     --port NAME    MIDI port to use, when it is not the FANTOM's usual one
//!
//! A ZEN-Core bank answers with its four partials; a drum bank (MSB 92 or 100) with its 88 key
//! instruments, whose first `WMT` block is read. Rows go to stdout as TSV.
//!
//! **This writes to the temporary scene**, exactly as `dump-sounds` does: nothing is stored unless
//! you press Write, but audition on a scratch scene rather than one you care about.

use std::time::Duration;

use fantom_midi::{dt1, offset_addr, zone_block, Session, TempArea, Unanswered};

/// `WAV_GTYPE` through `WAV_NUM_R`: one 7-bit byte then three 4-nibble words.
const WAVE_FIELDS: u32 = 13;

/// Long enough for a bank page that has to be paged in before it answers.
const REPLY: Duration = Duration::from_millis(800);

/// A ZEN-Core partial's wave block: `PCMT_PTL` instance `p` at `00 2p 00`, fields at `+27`.
const TONE_PARTIALS: usize = 4;
const TONE_PARTIAL_BLOCK: u8 = 0x20;
const TONE_WAVE_AT: u8 = 27;

/// A drum kit's Inst Set is its own area, one `Inst` per key from 21, and the first `WMT` block's
/// wave fields sit at `+0x1e`.
const DRUM_KEYS: usize = 88;
const DRUM_INST_STRIDE: usize = 5 << 7;
const DRUM_WAVE_AT: u8 = 0x1e;

struct Options {
    msb: u8,
    lsb: u8,
    zone: u8,
    first: u8,
    last: u8,
    delay: Duration,
    port: Option<String>,
}

fn options() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let msb = args
        .next()
        .and_then(|a| a.parse().ok())
        .ok_or("usage: dump-wave-groups <msb> <lsb> [--zone N] [--first PC] [--last PC] [--delay MS] [--port NAME]")?;
    let lsb = args
        .next()
        .and_then(|a| a.parse().ok())
        .ok_or("a bank is an MSB and an LSB")?;
    let mut o = Options {
        msb,
        lsb,
        zone: 0,
        first: 0,
        last: 4,
        delay: Duration::from_millis(150),
        port: None,
    };
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        let number = || {
            value
                .parse::<u32>()
                .map_err(|_| format!("{flag} wants a number"))
        };
        match flag.as_str() {
            "--zone" => o.zone = (number()?.max(1) - 1).min(15) as u8,
            "--first" => o.first = number()?.min(127) as u8,
            "--last" => o.last = number()?.min(127) as u8,
            "--delay" => o.delay = Duration::from_millis(number()? as u64),
            "--port" => o.port = Some(value.clone()),
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    Ok(o)
}

/// Where one sound's wave blocks live, and what to call each of them in a row.
fn blocks(area: TempArea, zone: u8) -> Vec<(String, [u8; 4])> {
    match area {
        // A drum kit's waves are per key, in the Inst Set area rather than the kit's own.
        TempArea::DrumKit => {
            let base = [0x03, 4 * zone, 0x00, 0x00];
            (0..DRUM_KEYS)
                .map(|key| {
                    let step = key * DRUM_INST_STRIDE;
                    let at = offset_addr(
                        base,
                        [
                            ((step >> 14) & 0x7f) as u8,
                            ((step >> 7) & 0x7f) as u8,
                            (step & 0x7f) as u8,
                        ],
                    );
                    (
                        format!("key {}", key + 21),
                        offset_addr(at, [0x00, 0x00, DRUM_WAVE_AT]),
                    )
                })
                .collect()
        }
        _ => (0..TONE_PARTIALS)
            .map(|partial| {
                (
                    format!("partial {}", partial + 1),
                    offset_addr(
                        area.name_addr(zone),
                        [0x00, TONE_PARTIAL_BLOCK + partial as u8, TONE_WAVE_AT],
                    ),
                )
            })
            .collect(),
    }
}

/// A four-nibble wire word, most significant nibble first.
fn word(data: &[u8], at: usize) -> u16 {
    (data[at] as u16) << 12
        | (data[at + 1] as u16) << 8
        | (data[at + 2] as u16) << 4
        | data[at + 3] as u16
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o = options()?;
    let area = TempArea::for_bank(o.msb, o.lsb).ok_or_else(|| {
        format!(
            "no temporary area known for MSB {} LSB {}, so nothing to read",
            o.msb, o.lsb
        )
    })?;
    let wave_blocks = blocks(area, o.zone);
    eprintln!(
        "reading {}/{} from the {area:?} area, zone {}, {} blocks per sound",
        o.msb,
        o.lsb,
        o.zone + 1,
        wave_blocks.len()
    );

    let mut fantom = Session::open(o.port.as_deref())?.with_timeout(REPLY);

    println!("msb\tlsb\tpc\tblock\tgroup_type\tgroup_id\twave_l\twave_r");
    for pc in o.first..=o.last {
        fantom.send(&dt1(zone_block(o.zone), &[o.msb, o.lsb, pc]))?;
        std::thread::sleep(o.delay);
        for (label, at) in &wave_blocks {
            let data = match fantom.read(*at, WAVE_FIELDS) {
                Ok(data) => data,
                Err(Unanswered::Silence) => {
                    eprintln!("PC {pc} {label}: no answer; try a longer --delay");
                    continue;
                }
                Err(short) => {
                    eprintln!("PC {pc} {label}: {short}");
                    continue;
                }
            };
            // The panel numbers a bank from one; the wire counts programs from zero.
            println!(
                "{}\t{}\t{}\t{label}\t{}\t{}\t{}\t{}",
                o.msb,
                o.lsb,
                pc + 1,
                data[0],
                word(&data, 1),
                word(&data, 5),
                word(&data, 9)
            );
        }
    }
    eprintln!("The zone is left holding the last sound; nothing is stored.");
    Ok(())
}
