//! Reading and editing the catalog: what the library shows, and the handful of things v1 lets a
//! user change about it.

use std::collections::{HashMap, HashSet};

use fantom_core::expansions::{self, Family};
use fantom_core::model::ToneType;
use rusqlite::{params, params_from_iter, Row};

use crate::facet;
use crate::model::*;
use crate::workspace::Workspace;
use crate::{now, Error, Result};

/// The joins, conditions, and bound values one [`Query`] turns into.
///
/// Built once and shared by the row query and the count query, so the number above the list can
/// never disagree with the list.
#[derive(Default)]
struct Filter {
    joins: String,
    wheres: Vec<String>,
    binds: Vec<Box<dyn rusqlite::ToSql>>,
}

impl Filter {
    /// `with_kind` is false for the counts, which group by kind rather than select one.
    fn build(query: &Query, with_kind: bool) -> Self {
        let mut f = Self::default();

        // One join serves both scopes; a file id is just a narrower cut of the same relation.
        if query.source_id.is_some() || query.file_id.is_some() {
            f.joins.push_str(
                " JOIN occurrences o ON o.asset_id = a.id JOIN files f ON f.id = o.file_id",
            );
            if query.source_id.is_some() {
                f.push("f.source_id", Box::new(query.source_id));
            }
            if query.file_id.is_some() {
                f.push("f.id", Box::new(query.file_id));
            }
        }
        if query.song_id.is_some() {
            f.joins
                .push_str(" JOIN song_assets sa ON sa.asset_id = a.id");
            f.push("sa.song_id", Box::new(query.song_id));
        }
        if with_kind {
            if let Some(kind) = query.kind {
                f.push("a.kind", Box::new(kind.as_str().to_string()));
            }
        }
        let search = query.search.trim();
        if !search.is_empty() {
            let n = f.binds.len() + 1;
            f.wheres.push(format!(
                "(a.fantom_name LIKE ?{n} ESCAPE '\\' OR a.imported_name LIKE ?{n} ESCAPE '\\'
                  OR a.note LIKE ?{n} ESCAPE '\\' OR a.memo LIKE ?{n} ESCAPE '\\')"
            ));
            f.binds.push(Box::new(format!("%{}%", escape_like(search))));
        }
        // Every listed tag must be present, so each one gets its own EXISTS rather than an IN.
        for tag in query.tags.iter().filter(|t| !t.trim().is_empty()) {
            let n = f.binds.len() + 1;
            f.wheres.push(format!(
                "EXISTS (SELECT 1 FROM asset_tags at JOIN tags t ON t.id = at.tag_id
                          WHERE at.asset_id = a.id AND t.name = ?{n} COLLATE NOCASE)"
            ));
            f.binds.push(Box::new(tag.trim().to_string()));
        }
        if !query.include_archived {
            f.wheres.push("a.archived_at IS NULL".into());
        }
        f
    }

    fn push(&mut self, column: &str, value: Box<dyn rusqlite::ToSql>) {
        self.wheres
            .push(format!("{column} = ?{}", self.binds.len() + 1));
        self.binds.push(value);
    }

    fn apply(&self, sql: &mut String) {
        sql.push_str(&self.joins);
        if !self.wheres.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.wheres.join(" AND "));
        }
    }
}

/// How many scenes and how many tones the current scope holds, before the kind filter narrows it.
/// This is what the kind filter's own counts show, so they stay meaningful while it is set.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct KindCounts {
    pub scenes: i64,
    pub tones: i64,
}

