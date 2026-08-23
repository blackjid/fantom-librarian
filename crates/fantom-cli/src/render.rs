//! Turning model values into the text the terminal shows.
//!
//! Kept apart from the `run_*` commands so that *what* a command reports and *how* a value looks
//! are separate decisions. Anything here is presentation only: it takes typed values and returns
//! strings, never opens a file, and never decides which fields matter.
//!
//! Parameter *values* format themselves — `fantom_core::params::render` knows that Zone Mono/Poly
//! of 2 is `TONE`, from data Roland publishes. What lives here is the rest: labels the domain
//! model implies rather than the parameter table (a tone's name, a MIDI note), and page layout.

use std::fmt::Write as _;

use fantom_core::model::{ToneRef, ToneType, Zone};
use fantom_core::requirements::{Requirements, SlotRequirement};

/// Which way a column's cells sit against its width.
#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Right,
}

/// A text table that sizes its own columns.
///
/// The header and the rows are described once. Before this, `show` carried the same twelve-field
/// format string twice — once for the header, once for each row — and widening a column meant
/// editing both in step with nothing to catch a mismatch.
pub struct Table {
    columns: Vec<(&'static str, Align)>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(columns: Vec<(&'static str, Align)>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
        }
    }

    /// Add a row. Cells are matched to columns by position; a short row is padded out.
    pub fn row(&mut self, cells: Vec<String>) {
        debug_assert!(
            cells.len() <= self.columns.len(),
            "row has more cells than the table has columns"
        );
        self.rows.push(cells);
    }

    /// Render as aligned columns, two spaces apart, with no trailing whitespace on any line.
    pub fn render(&self) -> String {
        let widths: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, (header, _))| {
                self.rows
                    .iter()
                    .filter_map(|r| r.get(i))
                    .map(|c| c.chars().count())
                    .chain(std::iter::once(header.chars().count()))
                    .max()
                    .unwrap_or(0)
            })
            .collect();

        let mut out = String::new();
        let headers: Vec<String> = self.columns.iter().map(|(h, _)| (*h).to_string()).collect();
        for cells in std::iter::once(&headers).chain(self.rows.iter()) {
            let mut line = String::new();
            for (i, (_, align)) in self.columns.iter().enumerate() {
                let cell = cells.get(i).map(String::as_str).unwrap_or("");
                if i > 0 {
                    line.push_str("  ");
                }
                let pad = widths[i].saturating_sub(cell.chars().count());
                match align {
                    Align::Left => {
                        line.push_str(cell);
                        line.push_str(&" ".repeat(pad));
                    }
                    Align::Right => {
                        line.push_str(&" ".repeat(pad));
                        line.push_str(cell);
                    }
                }
            }
            let _ = writeln!(out, "{}", line.trim_end());
        }
        out
    }
}

/// A MIDI note number as a name, e.g. 60 -> `C4` (Roland convention: middle C = C4).
pub fn note(n: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NAMES[(n % 12) as usize], (n / 12) as i16 - 1)
}

/// An inclusive range of two values, as `low..high`.
pub fn range(low: impl std::fmt::Display, high: impl std::fmt::Display) -> String {
    format!("{low}..{high}")
}

/// A zone is off, muted, or on — muted is its own state, since a muted zone still receives.
pub fn zone_state(z: &Zone) -> &'static str {
    match (z.enabled, z.muted) {
        (false, _) => "off",
        (true, true) => "mut",
        (true, false) => "on",
    }
}

/// A signed offset. Zero prints as `0`, never blank: a blank cell reads as "could not be
/// decoded" rather than "no offset", which is why pan prints `C` rather than nothing.
pub fn signed(v: i8) -> String {
    if v == 0 {
        "0".to_string()
    } else {
        format!("{v:+}")
    }
}

