//! The JSON the front end actually receives.
//!
//! `desktop/src/lib/api.ts` hand-mirrors these types. Nothing makes the two agree at compile
//! time, so this pins the field names and tagging a renamed Rust field would silently break.

use fantom_core::model::ToneType;
use fantom_core::requirements::{Requirements, SlotRequirement, WaveExpansion};
use fantom_core::role::Role;
use fantom_library::catalog::Stats;
use fantom_library::model::*;
use fantom_library::workspace::Upgrade;
use serde_json::{json, Value};

/// An object's field names, sorted — the front end reads by name, so only the set matters.
fn keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .expect("expected a JSON object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

fn sorted<const N: usize>(names: [&str; N]) -> Vec<String> {
    let mut names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
    names.sort();
    names
}

#[test]
fn an_asset_serialises_with_the_fields_the_front_end_reads() {
    let asset = Asset {
        id: 1,
        kind: AssetKind::Scene,
        fantom_name: "Ballad Rhodes".into(),
        imported_name: "Ballad Rhodes".into(),
        note: String::new(),
        memo: "verse only".into(),
        engine: "ZEN-Core".into(),
        detail: AssetDetail::Scene(SceneDetail {
            bpm: 116.0,
            level: 100,
            active_zones: 4,
            zones: Vec::new(),
            engines: vec!["ZEN-Core".into()],
            groups: Vec::new(),
            user_tones: vec!["Mk1 Rhodes".into()],
            external_refs: Vec::new(),
            requirements: Requirements::default(),
        }),
        origin: Origin::User,
        created_at: 0,
        archived_at: None,
        tags: vec!["rhodes".into()],
        sources: Vec::new(),
    };

    let json = serde_json::to_value(&asset).unwrap();
    assert_eq!(
        keys(&json),
        sorted([
            "id",
            "kind",
            "fantom_name",
            "imported_name",
            "note",
            "memo",
            "engine",
            "detail",
            "origin",
            "created_at",
            "archived_at",
            "tags",
            "sources",
        ])
    );

    // The front end narrows on `detail.kind`, so the internal tag has to be there and lowercase.
    assert_eq!(json["kind"], "scene");
    assert_eq!(json["detail"]["kind"], "scene");
    assert_eq!(json["archived_at"], Value::Null);
}

#[test]
fn a_tone_detail_keeps_its_own_tag() {
    let detail = AssetDetail::Tone(ToneDetail {
        engine: "MODEL".into(),
        area: "MDLa".into(),
        index: 3,
        bank: None,
        address: None,
        category: None,
        model_id: Some(9),
        requirements: Requirements::default(),
    });
    let json = serde_json::to_value(&detail).unwrap();
    assert_eq!(json["kind"], "tone");
    assert_eq!(json["area"], "MDLa");
    assert_eq!(json["model_id"], 9);
}

/// What an asset needs travels with it, spelled the way the front end reads it.
#[test]
fn requirements_serialise_with_the_fields_the_front_end_reads() {
    let requirements = Requirements {
        engines: vec![ToneType::ZenCore],
        samples: vec![SlotRequirement {
            slot: 22,
            name: Some("doh duh 2".into()),
            carried: false,
            silent: false,
            played_by: vec!["Beat It Gong".into()],
        }],
        wave_expansions: vec![WaveExpansion::new(1005)],
        ..Requirements::default()
    };
    let json = serde_json::to_value(&requirements).unwrap();
    assert_eq!(
        keys(&json),
        sorted([
            "engines",
            "user_tones",
            "banks",
            "samples",
            "multisamples",
            "wave_expansions",
            "unclassified",
            "carries_audio",
        ])
    );
    // An engine is a name on the wire, not a number: the front end shows it as a label.
    assert_eq!(json["engines"][0], "zen-core");
    assert_eq!(
        keys(&json["samples"][0]),
        sorted(["slot", "name", "carried", "silent", "played_by"])
    );
    assert_eq!(json["samples"][0]["slot"], 22);

    // A wave expansion crosses as the product it decodes to, with the stored id beside it.
    assert_eq!(keys(&json["wave_expansions"][0]), sorted(["id", "product"]));
    assert_eq!(json["wave_expansions"][0]["id"], 1005);
    assert_eq!(json["wave_expansions"][0]["product"], "EXZ005");

    // A row written before the id was decoded stored bare numbers. It must still read, and read
    // as the product: `row_to_asset` falls back to a *blank* detail when a row fails to parse, so
    // a shape change here would silently empty every asset already in a library.
    let stored: Requirements = serde_json::from_value(json!({
        "wave_expansions": [1005, 4],
    }))
    .expect("a catalog row written before the decode must still deserialise");
    assert_eq!(stored.wave_expansions[0].id, 1005);
    assert_eq!(stored.wave_expansions[0].product.as_deref(), Some("EXZ005"));
    assert_eq!(stored.wave_expansions[1].product, None);

    // Old catalog rows predate the field, and must still read.
    let detail: SceneDetail = serde_json::from_value(json!({
        "bpm": 120.0,
        "level": 100,
        "active_zones": 1,
        "zones": [],
        "engines": [],
        "groups": [],
        "user_tones": [],
        "external_refs": [],
    }))
    .expect("a detail written before requirements existed must still deserialise");
    assert!(detail.requirements.is_empty());
}

#[test]
fn a_query_accepts_what_the_front_end_sends() {
    // Exactly the payload `api.listAssets` builds, nulls and all.
    let query: Query = serde_json::from_value(json!({
        "search": "rhodes",
        "kind": "scene",
        "source_id": null,
        "song_id": null,
        "tags": ["ballad"],
        "limit": 500,
    }))
    .expect("the front end's query shape must deserialise");

    assert_eq!(query.search, "rhodes");
    assert_eq!(query.kind, Some(AssetKind::Scene));
    assert_eq!(query.source_id, None);
    assert_eq!(query.tags, ["ballad"]);
    assert_eq!(query.limit, Some(500));
    // Not sent by the front end, and must default rather than fail.
    assert!(!query.include_archived);

    // An empty object is the "everything" query the library view opens with.
    let empty: Query = serde_json::from_value(json!({})).expect("an empty query must deserialise");
    assert_eq!(empty.kind, None);
    assert!(empty.search.is_empty());
}

#[test]
fn source_info_tolerates_a_blank_import_form() {
    let info: SourceInfo =
        serde_json::from_value(json!({})).expect("provenance is optional at every field");
    assert!(info.name.is_empty());

    let filled: SourceInfo = serde_json::from_value(json!({
        "name": "NARF Sounds",
        "vendor": "NARF",
        "url": "",
        "licence_note": "purchased",
        "note": "",
    }))
    .unwrap();
    assert_eq!(filled.licence_note, "purchased");
}

#[test]
fn the_import_report_and_stats_keep_their_names() {
    let report = ImportReport {
        source_id: 1,
        source_name: "NARF".into(),
        files_imported: 2,
        files_skipped: 0,
        files_invalid: 0,
        scenes_added: 5,
        tones_added: 9,
        assets_consolidated: 3,
        samples_catalogued: 50,
        warnings: vec!["a.sdz: Roland Cloud .sdz is not supported".into()],
    };
    assert_eq!(
        keys(&serde_json::to_value(&report).unwrap()),
        sorted([
            "source_id",
            "source_name",
            "files_imported",
            "files_skipped",
            "files_invalid",
            "scenes_added",
            "tones_added",
            "assets_consolidated",
            "samples_catalogued",
            "warnings",
        ])
    );

    let stats = Stats {
        scenes: 1,
        tones: 2,
        sources: 3,
        songs: 4,
        samples: 5,
    };
    assert_eq!(
        keys(&serde_json::to_value(&stats).unwrap()),
        sorted(["scenes", "tones", "sources", "songs", "samples"])
    );
}

#[test]
fn a_song_carries_its_links_inline() {
    let song = Song {
        id: 1,
        title: "Riders on the Storm".into(),
        artist: "The Doors".into(),
        song_key: "Em".into(),
        notes: String::new(),
        created_at: 0,
        links: vec![SongLink {
            asset_id: 2,
            asset_name: "Ballad Rhodes".into(),
            asset_kind: AssetKind::Scene,
            note: "verse".into(),
        }],
    };
    let json = serde_json::to_value(&song).unwrap();
    assert_eq!(json["song_key"], "Em");
    assert_eq!(json["links"][0]["asset_kind"], "scene");
    assert_eq!(json["links"][0]["asset_name"], "Ballad Rhodes");
}

#[test]
fn a_file_reports_its_status_and_problems() {
    let file = LibraryFile {
        id: 1,
        source_id: 1,
        file_name: "FANTOM.SVD".into(),
        origin_path: "/Volumes/USB/FANTOM.SVD".into(),
        content_hash: "abc".into(),
        size: 1024,
        stored_path: "originals/ab/abc.svd".into(),
        kind: "svd".into(),
        role: Role::Backup,
        status: FileStatus::Invalid,
        problems: vec!["checksum mismatch".into()],
        asset_count: 0,
        sample_count: 0,
    };
    let json = serde_json::to_value(&file).unwrap();
    assert_eq!(json["status"], "invalid");
    assert_eq!(json["problems"][0], "checksum mismatch");
    assert_eq!(json["role"], "backup");
}

/// The front end keys an icon and a label off each role, so every variant has to serialise to the
/// kebab-case string its `Role` union in `api.ts` lists.
#[test]
fn every_role_serialises_to_its_front_end_name() {
    for (role, expected) in [
        (Role::Backup, "backup"),
        (Role::SceneBank, "scene-bank"),
        (Role::ToneBank, "tone-bank"),
        (Role::SampleBank, "sample-bank"),
        (Role::Unknown, "unknown"),
    ] {
        assert_eq!(serde_json::to_value(role).unwrap(), expected);
        // The catalog stores the same spelling it sends, so a round trip has to hold.
        assert_eq!(Role::parse(role.as_str()), role);
        assert_eq!(role.as_str(), expected);
    }
}

/// Scoping to one file is what the sidebar does when a `.svd` inside a source is clicked.
#[test]
fn a_query_can_scope_to_a_single_file() {
    let query: Query = serde_json::from_value(json!({
        "search": "",
        "kind": null,
        "source_id": null,
        "file_id": 7,
        "tags": [],
    }))
    .unwrap();
    assert_eq!(query.file_id, Some(7));
    assert_eq!(query.source_id, None);
}

/// The zone table keys an icon, a colour, and a tooltip off each state, so every variant has to
/// serialise to the string its `ZoneState` union in `api.ts` lists.
#[test]
fn every_zone_state_serialises_to_its_front_end_name() {
    use fantom_core::model::ZoneState;
    for (state, expected) in [
        (ZoneState::On, "on"),
        (ZoneState::Muted, "muted"),
        (ZoneState::Grouped, "grouped"),
        (ZoneState::Off, "off"),
        (ZoneState::Unused, "unused"),
    ] {
        assert_eq!(state.as_str(), expected);
    }
}

/// A zone's group membership travels with the zone, and the scene's groups with the scene, so the
/// table can colour a row without cross-referencing.
#[test]
fn a_scene_detail_carries_its_keyboard_groups() {
    let detail = SceneDetail {
        requirements: Requirements::default(),
        bpm: 120.0,
        level: 100,
        active_zones: 2,
        zones: vec![ZoneDetail {
            number: 3,
            enabled: false,
            muted: false,
            state: "grouped".into(),
            groups: vec![2, 5],
            engine: "ZEN-Core".into(),
            bank: "USER".into(),
            tone: "Sub Bass".into(),
            msb: 87,
            lsb: 0,
            pc: 8,
            key_low: 0,
            key_high: 57,
            velocity_low: 1,
            velocity_high: 127,
            level: 85,
            pan: 0,
            transpose: 0,
            octave: 0,
            midi_channel: 3,
            arpeggio: false,
        }],
        engines: vec!["ZEN-Core".into()],
        groups: vec![KeyboardGroupDetail {
            number: 2,
            zones: vec![3, 4],
        }],
        user_tones: vec!["Sub Bass".into()],
        external_refs: Vec::new(),
    };

    let json = serde_json::to_value(&detail).unwrap();
    assert_eq!(json["groups"][0]["number"], 2);
    assert_eq!(json["groups"][0]["zones"][1], 4);
    assert_eq!(json["zones"][0]["state"], "grouped");
    assert_eq!(json["zones"][0]["groups"][0], 2);
    // A grouped zone is switched off, and still counts as a dependency.
    assert_eq!(json["zones"][0]["enabled"], false);
    assert_eq!(json["user_tones"][0], "Sub Bass");
}

/// The header, the development warning, and the upgrade notice all read from one payload.
#[test]
fn workspace_info_tells_the_front_end_which_installation_it_is() {
    let info = fantom_desktop_lib::WorkspaceInfo {
        path: "/Users/me/Documents/FANTOM Librarian".into(),
        name: "FANTOM Librarian".into(),
        installation: fantom_desktop_lib::Installation::Development,
        upgrade: None,
        stats: Stats {
            scenes: 1,
            tones: 2,
            sources: 3,
            songs: 4,
            samples: 5,
        },
    };

    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(
        keys(&json),
        sorted(["path", "name", "installation", "upgrade", "stats"])
    );
    // The two installations are told apart by name, not by a flag the UI has to interpret.
    assert_eq!(json["installation"], "development");
    assert_eq!(
        serde_json::to_value(fantom_desktop_lib::Installation::Personal).unwrap(),
        "personal"
    );
    assert!(json["upgrade"].is_null());
}

/// After an upgrade the app has to be able to say where the copy of the old library went.
#[test]
fn an_upgrade_crosses_with_the_backup_it_took() {
    let info = fantom_desktop_lib::WorkspaceInfo {
        path: "/Users/me/Documents/FANTOM Librarian".into(),
        name: "FANTOM Librarian".into(),
        installation: fantom_desktop_lib::Installation::Personal,
        upgrade: Some(Upgrade {
            from_format: 1,
            to_format: 2,
            backup_path: "/Users/me/Documents/FANTOM Librarian backup 2026-09-02 201530".into(),
        }),
        stats: Stats {
            scenes: 0,
            tones: 0,
            sources: 0,
            songs: 0,
            samples: 0,
        },
    };

    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(
        keys(&json["upgrade"]),
        sorted(["from_format", "to_format", "backup_path"])
    );
    assert_eq!(json["upgrade"]["from_format"], 1);
    assert_eq!(json["upgrade"]["to_format"], 2);
    assert_eq!(
        json["upgrade"]["backup_path"],
        "/Users/me/Documents/FANTOM Librarian backup 2026-09-02 201530"
    );
}
