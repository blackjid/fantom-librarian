// Keep the console window away on Windows release builds; the app has no terminal output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    fantom_desktop_lib::run()
}
