//! Filling in what a catalog was written too early to know.
//!
//! Detail is stored as JSON so its shape can grow without a migration, and a field added later
//! reads back as `None` on every row written before it. Re-importing the source fixes that, but a
//! workspace keeps the file each asset came from, so the answer is already here — this reads it out
//! rather than asking the user to import their library again.

use std::collections::HashMap;

use fantom_core::codec;
use fantom_core::container::Raw;
use fantom_core::requirements::Reader;

use crate::model::AssetDetail;
use crate::workspace::Workspace;
use crate::Result;

/// What the bundled sound lists could name when this build was made.
///
/// A scene's zone names are decided at import and stored, so teaching the lists a new address does
/// nothing for a catalog that already exists. Bumping this is what sends [`scene_names`] over it
/// once — and only once, however many times the workspace is opened afterwards.
///
/// 1. Drum kits, SN-A, V-Piano and ACB from the base sound list; every bundled expansion catalog.
/// 2. The four sound-list rows a page break, a wrapped name and a numeric name had hidden from
///    the extractor — `TR-808`, `FUTURE BRASS`, `1981 Hammer Lead`, `2080`.
/// 3. Every expansion a FANTOM-6 could be asked for: all fifteen `EXZ`, `EXSN04`, the VTW presets,
///    and the MODEL banks under their panel-confirmed labels.
pub const NAMING_REVISION: u32 = 3;

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

/// Re-read every catalogued scene and give its zones the names this build can resolve.
///
/// Returns how many scenes changed. Costs nothing at all until [`NAMING_REVISION`] moves: the
/// workspace records the revision it was last brought to, and a match returns before any query
/// runs. When it does move, each source file is opened once, however many of its scenes are
/// catalogued.
///
/// The whole detail is rebuilt rather than patched, because every part of it that names a tone —
/// the zone table, the user-tone list, the external requirements — is derived from the same
/// addresses. A scene whose file has gone missing keeps what it has.
pub fn scene_names(ws: &Workspace) -> Result<usize> {
    let db = ws.db();
    if revision(ws)? == Some(NAMING_REVISION) {
        return Ok(0);
    }

    let mut stmt = db.prepare(
        "SELECT f.stored_path, o.slot, a.id, a.detail
           FROM assets a
           JOIN occurrences o ON o.asset_id = a.id
           JOIN files f ON f.id = o.file_id
          WHERE a.kind = 'scene'
            AND a.origin = 'user'
          GROUP BY a.id",
    )?;
    // One entry per file, so it is read, framed and its area table parsed once however many of
    // its scenes are catalogued.
    let mut by_file: HashMap<String, Vec<(i64, i64, String)>> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            (row.get(1)?, row.get(2)?, row.get(3)?),
        ))
    })?;
    for row in rows {
        let (stored_path, scene) = row?;
        by_file.entry(stored_path).or_default().push(scene);
    }
    drop(stmt);

    let mut renamed = 0;
    for (stored_path, catalogued) in by_file {
        let Ok(bytes) = std::fs::read(ws.resolve(&stored_path)) else {
            continue; // A scene whose file has gone missing keeps what it has.
        };
        let raw = Raw::from_bytes(bytes);
        let Ok(scenes) = codec::read_scenes(&raw) else {
            continue;
        };
        let reader = Reader::open(&raw).ok();
        for (slot, id, detail) in catalogued {
            // `slot` is the 1-based scene number the import recorded.
            let Some(scene) = usize::try_from(slot).ok().and_then(|n| scenes.get(n - 1)) else {
                continue;
            };
            let rebuilt = AssetDetail::Scene(crate::import::scene_detail(scene, reader.as_ref()));
            // A detail that will not serialise is one this process just built; skip it rather
            // than fail a whole workspace open over one row.
            let Ok(json) = serde_json::to_string(&rebuilt) else {
                continue;
            };
            if json == detail {
                continue;
            }
            db.execute("UPDATE assets SET detail = ?1 WHERE id = ?2", (json, id))?;
            renamed += 1;
        }
    }

    db.execute(
        "INSERT INTO meta (key, value) VALUES ('naming', ?1)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        [NAMING_REVISION.to_string()],
    )?;
    Ok(renamed)
}

/// The naming revision this workspace was last brought to, or nothing if it never has been.
fn revision(ws: &Workspace) -> Result<Option<u32>> {
    let found: Option<String> = ws
        .db()
        .query_row("SELECT value FROM meta WHERE key = 'naming'", [], |row| {
            row.get(0)
        })
        .ok();
    Ok(found.and_then(|value| value.parse().ok()))
}
