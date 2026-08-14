//! The JSON the front end actually receives.
//!
//! `desktop/src/lib/api.ts` hand-mirrors these types. Nothing makes the two agree at compile
//! time, so this pins the field names and tagging a renamed Rust field would silently break.

use fantom_core::role::Role;
use fantom_library::catalog::Stats;
use fantom_library::model::*;
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
            user_tones: vec!["Mk1 Rhodes".into()],
            external_refs: Vec::new(),
        }),
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
    });
    let json = serde_json::to_value(&detail).unwrap();
    assert_eq!(json["kind"], "tone");
    assert_eq!(json["area"], "MDLa");
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
