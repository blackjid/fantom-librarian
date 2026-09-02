//! Filling in what a catalog was written too early to know.
//!
//! Detail is stored as JSON so its shape can grow without a migration, and a field added later
//! reads back as `None` on every row written before it. Re-importing the source fixes that, but a
//! workspace keeps the file each asset came from, so the answer is already here — this reads it out
//! rather than asking the user to import their library again.

use std::collections::HashMap;

use fantom_core::codec;
use fantom_core::container::Raw;

use crate::model::AssetDetail;
use crate::workspace::Workspace;
use crate::Result;

/// Give every `MODEL` and `ACB` tone the model selector its record carries.
///
/// Returns how many assets were filled in. Cheap when there is nothing to do: one query that
/// matches no rows, and no file is opened.
pub fn model_ids(ws: &Workspace) -> Result<usize> {
    let db = ws.db();
    let mut stmt = db.prepare(
        "SELECT DISTINCT a.id, a.detail, f.stored_path
           FROM assets a
           JOIN occurrences o ON o.asset_id = a.id
           JOIN files f ON f.id = o.file_id
          WHERE a.kind = 'tone'
            AND json_extract(a.detail, '$.area') IN ('MDLa', 'ACBa')
            AND json_extract(a.detail, '$.model_id') IS NULL",
    )?;
    let candidates: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    // One file holds many of these; read each at most once.
    let mut selectors: HashMap<String, HashMap<(String, usize), u32>> = HashMap::new();
    let mut filled = 0;
    for (id, detail, stored_path) in candidates {
        let Ok(AssetDetail::Tone(mut tone)) = serde_json::from_str::<AssetDetail>(&detail) else {
            continue;
        };
        let found = selectors.entry(stored_path.clone()).or_insert_with(|| {
            let mut map = HashMap::new();
            if let Ok(bytes) = std::fs::read(ws.resolve(&stored_path)) {
                let raw = Raw::from_bytes(bytes);
                for bundled in codec::read_bundled_tones(&raw).unwrap_or_default() {
                    if let Some(model_id) = bundled.model_id {
                        let area = String::from_utf8_lossy(&bundled.area).to_string();
                        map.insert((area, bundled.index), model_id);
                    }
                }
            }
            map
        });
        let Some(&model_id) = found.get(&(tone.area.clone(), tone.index)) else {
            continue;
        };
        tone.model_id = Some(model_id);
        // A detail that will not serialise is one this process just built; skip it rather than
        // fail a whole workspace open over one row.
        let Ok(json) = serde_json::to_string(&AssetDetail::Tone(tone)) else {
            continue;
        };
        db.execute("UPDATE assets SET detail = ?1 WHERE id = ?2", (json, id))?;
        filled += 1;
    }
    Ok(filled)
}
