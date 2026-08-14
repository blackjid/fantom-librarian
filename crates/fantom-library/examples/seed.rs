//! Dev helper: build a workspace from a set of paths so the app has something to show.
//!
//! `cargo run -p fantom-library --example seed -- <workspace> <path>...`

fn main() {
    let mut args = std::env::args().skip(1);
    let root = args
        .next()
        .expect("usage: seed <workspace> <import path>...");
    let mut ws = fantom_library::Workspace::open_or_create(&root).unwrap();
    for path in args {
        let p = std::path::PathBuf::from(&path);
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        let info = fantom_library::model::SourceInfo {
            name,
            ..Default::default()
        };
        match fantom_library::import(&mut ws, &[p], &info) {
            Ok(r) => println!(
                "{path}: {} files, {} scenes, {} tones, {} samples",
                r.files_imported, r.scenes_added, r.tones_added, r.samples_catalogued
            ),
            Err(e) => println!("{path}: {e}"),
        }
    }
    println!("\n{:#?}", fantom_library::catalog::stats(&ws).unwrap());
    for source in fantom_library::catalog::sources(&ws, false).unwrap() {
        println!("\n{} ({} assets)", source.name, source.asset_count);
        for f in &source.files {
            println!(
                "  {:<16} {:<12} {:>5} assets {:>4} samples",
                f.file_name,
                f.role.as_str(),
                f.asset_count,
                f.sample_count
            );
        }
    }
}
