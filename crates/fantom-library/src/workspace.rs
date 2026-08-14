//! The workspace folder: the one portable thing a user owns, copies, and backs up.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

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
pub const FORMAT_VERSION: u32 = 1;

/// An open workspace: a root folder and a connection to its catalog.
pub struct Workspace {
    root: PathBuf,
    db: Connection,
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
        let marker = root.join(MARKER);
        let contents = format!("{{\n  \"format\": {FORMAT_VERSION}\n}}\n");
        fs::write(&marker, contents).map_err(|e| Error::io(&marker, e))?;

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
        if format > FORMAT_VERSION {
            return Err(Error::WorkspaceTooNew(format));
        }
        let db = Connection::open(root.join(DB_FILE))?;
        db.execute_batch(include_str!("schema.sql"))?;
        migrate(&db)?;
        db.execute(
            "INSERT INTO meta (key, value) VALUES ('format', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            [FORMAT_VERSION.to_string()],
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            db,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
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

/// Bring an older catalog up to the current shape.
///
/// `schema.sql` creates what is missing but never alters what exists, so a column added to a table
/// an existing workspace already has has to be added here too. Each step is idempotent.
fn migrate(db: &Connection) -> Result<()> {
    for (table, column, definition) in [("files", "role", "TEXT NOT NULL DEFAULT 'unknown'")] {
        if !has_column(db, table, column)? {
            db.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {column} {definition}"
            ))?;
        }
    }
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
