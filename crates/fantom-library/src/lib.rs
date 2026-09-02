//! `fantom-library` — the librarian's workspace and catalog.
//!
//! Sits between [`fantom_core`], which turns bytes into typed values, and whatever drives the
//! app. It owns the things a librarian needs but a parser should not know about: a portable
//! workspace folder, a SQLite catalog of what has been imported, provenance, tags, and songs.
//!
//! It is headless on purpose — no Tauri, no terminal, no dialogs — so the desktop app and the CLI
//! can be two front ends over one library rather than two implementations of it.
//!
//! Layering:
//! - [`workspace`] — the folder: marker, managed originals, exports, catalog connection.
//! - [`import`] — files in: validate, copy, catalogue, report.
//! - [`catalog`] — everything out: browse, search, tag, relate to songs.
//!
//! What a file *is* — backup, scene bank, tone bank — is a fact about its bytes, so it lives in
//! [`fantom_core::role`] rather than here. This crate only decides what to do about it.
//! - [`role`] — what a file is for, which its envelope alone does not say.
//! - [`model`] — the serialisable types both front ends speak.

pub mod catalog;
pub mod facet;
pub mod factory;
pub mod import;
pub mod model;
pub mod rescan;
pub mod workspace;

mod error;

pub use error::{Error, Result};
pub use import::import;
pub use workspace::Workspace;