/// A zone's tone, named if the file or the sound list can name it.
pub fn tone(tone: &ToneRef) -> String {
    match (tone.preset(), tone.name()) {
        (Some(p), _) => format!("{:04} {}", p.number, p.name),
        (_, Some(name)) => name.to_owned(),
        _ => format!("PC {:03}", tone.address.pc),
    }
}

/// The sound engine, or the raw MSB when the address is one we cannot place.
pub fn tone_type(tone: &ToneRef) -> String {
    match tone.tone_type() {
        ToneType::Unknown => format!("MSB {}", tone.address.msb),
        known => known.label().to_owned(),
    }
}

/// The bank label, or the raw LSB when its mapping is unconfirmed. Showing the number beats
/// inventing a name for a bank we have not identified.
pub fn bank(tone: &ToneRef) -> String {
    tone.bank()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("LSB {}", tone.address.lsb))
}

/// A four-byte area tag as text.
pub fn area_tag(tag: &[u8; 4]) -> String {
    String::from_utf8_lossy(tag).into_owned()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Bytes as text, with anything unprintable shown as `.`.
pub fn ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

/// What a file needs from wherever it is loaded, as the terminal says it.
///
/// One renderer behind every command that has to raise the subject — `check`, and the rebuild
/// commands handing back a bank whose dependencies have just become somebody else's problem. It
/// returns an empty string when nothing is required, so a caller can print it unconditionally.
///
/// The explanations are as long as they are on purpose. A missing sample or an uninstalled
/// expansion produces no error on the instrument: the bank loads and plays the wrong sound, so the
/// only place the user can learn what happened is here, before they carry the file across.
pub fn requirements(needs: &Requirements) -> String {
    let mut out = String::new();

    let missing: Vec<_> = needs.missing_tones().collect();
    if !missing.is_empty() {
        let _ = writeln!(
            out,
            "warning: {} user tone{} {} referenced but not bundled. A zone pointing at an empty\n\
             \x20        slot plays whatever the destination keeps there:",
            missing.len(),
            plural(missing.len()),
            if missing.len() == 1 { "is" } else { "are" },
        );
        for tone in missing {
            let _ = writeln!(
                out,
                "           {}[{}]  MSB {} LSB {} PC {:03}",
                tone.area, tone.index, tone.address.msb, tone.address.lsb, tone.address.pc
            );
        }
    }

    let samples: Vec<_> = needs.missing_samples().collect();
    if !samples.is_empty() {
        let _ = writeln!(
            out,
            "warning: the tones play {} user sample{} this file does not carry — a tone references\n\
             \x20        a sample *slot*, so the audio stays on the instrument. The destination\n\
             \x20        needs these samples in these slots:",
            samples.len(),
            plural(samples.len()),
        );
        for sample in samples {
            let _ = writeln!(out, "           {}", slot_line("slot", sample));
        }
    }

    let multisamples: Vec<_> = needs.multisamples.iter().filter(|s| !s.carried).collect();
    if !multisamples.is_empty() {
        let _ = writeln!(
            out,
            "warning: the tones also play {} user multisample{}. The samples each one maps across\n\
             \x20        the keyboard are in the list above, but the multisample itself cannot\n\
             \x20        travel in a scene bank — the destination must hold it, or you must\n\
             \x20        rebuild it over those slots:",
            multisamples.len(),
            plural(multisamples.len()),
        );
        for multisample in multisamples {
            let _ = writeln!(out, "           {}", slot_line("multisample", multisample));
        }
    }

    if needs.needs_installed_content() {
        let _ = writeln!(
            out,
            "note: this material also plays content that lives in the instrument rather than in\n\
             \x20     any file, and is never substituted. The destination must already have it:",
        );
        for bank in needs.expansions() {
            let _ = writeln!(out, "        {}", bank.label());
        }
        for id in &needs.wave_expansions {
            let _ = writeln!(out, "        wave expansion, group id {id}");
        }
        for address in &needs.unclassified {
            let _ = writeln!(
                out,
                "        MSB {} LSB {} PC {:03}  (an address this version cannot classify)",
                address.msb, address.lsb, address.pc
            );
        }
    }

    // The one failure a structural check cannot catch: everything present, nothing audible.
    let silent: Vec<_> = needs.silent_samples().collect();
    if !silent.is_empty() {
        let _ = writeln!(
            out,
            "warning: {} of the sample{} carried here hold{} no audio at all — the source keeps\n\
             \x20        their names and lengths and none of their sound. The instrument may well\n\
             \x20        still play them: a tone the instrument itself exported carried zeros for\n\
             \x20        these too, and nothing in a file marks the difference (see\n\
             \x20        docs/FORMAT.md). Re-import the samples on the instrument and take a\n\
             \x20        fresh backup, or they cannot travel:",
            silent.len(),
            plural(needs.samples.len()),
            if silent.len() == 1 { "s" } else { "" },
        );
        for sample in silent {
            let _ = writeln!(out, "           {}", slot_line("slot", sample));
        }
    }

    // Content the file brings with it is not a requirement of the destination, but it is still the
    // reason the file is 90 MB, and a transfer is checked against what it says it carries.
    let mut carried = Vec::new();
    let samples = needs.samples.len() - needs.missing_samples().count();
    if samples > 0 {
        carried.push(format!("{samples} user sample{}", plural(samples)));
    }
    let multisamples = needs
        .multisamples
        .iter()
        .filter(|slot| slot.carried)
        .count();
    if multisamples > 0 {
        carried.push(format!(
            "{multisamples} multisample{}",
            plural(multisamples)
        ));
    }
    if !carried.is_empty() {
        let _ = writeln!(out, "note: carries the {} it plays.", carried.join(" and "));
    }

    // Worth one line and no more: a factory sound is a dependency, but not one anybody has to act
    // on — every FANTOM has it, so listing them would bury the ones that need installing.
    let factory = needs.banks.len() - needs.expansions().count();
    if factory > 0 {
        let _ = writeln!(
            out,
            "note: also plays {factory} factory sound{}, which every FANTOM has.",
            plural(factory)
        );
    }

    out
}

/// One slot requirement: its number, what is in it, and which sound goes quiet without it.
fn slot_line(kind: &str, slot: &SlotRequirement) -> String {
    let name = slot.name.as_deref().unwrap_or("<not in this file>");
    let played_by = match slot.played_by.as_slice() {
        [] => String::new(),
        players => format!(" (played by {})", quoted(players)),
    };
    format!("{kind} {:>3}  {name:<20}{played_by}", slot.slot)
}

fn quoted(names: &[String]) -> String {
    names
        .iter()
        .map(|name| format!("{name:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The `s` of a plural count. Every report here counts something.
pub fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod requirement_tests {
    use super::*;
    use fantom_core::model::{ToneAddress, ToneType};
    use fantom_core::requirements::{BankRequirement, ToneRequirement};

    /// A sample that is present, named, full-length and silent is the one failure every other
    /// check passes.
    #[test]
    fn audio_that_is_silence_is_called_out_by_name() {
        let needs = Requirements {
            samples: vec![
                SlotRequirement {
                    slot: 30,
                    name: Some("Sledge 1".into()),
                    carried: true,
                    silent: true,
                    played_by: vec!["Sledge + Hammer".into()],
                },
                SlotRequirement {
                    slot: 55,
                    name: Some("upiano1_55_a3".into()),
                    carried: true,
                    silent: false,
                    played_by: Vec::new(),
                },
            ],
            ..Requirements::default()
        };
        let report = requirements(&needs);
        assert!(report.contains("no audio at all"), "{report}");
        assert!(report.contains("exported carried zeros"), "{report}");
        assert!(report.contains("slot  30  Sledge 1"), "{report}");
        assert!(!report.contains("upiano1_55_a3"), "{report}");
        // It still says what it carries; the warning is about quality, not presence.
        assert!(report.contains("carries the 2 user samples"), "{report}");
    }

    #[test]
    fn a_file_that_needs_nothing_prints_nothing() {
        assert_eq!(requirements(&Requirements::default()), "");
    }

    /// A factory sound is a dependency nobody has to act on, so it gets a count and not a list.
    #[test]
    fn factory_sounds_are_counted_while_expansions_are_named() {
        let bank = |label: &str| BankRequirement {
            engine: ToneType::ZenCore,
            bank: Some(label.into()),
            tone: None,
            address: ToneAddress {
                msb: 87,
                lsb: 64,
                pc: 0,
            },
        };
        let needs = Requirements {
            banks: vec![bank("PR-A"), bank("PR-B"), bank("EXZ007")],
            ..Requirements::default()
        };
        let report = requirements(&needs);
        assert!(report.contains("ZEN-Core EXZ007 PC 000"));
        assert!(report.contains("also plays 2 factory sounds"));
        assert!(!report.contains("PR-A"));
    }

    /// Carried audio is not a requirement: an `.svz` brings its samples with it.
    #[test]
    fn only_what_the_file_cannot_supply_is_reported() {
        let needs = Requirements {
            samples: vec![
                SlotRequirement {
                    slot: 1,
                    name: Some("Beat It Gong".into()),
                    carried: true,
                    silent: false,
                    played_by: vec!["Beat It".into()],
                },
                SlotRequirement {
                    slot: 22,
                    name: None,
                    carried: false,
                    silent: false,
                    played_by: vec!["Beat It".into()],
                },
            ],
            ..Requirements::default()
        };
        let report = requirements(&needs);
        assert!(report.contains("1 user sample this file does not carry"));
        assert!(report.contains("slot  22  <not in this file>   (played by \"Beat It\")"));
        assert!(!report.contains("Beat It Gong"));
    }

    #[test]
    fn a_missing_tone_and_an_expansion_are_both_named() {
        let needs = Requirements {
            user_tones: vec![ToneRequirement {
                area: "PATa".into(),
                index: 443,
                engine: ToneType::ZenCore,
                name: None,
                address: ToneAddress {
                    msb: 87,
                    lsb: 3,
                    pc: 59,
                },
                present: false,
            }],
            banks: vec![BankRequirement {
                engine: ToneType::Exz,
                bank: Some("EXZ007".into()),
                tone: None,
                address: ToneAddress {
                    msb: 93,
                    lsb: 7,
                    pc: 0,
                },
            }],
            wave_expansions: vec![1005],
            ..Requirements::default()
        };
        let report = requirements(&needs);
        assert!(report.contains("PATa[443]  MSB 87 LSB 3 PC 059"));
        assert!(report.contains("EXZ EXZ007 PC 000"));
        assert!(report.contains("wave expansion, group id 1005"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_follow_the_roland_octave_convention() {
        assert_eq!(note(60), "C4");
        assert_eq!(note(0), "C-1");
        assert_eq!(note(127), "G9");
    }

    #[test]
    fn signed_offsets_always_print_a_number() {
        assert_eq!(signed(0), "0");
        assert_eq!(signed(-24), "-24");
        assert_eq!(signed(12), "+12");
    }

    #[test]
    fn a_table_sizes_columns_to_the_widest_cell() {
        let mut t = Table::new(vec![("zone", Align::Right), ("tone", Align::Left)]);
        t.row(vec!["1".into(), "Africa Brass".into()]);
        t.row(vec!["16".into(), "Sax".into()]);
        assert_eq!(t.render(), "zone  tone\n   1  Africa Brass\n  16  Sax\n");
    }

    /// A short row must not panic or misalign the columns after it.
    #[test]
    fn a_short_row_is_padded() {
        let mut t = Table::new(vec![("a", Align::Left), ("b", Align::Left)]);
        t.row(vec!["x".into()]);
        assert_eq!(t.render(), "a  b\nx\n");
    }
}
