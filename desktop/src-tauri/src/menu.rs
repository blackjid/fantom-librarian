//! The application menu.
//!
//! Tauri builds a sensible default, and it is nearly right: the Edit menu it provides is what
//! makes Cut, Copy, Paste and Select All work inside a webview at all. Two things are wrong with
//! it here. It names the app from the crate — "Quit fantom-desktop" — rather than from
//! `productName`, and it offers nothing this app actually does.
//!
//! So the default is rebuilt with the product's name, and a File menu is added for the handful of
//! actions that deserve a keyboard shortcut. Each of those emits an event the front end listens
//! for, which keeps the menu a second way to reach the same code rather than a second copy of it.

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// What the product is called everywhere a person sees it.
const APP_NAME: &str = "FANTOM Librarian";

/// Menu ids, which double as the event names the front end listens for.
pub const OPEN_LIBRARY: &str = "open-library";
pub const CLOSE_LIBRARY: &str = "close-library";
pub const IMPORT: &str = "import";
pub const REVEAL_LIBRARY: &str = "reveal-library";
pub const FIND: &str = "find";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let about = AboutMetadata {
        name: Some(APP_NAME.into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        copyright: Some("MIT".into()),
        website: Some(env!("CARGO_PKG_REPOSITORY").into()),
        ..Default::default()
    };

    let app_menu = Submenu::with_items(
        app,
        APP_NAME,
        true,
        &[
            &PredefinedMenuItem::about(app, Some(&format!("About {APP_NAME}")), Some(about))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, Some(&format!("Hide {APP_NAME}")))?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some(&format!("Quit {APP_NAME}")))?,
        ],
    )?;

    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &MenuItem::with_id(app, IMPORT, "Import…", true, Some("CmdOrCtrl+I"))?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                OPEN_LIBRARY,
                "Open Library…",
                true,
                Some("CmdOrCtrl+O"),
            )?,
            &MenuItem::with_id(
                app,
                REVEAL_LIBRARY,
                "Reveal Library in Finder",
                true,
                Some("CmdOrCtrl+Shift+R"),
            )?,
            &MenuItem::with_id(app, CLOSE_LIBRARY, "Close Library", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some("Close Window"))?,
        ],
    )?;

    // The Edit menu is not decoration: without it the webview has no clipboard shortcuts.
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, FIND, "Find in Library", true, Some("CmdOrCtrl+F"))?,
        ],
    )?;

    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[&PredefinedMenuItem::fullscreen(app, None)?],
    )?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, Some("Zoom"))?,
        ],
    )?;

    Menu::with_items(
        app,
        &[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu],
    )
}

/// Turn a menu selection into an event the front end already knows how to handle.
///
/// The menu deliberately does no work of its own — every item here has a button or a shortcut in
/// the window too, and both must end up in the same place.
pub fn on_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    if matches!(
        id,
        OPEN_LIBRARY | CLOSE_LIBRARY | IMPORT | REVEAL_LIBRARY | FIND
    ) {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("menu", id);
        }
    }
}