/// Unix seconds. Timestamps are stored as integers so the front end owns all date formatting.
pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AssetKind, Query, SourceInfo};
    use std::path::PathBuf;

    /// A workspace in a temp dir, plus the repo's committed fixtures.
    fn workspace() -> (tempfile::TempDir, Workspace) {
        let dir = tempfile::tempdir().expect("temp dir");
        let ws = Workspace::create(dir.path()).expect("create workspace");
        (dir, ws)
    }

    fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    #[test]
    fn create_then_open_round_trips() {
        let (dir, _ws) = workspace();
        assert!(workspace::is_workspace(dir.path()));
        assert!(dir.path().join(workspace::DB_FILE).exists());
        assert!(dir.path().join(workspace::ORIGINALS_DIR).is_dir());
        Workspace::open(dir.path()).expect("reopen");
    }

    #[test]
    fn the_instruments_own_sounds_seed_once_and_stay_put() {
        let (_dir, mut ws) = workspace();
        let added = crate::factory::seed(&mut ws).unwrap();
        assert!(added > 3000, "the bundled sound lists are thin: {added}");
        // Opening a library again must not double its factory rows.
        assert_eq!(crate::factory::seed(&mut ws).unwrap(), 0);

        let query = Query {
            kind: Some(AssetKind::Tone),
            origin: Some(crate::model::Origin::Factory),
            ..Default::default()
        };
        let sounds = catalog::assets(&ws, &query).unwrap();
        assert_eq!(sounds.len(), added);

        // Each one knows the bank it sits in, which is what the model filter offers.
        let piano = sounds
            .iter()
            .find(|sound| sound.fantom_name == "Stage Grand")
            .expect("the V-Piano bank");
        assert_eq!(crate::facet::models_of(piano), ["VPiano PRST"]);
        assert!(piano.sources.is_empty(), "no file carries a built-in sound");
    }

    #[test]
    fn refuses_to_create_over_an_existing_workspace() {
        let (dir, _ws) = workspace();
        assert!(matches!(
            Workspace::create(dir.path()),
            Err(Error::AlreadyAWorkspace(_))
        ));
    }

    #[test]
    fn refuses_to_open_a_plain_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(matches!(
            Workspace::open(dir.path()),
            Err(Error::NotAWorkspace(_))
        ));
    }

    #[test]
    fn an_import_with_nothing_to_take_is_rejected() {
        let (dir, mut ws) = workspace();
        let empty = dir.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(matches!(
            import(&mut ws, &[empty], &SourceInfo::default()),
            Err(Error::Rejected(_))
        ));
    }

    #[test]
    fn fantom_names_are_checked_against_the_device() {
        assert!(catalog::check_fantom_name("Ballad Rhodes").is_ok());
        assert!(catalog::check_fantom_name("").is_err());
        assert!(catalog::check_fantom_name("this name is far too long").is_err());
        assert!(catalog::check_fantom_name("café").is_err());
    }

    #[test]
    fn tags_and_songs_survive_a_round_trip() {
        let (_dir, ws) = workspace();
        // A bare asset row is enough to exercise the relational edges without a fixture.
        ws.db()
            .execute(
                "INSERT INTO assets (kind, identity_hash, fantom_name, imported_name, created_at)
                 VALUES ('scene', 'x', 'Ballad Rhodes', 'Ballad Rhodes', 0)",
                [],
            )
            .unwrap();
        let asset_id = ws.db().last_insert_rowid();

        catalog::add_tag(&ws, asset_id, "rhodes").unwrap();
        catalog::add_tag(&ws, asset_id, "ballad").unwrap();
        catalog::add_tag(&ws, asset_id, "rhodes").unwrap(); // idempotent
        assert_eq!(catalog::tags(&ws).unwrap().len(), 2);

        let found = catalog::assets(
            &ws,
            &Query {
                tags: vec!["rhodes".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].tags, vec!["ballad", "rhodes"]);

        catalog::remove_tag(&ws, asset_id, "rhodes").unwrap();
        assert!(catalog::assets(
            &ws,
            &Query {
                tags: vec!["rhodes".into()],
                ..Default::default()
            }
        )
        .unwrap()
        .is_empty());

        let song = catalog::create_song(&ws, "Riders on the Storm", "The Doors", "Em", "").unwrap();
        catalog::link_song(&ws, song, asset_id, "verse").unwrap();
        let songs = catalog::songs(&ws, "riders").unwrap();
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].links.len(), 1);
        assert_eq!(songs[0].links[0].asset_name, "Ballad Rhodes");
    }

    #[test]
    fn a_tone_cannot_be_renamed_yet() {
        let (_dir, ws) = workspace();
        ws.db()
            .execute(
                "INSERT INTO assets (kind, identity_hash, fantom_name, imported_name, created_at)
                 VALUES ('tone', 'y', 'Mk1 Rhodes', 'Mk1 Rhodes', 0)",
                [],
            )
            .unwrap();
        let id = ws.db().last_insert_rowid();
        assert!(matches!(
            catalog::rename_asset(&ws, id, "Mk2 Rhodes"),
            Err(Error::Rejected(_))
        ));
    }

    /// The real thing, when a fixture is present. Skips rather than fails so the suite still runs
    /// on a checkout without the private corpus.
    #[test]
    fn imports_a_fixture_and_catalogues_it() {
        let dir = fixtures();
        if !dir.is_dir() || std::fs::read_dir(&dir).into_iter().flatten().count() == 0 {
            eprintln!("skipping: no fixtures in {}", dir.display());
            return;
        }
        let (_tmp, mut ws) = workspace();
        let report = match import(
            &mut ws,
            std::slice::from_ref(&dir),
            &SourceInfo {
                name: "Fixtures".into(),
                vendor: "repo".into(),
                ..Default::default()
            },
        ) {
            Ok(report) => report,
            Err(Error::Rejected(_)) => {
                eprintln!("skipping: no .svd/.svz fixtures committed");
                return;
            }
            Err(e) => panic!("import failed: {e}"),
        };

        assert!(report.files_imported > 0, "nothing imported: {report:?}");
        let stats = catalog::stats(&ws).unwrap();
        assert_eq!(stats.sources, 1);

        // Re-importing the same folder consolidates onto the same canonical assets.
        let again = import(&mut ws, &[dir], &SourceInfo::default()).unwrap();
        assert_eq!(again.scenes_added + again.tones_added, 0);
        let after = catalog::stats(&ws).unwrap();
        assert_eq!(after.scenes, stats.scenes);
        assert_eq!(after.tones, stats.tones);

        // Every asset now names two sources, and search reaches them.
        let all = catalog::assets(&ws, &Query::default()).unwrap();
        if let Some(first) = all.first() {
            assert!(first.sources.len() >= 2, "second source not linked");
            let hit = catalog::assets(
                &ws,
                &Query {
                    search: first.fantom_name.clone(),
                    kind: Some(first.kind),
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(hit.iter().any(|a| a.id == first.id));
        }
        // The type filter narrows without losing anything: the two kinds partition the library.
        let scenes = catalog::assets(
            &ws,
            &Query {
                kind: Some(AssetKind::Scene),
                ..Default::default()
            },
        )
        .unwrap();
        let tones = catalog::assets(
            &ws,
            &Query {
                kind: Some(AssetKind::Tone),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(scenes.iter().all(|a| a.kind == AssetKind::Scene));
        assert_eq!(scenes.len() + tones.len(), all.len());

        // What each asset needs is decided at import and stored with it, so the library can
        // answer "can I export this" without reopening the file it came from.
        let requirements = scenes
            .iter()
            .filter_map(|asset| match &asset.detail {
                model::AssetDetail::Scene(scene) => Some(&scene.requirements),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!requirements.is_empty(), "no scenes to check");
        assert!(
            requirements.iter().all(|needs| !needs.engines.is_empty()),
            "a scene was catalogued without knowing what engine it plays"
        );
    }
}
