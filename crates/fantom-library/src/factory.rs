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
use rusqlite::Transaction;

use crate::model::{AssetDetail, AssetKind, ToneDetail};
use crate::workspace::Workspace;
use crate::{now, Result};

/// Add every built-in sound the catalog does not already hold, and report how many were added.
///
/// Idempotent: each sound is identified by its address, so a second run inserts nothing. Cheap
/// enough to call on every open — around four thousand `INSERT OR IGNORE`s in one transaction.
pub fn seed(ws: &mut Workspace) -> Result<usize> {
    // Read the workspace's own lists first: file I/O has no business inside the transaction, and
    // the borrow of `ws` for the database has to come after it.
    let installed = installed_lists(ws);
    let at = now();
    let tx = ws.db_mut().transaction()?;
    let mut added = 0;
    for sound in fantom_core::factory::all() {
        added += insert(&tx, &sound, at)?;
    }
    for list in &installed {
        for sound in fantom_core::factory::parse(list) {
            added += insert(&tx, &sound, at)?;
        }
    }
    tx.commit()?;
    Ok(added)
}

/// Every sound list dropped into the workspace's `sounds/` folder.
///
/// What a FANTOM has installed is a fact about that instrument, not about this program, so an
/// expansion's sounds arrive as a file — `dump-sounds` writes one, and so does
/// `tools/gen_sound_list.py` over an expansion's PDF. An unreadable or unparsable file is skipped:
/// a library still opens.
fn installed_lists(ws: &Workspace) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(ws.sounds_dir()) else {
        return Vec::new();
    };
    let mut lists: Vec<(std::path::PathBuf, String)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e.eq_ignore_ascii_case("tsv")))
        .filter_map(|path| std::fs::read_to_string(&path).ok().map(|text| (path, text)))
        .collect();
    // A stable order, so two lists claiming one address always resolve the same way.
    lists.sort_by(|a, b| a.0.cmp(&b.0));
    lists.into_iter().map(|(_, text)| text).collect()
}

fn insert(tx: &Transaction<'_>, sound: &FactorySound, at: i64) -> Result<usize> {
    let engine = sound.engine();
    // A drum kit is a kit rather than a tone, but the library browses both as tones, as the
    // instrument's own bank lists do.
    let detail = AssetDetail::Tone(ToneDetail {
        engine: engine.label().to_string(),
        area: String::new(),
        index: 0,
        bank: sound.bank().map(str::to_string),
        address: Some(sound.address),
        category: (!sound.category.is_empty()).then(|| sound.category.to_string()),
        model_id: None,
        requirements: Default::default(),
    });
    let rows = tx.execute(
        "INSERT OR IGNORE INTO assets
           (kind, identity_hash, fantom_name, imported_name, memo, engine, detail, origin,
            created_at)
         VALUES (?1, ?2, ?3, ?3, '', ?4, ?5, 'factory', ?6)",
        (
            AssetKind::Tone.as_str(),
            identity(sound),
            sound.name,
            engine.label(),
            serde_json::to_string(&detail).unwrap_or_else(|_| "{}".into()),
            at,
        ),
    )?;
    Ok(rows)
}

/// What makes a built-in sound itself: the address the instrument selects it by.
///
/// Deliberately not a hash of bytes, the way an imported record's identity is — there are no bytes.
/// Two instruments' `87/64/0` are the same sound, and a sound list corrected between firmware
/// versions must land on the row it replaces rather than beside it.
fn identity(sound: &FactorySound) -> String {
    format!(
        "factory:{}/{}/{}",
        sound.address.msb, sound.address.lsb, sound.address.pc
    )
}
