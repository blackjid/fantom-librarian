//! Scene chains (setlists): `SCENE_CHAIN/ChainSet<n>.xml` beside `FANTOM.SVD`. Read-only.
//!
//! Each file names a chain and lists its steps as 0-based scene indices, with optional markers.
//! `ChainSetEmpty.xml` and `ChainSetTmp.xml` are working files, not numbered chains.

use std::path::Path;

use crate::{Error, Result};

/// Folder holding the chain files, relative to the folder holding `FANTOM.SVD`.
pub const DIR: &str = "SCENE_CHAIN";

/// A label on one step of a chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    /// 0-based step the marker sits on.
    pub step: usize,
    /// 0..=61, label `0-9`, `A-Z`, `a-z`.
    pub name_index: u8,
}

impl Marker {
    pub fn label(&self) -> Option<char> {
        let i = self.name_index;
        match i {
            0..=9 => Some(char::from(b'0' + i)),
            10..=35 => Some(char::from(b'A' + i - 10)),
            36..=61 => Some(char::from(b'a' + i - 36)),
            _ => None,
        }
    }
}

/// One `ChainSet*.xml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    /// File name part after `ChainSet`: `"1"`, `"Empty"`, `"Tmp"`.
    pub file: String,
    pub name: String,
    /// 0-based scene indices, in play order.
    pub scenes: Vec<usize>,
    pub markers: Vec<Marker>,
}

impl Chain {
    /// The chain number; `None` for the Empty and Tmp files.
    pub fn number(&self) -> Option<u32> {
        self.file.parse().ok()
    }
}

/// Parses one chain file; `file` is the part of its name after `ChainSet`.
pub fn parse(file: &str, xml: &str) -> Result<Chain> {
    let bad = |why: String| Error::Unrecognized(format!("ChainSet{file}.xml: {why}"));
    if !xml.contains("<SCENE_CHAIN_SET") {
        return Err(bad("no SCENE_CHAIN_SET".into()));
    }
    let mut name = None;
    let mut steps: Vec<(usize, usize)> = Vec::new();
    let mut markers: Vec<(usize, Marker)> = Vec::new();
    for tag in xml.split('<').skip(1) {
        let tag = tag.split('>').next().unwrap_or("");
        let element = tag.split_whitespace().next().unwrap_or("");
        let number = |prefix: &str| element.strip_prefix(prefix)?.parse::<usize>().ok();
        let attr_num = |key: &str| -> Result<usize> {
            attr(tag, key)
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| bad(format!("{element} lacks a numeric {key}")))
        };
        if element == "CHAIN_SET_NAME" {
            name = attr(tag, "ChainSetName");
        } else if let Some(n) = number("CHAIN") {
            steps.push((n, attr_num("SceneIdx")?));
        } else if let Some(n) = number("MARKER") {
            let name_index = u8::try_from(attr_num("NameIndex")?)
                .map_err(|_| bad(format!("{element} NameIndex out of range")))?;
            let step = attr_num("ChainIdx")?;
            markers.push((n, Marker { step, name_index }));
        }
    }
    steps.sort();
    markers.sort_by_key(|(n, _)| *n);
    Ok(Chain {
        file: file.to_string(),
        name: name.ok_or_else(|| bad("no CHAIN_SET_NAME".into()))?,
        scenes: steps.into_iter().map(|(_, s)| s).collect(),
        markers: markers.into_iter().map(|(_, m)| m).collect(),
    })
}

/// The value of `key="..."` in a tag body, with XML entities decoded.
fn attr(tag: &str, key: &str) -> Option<String> {
    let start = tag.find(&format!("{key}=\""))? + key.len() + 2;
    let value = &tag[start..];
    let value = &value[..value.find('"')?];
    Some(
        value
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
    )
}

/// Every chain file under `backup/SCENE_CHAIN`: numbered chains in order, then Empty, then Tmp.
/// A backup without the folder has none.
pub fn read_dir(backup: &Path) -> Result<Vec<Chain>> {
    let dir = backup.join(DIR);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let file_err = |action, path: &Path, source| Error::File {
        action,
        path: path.display().to_string(),
        source,
    };
    let mut chains = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| file_err("read", &dir, e))? {
        let path = entry.map_err(|e| file_err("read", &dir, e))?.path();
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("ChainSet")?.strip_suffix(".xml"));
        let Some(file) = file else { continue };
        let xml = std::fs::read_to_string(&path).map_err(|e| file_err("read", &path, e))?;
        chains.push(parse(file, &xml)?);
    }
    chains.sort_by_key(|c| (c.number().is_none(), c.number(), c.file.clone()));
    Ok(chains)
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>

<SCENE_CHAIN_SET>
  <CHAIN_SET_NAME ChainSetName="flesh &amp; blood"/>
  <CHAIN1 SceneIdx="468"/>
  <CHAIN2 SceneIdx="449"/>
  <MARKER1 ChainIdx="0" NameIndex="1"/>
  <MARKER2 ChainIdx="1" NameIndex="36"/>
</SCENE_CHAIN_SET>
"#;

    #[test]
    fn parses_name_steps_and_markers() {
        let chain = parse("7", XML).unwrap();
        assert_eq!(chain.number(), Some(7));
        assert_eq!(chain.name, "flesh & blood");
        assert_eq!(chain.scenes, [468, 449]);
        assert_eq!(chain.markers.len(), 2);
        assert_eq!(chain.markers[0].label(), Some('1'));
        assert_eq!(chain.markers[1].step, 1);
        assert_eq!(chain.markers[1].label(), Some('a'));
    }

    #[test]
    fn an_empty_chain_has_no_steps() {
        let xml = "<SCENE_CHAIN_SET>\n<CHAIN_SET_NAME ChainSetName=\"\"/>\n</SCENE_CHAIN_SET>";
        let chain = parse("Empty", xml).unwrap();
        assert_eq!(chain.number(), None);
        assert!(chain.scenes.is_empty());
    }

    #[test]
    fn rejects_other_xml() {
        assert!(parse("1", "<OTHER/>").is_err());
        assert!(parse(
            "1",
            "<SCENE_CHAIN_SET><CHAIN1 SceneIdx=\"x\"/></SCENE_CHAIN_SET>"
        )
        .is_err());
    }
}
