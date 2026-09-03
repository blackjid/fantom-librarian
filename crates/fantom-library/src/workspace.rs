//! The workspace folder: the one portable thing a user owns, copies, and backs up.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Marker file at the workspace root. Its presence is what makes a folder a workspace.
pub const MARKER: &str = "fantom-library.json";
/// The catalog, next to the material it describes.
pub const DB_FILE: &str = "library.db";
/// Managed copies of imported files. Content-addressed, so a repeat import costs no storage.
pub const ORIGINALS_DIR: &str = "originals";
/// Generated deployment folders. Never overwritten; each build is its own timestamped folder.
pub const EXPORTS_DIR: &str = "exports";

/// Bumped when the on-disk layout changes in a way an older build cannot read.
pub const FORMAT_VERSION: u32 = 3;

/// What opening a workspace had to do to bring it up to this build.
///
/// An upgrade rewrites the library in place, so a copy of the whole folder is taken first and
/// reported here: the app tells the user where it went, and it is the way back if the upgrade
/// turns out to have been wrong.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upgrade {
    /// The format the library was written at.
    pub from_format: u32,
    /// The format it now holds — this build's [`FORMAT_VERSION`].
    pub to_format: u32,
    /// The copy taken before anything was written, beside the library itself.
    pub backup_path: PathBuf,
}

/// An open workspace: a root folder and a connection to its catalog.
pub struct Workspace {
    root: PathBuf,
    db: Connection,
    upgrade: Option<Upgrade>,
}

impl Workspace {
    /// Create a new workspace in `root`, which may or may not exist but must not already be one.
    pub fn create(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        if root.join(MARKER).exists() {
            return Err(Error::AlreadyAWorkspace(root.to_path_buf()));
        }
        fs::create_dir_all(root).map_err(|e| Error::io(root, e))?;
        for dir in [ORIGINALS_DIR, EXPORTS_DIR] {
            let path = root.join(dir);
            fs::create_dir_all(&path).map_err(|e| Error::io(&path, e))?;
        }
        write_marker(root)?;

        Self::open_at(root)
    }

    /// Open the workspace at `root`, or fail if it is not one.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        if !root.join(MARKER).exists() {
            return Err(Error::NotAWorkspace(root.to_path_buf()));
        }
        Self::open_at(root)
    }

    /// Create the workspace if `root` is empty of one, otherwise open it.
    pub fn open_or_create(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        if root.join(MARKER).exists() {
            Self::open(root)
        } else {
            Self::create(root)
        }
    }

    fn open_at(root: &Path) -> Result<Self> {
        let format = read_format(root)?;
        // A library a newer build has already upgraded is refused before a single byte is read:
        // this build would migrate it by rules that no longer describe it.
        if format > FORMAT_VERSION {
            return Err(Error::WorkspaceTooNew {
                path: root.to_path_buf(),
                format,
            });
        }
        // Taken before the catalog is even opened, so a failure here leaves the library exactly as
        // it was and nothing has to be undone.
        let upgrade = if format < FORMAT_VERSION {
            Some(Upgrade {
                from_format: format,
                to_format: FORMAT_VERSION,
                backup_path: back_up(root, format)?,
            })
        } else {
            None
        };

        let mut db = Connection::open(root.join(DB_FILE))?;
        db.execute_batch(include_str!("schema.sql"))?;
        migrate(&mut db)?;
        retire_sound_lists(root)?;
        db.execute(
            "INSERT INTO meta (key, value) VALUES ('format', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            [FORMAT_VERSION.to_string()],
        )?;
        // Only once the upgrade has actually happened: the marker is what the next open reads, so
        // stamping it earlier would lose the copy that a failed run still needs.
        if upgrade.is_some() {
            write_marker(root)?;
        }
        Ok(Self {
            root: root.to_path_buf(),
            db,
            upgrade,
        })
    }

    /// What opening this workspace had to do to it, or nothing if it was already current.
    pub fn upgrade(&self) -> Option<&Upgrade> {
        self.upgrade.as_ref()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// What the user called their library: the folder's own name.
    pub fn name(&self) -> String {
        folder_name(&self.root)
    }

    pub fn exports_dir(&self) -> PathBuf {
        self.root.join(EXPORTS_DIR)
    }

    /// The catalog connection. Crate-internal so callers go through the typed API.
    pub(crate) fn db(&self) -> &Connection {
        &self.db
    }

    pub(crate) fn db_mut(&mut self) -> &mut Connection {
        &mut self.db
    }

    /// Absolute path of a `stored_path` recorded in the catalog.
    pub fn resolve(&self, stored_path: &str) -> PathBuf {
        self.root.join(stored_path)
    }
}

/// The old per-workspace sound-list cache duplicated catalogs now bundled with the application.
/// It never held user-authored library metadata, and removing it ensures it cannot become a
/// second source of truth after an upgrade.
fn retire_sound_lists(root: &Path) -> Result<()> {
    let legacy = root.join("sounds");
    if legacy.exists() {
        fs::remove_dir_all(&legacy).map_err(|e| Error::io(&legacy, e))?;
    }
    Ok(())
}

