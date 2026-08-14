//! Taking files into the library.
//!
//! An import is one transaction over one source group. It copies rather than moves, validates
//! before cataloguing, and reports everything it declined to do rather than dropping it quietly.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use fantom_core::container::{Raw, Svd};
use fantom_core::model::{Scene, ToneRef};
use fantom_core::{codec, container, verify};
use rusqlite::params;
use sha2::{Digest, Sha256};

use crate::model::*;
use crate::workspace::{Workspace, ORIGINALS_DIR};
use crate::{now, Error, Result};

/// Extensions v1 takes in. `.sdz` is Roland Cloud content and explicitly out of scope.
const ACCEPTED: [&str; 2] = ["svd", "svz"];

/// Import `paths` — files, folders, or a mix — as a single source group.
///
/// The whole import is one transaction: if cataloguing fails the catalog is left untouched, though
/// any bytes already copied into managed storage stay, harmlessly, as unreferenced content.
pub fn import(ws: &mut Workspace, paths: &[PathBuf], info: &SourceInfo) -> Result<ImportReport> {
    let mut warnings = Vec::new();
    let candidates = collect(paths, &mut warnings)?;
    if candidates.is_empty() {
        return Err(Error::Rejected(
            "no .svd or .svz files found in the selection".into(),
        ));
    }

    let name = match info.name.trim() {
        "" => default_source_name(paths, &candidates),
        given => given.to_string(),
    };
    let origin = paths
        .first()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let root = ws.root().to_path_buf();
    let imported_at = now();

    let tx = ws.db_mut().transaction()?;
    tx.execute(
        "INSERT INTO sources (name, vendor, url, licence_note, note, origin_path, imported_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            name,
            info.vendor.trim(),
            info.url.trim(),
            info.licence_note.trim(),
            info.note.trim(),
            origin,
            imported_at
        ],
    )?;
    let source_id = tx.last_insert_rowid();

    let mut report = ImportReport {
        source_id,
        source_name: name.clone(),
        files_imported: 0,
        files_skipped: 0,
        files_invalid: 0,
        scenes_added: 0,
        tones_added: 0,
        assets_consolidated: 0,
        samples_catalogued: 0,
        warnings,
    };

    for path in &candidates {
        // A pack of per-scene folders is all `FANTOM.SVD`; only the path inside the import tells
        // them apart, so that is what the library shows.
        let display_name = relative_name(paths, path);
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                report
                    .warnings
                    .push(format!("{}: could not read ({e})", path.display()));
                continue;
            }
        };
        let hash = hex(&Sha256::digest(&bytes));

        // Byte-identical content already in this source is the same file arriving twice.
        let already: Option<i64> = tx
            .query_row(
                "SELECT id FROM files WHERE source_id = ?1 AND content_hash = ?2",
                params![source_id, hash],
                |row| row.get(0),
            )
            .ok();
        if already.is_some() {
            report.files_skipped += 1;
            continue;
        }

        let stored = store(&root, &hash, path, &bytes)?;
        let raw = Raw::from_bytes(bytes);

        let (status, problems) = validate(&raw);
        if status == FileStatus::Invalid {
            report.files_invalid += 1;
            for problem in &problems {
                report
                    .warnings
                    .push(format!("{}: {problem}", file_name(path)));
            }
        }

        tx.execute(
            "INSERT INTO files
               (source_id, file_name, origin_path, content_hash, size, stored_path, kind, role,
                status, problems)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                source_id,
                display_name,
                path.display().to_string(),
                hash,
                raw.len() as i64,
                stored,
                extension(path),
                fantom_core::role::of(&raw).as_str(),
                status.as_str(),
                serde_json::to_string(&problems).unwrap_or_else(|_| "[]".into()),
            ],
        )?;
        let file_id = tx.last_insert_rowid();
        report.files_imported += 1;

        // An invalid file stays visible with its report but contributes nothing usable.
        if status == FileStatus::Invalid {
            continue;
        }
        catalogue(&tx, file_id, &raw, imported_at, &display_name, &mut report);
    }

    tx.commit()?;
    tidy(&mut report.warnings);
    Ok(report)
}

