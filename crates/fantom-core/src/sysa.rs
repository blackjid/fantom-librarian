//! Reading the system settings (`SYSa`) by name; read-only.
//!
//! One 904-byte record described by [`params::sysa`]. Layout in `docs/FORMAT.md`.

use crate::container::{Raw, RecordTable, Svd};
use crate::params::sysa::{RECORD_LEN, SYSA};
use crate::params::SysParam;
use crate::{Error, Result};

const TAG: &[u8; 4] = b"SYSa";
const REVERB_TYPE: &str = "IN_REV:Reverb Type";

/// The system settings record.
#[derive(Debug, Clone)]
pub struct Sysa {
    record: [u8; RECORD_LEN],
}

/// One setting: its table entry and stored value.
#[derive(Debug)]
pub struct SysField {
    pub param: &'static SysParam,
    pub value: i64,
}

/// One setting (or pad byte) that differs between two files.
#[derive(Debug, PartialEq, Eq)]
pub struct SysChange {
    pub offset: usize,
    /// Setting name; `(pad)` for bytes outside every setting.
    pub name: &'static str,
    pub reserved: bool,
    pub left: String,
    pub right: String,
}

/// The system settings of a backup or scene export.
pub fn read_sysa(raw: &Raw) -> Result<Sysa> {
    let svd = Svd::parse(raw)?;
    let table = RecordTable::from_svd(raw, &svd, TAG)?
        .ok_or_else(|| Error::Unrecognized("no SYSa area in file".into()))?;
    if table.declared_count != 1 || table.record_size != RECORD_LEN {
        return Err(Error::Unrecognized(format!(
            "SYSa is {} x {} bytes, expected 1 x {RECORD_LEN}",
            table.declared_count, table.record_size
        )));
    }
    let record = table
        .record(0)
        .ok_or_else(|| Error::Unrecognized("SYSa area is truncated".into()))?;
    Ok(Sysa {
        record: record.try_into().unwrap(),
    })
}

impl Sysa {
    /// Every setting, in record order.
    pub fn fields(&self) -> Vec<SysField> {
        SYSA.iter()
            .map(|param| SysField {
                param,
                value: self.value(param),
            })
            .collect()
    }

    /// Stored value of setting `id`.
    pub fn get(&self, id: &str) -> Option<i64> {
        SYSA.iter().find(|p| p.id == id).map(|p| self.value(p))
    }

    /// The stored value, little-endian, signed per the table.
    pub fn value(&self, param: &SysParam) -> i64 {
        let at = param.offset as usize;
        let bytes = &self.record[at..at + param.size as usize];
        let unsigned = bytes
            .iter()
            .rev()
            .fold(0i64, |acc, &b| (acc << 8) | b as i64);
        let bits = 8 * param.size as u32;
        if param.signed && unsigned >= 1 << (bits - 1) {
            unsigned - (1 << bits)
        } else {
            unsigned
        }
    }

    /// The value as shown: label, wire value of centred fields, and the reverb parameter's name
    /// under the current reverb type.
    pub fn describe(&self, field: &SysField) -> String {
        let (p, v) = (field.param, field.value);
        if !p.by_type.is_empty() {
            let kind = self.get(REVERB_TYPE).unwrap_or(0);
            if let Some(r) = p.by_type.iter().find(|r| r.reverb_type == kind) {
                return format!("{v}  [{}, {}]", r.name, r.range);
            }
        }
        label(field)
    }

    /// Whether the setting holds its firmware init value (zero where there is none).
    pub fn at_init(&self, field: &SysField) -> bool {
        field.value == field.param.init.unwrap_or(0)
    }

    /// Settings and pad bytes that differ from `other`, in record order, settings first.
    pub fn diff(&self, other: &Sysa) -> Vec<SysChange> {
        let mut out = Vec::new();
        let mut covered = [false; RECORD_LEN];
        for (a, b) in self.fields().iter().zip(other.fields()) {
            let p = a.param;
            covered[p.offset as usize..][..p.size as usize].fill(true);
            if a.value != b.value {
                out.push(SysChange {
                    offset: p.offset as usize,
                    name: p.id,
                    reserved: p.reserved,
                    left: label(a),
                    right: label(&b),
                });
            }
        }
        for i in (0..RECORD_LEN).filter(|&i| !covered[i] && self.record[i] != other.record[i]) {
            out.push(SysChange {
                offset: i,
                name: "(pad)",
                reserved: false,
                left: self.record[i].to_string(),
                right: other.record[i].to_string(),
            });
        }
        out
    }
}

/// The value with its label and wire value, without the reverb type context (as `diff` shows it).
fn label(field: &SysField) -> String {
    let (p, v) = (field.param, field.value);
    let mut s = match p.labels.iter().find(|(k, _)| *k == v) {
        Some((_, label)) => format!("{v} {label}"),
        None => v.to_string(),
    };
    if p.bias != 0 {
        s.push_str(&format!(" (wire {})", v + p.bias as i64));
    }
    if p.id.starts_with("SC:CLOCK_YEAR") {
        s.push_str(&format!(" (20{v:02})"));
    }
    s
}