/// Bring an older catalog up to the current shape.
///
/// `schema.sql` creates what is missing but never alters what exists, so a column added to a table
/// an existing workspace already has has to be added here too. Each step is idempotent.
/// One transaction for the whole set: SQLite makes DDL transactional, so a run that fails part way
/// leaves the catalog at the shape it started from rather than half-way between two.
fn migrate(db: &mut Connection) -> Result<()> {
    let tx = db.transaction()?;
    for (table, column, definition) in [
        ("files", "role", "TEXT NOT NULL DEFAULT 'unknown'"),
        ("assets", "origin", "TEXT NOT NULL DEFAULT 'user'"),
    ] {
        if !has_column(&tx, table, column)? {
            tx.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {column} {definition}"
            ))?;
        }
    }
    // The inventory kept `owned` and `installed` as independent flags, which could say an
    // expansion was loaded but not owned — a state nothing ever acted on differently. One column
    // holds the ladder instead, and a row that claimed neither is dropped rather than migrated.
    if has_column(&tx, "expansions", "owned")? {
        tx.execute_batch(
            "CREATE TABLE expansions_ladder (
                 code  TEXT PRIMARY KEY,
                 state TEXT NOT NULL
             );
             INSERT INTO expansions_ladder (code, state)
                 SELECT code, CASE WHEN installed != 0 THEN 'loaded' ELSE 'owned' END
                   FROM expansions
                  WHERE owned != 0 OR installed != 0;
             DROP TABLE expansions;
             ALTER TABLE expansions_ladder RENAME TO expansions;",
        )?;
    }
    // Indexes over a migrated column belong here rather than in `schema.sql`, which runs first and
    // would name a column an older catalog does not have yet.
    tx.execute_batch("CREATE INDEX IF NOT EXISTS assets_origin ON assets (origin)")?;
    tx.commit()?;
    Ok(())
}

fn has_column(db: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = db.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        if row.get::<_, String>(1)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Copy the whole library beside itself, and answer where it went.
///
/// Beside rather than inside: a copy kept within the folder would be swept into the next copy, and
/// would double the library on every upgrade.
fn back_up(root: &Path, from_format: u32) -> Result<PathBuf> {
    let parent = root.parent().ok_or_else(|| {
        Error::Rejected(format!(
            "{} has nowhere beside it to keep a backup",
            root.display()
        ))
    })?;
    let prefix = format!("{} backup format {from_format} ", folder_name(root));

    // An upgrade that fails is tried again on the next open, and copying again would both fill the
    // disk and replace the untouched copy with one of the half-upgraded library. The first copy
    // taken at this format is the one worth keeping, so it is reused rather than remade.
    if let Some(existing) = complete_backup(parent, &prefix)? {
        return Ok(existing);
    }

    // The date is part of a folder name a person reads and sorts in Finder, not a value crossing
    // to a front end, so it is formatted here rather than kept as the seconds `crate::now` returns.
    let destination = parent.join(format!("{prefix}{}", utc_stamp(crate::now())));
    // Copied under a hidden name and moved into place, so a copy interrupted half way through is
    // never mistaken for a backup: only the rename, which is atomic, publishes one.
    let staging = parent.join(format!(".{}.incomplete", destination_name(&destination)));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| Error::io(&staging, e))?;
    }
    copy_tree(root, &staging)?;
    fs::rename(&staging, &destination).map_err(|e| Error::io(&staging, e))?;
    Ok(destination)
}

fn destination_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// The newest finished backup of this library at this format, if one is already there.
fn complete_backup(parent: &Path, prefix: &str) -> Result<Option<PathBuf>> {
    let mut found: Option<PathBuf> = None;
    for entry in fs::read_dir(parent).map_err(|e| Error::io(parent, e))? {
        let entry = entry.map_err(|e| Error::io(parent, e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        // A staging copy is hidden and carries `.incomplete`; it is not a backup.
        if !name.starts_with(prefix) || !entry.path().is_dir() {
            continue;
        }
        if found.as_ref().is_none_or(|best| entry.path() > *best) {
            found = Some(entry.path());
        }
    }
    Ok(found)
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).map_err(|e| Error::io(to, e))?;
    for entry in fs::read_dir(from).map_err(|e| Error::io(from, e))? {
        let entry = entry.map_err(|e| Error::io(from, e))?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let kind = entry.file_type().map_err(|e| Error::io(&source, e))?;
        if kind.is_dir() {
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|e| Error::io(&source, e))?;
        }
    }
    Ok(())
}

fn folder_name(root: &Path) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.display().to_string())
}

/// The marker, holding this build's format. Written on create, and again once an upgrade is done.
fn write_marker(root: &Path) -> Result<()> {
    let marker = root.join(MARKER);
    let contents = format!("{{\n  \"format\": {FORMAT_VERSION}\n}}\n");
    fs::write(&marker, contents).map_err(|e| Error::io(&marker, e))
}

/// `2026-09-02 201530`, in UTC so a backup taken either side of a timezone change still sorts.
fn utc_stamp(seconds: i64) -> String {
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    let time = seconds.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}{:02}{:02}",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

/// Days since the Unix epoch to a calendar date, by Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Whether `root` looks like a workspace, without opening its catalog.
pub fn is_workspace(root: impl AsRef<Path>) -> bool {
    root.as_ref().join(MARKER).exists()
}

/// The format version recorded in the marker, treating an unreadable one as current rather than
/// failing an otherwise healthy workspace.
fn read_format(root: &Path) -> Result<u32> {
    let marker = root.join(MARKER);
    let text = fs::read_to_string(&marker).map_err(|e| Error::io(&marker, e))?;
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    Ok(value
        .get("format")
        .and_then(|v| v.as_u64())
        .unwrap_or(FORMAT_VERSION as u64) as u32)
}