/// Most warnings repeat across a large import. Collapse them and cap the list, keeping a final
/// line that says how much was left out rather than truncating in silence.
fn tidy(warnings: &mut Vec<String>) {
    const KEEP: usize = 40;
    let mut seen = BTreeSet::new();
    warnings.retain(|w| seen.insert(w.clone()));
    if warnings.len() > KEEP {
        let hidden = warnings.len() - KEEP;
        warnings.truncate(KEEP);
        warnings.push(format!("… and {hidden} more"));
    }
}

/// Read one file's scenes, user tones, and sample slots into the catalog.
fn catalogue(
    tx: &rusqlite::Transaction<'_>,
    file_id: i64,
    raw: &Raw,
    at: i64,
    name: &str,
    report: &mut ImportReport,
) {
    let before = report.scenes_added
        + report.tones_added
        + report.assets_consolidated
        + report.samples_catalogued;

    // A file legitimately holds no scenes — a tone bank or a sample companion has no PRFa area —
    // so a missing scene area is only worth reporting if nothing else turned up either.
    if let (Ok(scenes), Ok(records)) = (codec::read_scenes(raw), codec::read_scene_records(raw)) {
        for (i, scene) in scenes.iter().enumerate() {
            if codec::is_placeholder_name(&scene.name) {
                continue;
            }
            let Some(record) = records.get(i) else {
                continue;
            };
            let detail = scene_detail(scene);
            let candidate = Candidate {
                kind: AssetKind::Scene,
                // Fingerprinted rather than hashed raw: a scene whose user tones were renumbered
                // by repackaging is still the same scene.
                identity: identity_hash("scene", b"PRFa", &codec::scene_fingerprint(raw, record)),
                name: scene.name.clone(),
                memo: scene.comment.clone(),
                engine: detail
                    .engines
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "—".to_string()),
                detail: AssetDetail::Scene(detail),
            };
            match upsert_asset(tx, &candidate, at) {
                Ok((asset_id, fresh)) => {
                    if fresh {
                        report.scenes_added += 1;
                    } else {
                        report.assets_consolidated += 1;
                    }
                    link(tx, asset_id, file_id, (i + 1) as i64, "PRFa", &scene.name);
                }
                Err(e) => {
                    report
                        .warnings
                        .push(format!("{name}: scene {} \"{}\": {e}", i + 1, scene.name))
                }
            }
        }
    }

    if let (Ok(tones), Ok(svd)) = (codec::read_bundled_tones(raw), Svd::parse(raw)) {
        for tone in &tones {
            if codec::is_placeholder_name(&tone.name) {
                continue;
            }
            let Some(record) = container::RecordTable::from_svd(raw, &svd, &tone.area)
                .ok()
                .flatten()
                .and_then(|table| table.record(tone.index).map(<[u8]>::to_vec))
            else {
                continue;
            };
            let area = String::from_utf8_lossy(&tone.area).to_string();
            let engine = tone.tone_type.label().to_string();
            let candidate = Candidate {
                kind: AssetKind::Tone,
                identity: identity_hash("tone", &tone.area, &record),
                name: tone.name.clone(),
                memo: String::new(),
                detail: AssetDetail::Tone(ToneDetail {
                    engine: engine.clone(),
                    area: area.clone(),
                    index: tone.index,
                }),
                engine,
            };
            match upsert_asset(tx, &candidate, at) {
                Ok((asset_id, fresh)) => {
                    if fresh {
                        report.tones_added += 1;
                    } else {
                        report.assets_consolidated += 1;
                    }
                    link(tx, asset_id, file_id, tone.index as i64, &area, &tone.name);
                }
                Err(e) => report
                    .warnings
                    .push(format!("{name}: tone \"{}\": {e}", tone.name)),
            }
        }
    }

    if let Ok(svd) = Svd::parse(raw) {
        if let Ok(bank) = container::read_samples(raw, &svd) {
            for slot in &bank.slots {
                let data = bank.data.iter().find(|d| d.slot as usize == slot.index);
                let inserted = tx.execute(
                    "INSERT OR IGNORE INTO samples
                       (file_id, slot, name, frames, seconds, original_key, has_audio)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        file_id,
                        slot.index as i64,
                        slot.name,
                        slot.end as i64,
                        data.map(|d| d.seconds()).unwrap_or(0.0),
                        slot.original_key,
                        data.is_some() as i64,
                    ],
                );
                if matches!(inserted, Ok(1)) {
                    report.samples_catalogued += 1;
                }
            }
            for orphan in bank.orphans() {
                report.warnings.push(format!("{name}: {orphan}"));
            }
        }
    }

    if report.scenes_added
        + report.tones_added
        + report.assets_consolidated
        + report.samples_catalogued
        == before
    {
        report
            .warnings
            .push(format!("{name}: no scenes, tones, or samples found"));
    }
}

