-- The catalog. One file per workspace, alongside the managed originals it describes.
--
-- Two rules shape the layout:
--   * an import is never destructive, so every row keeps what it was at import time next to
--     whatever the user has since edited;
--   * an asset is canonical and its occurrences are plural, so the same scene arriving in two
--     source packs consolidates into one library item without losing either provenance.

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- One import: a folder or a set of files taken together as one pack.
CREATE TABLE IF NOT EXISTS sources (
    id            INTEGER PRIMARY KEY,
    name          TEXT    NOT NULL,
    vendor        TEXT    NOT NULL DEFAULT '',
    url           TEXT    NOT NULL DEFAULT '',
    licence_note  TEXT    NOT NULL DEFAULT '',
    note          TEXT    NOT NULL DEFAULT '',
    origin_path   TEXT    NOT NULL DEFAULT '',
    imported_at   INTEGER NOT NULL,
    archived_at   INTEGER
);

-- A managed copy of one imported file. `stored_path` is relative to the workspace root; the
-- original at `origin_path` is never touched.
CREATE TABLE IF NOT EXISTS files (
    id            INTEGER PRIMARY KEY,
    source_id     INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    file_name     TEXT    NOT NULL,
    origin_path   TEXT    NOT NULL DEFAULT '',
    content_hash  TEXT    NOT NULL,
    size          INTEGER NOT NULL,
    stored_path   TEXT    NOT NULL,
    kind          TEXT    NOT NULL,
    -- What the file is for: a whole-instrument backup reads very differently from a three-scene
    -- export, and both are `.svd`. See `role.rs` for how the two are told apart.
    role          TEXT    NOT NULL DEFAULT 'unknown',
    -- 'ok' once structure and record checksums pass; 'invalid' keeps the file visible with its
    -- report but bars it from contributing assets.
    status        TEXT    NOT NULL,
    problems      TEXT    NOT NULL DEFAULT '[]',
    UNIQUE (source_id, content_hash)
);

-- A canonical library item. `identity_hash` is over the stored record bytes, so only genuinely
-- identical material consolidates; renamed or edited near-duplicates stay distinct.
CREATE TABLE IF NOT EXISTS assets (
    id             INTEGER PRIMARY KEY,
    kind           TEXT    NOT NULL CHECK (kind IN ('scene', 'tone')),
    identity_hash  TEXT    NOT NULL UNIQUE,
    fantom_name    TEXT    NOT NULL,
    imported_name  TEXT    NOT NULL,
    note           TEXT    NOT NULL DEFAULT '',
    memo           TEXT    NOT NULL DEFAULT '',
    engine         TEXT    NOT NULL DEFAULT '',
    detail         TEXT    NOT NULL DEFAULT '{}',
    -- 'user' for a record that came out of an imported file, 'factory' for one the instrument
    -- ships with. A factory row has no occurrences: there is no file behind it.
    origin         TEXT    NOT NULL DEFAULT 'user' CHECK (origin IN ('user', 'factory')),
    created_at     INTEGER NOT NULL,
    archived_at    INTEGER
);

CREATE INDEX IF NOT EXISTS assets_name ON assets (fantom_name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS assets_kind ON assets (kind);

-- Where an asset was seen. One row per (asset, file, slot).
CREATE TABLE IF NOT EXISTS occurrences (
    id             INTEGER PRIMARY KEY,
    asset_id       INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    file_id        INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    -- 1-based scene number, or the 0-based record index within `area` for a tone.
    slot           INTEGER NOT NULL,
    area           TEXT    NOT NULL DEFAULT '',
    name_at_import TEXT    NOT NULL,
    UNIQUE (asset_id, file_id, slot)
);

CREATE INDEX IF NOT EXISTS occurrences_file ON occurrences (file_id);

-- User sample slots carried by an imported file. Catalogued at import so the slot ledger has
-- something to work from; not shown in the main library.
CREATE TABLE IF NOT EXISTS samples (
    id            INTEGER PRIMARY KEY,
    file_id       INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    slot          INTEGER NOT NULL,
    name          TEXT    NOT NULL,
    frames        INTEGER NOT NULL DEFAULT 0,
    seconds       REAL    NOT NULL DEFAULT 0,
    original_key  INTEGER NOT NULL DEFAULT 60,
    has_audio     INTEGER NOT NULL DEFAULT 0,
    UNIQUE (file_id, slot)
);

CREATE TABLE IF NOT EXISTS tags (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE IF NOT EXISTS asset_tags (
    asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    tag_id   INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (asset_id, tag_id)
);

CREATE TABLE IF NOT EXISTS songs (
    id         INTEGER PRIMARY KEY,
    title      TEXT    NOT NULL,
    artist     TEXT    NOT NULL DEFAULT '',
    song_key   TEXT    NOT NULL DEFAULT '',
    notes      TEXT    NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL
);

-- A song links to scenes and tones alike: a Rhodes tone can suit a song before any scene does.
CREATE TABLE IF NOT EXISTS song_assets (
    song_id  INTEGER NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    note     TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (song_id, asset_id)
);

-- How far each expansion has got towards playing: 'owned' or 'loaded'. The FANTOM's expansion
-- slots are finite, so an owned expansion is not necessarily a loaded one, and "buy it" and
-- "load it" are different instructions. A row exists only once something is claimed — no row is
-- 'unowned'. Everything the bundled catalogs know about is listed whether or not it has a row
-- here, and a code they do not know can be recorded all the same.
CREATE TABLE IF NOT EXISTS expansions (
    code  TEXT PRIMARY KEY,
    state TEXT NOT NULL
);
