//! The desktop app's backend: a thin command layer over [`fantom_library`].
//!
//! Nothing here decides anything. Opening a workspace, importing, and every query belong to the
//! library crate; this module only holds the one open workspace, turns library errors into
//! strings the front end can show, and settles what the app remembers between launches.

use std::path::PathBuf;
use std::sync::Mutex;

use fantom_library::catalog::{self, KindCounts, Stats};
use fantom_library::model::*;
use fantom_library::{workspace, Workspace};
use serde::{Deserialize, Serialize};
use tauri::Manager;

/// The single open workspace, or none before one is chosen.
#[derive(Default)]
struct AppState {
    workspace: Mutex<Option<Workspace>>,
}

/// Library errors reach the front end as plain strings — it shows them, it does not branch on
/// them — so this keeps `?` usable in every command.
type CmdResult<T> = Result<T, String>;

fn wrap<T>(result: fantom_library::Result<T>) -> CmdResult<T> {
    result.map_err(|e| e.to_string())
}

/// Run `f` against the open workspace, or say plainly that there isn't one.
fn with<T>(
    state: &tauri::State<'_, AppState>,
    f: impl FnOnce(&Workspace) -> fantom_library::Result<T>,
) -> CmdResult<T> {
    let guard = state
        .workspace
        .lock()
        .map_err(|_| "workspace lock poisoned")?;
    let ws = guard.as_ref().ok_or("no workspace is open")?;
    wrap(f(ws))
}

fn with_mut<T>(
    state: &tauri::State<'_, AppState>,
    f: impl FnOnce(&mut Workspace) -> fantom_library::Result<T>,
) -> CmdResult<T> {
    let mut guard = state
        .workspace
        .lock()
        .map_err(|_| "workspace lock poisoned")?;
    let ws = guard.as_mut().ok_or("no workspace is open")?;
    wrap(f(ws))
}

/// What the front end needs to render the header and decide whether to show the welcome screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub path: String,
    /// The folder's own name, which is what the user called their library.
    pub name: String,
    pub stats: Stats,
}

fn info(ws: &Workspace) -> fantom_library::Result<WorkspaceInfo> {
    Ok(WorkspaceInfo {
        path: ws.root().display().to_string(),
        name: ws
            .root()
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| ws.root().display().to_string()),
        stats: catalog::stats(ws)?,
    })
}

// ---- workspace -----------------------------------------------------------------------------

#[tauri::command]
fn open_workspace(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    path: String,
    create: bool,
) -> CmdResult<WorkspaceInfo> {
    let path = PathBuf::from(path);
    let ws = wrap(if create {
        Workspace::open_or_create(&path)
    } else {
        Workspace::open(&path)
    })?;
    let info = wrap(info(&ws))?;
    *state
        .workspace
        .lock()
        .map_err(|_| "workspace lock poisoned")? = Some(ws);
    remember(&app, &info.path);
    Ok(info)
}

/// Reopen the workspace from last launch, if it is still where it was.
#[tauri::command]
fn resume_workspace(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> CmdResult<Option<WorkspaceInfo>> {
    let Some(path) = recent(&app) else {
        return Ok(None);
    };
    if !workspace::is_workspace(&path) {
        return Ok(None);
    }
    let Ok(ws) = Workspace::open(&path) else {
        return Ok(None);
    };
    let info = wrap(info(&ws))?;
    *state
        .workspace
        .lock()
        .map_err(|_| "workspace lock poisoned")? = Some(ws);
    Ok(Some(info))
}

#[tauri::command]
fn workspace_info(state: tauri::State<'_, AppState>) -> CmdResult<WorkspaceInfo> {
    with(&state, info)
}

#[tauri::command]
fn close_workspace(state: tauri::State<'_, AppState>) -> CmdResult<()> {
    *state
        .workspace
        .lock()
        .map_err(|_| "workspace lock poisoned")? = None;
    Ok(())
}

/// Whether a folder already holds a workspace, so the picker can say "open" rather than "create".
#[tauri::command]
fn is_workspace(path: String) -> bool {
    workspace::is_workspace(path)
}

// ---- importing -----------------------------------------------------------------------------

#[tauri::command]
fn import_files(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
    info: SourceInfo,
) -> CmdResult<ImportReport> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    with_mut(&state, |ws| fantom_library::import(ws, &paths, &info))
}

// ---- browsing ------------------------------------------------------------------------------

#[tauri::command]
fn list_assets(state: tauri::State<'_, AppState>, query: Query) -> CmdResult<Vec<Asset>> {
    with(&state, |ws| catalog::assets(ws, &query))
}

/// Scene and tone totals for a scope, ignoring its kind filter — what the filter itself shows.
#[tauri::command]
fn count_assets(state: tauri::State<'_, AppState>, query: Query) -> CmdResult<KindCounts> {
    with(&state, |ws| catalog::counts(ws, &query))
}

#[tauri::command]
fn get_asset(state: tauri::State<'_, AppState>, id: i64) -> CmdResult<Asset> {
    with(&state, |ws| catalog::asset(ws, id))
}