pub fn counts(ws: &Workspace, query: &Query) -> Result<KindCounts> {
    // A facet is decided from the stored detail rather than in SQL, so when one is set the count
    // has to come from the same rows the list does — see [`crate::facet`].
    if query.narrows_by_facet() {
        let mut out = KindCounts::default();
        for asset in select(
            ws,
            &Query {
                kind: None,
                limit: None,
                ..query.clone()
            },
        )? {
            match asset.kind {
                AssetKind::Scene => out.scenes += 1,
                AssetKind::Tone => out.tones += 1,
            }
        }
        return Ok(out);
    }
    let filter = Filter::build(query, false);
    let mut sql = String::from("SELECT a.kind, COUNT(DISTINCT a.id) FROM assets a");
    filter.apply(&mut sql);
    sql.push_str(" GROUP BY a.kind");

    let db = ws.db();
    let mut stmt = db.prepare(&sql)?;
    let rows = stmt.query_map(
        params_from_iter(filter.binds.iter().map(|b| b.as_ref())),
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )?;

    let mut out = KindCounts::default();
    for row in rows {
        let (kind, count) = row?;
        match AssetKind::parse(&kind) {
            Some(AssetKind::Scene) => out.scenes = count,
            Some(AssetKind::Tone) => out.tones = count,
            None => {}
        }
    }
    Ok(out)
}

/// Assets matching `query`, with their tags and sources.
pub fn assets(ws: &Workspace, query: &Query) -> Result<Vec<Asset>> {
    let mut out = select(ws, query)?;
    for asset in &mut out {
        asset.tags = tags_of(ws, asset.id)?;
        asset.sources = sources_of(ws, asset.id)?;
    }
    Ok(out)
}

/// What the current scope offers to narrow by, and how much of it each value accounts for.
///
/// Counted over the scope with the facets themselves lifted, so choosing one never hides the
/// others — a filter the user cannot see their way out of is a dead end.
pub fn facets(ws: &Workspace, query: &Query) -> Result<Facets> {
    let scope = Query {
        engines: Vec::new(),
        models: Vec::new(),
        origin: None,
        hide_uninstalled_expansions: false,
        limit: None,
        ..query.clone()
    };
    Ok(facet::tally(&select(ws, &scope)?))
}

/// The rows themselves, without the per-asset tag and source queries. Facets are applied here, so
/// nothing downstream can see a row the query excluded.
fn select(ws: &Workspace, query: &Query) -> Result<Vec<Asset>> {
    let filter = Filter::build(query, true);
    let mut sql = String::from(
        "SELECT DISTINCT a.id, a.kind, a.fantom_name, a.imported_name, a.note, a.memo,
                a.engine, a.detail, a.created_at, a.archived_at, a.origin
           FROM assets a",
    );
    filter.apply(&mut sql);
    sql.push_str(" ORDER BY a.fantom_name COLLATE NOCASE, a.id");
    // A facet trims rows after SQL has chosen them, so the limit has to wait for it.
    let narrowing = query.narrows_by_facet();
    if let (Some(limit), false) = (query.limit, narrowing) {
        sql.push_str(&format!(" LIMIT {}", limit.max(0)));
    }

    let db = ws.db();
    let binds = &filter.binds;
    let mut stmt = db.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(binds.iter().map(|b| b.as_ref())), |row| {
        Ok(row_to_asset(row))
    })?;

    let mut out = rows.collect::<rusqlite::Result<Vec<Asset>>>()?;
    if narrowing {
        out.retain(|asset| facet::matches(asset, query));
        if query.hide_uninstalled_expansions {
            let installed = installed_expansions(ws)?;
            out.retain(|asset| !facet::needs_uninstalled_expansion(asset, &installed));
        }
        if let Some(limit) = query.limit {
            out.truncate(limit.max(0) as usize);
        }
    }
    Ok(out)
}

fn installed_expansions(ws: &Workspace) -> Result<HashSet<String>> {
    let mut statement = ws
        .db()
        .prepare("SELECT code FROM expansions WHERE installed != 0")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .map(|code| code.to_ascii_uppercase())
        .collect())
}

/// One asset by id, with its tags and every source it was seen in.
pub fn asset(ws: &Workspace, id: i64) -> Result<Asset> {
    let db = ws.db();
    let mut asset = db
        .query_row(
            "SELECT id, kind, fantom_name, imported_name, note, memo, engine, detail,
                    created_at, archived_at, origin
               FROM assets WHERE id = ?1",
            [id],
            |row| Ok(row_to_asset(row)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound { kind: "asset", id },
            other => other.into(),
        })?;
    asset.tags = tags_of(ws, id)?;
    asset.sources = sources_of(ws, id)?;
    Ok(asset)
}

