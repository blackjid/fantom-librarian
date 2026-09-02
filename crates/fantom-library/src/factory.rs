//! Putting the instrument's own sounds in the catalog.
//!
//! A library built from files knows every sound the user has and none of the ones their FANTOM was
//! born with — so a scene can call for `MODEL SH101` and the tone list has nothing to show. These
//! rows close that: one asset per built-in sound, from the lists in [`fantom_core::factory`],
//! marked [`Origin::Factory`] and carrying no occurrence, because no file holds them.
//!
//! They are ordinary rows otherwise. Tagging one, noting it, linking it to a song, hiding it — all
//! work; only renaming is refused, since the name is Roland's.

use fantom_core::factory::FactorySound;
use rusqlite::{params, Transaction};

use crate::model::{AssetDetail, AssetKind, ToneDetail};
use crate::workspace::Workspace;
use crate::{now, Result};

/// Add every bundled factory and expansion sound the catalog does not already hold, and report
/// how many were added.
///
/// Idempotent: each sound is identified by its address, so a second run inserts nothing. Cheap
/// enough to call on every open. Inventory is deliberately not consulted here: uninstalling an
/// expansion must not discard a factory row the player has annotated or linked to a song. The
/// catalog query decides whether a row needs an expansion that is not currently installed.
pub fn seed(ws: &mut Workspace) -> Result<usize> {
    let at = now();
    let tx = ws.db_mut().transaction()?;
    migrate_legacy_expansion_rows(&tx)?;
    let mut added = 0;
    for sound in fantom_core::factory::all() {
        added += insert(&tx, &sound, None, at)?;
    }
    for sound in fantom_core::expansions::all() {
        added += insert(&tx, &sound.sound, Some(sound.product), at)?;
    }
    tx.commit()?;
    Ok(added)
}

fn insert(
    tx: &Transaction<'_>,
    sound: &FactorySound,
    product: Option<&str>,
    at: i64,
) -> Result<usize> {
    let engine = sound.engine();
    let detail = detail(sound, product);
    let rows = tx.execute(
        "INSERT OR IGNORE INTO assets
           (kind, identity_hash, fantom_name, imported_name, memo, engine, detail, origin,
            created_at)
         VALUES (?1, ?2, ?3, ?3, '', ?4, ?5, 'factory', ?6)",
        (
            AssetKind::Tone.as_str(),
            identity(sound, product),
            sound.name,
            engine.label(),
            serde_json::to_string(&detail).unwrap_or_else(|_| "{}".into()),
            at,
        ),
    )?;
    Ok(rows)
}

fn detail(sound: &FactorySound, product: Option<&str>) -> AssetDetail {
    let engine = sound.engine();
    // A drum kit is a kit rather than a tone, but the library browses both as tones, as the
    // instrument's own bank lists do.
    AssetDetail::Tone(ToneDetail {
        engine: engine.label().to_string(),
        area: String::new(),
        index: 0,
        // Expansion products can occupy the same address on different instruments, so the
        // product catalog is the only truthful bank label here.
        bank: product
            .map(str::to_string)
            .or_else(|| sound.bank().map(str::to_string)),
        address: Some(sound.address),
        category: (!sound.category.is_empty()).then(|| sound.category.to_string()),
        model_id: None,
        requirements: Default::default(),
    })
}

/// Move rows seeded from the retired `sounds/` cache onto the product-qualified identity now used
/// by bundled catalogs. A name and address must identify exactly one product; ambiguous old rows
/// stay untouched rather than risking a wrong merge.
fn migrate_legacy_expansion_rows(tx: &Transaction<'_>) -> Result<()> {
    let mut statement = tx.prepare(
        "SELECT id, identity_hash, fantom_name FROM assets
          WHERE origin = 'factory' AND identity_hash GLOB 'factory:[0-9]*/*/*'",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;

    for (old_id, old_identity, name) in rows {
        let Some(address) = legacy_address(&old_identity) else {
            continue;
        };
        let matches: Vec<_> = fantom_core::expansions::all()
            .filter(|entry| entry.sound.address == address && entry.sound.name == name)
            .collect();
        let Some(entry) = matches
            .first()
            .filter(|first| matches.iter().all(|other| other.product == first.product))
        else {
            continue;
        };
        let new_identity = identity(&entry.sound, Some(entry.product));
        let target = tx
            .query_row(
                "SELECT id FROM assets WHERE identity_hash = ?1",
                [&new_identity],
                |row| row.get::<_, i64>(0),
            )
            .ok();
        match target {
            Some(new_id) if new_id != old_id => {
                merge_metadata(tx, old_id, new_id)?;
                tx.execute("DELETE FROM assets WHERE id = ?1", [old_id])?;
            }
            _ => {
                tx.execute(
                    "UPDATE assets SET identity_hash = ?2, detail = ?3 WHERE id = ?1",
                    params![
                        old_id,
                        new_identity,
                        serde_json::to_string(&detail(&entry.sound, Some(entry.product)))
                            .unwrap_or_else(|_| "{}".into()),
                    ],
                )?;
            }
        }
    }
    Ok(())
}

fn legacy_address(identity: &str) -> Option<fantom_core::model::ToneAddress> {
    let mut pieces = identity.strip_prefix("factory:")?.split('/');
    let address = fantom_core::model::ToneAddress {
        msb: pieces.next()?.parse().ok()?,
        lsb: pieces.next()?.parse().ok()?,
        pc: pieces.next()?.parse().ok()?,
    };
    pieces.next().is_none().then_some(address)
}

/// Consolidate every user-owned edge before an old duplicate row is removed.
fn merge_metadata(tx: &Transaction<'_>, old_id: i64, new_id: i64) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO asset_tags (asset_id, tag_id)
         SELECT ?2, tag_id FROM asset_tags WHERE asset_id = ?1",
        params![old_id, new_id],
    )?;
    tx.execute(
        "INSERT INTO song_assets (song_id, asset_id, note)
         SELECT song_id, ?2, note FROM song_assets WHERE asset_id = ?1
         ON CONFLICT (song_id, asset_id) DO UPDATE SET note = CASE
           WHEN song_assets.note = '' OR song_assets.note = excluded.note THEN song_assets.note
           WHEN excluded.note = '' THEN song_assets.note
           ELSE song_assets.note || '\n\nMigrated link note: ' || excluded.note END",
        params![old_id, new_id],
    )?;
    tx.execute(
        "UPDATE assets SET note = CASE
           WHEN note = '' THEN (SELECT note FROM assets WHERE id = ?1)
           WHEN (SELECT note FROM assets WHERE id = ?1) = '' THEN note
           WHEN note = (SELECT note FROM assets WHERE id = ?1) THEN note
           ELSE note || '\n\nMigrated note: ' || (SELECT note FROM assets WHERE id = ?1) END
         WHERE id = ?2",
        params![old_id, new_id],
    )?;
    Ok(())
}

/// What makes a built-in sound itself: the address the instrument selects it by.
///
/// Deliberately not a hash of bytes, the way an imported record's identity is — there are no bytes.
/// A base sound is identified by its address, while an expansion sound additionally needs its
/// product: products can occupy the same address on different instruments. Keeping the original
/// base identity also lets an upgraded library retain annotations on rows it already seeded.
fn identity(sound: &FactorySound, product: Option<&str>) -> String {
    match product {
        Some(product) => format!(
            "factory:{product}/{}/{}/{}",
            sound.address.msb, sound.address.lsb, sound.address.pc
        ),
        None => format!(
            "factory:{}/{}/{}",
            sound.address.msb, sound.address.lsb, sound.address.pc
        ),
    }
}