#[tauri::command]
fn list_sources(
    state: tauri::State<'_, AppState>,
    include_archived: bool,
) -> CmdResult<Vec<Source>> {
    with(&state, |ws| catalog::sources(ws, include_archived))
}

#[tauri::command]
fn list_files(state: tauri::State<'_, AppState>, source_id: i64) -> CmdResult<Vec<LibraryFile>> {
    with(&state, |ws| catalog::files(ws, source_id))
}

#[tauri::command]
fn list_tags(state: tauri::State<'_, AppState>) -> CmdResult<Vec<Tag>> {
    with(&state, catalog::tags)
}

#[tauri::command]
fn list_songs(state: tauri::State<'_, AppState>, search: String) -> CmdResult<Vec<Song>> {
    with(&state, |ws| catalog::songs(ws, &search))
}

#[tauri::command]
fn get_stats(state: tauri::State<'_, AppState>) -> CmdResult<Stats> {
    with(&state, catalog::stats)
}

// ---- editing -------------------------------------------------------------------------------

#[tauri::command]
fn rename_asset(state: tauri::State<'_, AppState>, id: i64, name: String) -> CmdResult<()> {
    with(&state, |ws| catalog::rename_asset(ws, id, &name))
}

#[tauri::command]
fn set_asset_note(state: tauri::State<'_, AppState>, id: i64, note: String) -> CmdResult<()> {
    with(&state, |ws| catalog::set_asset_note(ws, id, &note))
}

#[tauri::command]
fn archive_asset(state: tauri::State<'_, AppState>, id: i64, archived: bool) -> CmdResult<()> {
    with(&state, |ws| catalog::archive_asset(ws, id, archived))
}

#[tauri::command]
fn archive_source(state: tauri::State<'_, AppState>, id: i64, archived: bool) -> CmdResult<()> {
    with(&state, |ws| catalog::archive_source(ws, id, archived))
}

#[tauri::command]
fn add_tag(state: tauri::State<'_, AppState>, asset_id: i64, tag: String) -> CmdResult<()> {
    with(&state, |ws| catalog::add_tag(ws, asset_id, &tag))
}

#[tauri::command]
fn remove_tag(state: tauri::State<'_, AppState>, asset_id: i64, tag: String) -> CmdResult<()> {
    with(&state, |ws| catalog::remove_tag(ws, asset_id, &tag))
}

/// A name the FANTOM would accept. Checked as the user types, so a rename never fails on save.
#[tauri::command]
fn check_name(name: String) -> Option<String> {
    catalog::check_fantom_name(&name)
        .err()
        .map(|e| e.to_string())
}

// ---- songs ---------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct SongInput {
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub song_key: String,
    #[serde(default)]
    pub notes: String,
}

#[tauri::command]
fn create_song(state: tauri::State<'_, AppState>, song: SongInput) -> CmdResult<i64> {
    with(&state, |ws| {
        catalog::create_song(ws, &song.title, &song.artist, &song.song_key, &song.notes)
    })
}

#[tauri::command]
fn update_song(state: tauri::State<'_, AppState>, id: i64, song: SongInput) -> CmdResult<()> {
    with(&state, |ws| {
        catalog::update_song(
            ws,
            id,
            &song.title,
            &song.artist,
            &song.song_key,
            &song.notes,
        )
    })
}

#[tauri::command]
fn delete_song(state: tauri::State<'_, AppState>, id: i64) -> CmdResult<()> {
    with(&state, |ws| catalog::delete_song(ws, id))
}

#[tauri::command]
fn link_song(
    state: tauri::State<'_, AppState>,
    song_id: i64,
    asset_id: i64,
    note: String,
) -> CmdResult<()> {
    with(&state, |ws| {
        catalog::link_song(ws, song_id, asset_id, &note)
    })
}

#[tauri::command]
fn unlink_song(state: tauri::State<'_, AppState>, song_id: i64, asset_id: i64) -> CmdResult<()> {
    with(&state, |ws| catalog::unlink_song(ws, song_id, asset_id))
}

// ---- recent workspace ----------------------------------------------------------------------

/// The one thing the app stores outside a workspace: which workspace to reopen. A single line in
/// the platform config dir, so nothing about the library itself lives outside the portable folder.
fn recent_file(app: &tauri::AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    Some(dir.join("recent-workspace.txt"))
}

fn remember(app: &tauri::AppHandle, path: &str) {
    if let Some(file) = recent_file(app) {
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(file, path);
    }
}

fn recent(app: &tauri::AppHandle) -> Option<PathBuf> {
    let text = std::fs::read_to_string(recent_file(app)?).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_workspace,
            resume_workspace,
            workspace_info,
            close_workspace,
            is_workspace,
            import_files,
            list_assets,
            count_assets,
            get_asset,
            list_sources,
            list_files,
            list_tags,
            list_songs,
            get_stats,
            rename_asset,
            set_asset_note,
            archive_asset,
            archive_source,
            add_tag,
            remove_tag,
            check_name,
            create_song,
            update_song,
            delete_song,
            link_song,
            unlink_song,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the FANTOM Librarian");
}
