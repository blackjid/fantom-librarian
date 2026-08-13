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
        assert_eq!(
            t.render(),
            "zone  tone\n   1  Africa Brass\n  16  Sax\n"
        );
    }

    /// A short row must not panic or misalign the columns after it.
    #[test]
    fn a_short_row_is_padded() {
        let mut t = Table::new(vec![("a", Align::Left), ("b", Align::Left)]);
        t.row(vec!["x".into()]);
        assert_eq!(t.render(), "a  b\nx\n");
    }
}