fn row_to_asset(row: &Row<'_>) -> Asset {
    let kind = row
        .get::<_, String>(1)
        .ok()
        .and_then(|k| AssetKind::parse(&k))
        .unwrap_or(AssetKind::Tone);
    let detail = row
        .get::<_, String>(7)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or(AssetDetail::Tone(ToneDetail {
            engine: String::new(),
            area: String::new(),
            index: 0,
            bank: None,
            address: None,
            category: None,
            model_id: None,
            requirements: Default::default(),
        }));
    Asset {
        id: row.get(0).unwrap_or_default(),
        kind,
        fantom_name: row.get(2).unwrap_or_default(),
        imported_name: row.get(3).unwrap_or_default(),
        note: row.get(4).unwrap_or_default(),
        memo: row.get(5).unwrap_or_default(),
        engine: row.get(6).unwrap_or_default(),
        detail,
        origin: row
            .get::<_, String>(10)
            .map(|origin| Origin::parse(&origin))
            .unwrap_or(Origin::User),
        created_at: row.get(8).unwrap_or_default(),
        archived_at: row.get(9).unwrap_or_default(),
        tags: Vec::new(),
        sources: Vec::new(),
    }
}

fn sources_of(ws: &Workspace, asset_id: i64) -> Result<Vec<AssetSource>> {
    let db = ws.db();
    let mut stmt = db.prepare(
        "SELECT s.id, s.name, f.id, f.file_name, o.slot, o.area, o.name_at_import
           FROM occurrences o
           JOIN files f ON f.id = o.file_id
           JOIN sources s ON s.id = f.source_id
          WHERE o.asset_id = ?1
          ORDER BY s.imported_at, f.file_name, o.slot",
    )?;
    let rows = stmt.query_map([asset_id], |row| {
        Ok(AssetSource {
            source_id: row.get(0)?,
            source_name: row.get(1)?,
            file_id: row.get(2)?,
            file_name: row.get(3)?,
            slot: row.get(4)?,
            area: row.get(5)?,
            name_at_import: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn tags_of(ws: &Workspace, asset_id: i64) -> Result<Vec<String>> {
    let db = ws.db();
    let mut stmt = db.prepare(
        "SELECT t.name FROM tags t JOIN asset_tags at ON at.tag_id = t.id
          WHERE at.asset_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([asset_id], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Every source group, newest import first.
pub fn sources(ws: &Workspace, include_archived: bool) -> Result<Vec<Source>> {
    let db = ws.db();
    let mut sql = String::from(
        "SELECT s.id, s.name, s.vendor, s.url, s.licence_note, s.note, s.origin_path,
                s.imported_at, s.archived_at,
                (SELECT COUNT(*) FROM files f WHERE f.source_id = s.id),
                (SELECT COUNT(DISTINCT o.asset_id) FROM occurrences o
                   JOIN files f2 ON f2.id = o.file_id WHERE f2.source_id = s.id)
           FROM sources s",
    );
    if !include_archived {
        sql.push_str(" WHERE s.archived_at IS NULL");
    }
    sql.push_str(" ORDER BY s.imported_at DESC, s.id DESC");
    let mut stmt = db.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(Source {
            id: row.get(0)?,
            name: row.get(1)?,
            vendor: row.get(2)?,
            url: row.get(3)?,
            licence_note: row.get(4)?,
            note: row.get(5)?,
            origin_path: row.get(6)?,
            imported_at: row.get(7)?,
            archived_at: row.get(8)?,
            file_count: row.get(9)?,
            asset_count: row.get(10)?,
            files: Vec::new(),
        })
    })?;

    let mut out: Vec<Source> = rows.collect::<rusqlite::Result<_>>()?;
    for source in &mut out {
        source.files = files(ws, source.id)?;
    }
    Ok(out)
}

/// The files one source contributed, with what each is for and why any of them is unusable.
pub fn files(ws: &Workspace, source_id: i64) -> Result<Vec<LibraryFile>> {
    let db = ws.db();
    let mut stmt = db.prepare(
        "SELECT f.id, f.source_id, f.file_name, f.origin_path, f.content_hash, f.size,
                f.stored_path, f.kind, f.role, f.status, f.problems,
                (SELECT COUNT(*) FROM occurrences o WHERE o.file_id = f.id),
                (SELECT COUNT(*) FROM samples sm WHERE sm.file_id = f.id)
           FROM files f WHERE f.source_id = ?1 ORDER BY f.file_name",
    )?;
    let rows = stmt.query_map([source_id], |row| {
        let role: String = row.get(8)?;
        let status: String = row.get(9)?;
        let problems: String = row.get(10)?;
        Ok(LibraryFile {
            id: row.get(0)?,
            source_id: row.get(1)?,
            file_name: row.get(2)?,
            origin_path: row.get(3)?,
            content_hash: row.get(4)?,
            size: row.get(5)?,
            stored_path: row.get(6)?,
            kind: row.get(7)?,
            role: fantom_core::role::Role::parse(&role),
            status: FileStatus::parse(&status),
            problems: serde_json::from_str(&problems).unwrap_or_default(),
            asset_count: row.get(11)?,
            sample_count: row.get(12)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Archive a source rather than delete it: its files, provenance, and history stay recoverable.
pub fn archive_source(ws: &Workspace, id: i64, archived: bool) -> Result<()> {
    let changed = ws.db().execute(
        "UPDATE sources SET archived_at = ?2 WHERE id = ?1",
        params![id, archived.then(now)],
    )?;
    if changed == 0 {
        return Err(Error::NotFound { kind: "source", id });
    }
    Ok(())
}

/// Rename an asset. Scenes only in v1 — a tone rename has no verified write path yet, so the
/// library refuses to record a name it could never put in an export.
pub fn rename_asset(ws: &Workspace, id: i64, name: &str) -> Result<()> {
    let (kind, origin): (String, String) = ws
        .db()
        .query_row("SELECT kind, origin FROM assets WHERE id = ?1", [id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .map_err(|_| Error::NotFound { kind: "asset", id })?;
    if kind != "scene" || origin == "factory" {
        return Err(Error::Rejected(
            "only imported scenes can be renamed until a verified write path exists".into(),
        ));
    }
    let name = check_fantom_name(name)?;
    ws.db().execute(
        "UPDATE assets SET fantom_name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    Ok(())
}

/// The library's own long-form note, distinct from the FANTOM memo.
pub fn set_asset_note(ws: &Workspace, id: i64, note: &str) -> Result<()> {
    let changed = ws.db().execute(
        "UPDATE assets SET note = ?2 WHERE id = ?1",
        params![id, note],
    )?;
    if changed == 0 {
        return Err(Error::NotFound { kind: "asset", id });
    }
    Ok(())
}

pub fn archive_asset(ws: &Workspace, id: i64, archived: bool) -> Result<()> {
    let changed = ws.db().execute(
        "UPDATE assets SET archived_at = ?2 WHERE id = ?1",
        params![id, archived.then(now)],
    )?;
    if changed == 0 {
        return Err(Error::NotFound { kind: "asset", id });
    }
    Ok(())
}

/// Check a name the way the instrument would, so the UI can reject one before it is saved.
///
/// The rule belongs to the format, not to the library: [`fantom_core::codec::check_name`] is the
/// same check [`fantom_core::codec::set_scene_name`] enforces when the bytes are actually written.
pub fn check_fantom_name(name: &str) -> Result<String> {
    if name.trim().is_empty() {
        return Err(Error::Rejected("name cannot be empty".into()));
    }
    fantom_core::codec::check_name(name)
        .map(str::to_string)
        .map_err(|e| Error::Rejected(e.to_string()))
}

/// Every tag in use, with how many assets carry it.
pub fn tags(ws: &Workspace) -> Result<Vec<Tag>> {
    let db = ws.db();
    let mut stmt = db.prepare(
        "SELECT t.name, COUNT(at.asset_id)
           FROM tags t LEFT JOIN asset_tags at ON at.tag_id = t.id
          GROUP BY t.id ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Tag {
            name: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn add_tag(ws: &Workspace, asset_id: i64, tag: &str) -> Result<()> {
    let tag = tag.trim();
    if tag.is_empty() {
        return Err(Error::Rejected("a tag cannot be empty".into()));
    }
    let db = ws.db();
    db.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [tag])?;
    let tag_id: i64 = db.query_row(
        "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
        [tag],
        |r| r.get(0),
    )?;
    db.execute(
        "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id) VALUES (?1, ?2)",
        params![asset_id, tag_id],
    )?;
    Ok(())
}

pub fn remove_tag(ws: &Workspace, asset_id: i64, tag: &str) -> Result<()> {
    ws.db().execute(
        "DELETE FROM asset_tags WHERE asset_id = ?1
           AND tag_id = (SELECT id FROM tags WHERE name = ?2 COLLATE NOCASE)",
        params![asset_id, tag.trim()],
    )?;
    Ok(())
}

pub fn songs(ws: &Workspace, search: &str) -> Result<Vec<Song>> {
    let db = ws.db();
    let search = search.trim();
    let mut stmt = db.prepare(
        "SELECT id, title, artist, song_key, notes, created_at FROM songs
          WHERE ?1 = '' OR title LIKE ?2 ESCAPE '\\' OR artist LIKE ?2 ESCAPE '\\'
          ORDER BY title COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(
        params![search, format!("%{}%", escape_like(search))],
        |row| {
            Ok(Song {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                song_key: row.get(3)?,
                notes: row.get(4)?,
                created_at: row.get(5)?,
                links: Vec::new(),
            })
        },
    )?;
    let mut out: Vec<Song> = rows.collect::<rusqlite::Result<_>>()?;
    for song in &mut out {
        song.links = song_links(ws, song.id)?;
    }
    Ok(out)
}

fn song_links(ws: &Workspace, song_id: i64) -> Result<Vec<SongLink>> {
    let db = ws.db();
    let mut stmt = db.prepare(
        "SELECT a.id, a.fantom_name, a.kind, sa.note
           FROM song_assets sa JOIN assets a ON a.id = sa.asset_id
          WHERE sa.song_id = ?1 ORDER BY a.kind, a.fantom_name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([song_id], |row| {
        let kind: String = row.get(2)?;
        Ok(SongLink {
            asset_id: row.get(0)?,
            asset_name: row.get(1)?,
            asset_kind: AssetKind::parse(&kind).unwrap_or(AssetKind::Tone),
            note: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn create_song(
    ws: &Workspace,
    title: &str,
    artist: &str,
    key: &str,
    notes: &str,
) -> Result<i64> {
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Rejected("a song needs a title".into()));
    }
    let db = ws.db();
    db.execute(
        "INSERT INTO songs (title, artist, song_key, notes, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![title, artist.trim(), key.trim(), notes, now()],
    )?;
    Ok(db.last_insert_rowid())
}

pub fn update_song(
    ws: &Workspace,
    id: i64,
    title: &str,
    artist: &str,
    key: &str,
    notes: &str,
) -> Result<()> {
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Rejected("a song needs a title".into()));
    }
    let changed = ws.db().execute(
        "UPDATE songs SET title = ?2, artist = ?3, song_key = ?4, notes = ?5 WHERE id = ?1",
        params![id, title, artist.trim(), key.trim(), notes],
    )?;
    if changed == 0 {
        return Err(Error::NotFound { kind: "song", id });
    }
    Ok(())
}

pub fn delete_song(ws: &Workspace, id: i64) -> Result<()> {
    ws.db().execute("DELETE FROM songs WHERE id = ?1", [id])?;
    Ok(())
}

pub fn link_song(ws: &Workspace, song_id: i64, asset_id: i64, note: &str) -> Result<()> {
    ws.db().execute(
        "INSERT INTO song_assets (song_id, asset_id, note) VALUES (?1, ?2, ?3)
           ON CONFLICT (song_id, asset_id) DO UPDATE SET note = excluded.note",
        params![song_id, asset_id, note],
    )?;
    Ok(())
}

pub fn unlink_song(ws: &Workspace, song_id: i64, asset_id: i64) -> Result<()> {
    ws.db().execute(
        "DELETE FROM song_assets WHERE song_id = ?1 AND asset_id = ?2",
        params![song_id, asset_id],
    )?;
    Ok(())
}

/// A count of everything, for the empty-state and the header.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub scenes: i64,
    pub tones: i64,
    pub sources: i64,
    pub songs: i64,
    pub samples: i64,
}

/// The expansion inventory: everything the catalogs know about, plus anything else recorded.
///
/// Two flags per product, kept apart on purpose. Owning an expansion and having it loaded are
/// different facts — the FANTOM's slots are finite, so a player owns more than the instrument
/// holds — and "buy it" and "load it" are different instructions to give.
///
/// Ordered by family then code, which is the order a list wants to show them in.
pub fn expansions(ws: &Workspace) -> Result<Vec<ExpansionEntry>> {
    let mut stored: HashMap<String, (bool, bool)> = HashMap::new();
    let mut statement = ws
        .db()
        .prepare("SELECT code, owned, installed FROM expansions")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        stored.insert(
            row.get(0)?,
            (row.get::<_, i64>(1)? != 0, row.get::<_, i64>(2)? != 0),
        );
    }

    let mut entries: Vec<ExpansionEntry> = expansions::products()
        .iter()
        .map(|product| {
            let (owned, installed) = stored.remove(product.code).unwrap_or_default();
            ExpansionEntry {
                code: product.code.to_string(),
                family: product.family,
                engine: product.engine.label().to_string(),
                sounds: product.sounds,
                owned,
                installed,
                catalogued: true,
            }
        })
        .collect();

    // Whatever is left was recorded by hand, for an expansion this build carries no catalog of.
    // It is listed too: the inventory is the user's statement about their instrument, not this
    // build's statement about what it can name.
    for (code, (owned, installed)) in stored {
        entries.push(ExpansionEntry {
            family: Family::of(&code, ToneType::Unknown),
            code,
            engine: String::new(),
            sounds: 0,
            owned,
            installed,
            catalogued: false,
        });
    }

    entries.sort_by(|a, b| (a.family, &a.code).cmp(&(b.family, &b.code)));
    Ok(entries)
}

/// Record what the player owns and what the instrument holds, for one product.
///
/// Any code is accepted, including one no catalog covers: the user knows their instrument better
/// than this build's catalogs do. A product set back to neither owned nor installed loses its row
/// rather than keeping a row full of falsehoods.
pub fn set_expansion(ws: &Workspace, code: &str, owned: bool, installed: bool) -> Result<()> {
    let entered = code.trim();
    let code = expansions::products()
        .iter()
        .find(|product| product.code.eq_ignore_ascii_case(entered))
        .map_or_else(
            || entered.to_ascii_uppercase(),
            |product| product.code.to_string(),
        );
    if code.is_empty() {
        return Err(Error::Rejected("an expansion needs a product code".into()));
    }
    if !owned && !installed {
        ws.db()
            .execute("DELETE FROM expansions WHERE code = ?1", [&code])?;
        return Ok(());
    }
    ws.db().execute(
        "INSERT INTO expansions (code, owned, installed) VALUES (?1, ?2, ?3)
         ON CONFLICT (code) DO UPDATE SET owned = excluded.owned, installed = excluded.installed",
        params![code, owned as i64, installed as i64],
    )?;
    Ok(())
}

pub fn stats(ws: &Workspace) -> Result<Stats> {
    let db = ws.db();
    let one = |sql: &str| -> Result<i64> { Ok(db.query_row(sql, [], |r| r.get(0))?) };
    Ok(Stats {
        scenes: one("SELECT COUNT(*) FROM assets WHERE kind = 'scene' AND archived_at IS NULL")?,
        tones: one("SELECT COUNT(*) FROM assets WHERE kind = 'tone' AND archived_at IS NULL")?,
        sources: one("SELECT COUNT(*) FROM sources WHERE archived_at IS NULL")?,
        songs: one("SELECT COUNT(*) FROM songs")?,
        samples: one("SELECT COUNT(*) FROM samples")?,
    })
}

/// `%` and `_` are wildcards in LIKE; a user searching for them means the characters.
fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
