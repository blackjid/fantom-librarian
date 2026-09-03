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
    use fantom_core::requirements::{Holding, Verdict};
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
    fn opening_a_legacy_workspace_retires_its_sound_list_cache() {
        let (dir, ws) = workspace();
        let sounds = dir.path().join("sounds");
        std::fs::create_dir(&sounds).unwrap();
        std::fs::write(sounds.join("old-dump.tsv"), "a cache, not library data").unwrap();
        drop(ws);

        Workspace::open(dir.path()).unwrap();
        assert!(!sounds.exists());
    }

    /// An upgrade rewrites a library in place, so the copy has to be taken before it starts.
    ///
    /// The backup is the whole folder, not just the catalog: originals and exports are as much
    /// the user's work as the database that indexes them, and only all three together restore.
    #[test]
    fn an_older_workspace_is_copied_beside_itself_before_it_is_upgraded() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let root = tmp.path().join("My Library");
        let ws = Workspace::create(&root).unwrap();
        std::fs::write(root.join(workspace::ORIGINALS_DIR).join("keep.svd"), "mine").unwrap();
        drop(ws);
        // What a build one format older left behind.
        std::fs::write(root.join(workspace::MARKER), "{\n  \"format\": 1\n}\n").unwrap();

        let ws = Workspace::open(&root).unwrap();
        let upgrade = ws.upgrade().expect("the older format was upgraded");
        assert_eq!(upgrade.from_format, 1);
        assert_eq!(upgrade.to_format, workspace::FORMAT_VERSION);

        // Beside the library rather than inside it: a copy within the folder would be swept up by
        // the next copy, and doubles the library every upgrade.
        let backup = upgrade.backup_path.clone();
        assert_eq!(backup.parent(), root.parent());
        assert!(backup.join(workspace::DB_FILE).is_file());
        assert_eq!(
            std::fs::read_to_string(backup.join(workspace::ORIGINALS_DIR).join("keep.svd"))
                .unwrap(),
            "mine"
        );
        // The copy keeps the format it was taken at, so the build that wrote it can still open it.
        assert!(std::fs::read_to_string(backup.join(workspace::MARKER))
            .unwrap()
            .contains("\"format\": 1"));

        // The library itself is now at this build's format, and opening it again copies nothing.
        drop(ws);
        let reopened = Workspace::open(&root).unwrap();
        assert!(
            reopened.upgrade().is_none(),
            "a second open backed up again"
        );
    }

    /// A library a newer build has already upgraded is refused, and refused without touching it.
    ///
    /// This build would migrate it by rules that no longer describe it, so the only safe move is
    /// to stop and say which library it was and what wrote it.
    #[test]
    fn a_workspace_from_a_newer_build_is_refused_and_left_alone() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let root = tmp.path().join("My Library");
        drop(Workspace::create(&root).unwrap());
        std::fs::write(root.join(workspace::MARKER), "{\n  \"format\": 99\n}\n").unwrap();

        let refused = match Workspace::open(&root) {
            Ok(_) => panic!("a library from a newer build was opened anyway"),
            Err(refused) => refused,
        };
        let Error::WorkspaceTooNew { path, format } = &refused else {
            panic!("expected a refusal, got {refused:?}");
        };
        assert_eq!(path, &root);
        assert_eq!(*format, 99);
        // The message is what the recovery screen shows, so it has to name the library itself.
        assert!(refused.to_string().contains("My Library"), "{refused}");

        // Untouched: still at the format the newer build left, and nothing was copied for it.
        assert!(std::fs::read_to_string(root.join(workspace::MARKER))
            .unwrap()
            .contains("99"));
        let siblings = std::fs::read_dir(tmp.path()).unwrap().count();
        assert_eq!(
            siblings, 1,
            "a refused open left something beside the library"
        );
    }

    /// An upgrade that fails is tried again on the next open, and the copy must not be remade.
    ///
    /// Copying again would fill the disk a library at a time, and — worse — would replace the
    /// untouched copy with one taken of the half-upgraded library, losing the only way back.
    #[test]
    fn a_second_attempt_at_the_same_upgrade_keeps_the_first_copy() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let root = tmp.path().join("My Library");
        drop(Workspace::create(&root).unwrap());
        std::fs::write(root.join(workspace::MARKER), "{\n  \"format\": 1\n}\n").unwrap();

        let first = Workspace::open(&root).unwrap().upgrade().unwrap().clone();
        // What a failed upgrade leaves behind: the library still declaring the older format.
        std::fs::write(root.join(workspace::MARKER), "{\n  \"format\": 1\n}\n").unwrap();
        let second = Workspace::open(&root).unwrap().upgrade().unwrap().clone();

        assert_eq!(second.backup_path, first.backup_path);
        let backups = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().contains("backup"))
            .count();
        assert_eq!(backups, 1, "the same upgrade was copied twice");
    }

    /// Owning an expansion and having it loaded are two rungs of one ladder, not two flags.
    ///
    /// The FANTOM's expansion slots are finite, so a player owns more than the instrument holds.
    /// "You own EXSN03, load it" and "you do not own EXSN03" are different instructions, and only
    /// a store that keeps the rungs apart can say which one applies.
    #[test]
    fn an_expansion_is_recorded_at_the_rung_it_has_reached() {
        let (dir, ws) = workspace();

        // Every expansion the catalogs know about is listed, so an unowned one is still selectable.
        let known = catalog::expansions(&ws).unwrap();
        assert!(known.len() >= 25, "{} listed", known.len());
        assert!(known.iter().all(|entry| entry.state == Holding::Unowned));
        assert!(known.iter().all(|entry| entry.catalogued));

        catalog::set_expansion(&ws, "EXZ007", Holding::Loaded).unwrap();
        // Owned, sitting on the shelf: the instrument has no free slot for it.
        catalog::set_expansion(&ws, "EXSN03", Holding::Owned).unwrap();

        let state = |entries: &[crate::model::ExpansionEntry], code: &str| {
            entries
                .iter()
                .find(|entry| entry.code == code)
                .map(|entry| entry.state)
        };
        let listed = catalog::expansions(&ws).unwrap();
        assert_eq!(state(&listed, "EXZ007"), Some(Holding::Loaded));
        assert_eq!(state(&listed, "EXSN03"), Some(Holding::Owned));
        assert_eq!(state(&listed, "EXZ008"), Some(Holding::Unowned));

        // Casing and surrounding whitespace still address the catalogued product's canonical code.
        catalog::set_expansion(&ws, " exz007 ", Holding::Owned).unwrap();
        let listed = catalog::expansions(&ws).unwrap();
        assert_eq!(state(&listed, "EXZ007"), Some(Holding::Owned));
        assert!(!listed.iter().any(|entry| entry.code == "exz007"));

        // Back to the bottom rung: the row goes, rather than claiming nothing.
        catalog::set_expansion(&ws, "EXSN03", Holding::Unowned).unwrap();
        assert_eq!(
            state(&catalog::expansions(&ws).unwrap(), "EXSN03"),
            Some(Holding::Unowned)
        );

        // A product no catalog covers is still recordable, and says that it is uncatalogued.
        catalog::set_expansion(&ws, "EXZ099", Holding::Owned).unwrap();
        let listed = catalog::expansions(&ws).unwrap();
        let unknown = listed
            .iter()
            .find(|entry| entry.code == "EXZ099")
            .expect("a recorded code the catalogs do not know");
        assert!(unknown.state == Holding::Owned && !unknown.catalogued);
        assert_eq!(unknown.family, fantom_core::expansions::Family::Wave);

        // It travels with the folder rather than with this machine.
        drop(ws);
        let reopened = Workspace::open(dir.path()).unwrap();
        let listed = catalog::expansions(&reopened).unwrap();
        assert_eq!(state(&listed, "EXZ007"), Some(Holding::Owned));
        assert_eq!(state(&listed, "EXZ099"), Some(Holding::Owned));
    }

    /// A library written when the inventory was two flags reads back on the ladder.
    ///
    /// The pair could say an expansion was loaded but not owned, which nothing ever acted on
    /// differently — so it lands on `loaded`, the rung that decides whether it plays. A row
    /// claiming neither said nothing and is not carried over.
    #[test]
    fn a_two_flag_inventory_migrates_onto_the_ladder() {
        let (dir, ws) = workspace();
        ws.db()
            .execute_batch(
                "DROP TABLE expansions;
                 CREATE TABLE expansions (
                     code      TEXT PRIMARY KEY,
                     owned     INTEGER NOT NULL DEFAULT 0,
                     installed INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT INTO expansions (code, owned, installed) VALUES
                     ('EXZ007', 1, 1),
                     ('EXSN03', 1, 0),
                     ('JP8',    0, 1),
                     ('EXZ099', 0, 0);",
            )
            .unwrap();
        drop(ws);
        std::fs::write(
            dir.path().join(workspace::MARKER),
            "{\n  \"format\": 2\n}\n",
        )
        .unwrap();

        let ws = Workspace::open(dir.path()).unwrap();
        assert_eq!(ws.upgrade().expect("an upgrade ran").from_format, 2);

        let held = catalog::expansion_inventory(&ws).unwrap();
        assert_eq!(held.holding("EXZ007"), Holding::Loaded);
        assert_eq!(held.holding("EXSN03"), Holding::Owned);
        assert_eq!(held.holding("JP8"), Holding::Loaded);
        assert_eq!(held.holding("EXZ099"), Holding::Unowned);

        // And the table is the new shape, so the next write does not have to satisfy both.
        catalog::set_expansion(&ws, "EXZ008", Holding::Owned).unwrap();
        assert_eq!(
            catalog::expansion_inventory(&ws).unwrap().holding("EXZ008"),
            Holding::Owned
        );
    }

    /// A preset every FANTOM ships with is a dependency, but not one to warn anybody about.
    ///
    /// The catalog stores every external reference a scene makes in one list, so the two have to be
    /// told apart on the way out — and on the way out rather than at import, so a library
    /// catalogued before the distinction existed reads correctly without being rebuilt.
    #[test]
    fn factory_references_are_told_from_the_ones_to_act_on() {
        let (_dir, ws) = workspace();
        let stored = crate::model::SceneDetail {
            bpm: 120.0,
            level: 100,
            active_zones: 2,
            zones: Vec::new(),
            engines: Vec::new(),
            groups: Vec::new(),
            user_tones: Vec::new(),
            // As every catalog wrote it: one list, factory and installable together.
            external_refs: vec![
                "ZEN-Core PR-A PC 060 \"Ac Pop Piano 1\"".into(),
                "SN-AP EXSN03 PC 000 \"Classic Piano\"".into(),
                "Drum CMN PC 051 \"TR-707&727 comp\"".into(),
                "MODEL JP8 PC 001 \"Brass\"".into(),
            ],
            factory_refs: Vec::new(),
            requirements: Default::default(),
        };
        ws.db()
            .execute(
                "INSERT INTO assets (kind, identity_hash, fantom_name, imported_name, created_at, detail)
                 VALUES ('scene', 'split', 'Encore', 'Encore', 0, ?1)",
                [serde_json::to_string(&crate::model::AssetDetail::Scene(stored)).unwrap()],
            )
            .unwrap();
        let id = ws.db().last_insert_rowid();

        let asset = catalog::asset(&ws, id).unwrap();
        let crate::model::AssetDetail::Scene(scene) = &asset.detail else {
            panic!("a scene");
        };
        assert_eq!(
            scene.external_refs,
            [
                "SN-AP EXSN03 PC 000 \"Classic Piano\"",
                "MODEL JP8 PC 001 \"Brass\"",
            ]
        );
        assert_eq!(
            scene.factory_refs,
            [
                "ZEN-Core PR-A PC 060 \"Ac Pop Piano 1\"",
                "Drum CMN PC 051 \"TR-707&727 comp\"",
            ]
        );

        // Which list a reference lands in is about acting on it, not finding it: both are still
        // filterable, and a scene needing an expansion still reads as needing yours.
        let models = facet::models_of(&asset);
        assert!(models.contains(&"ZEN-Core PR-A".to_string()), "{models:?}");
        assert!(models.contains(&"MODEL JP8".to_string()), "{models:?}");
        assert_eq!(
            facet::plays_of(&asset),
            Some(crate::model::Plays::NeedsYours)
        );
    }

    /// The inventory is the input a requirements check had no way to ask for.
    ///
    /// A file states what it needs and a backup states what samples an instrument holds, but only
    /// the player can say which expansions are bought and which are loaded — so the verdicts are
    /// only as good as this reading of their note.
    #[test]
    fn the_recorded_inventory_answers_what_no_file_can() {
        let (_dir, ws) = workspace();

        // A workspace nobody has told anything reads as a note with nothing in it, which is what
        // lets a check tell "not owned" apart from "never said".
        assert!(catalog::expansion_inventory(&ws).unwrap().is_empty());

        catalog::set_expansion(&ws, "EXZ007", Holding::Loaded).unwrap();
        catalog::set_expansion(&ws, "EXSN03", Holding::Owned).unwrap();
        catalog::set_expansion(&ws, "n/zyme", Holding::Loaded).unwrap();

        let held = catalog::expansion_inventory(&ws).unwrap();
        assert_eq!(held.verdict("EXZ007"), Verdict::Met);
        assert_eq!(held.verdict("EXSN03"), Verdict::NotLoaded);
        assert_eq!(held.verdict("EXZ008"), Verdict::Missing);
        // Stored under the catalog's own case, addressed by the case a bank label uses.
        assert_eq!(held.verdict("N/ZYME"), Verdict::Met);
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

    /// Seeding puts sounds in the library and nothing else.
    ///
    /// The scenes a FANTOM ships with were seeded here once, from the names in Roland's sound
    /// list, and it was withdrawn: a name is all that list gives, so every one of them was a row
    /// with no tempo, no zones and no requirements — and a user importing a backup of an untouched
    /// instrument got each of those names a second time, once empty and once real. Identifying a
    /// factory scene is a job for its bytes, at import; see `docs/SOUND_NAME_CAPTURE.md`.
    #[test]
    fn seeding_adds_no_scenes() {
        let (_dir, mut ws) = workspace();
        crate::factory::seed(&mut ws).unwrap();

        let scenes = catalog::assets(
            &ws,
            &Query {
                kind: Some(AssetKind::Scene),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(scenes.is_empty(), "{} scenes were seeded", scenes.len());
    }

    #[test]
    fn expansion_catalogs_stay_in_the_library_but_can_be_filtered_by_installation() {
        let (_dir, mut ws) = workspace();
        crate::factory::seed(&mut ws).unwrap();

        let exz007 = Query {
            kind: Some(AssetKind::Tone),
            ..Default::default()
        };
        let has_exz007 = |query: &Query| {
            catalog::assets(&ws, query)
                .unwrap()
                .iter()
                .any(|asset| matches!(&asset.detail, crate::model::AssetDetail::Tone(tone) if tone.bank.as_deref() == Some("EXZ007")))
        };
        assert!(
            has_exz007(&exz007),
            "bundled expansion sounds are kept even before the expansion is installed"
        );
        let exz007_id = catalog::assets(&ws, &exz007)
            .unwrap()
            .into_iter()
            .find_map(|asset| matches!(&asset.detail, crate::model::AssetDetail::Tone(tone) if tone.bank.as_deref() == Some("EXZ007")).then_some(asset.id))
            .expect("an EXZ007 tone");
        catalog::add_tag(&ws, exz007_id, "favourite").unwrap();
        catalog::set_asset_note(&ws, exz007_id, "use in the encore").unwrap();
        let song = catalog::create_song(&ws, "Encore", "", "", "").unwrap();
        catalog::link_song(&ws, song, exz007_id, "chorus").unwrap();

        let unavailable = Query {
            hide_uninstalled_expansions: true,
            ..exz007.clone()
        };
        assert!(!has_exz007(&unavailable));

        catalog::set_expansion(&ws, "EXZ007", Holding::Loaded).unwrap();
        assert!(has_exz007(&unavailable));

        // Unloading an expansion only changes what the filter shows; it never drops the factory
        // rows, so notes, tags, and song links on them remain attached.
        catalog::set_expansion(&ws, "EXZ007", Holding::Owned).unwrap();
        assert!(!has_exz007(&unavailable));
        assert!(has_exz007(&exz007));
        let preserved = catalog::asset(&ws, exz007_id).unwrap();
        assert_eq!(preserved.tags, ["favourite"]);
        assert_eq!(preserved.note, "use in the encore");
        assert_eq!(
            catalog::songs(&ws, "Encore").unwrap()[0].links[0].asset_id,
            exz007_id
        );
    }

    #[test]
    fn legacy_expansion_rows_keep_their_metadata_when_rekeyed() {
        let (_dir, mut ws) = workspace();
        let legacy = fantom_core::expansions::catalog("EXZ007")
            .next()
            .expect("bundled EXZ007 catalog");
        ws.db()
            .execute(
                "INSERT INTO assets (kind, identity_hash, fantom_name, imported_name, created_at, origin)
                 VALUES ('tone', ?1, ?2, ?2, 0, 'factory')",
                (
                    format!(
                        "factory:{}/{}/{}",
                        legacy.sound.address.msb, legacy.sound.address.lsb, legacy.sound.address.pc
                    ),
                    legacy.sound.name,
                ),
            )
            .unwrap();
        let old_id = ws.db().last_insert_rowid();
        catalog::add_tag(&ws, old_id, "favourite").unwrap();
        catalog::set_asset_note(&ws, old_id, "use in the encore").unwrap();
        let song = catalog::create_song(&ws, "Encore", "", "", "").unwrap();
        catalog::link_song(&ws, song, old_id, "chorus").unwrap();

        crate::factory::seed(&mut ws).unwrap();

        let migrated = catalog::asset(&ws, old_id).unwrap();
        assert_eq!(migrated.tags, ["favourite"]);
        assert_eq!(migrated.note, "use in the encore");
        assert_eq!(
            catalog::songs(&ws, "Encore").unwrap()[0].links[0].asset_id,
            old_id
        );
        let factory_rows: i64 = ws
            .db()
            .query_row(
                "SELECT COUNT(*) FROM assets WHERE identity_hash = ?1",
                [format!(
                    "factory:EXZ007/{}/{}/{}",
                    legacy.sound.address.msb, legacy.sound.address.lsb, legacy.sound.address.pc
                )],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(factory_rows, 1);
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

    /// A catalog written before the bundled lists could name an address is brought forward from
    /// the files it already keeps, rather than by asking for the import again.
    #[test]
    fn stale_scene_names_are_read_back_out_of_the_source_file() {
        let dir = fixtures();
        if !dir.is_dir() {
            eprintln!("skipping: no fixtures in {}", dir.display());
            return;
        }
        let (_tmp, mut ws) = workspace();
        let Ok(report) = import(&mut ws, &[dir], &SourceInfo::default()) else {
            eprintln!("skipping: no .svd/.svz fixtures committed");
            return;
        };
        assert!(report.scenes_added > 0, "no scene to go stale");

        // What an older build left behind: the zone table, with every name it could not resolve.
        let (id, fresh): (i64, String) = ws
            .db()
            .query_row(
                "SELECT id, detail FROM assets WHERE kind = 'scene' LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let stale = fresh.replace("\"tone\":\"", "\"tone\":\"—");
        assert_ne!(stale, fresh, "the fixture scene names no tone at all");
        ws.db()
            .execute("UPDATE assets SET detail = ?1 WHERE id = ?2", (&stale, id))
            .unwrap();
        ws.db()
            .execute("DELETE FROM meta WHERE key = 'naming'", [])
            .unwrap();

        assert_eq!(rescan::scene_names(&ws).unwrap(), 1);
        let brought_forward: String = ws
            .db()
            .query_row("SELECT detail FROM assets WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(brought_forward, fresh);

        // The workspace records where it got to, so opening it again reads no files at all.
        assert_eq!(rescan::scene_names(&ws).unwrap(), 0);
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
