//! Reading and editing ACB (`ACBa`) tone parameters by name.
//!
//! A record is 2419 u32 LE words described by [`params::acb`]. Editing happens on a [`Raw`] in
//! place; call [`Raw::save`] to persist. Layout in `docs/FORMAT.md`.

use std::fmt;

use crate::container::{patch_record, Raw, RecordTable, Svd};
use crate::params::acb::{ACB, RECORD_LEN};
use crate::params::{AcbKind, AcbParam};
use crate::{Error, Result};

const TAG: &[u8; 4] = b"ACBa";

/// A parameter's decoded value.
#[derive(Debug, Clone, PartialEq)]
pub enum AcbValue {
    Int(i64),
    Float(f32),
    Text(String),
}

impl fmt::Display for AcbValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v:.4}"),
            Self::Text(v) => write!(f, "{v:?}"),
        }
    }
}

/// One parameter of a record: its table entry, stored word, and decoded value.
#[derive(Debug)]
pub struct AcbField {
    pub param: &'static AcbParam,
    pub word: u32,
    pub value: AcbValue,
}

/// Every parameter of ACB record `index`, in record order.
pub fn read_acb_params(raw: &Raw, index: usize) -> Result<Vec<AcbField>> {
    let svd = Svd::parse(raw)?;
    let table = table(raw, &svd)?;
    let record = record(&table, index)?;
    Ok(ACB
        .iter()
        .map(|param| {
            let word = word(record, param);
            AcbField {
                param,
                word,
                value: decode(param, word),
            }
        })
        .collect())
}

/// Set parameter `name` of ACB record `index` from text, checked against its range.
///
/// Integer parameters take a number, text words up to four printable characters. Float mirrors
/// cannot be set directly: setting their source moves them. The per-record CRC-32 an `.svz` area
/// carries is refreshed; an SVD area has none.
pub fn set_acb_param(raw: &mut Raw, index: usize, name: &str, value: &str) -> Result<()> {
    let param = find(name)?;
    let new = encode(param, value)?;
    let mirror = param
        .mirror
        .map(|offset| (offset, (new as i64 as f32 / 255.0).to_bits()));

    {
        let svd = Svd::parse(raw)?;
        record(&table(raw, &svd)?, index)?;
    }
    let new_bytes = new.to_le_bytes();
    let mirror_bytes = mirror.map(|(offset, bits)| (offset as usize, bits.to_le_bytes()));
    let mut patches: Vec<(usize, &[u8])> = vec![(param.offset as usize, &new_bytes)];
    if let Some((offset, bytes)) = &mirror_bytes {
        patches.push((*offset, bytes));
    }
    patch_record(raw, TAG, index, &patches)
}

fn table<'a>(raw: &'a Raw, svd: &Svd) -> Result<RecordTable<'a>> {
    RecordTable::from_svd(raw, svd, TAG)?
        .ok_or_else(|| Error::Unrecognized("no ACBa area in file".into()))
}

fn record<'a>(table: &RecordTable<'a>, index: usize) -> Result<&'a [u8]> {
    let record = table.record(index).ok_or_else(|| {
        Error::Unrecognized(format!(
            "ACB tone {index} out of range (file has {})",
            table.len()
        ))
    })?;
    if record.len() != RECORD_LEN {
        return Err(Error::Unrecognized(format!(
            "ACBa record size is {}, expected {RECORD_LEN}",
            record.len()
        )));
    }
    Ok(record)
}

fn find(name: &str) -> Result<&'static AcbParam> {
    if let Some(param) = ACB.iter().find(|p| p.id == name) {
        return Ok(param);
    }
    let needle = name.to_ascii_lowercase();
    let near: Vec<&str> = ACB
        .iter()
        .filter(|p| !needle.is_empty() && p.id.to_ascii_lowercase().contains(&needle))
        .map(|p| p.id)
        .take(8)
        .collect();
    Err(Error::Unrecognized(if near.is_empty() {
        format!("no ACB parameter {name:?}")
    } else {
        format!("no ACB parameter {name:?}; similar: {}", near.join(", "))
    }))
}

fn word(record: &[u8], param: &AcbParam) -> u32 {
    let at = param.offset as usize;
    u32::from_le_bytes(record[at..at + 4].try_into().unwrap())
}

fn decode(param: &AcbParam, word: u32) -> AcbValue {
    match param.kind {
        AcbKind::Unsigned => AcbValue::Int(word as i64),
        AcbKind::Signed | AcbKind::Id => AcbValue::Int(word as i32 as i64),
        AcbKind::Float => AcbValue::Float(f32::from_bits(word)),
        AcbKind::Text => AcbValue::Text(
            word.to_le_bytes()
                .iter()
                .rev()
                .map(|&b| if b == 0 { '.' } else { b as char })
                .collect(),
        ),
    }
}

