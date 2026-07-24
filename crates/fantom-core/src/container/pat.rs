use crate::container::{ascii_trim, Area, Raw};
use crate::{Error, Result};

/// The `PATa` (patch/tone) area: a header plus fixed-stride tone records.
///
/// Same envelope as `PRFa`: a 16-byte header (`count`, `record_size`, `data_start`) then `count`
/// records. Each record's first 16 bytes are the tone name; byte `+0x10` is the tone category.
/// A scene's per-zone `tone_id` indexes this list for user tones (see `docs/FORMAT.md`).
pub struct PatArea {
    tones: Vec<Tone>,
}

/// One tone stored in a `PATa` area.
#[derive(Debug, Clone, PartialEq)]
pub struct Tone {
    pub name: String,
    /// Tone category byte (e.g. `0x23` = brass); meaning of most values still TBD.
    pub category: u8,
}

const HEADER_LEN: usize = 0x10;
const COUNT_OFFSET: usize = 0x00;
const RECORD_SIZE_OFFSET: usize = 0x04;
const NAME_LEN: usize = 16;
const CATEGORY_OFFSET: usize = 0x10;

impl PatArea {
    /// Parse the tone list from a `PATa` area's bytes.
    pub fn parse(area: &[u8]) -> Result<Self> {
        let count = read_u32(area, COUNT_OFFSET)? as usize;
        let record_size = read_u32(area, RECORD_SIZE_OFFSET)? as usize;
        if record_size == 0 {
            return Err(Error::Unrecognized("PATa record size is zero".into()));
        }

        let mut tones = Vec::with_capacity(count);
        for i in 0..count {
            let start = HEADER_LEN + i * record_size;
            let Some(record) = area.get(start..start + record_size) else {
                break; // truncated area — keep what parsed
            };
            tones.push(Tone {
                name: ascii_trim(&record[..NAME_LEN]),
                category: record[CATEGORY_OFFSET],
            });
        }
        Ok(Self { tones })
    }

    /// Convenience: locate and parse the `PATa` area of an SVD file.
    pub fn from_svd(raw: &Raw, svd: &crate::container::Svd) -> Result<Self> {
        let area = svd
            .area(b"PATa")
            .ok_or_else(|| Error::Unrecognized("no PATa (tone) area in file".into()))?;
        Self::parse(svd.area_bytes(raw, area)?)
    }

    /// All tones, in storage order.
    pub fn tones(&self) -> &[Tone] {
        &self.tones
    }

    /// The tone at `index`, if present.
    pub fn get(&self, index: usize) -> Option<&Tone> {
        self.tones.get(index)
    }
}

impl Area {
    /// The `PATa` tag, exposed for callers building tone lookups.
    pub const PATA: &'static [u8; 4] = b"PATa";
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32> {
    let slice = bytes
        .get(at..at + 4)
        .ok_or_else(|| Error::Unrecognized(format!("PATa area truncated at offset {at}")))?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area_with(names: &[(&str, u8)], record_size: usize) -> Vec<u8> {
        let mut a = Vec::new();
        a.extend_from_slice(&(names.len() as u32).to_le_bytes());
        a.extend_from_slice(&(record_size as u32).to_le_bytes());
        a.extend_from_slice(&[0u8; 8]);
        for (name, cat) in names {
            let mut rec = vec![0u8; record_size];
            let b = name.as_bytes();
            rec[..b.len().min(NAME_LEN)].copy_from_slice(&b[..b.len().min(NAME_LEN)]);
            rec[CATEGORY_OFFSET] = *cat;
            a.extend_from_slice(&rec);
        }
        a
    }

    #[test]
    fn parses_tone_names_and_categories() {
        let area = area_with(&[("Africa Brass", 0x23), ("Africa Kalimba", 0x0f)], 64);
        let pat = PatArea::parse(&area).unwrap();
        assert_eq!(pat.tones().len(), 2);
        assert_eq!(pat.get(0).unwrap().name, "Africa Brass");
        assert_eq!(pat.get(0).unwrap().category, 0x23);
        assert_eq!(pat.get(1).unwrap().name, "Africa Kalimba");
        assert!(pat.get(2).is_none());
    }
}