/// One decoded record, ready to become a library item.
struct Candidate {
    kind: AssetKind,
    identity: String,
    name: String,
    memo: String,
    engine: String,
    detail: AssetDetail,
}

/// Insert the asset, or find the existing canonical item with the same bytes.
///
/// Returns the id and whether this import created it. An existing item keeps the name the user
/// may since have edited; only its occurrences grow.
fn upsert_asset(
    tx: &rusqlite::Transaction<'_>,
    candidate: &Candidate,
    at: i64,
) -> Result<(i64, bool)> {
    if let Ok(id) = tx.query_row(
        "SELECT id FROM assets WHERE identity_hash = ?1",
        [&candidate.identity],
        |row| row.get::<_, i64>(0),
    ) {
        return Ok((id, false));
    }
    tx.execute(
        "INSERT INTO assets
           (kind, identity_hash, fantom_name, imported_name, memo, engine, detail, created_at)
         VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7)",
        params![
            candidate.kind.as_str(),
            candidate.identity,
            candidate.name,
            candidate.memo,
            candidate.engine,
            serde_json::to_string(&candidate.detail).unwrap_or_else(|_| "{}".into()),
            at
        ],
    )?;
    Ok((tx.last_insert_rowid(), true))
}

fn link(
    tx: &rusqlite::Transaction<'_>,
    asset_id: i64,
    file_id: i64,
    slot: i64,
    area: &str,
    name: &str,
) {
    let _ = tx.execute(
        "INSERT OR IGNORE INTO occurrences (asset_id, file_id, slot, area, name_at_import)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![asset_id, file_id, slot, area, name],
    );
}

/// Everything the library shows about a scene without reopening its file.
fn scene_detail(scene: &Scene) -> SceneDetail {
    let mut engines: Vec<String> = Vec::new();
    let mut user_tones: Vec<String> = Vec::new();
    let mut external: BTreeSet<String> = BTreeSet::new();
    let mut zones = Vec::with_capacity(scene.zones.len());

    for zone in &scene.zones {
        let tone = &zone.tone;
        let engine = tone.tone_type().label().to_string();
        let state = scene.zone_state(zone);
        let groups = scene.groups_containing(zone.number + 1);

        // A zone switched off today may be one pad press from sounding, so its tone is still a
        // dependency. Only a zone nobody ever configured is not — it still points at the factory
        // default it was born with.
        if state.is_played() {
            if !engines.contains(&engine) {
                engines.push(engine.clone());
            }
            match tone.bank() {
                Some("USER") => {
                    if let Some(name) = tone.name() {
                        let name = name.to_string();
                        if !user_tones.contains(&name) {
                            user_tones.push(name);
                        }
                    }
                }
                // Factory, expansion, and model content is a requirement the app never
                // substitutes; it has to survive into the install guide.
                _ => {
                    external.insert(reference(tone));
                }
            }
        }
        zones.push(ZoneDetail {
            number: zone.number + 1,
            enabled: zone.enabled,
            muted: zone.muted,
            state: state.as_str().to_string(),
            groups,
            engine,
            bank: tone.bank().unwrap_or("raw").to_string(),
            tone: tone.name().unwrap_or("—").to_string(),
            msb: tone.address.msb,
            lsb: tone.address.lsb,
            pc: tone.address.pc,
            key_low: zone.key_low,
            key_high: zone.key_high,
            velocity_low: zone.velocity_low,
            velocity_high: zone.velocity_high,
            level: zone.level,
            pan: zone.pan,
            transpose: zone.transpose,
            octave: zone.octave,
            midi_channel: zone.midi_channel + 1,
            arpeggio: zone.arpeggio,
        });
    }

    SceneDetail {
        bpm: scene.bpm(),
        level: scene.level,
        active_zones: scene.zones.iter().filter(|z| z.enabled).count(),
        zones,
        engines,
        groups: scene
            .groups
            .iter()
            .map(|group| KeyboardGroupDetail {
                number: group.number,
                zones: group.zones.clone(),
            })
            .collect(),
        user_tones,
        external_refs: external.into_iter().collect(),
    }
}

