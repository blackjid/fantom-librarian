//! Ask the instrument what is in one of its banks, sound by sound.
//!
//! A model or wave expansion ships its sounds inside the instrument and its names in a PDF of its
//! own. This is the other way to get them: select each program in turn and read back the name the
//! engine is now holding, which works for whatever is actually installed and needs no document.
//!
//!     cargo run -p fantom-midi --bin dump-sounds -- <msb> <lsb> [options]
//!     cargo run -p fantom-midi --bin dump-sounds -- 97 68 --delay 60 > juno106.tsv
//!
//! Options:
//!     --zone N       zone to audition in, 1-based (default 1)
//!     --first PC     first program, 0-based (default 0)
//!     --last PC      last program, 0-based (default 127)
//!     --delay MS     how long to let the engine settle before reading (default 60)
//!     --engine NAME  temporary area to read, for a bank this version cannot place
//!     --repeats N    consecutive identical names that mean the bank ended (default 4)
//!     --port NAME    MIDI port to use, when it is not the FANTOM's usual one
//!
//! Rows go to stdout in the same shape `tools/gen_sound_list.py` writes, so both sources feed one
//! table. **This writes to the temporary scene**, which picks up the edited asterisk — nothing is
//! stored unless you press Write, but audition on a scratch scene rather than one you care about.

use std::time::Duration;

use fantom_midi::{dt1, zone_block, Session, TempArea, Unanswered};

struct Options {
    msb: u8,
    lsb: u8,
    zone: u8,
    first: u8,
    last: u8,
    delay: Duration,
    repeats: usize,
    engine: Option<TempArea>,
    port: Option<String>,
}

fn options() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let msb = args
        .next()
        .and_then(|a| a.parse().ok())
        .ok_or("usage: dump-sounds <msb> <lsb> [--zone N] [--first PC] [--last PC] [--delay MS] [--engine NAME] [--repeats N] [--port NAME]")?;
    let lsb = args
        .next()
        .and_then(|a| a.parse().ok())
        .ok_or("a bank is an MSB and an LSB")?;
    let mut o = Options {
        msb,
        lsb,
        zone: 0,
        first: 0,
        last: 127,
        delay: Duration::from_millis(60),
        repeats: 4,
        engine: None,
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
            "--repeats" => o.repeats = number()? as usize,
            "--port" => o.port = Some(value.clone()),
            "--engine" => {
                o.engine =
                    Some(TempArea::parse(&value).ok_or(format!("no engine called {value:?}"))?)
            }
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    Ok(o)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o = options()?;
    let area = o
        .engine
        .or_else(|| TempArea::for_bank(o.msb, o.lsb))
        .ok_or_else(|| {
            format!(
                "no temporary area known for MSB {} LSB {} — name one with --engine",
                o.msb, o.lsb
            )
        })?;
    eprintln!(
        "reading {}/{} from the {area:?} area, zone {}, {:?} per sound",
        o.msb,
        o.lsb,
        o.zone + 1,
        o.delay
    );

    let mut fantom = Session::open(o.port.as_deref())?;

    let zone = zone_block(o.zone);
    let name_at = area.name_addr(o.zone);
    let mut rows: Vec<(u8, String)> = Vec::new();
    let mut repeated = 0;

    for pc in o.first..=o.last {
        fantom.send(&dt1(zone, &[o.msb, o.lsb, pc]))?;
        std::thread::sleep(o.delay);
        let name = match fantom.read_name(name_at) {
            Ok(name) => name,
            Err(Unanswered::Silence) => {
                eprintln!("PC {pc}: no answer; try a longer --delay");
                continue;
            }
            Err(short) => {
                eprintln!("PC {pc}: {short}");
                continue;
            }
        };

        // A bank ends where the instrument stops changing its answer: an empty slot leaves the
        // last sound in place, so a run of identical names is the end rather than the content.
        if rows.last().map(|(_, last)| last == &name).unwrap_or(false) {
            repeated += 1;
            if repeated >= o.repeats {
                eprintln!("stopping at PC {pc}: {repeated} × {name:?}");
                rows.truncate(rows.len() - (repeated - 1));
                break;
            }
        } else {
            repeated = 0;
        }
        rows.push((pc, name));
    }

    println!("msb\tlsb\tpc\tnumber\tname\tcategory");
    for (pc, name) in &rows {
        // The panel numbers a bank from one; the wire counts programs from zero.
        println!("{}\t{}\t{}\t{}\t{name}\t", o.msb, o.lsb, pc + 1, pc + 1);
    }
    eprintln!(
        "{} sounds. The zone is left holding the last one; nothing is stored.",
        rows.len()
    );
    Ok(())
}