/// The stored word for `text`, or why it is not a valid value of `param`.
fn encode(param: &AcbParam, text: &str) -> Result<u32> {
    let id = param.id;
    match param.kind {
        AcbKind::Float => {
            let source = ACB.iter().find(|p| p.mirror == Some(param.offset));
            Err(Error::Unrecognized(match source {
                Some(s) => format!("{id} is derived from {}; set that instead", s.id),
                None => format!("{id} cannot be set directly"),
            }))
        }
        AcbKind::Text => {
            if text.len() > 4 || !text.chars().all(|c| (' '..='~').contains(&c)) {
                return Err(Error::Unrecognized(format!(
                    "{id} takes up to four printable ASCII characters, got {text:?}"
                )));
            }
            let mut chars = [b' '; 4];
            chars[..text.len()].copy_from_slice(text.as_bytes());
            chars.reverse();
            Ok(u32::from_le_bytes(chars))
        }
        AcbKind::Unsigned | AcbKind::Signed | AcbKind::Id => {
            let v: i64 = text
                .trim()
                .parse()
                .map_err(|_| Error::Unrecognized(format!("{id}: {text:?} is not an integer")))?;
            if param.kind != AcbKind::Id && !(param.min..=param.max).contains(&v) {
                return Err(Error::Unrecognized(format!(
                    "{id}: {v} is outside {}..={}",
                    param.min, param.max
                )));
            }
            if !(i32::MIN as i64..=u32::MAX as i64).contains(&v) {
                return Err(Error::Unrecognized(format!(
                    "{id}: {v} does not fit a word"
                )));
            }
            Ok(v as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::crc32;
    use crate::container::PREAMBLE_LEN;

    /// An SVZ whose `ACBa` area holds one zeroed record and a CRC-32 info word.
    fn svz(with_crc: bool) -> Raw {
        let info_len = 16 + if with_crc { 4 } else { 0 };
        let record = vec![0u8; RECORD_LEN];
        let mut body = Vec::new();
        body.extend_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&(RECORD_LEN as u32).to_le_bytes());
        body.extend_from_slice(&(info_len as u32).to_le_bytes());
        body.extend_from_slice(&[0u8; 4]);
        if with_crc {
            body.extend_from_slice(&crc32(&record).to_le_bytes());
        }
        body.extend_from_slice(&record);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"SVZa");
        bytes.push(1);
        bytes.push(3);
        bytes.extend_from_slice(b"KY019$");
        bytes.extend_from_slice(&[0u8; 4]);
        bytes.extend_from_slice(b"ACBa");
        bytes.extend_from_slice(b"ZCOR");
        bytes.extend_from_slice(&((PREAMBLE_LEN + 16) as u32).to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&body);
        Raw::from_bytes(bytes)
    }

    fn get(raw: &Raw, name: &str) -> AcbValue {
        let fields = read_acb_params(raw, 0).unwrap();
        fields
            .into_iter()
            .find(|f| f.param.id == name)
            .unwrap()
            .value
    }

    #[test]
    fn table_covers_the_record() {
        assert_eq!(ACB.len(), 2419);
        assert_eq!(ACB[0].offset, 0x10);
        assert!(ACB.windows(2).all(|w| w[1].offset == w[0].offset + 4));
    }

    #[test]
    fn a_value_round_trips() {
        let mut raw = svz(false);
        set_acb_param(&mut raw, 0, "OSC1_wave", "2").unwrap();
        assert_eq!(get(&raw, "OSC1_wave"), AcbValue::Int(2));
        set_acb_param(&mut raw, 0, "SYS_COM_startup_mem", "-5").unwrap();
        assert_eq!(get(&raw, "SYS_COM_startup_mem"), AcbValue::Int(-5));
    }

    #[test]
    fn a_value_out_of_range_is_refused_and_leaves_the_file_alone() {
        let mut raw = svz(false);
        let before = raw.bytes().to_vec();
        for bad in ["256", "-1", "x"] {
            assert!(set_acb_param(&mut raw, 0, "LFO_rate", bad).is_err());
        }
        assert_eq!(raw.bytes(), before);
    }

    #[test]
    fn setting_an_integer_moves_its_float_mirror() {
        let mut raw = svz(false);
        set_acb_param(&mut raw, 0, "LFO_rate", "255").unwrap();
        assert_eq!(get(&raw, "PAT2_LFO_rate"), AcbValue::Float(1.0));
        set_acb_param(&mut raw, 0, "FLT_lpf", "51").unwrap();
        assert_eq!(get(&raw, "PAT2_FLT_lpf"), AcbValue::Float(0.2));
    }

    #[test]
    fn a_mirror_cannot_be_set_directly() {
        let err = set_acb_param(&mut svz(false), 0, "PAT2_LFO_rate", "1").unwrap_err();
        assert!(err.to_string().contains("LFO_rate"), "{err}");
    }

    #[test]
    fn an_unknown_name_lists_near_matches() {
        let err = set_acb_param(&mut svz(false), 0, "lfo_ra", "1").unwrap_err();
        assert!(err.to_string().contains("LFO_rate"), "{err}");
    }

    #[test]
    fn a_missing_record_is_refused() {
        assert!(set_acb_param(&mut svz(false), 1, "OSC1_wave", "1").is_err());
    }

    #[test]
    fn text_words_are_stored_byte_reversed() {
        let mut raw = svz(false);
        set_acb_param(&mut raw, 0, "PAT_NAME__000", "Saw").unwrap();
        assert_eq!(get(&raw, "PAT_NAME__000"), AcbValue::Text("Saw ".into()));
        assert!(set_acb_param(&mut raw, 0, "PAT_NAME__000", "toolong").is_err());
    }

    #[test]
    fn an_edit_refreshes_the_record_checksum() {
        let mut raw = svz(true);
        set_acb_param(&mut raw, 0, "LFO_rate", "10").unwrap();
        let report = crate::verify::check(&raw).unwrap();
        assert_eq!(report.checked, 1);
        assert!(report.is_ok(), "{:?}", report.problems);
    }
}