/// How a non-user tone reference reads in a dependency list.
fn reference(tone: &ToneRef) -> String {
    let bank = tone.bank().unwrap_or("raw");
    let name = tone.name().map(|n| format!(" \"{n}\"")).unwrap_or_default();
    format!(
        "{} {bank} PC {:03}{name}",
        tone.tone_type().label(),
        tone.address.pc
    )
}

/// Structure and record checksums. A file that cannot even be framed is invalid; a checksum
/// problem is reported but still lets the file be catalogued, since its records read fine.
fn validate(raw: &Raw) -> (FileStatus, Vec<String>) {
    match verify::check(raw) {
        Ok(report) => (
            FileStatus::Ok,
            report.problems.iter().map(|p| p.to_string()).collect(),
        ),
        Err(e) => (FileStatus::Invalid, vec![e.to_string()]),
    }
}

/// Copy the bytes into content-addressed managed storage, if they are not there already.
fn store(root: &Path, hash: &str, from: &Path, bytes: &[u8]) -> Result<String> {
    let shard = &hash[..2];
    let ext = extension(from);
    let relative = format!("{ORIGINALS_DIR}/{shard}/{hash}.{ext}");
    let target = root.join(&relative);
    if !target.exists() {
        let dir = target.parent().unwrap_or(root);
        fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
        fs::write(&target, bytes).map_err(|e| Error::io(&target, e))?;
    }
    Ok(relative)
}

/// Expand the selection into accepted files, noting what was passed over and why.
fn collect(paths: &[PathBuf], warnings: &mut Vec<String>) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for path in paths {
        if path.is_dir() {
            walk(path, &mut out, warnings)?;
        } else if accepted(path) {
            out.push(path.clone());
        } else {
            warnings.push(skipped(path));
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>, warnings: &mut Vec<String>) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|e| Error::io(dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| Error::io(dir, e))?;
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out, warnings)?;
        } else if accepted(&path) {
            out.push(path);
        } else if !is_noise(&path) {
            warnings.push(skipped(&path));
        }
    }
    Ok(())
}

fn skipped(path: &Path) -> String {
    if extension(path) == "sdz" {
        format!("{}: Roland Cloud .sdz is not supported", file_name(path))
    } else {
        format!("{}: not a .svd or .svz file", file_name(path))
    }
}

/// Files nobody wants told about: OS clutter, the readme that came with the pack, and the
/// sequencer and settings data a full backup carries alongside its sound material.
fn is_noise(path: &Path) -> bool {
    let name = file_name(path);
    name.starts_with('.')
        || matches!(
            extension(path).as_str(),
            "txt"
                | "pdf"
                | "md"
                | "rtf"
                | "png"
                | "jpg"
                | "jpeg"
                | "doc"
                | "docx"
                | "html"
                | "htm"
                | "xml"
                | "sqs"
                | "smp"
                | "exz"
                | "bin"
                | "dat"
                | "prm"
                | "log"
                | "wav"
                | "aif"
                | "aiff"
        )
}

fn accepted(path: &Path) -> bool {
    ACCEPTED.contains(&extension(path).as_str())
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// How a file is named within its import: its path relative to whichever selected folder contains
/// it, so `backup/TONEMAP4/FANTOM.SVD` reads as `TONEMAP4/FANTOM.SVD` rather than as one of five
/// indistinguishable `FANTOM.SVD`s.
fn relative_name(roots: &[PathBuf], path: &Path) -> String {
    roots
        .iter()
        .filter(|root| root.is_dir())
        .filter_map(|root| path.strip_prefix(root).ok())
        // The deepest matching root gives the shortest, least redundant name.
        .min_by_key(|relative| relative.components().count())
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| file_name(path))
}

/// Name an unnamed import after what it came from: the folder, or the single file.
fn default_source_name(paths: &[PathBuf], candidates: &[PathBuf]) -> String {
    paths
        .first()
        .filter(|p| p.is_dir())
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .or_else(|| {
            (candidates.len() == 1).then(|| {
                candidates[0]
                    .file_stem()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            })
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Imported files".to_string())
}

/// What makes two records the same library item: their kind, area, and stored bytes.
fn identity_hash(kind: &str, area: &[u8; 4], record: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(area);
    hasher.update(record);
    hex(&hasher.finalize())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}
