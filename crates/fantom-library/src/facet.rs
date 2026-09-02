//! What a list of assets can be narrowed by, beyond its scope and its tags.
//!
//! Three questions, asked of one asset: which engine is it, or does it play; which model or
//! expansion; and does it need anything the instrument does not already have. A scene answers from
//! the banks its zones point at, a tone from the record it is.
//!
//! Everything here reads the stored detail rather than the database, and the catalog applies it to
//! the rows a query returns rather than in SQL. A library is thousands of assets, not millions, so
//! the cost is nothing, and the alternative — this vocabulary half in Rust and half in JSON-path
//! SQL — is the kind of split that drifts.

use fantom_core::model::{model_label, ToneType};
use fantom_core::requirements::is_factory_bank;

use std::collections::HashSet;

use crate::model::{Asset, AssetDetail, Facet, Facets, Plays, Query};

/// The engine an asset is, or the dominant one it plays.
pub fn engine_of(asset: &Asset) -> Option<String> {
    // A scene of mixed engines stores `—`; that is not something to filter by.
    (!asset.engine.is_empty() && asset.engine != "—").then(|| asset.engine.clone())
}

/// The models and expansions an asset is, or plays: `MODEL JP8`, `SN-AP EXSN01`, `ZEN-Core PR-A`.
///
/// A tone is one model — the record says which. A scene is however many its zones reach for.
pub fn models_of(asset: &Asset) -> Vec<String> {
    match &asset.detail {
        // A built-in sound is its bank; a record in a file is whichever model it was saved from.
        AssetDetail::Tone(tone) => tone
            .bank
            .as_ref()
            .map(|bank| format!("{} {bank}", tone.engine))
            .or_else(|| tone.model_id.map(|id| model_name(&tone.engine, id)))
            .into_iter()
            .collect(),
        AssetDetail::Scene(scene) => {
            let mut out: Vec<String> = Vec::new();
            for reference in &scene.external_refs {
                let bank = bank_of(reference);
                if !bank.is_empty() && !out.contains(&bank) {
                    out.push(bank);
                }
            }
            out
        }
    }
}

/// A tone's model, named the way a scene names the same bank — `MODEL JP8`.
///
/// A selector nobody has confirmed reads as its number: it is real and it tells two models apart,
/// which is what a filter needs, and inventing a name for it would not.
fn model_name(engine: &str, id: u32) -> String {
    match ToneType::parse(engine).and_then(|kind| model_label(kind, id)) {
        Some(model) => format!("{engine} {model}"),
        None => format!("{engine} #{id}"),
    }
}

/// The `ENGINE BANK` an external reference opens with, dropping the `PC nnn "name"` that follows.
fn bank_of(reference: &str) -> String {
    reference
        .split_once(" PC ")
        .map(|(bank, _)| bank)
        .unwrap_or(reference)
        .to_string()
}

/// Whether an asset plays anywhere as its author heard it, where that can be decided.
///
/// A scene can be: it names every bank its zones point at, and it lists the bundled user tones it
/// needs. A tone cannot — what a record plays is its samples and waves, which is the requirements
/// report's question, not this one.
pub fn plays_of(asset: &Asset) -> Option<Plays> {
    let AssetDetail::Scene(scene) = &asset.detail else {
        return None;
    };
    let needs_user_tone = !scene.user_tones.is_empty();
    let needs_expansion = scene
        .external_refs
        .iter()
        .any(|reference| !is_factory_bank(bank_label(reference)));
    Some(if needs_user_tone || needs_expansion {
        Plays::NeedsYours
    } else {
        Plays::FactoryOnly
    })
}

/// Whether an asset depends on a named expansion that is absent from the instrument inventory.
///
/// A tone can name an expansion through its own bank or the waves its partials use. A scene can
/// do the same through its zones. Unknown bank addresses remain visible: without a product code,
/// the inventory cannot honestly say whether they are installed.
pub fn needs_uninstalled_expansion(asset: &Asset, installed: &HashSet<String>) -> bool {
    expansion_codes(asset)
        .into_iter()
        .any(|code| !installed.contains(&code.to_ascii_uppercase()))
}

fn expansion_codes(asset: &Asset) -> Vec<&str> {
    let (mut codes, requirements): (Vec<&str>, Option<_>) = match &asset.detail {
        AssetDetail::Tone(tone) => (
            tone.bank.iter().map(String::as_str).collect(),
            Some(&tone.requirements),
        ),
        AssetDetail::Scene(scene) => (
            scene
                .external_refs
                .iter()
                .map(|reference| bank_label(reference))
                .collect(),
            Some(&scene.requirements),
        ),
    };
    if let Some(requirements) = requirements {
        codes.extend(
            requirements
                .banks
                .iter()
                .filter_map(|bank| bank.bank.as_deref()),
        );
        codes.extend(
            requirements
                .wave_expansions
                .iter()
                .filter_map(|wave| wave.product.as_deref()),
        );
    }
    codes.retain(|code| fantom_core::expansions::is_product(code));
    codes
}

/// The bank alone — `PR-A`, `JP8` — out of an `ENGINE BANK PC nnn` reference.
fn bank_label(reference: &str) -> &str {
    let bank = reference
        .split_once(" PC ")
        .map(|(bank, _)| bank)
        .unwrap_or(reference);
    bank.split_once(' ').map(|(_, bank)| bank).unwrap_or(bank)
}

/// Whether an asset survives the facets a query sets. An empty facet asks nothing.
pub fn matches(asset: &Asset, query: &Query) -> bool {
    if !query.engines.is_empty()
        && !engine_of(asset).is_some_and(|engine| query.engines.contains(&engine))
    {
        return false;
    }
    if !query.models.is_empty() {
        let models = models_of(asset);
        if !models.iter().any(|model| query.models.contains(model)) {
            return false;
        }
    }
    if let Some(wanted) = query.origin {
        if asset.origin != wanted {
            return false;
        }
    }
    if let Some(wanted) = query.plays {
        if plays_of(asset) != Some(wanted) {
            return false;
        }
    }
    true
}

/// Every value these assets take, most used first, so the sidebar can offer them with their counts.
pub fn tally(assets: &[Asset]) -> Facets {
    let mut engines = Tally::default();
    let mut models = Tally::default();
    let mut origins = Tally::default();
    let mut plays = Tally::default();
    for asset in assets {
        if let Some(engine) = engine_of(asset) {
            engines.add(engine);
        }
        for model in models_of(asset) {
            models.add(model);
        }
        origins.add(asset.origin.as_str().to_string());
        if let Some(needs) = plays_of(asset) {
            plays.add(
                match needs {
                    Plays::FactoryOnly => "factory-only",
                    Plays::NeedsYours => "needs-yours",
                }
                .to_string(),
            );
        }
    }
    Facets {
        engines: engines.into_facets(),
        models: models.into_facets(),
        origins: origins.into_facets(),
        plays: plays.into_facets(),
    }
}

/// Counts in first-seen order, so a tie never reshuffles the sidebar between two loads.
#[derive(Default)]
struct Tally(Vec<Facet>);

impl Tally {
    fn add(&mut self, value: String) {
        match self.0.iter_mut().find(|facet| facet.value == value) {
            Some(facet) => facet.count += 1,
            None => self.0.push(Facet { value, count: 1 }),
        }
    }

    fn into_facets(mut self) -> Vec<Facet> {
        self.0
            .sort_by(|a, b| b.count.cmp(&a.count).then(a.value.cmp(&b.value)));
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AssetKind, Origin, SceneDetail, ToneDetail};

    fn scene(user_tones: &[&str], refs: &[&str]) -> Asset {
        asset(AssetDetail::Scene(SceneDetail {
            bpm: 120.0,
            level: 100,
            active_zones: 1,
            zones: Vec::new(),
            engines: Vec::new(),
            groups: Vec::new(),
            user_tones: user_tones.iter().map(|t| t.to_string()).collect(),
            external_refs: refs.iter().map(|r| r.to_string()).collect(),
            requirements: Default::default(),
        }))
    }

    fn tone(engine: &str, model_id: Option<u32>) -> Asset {
        let mut asset = asset(AssetDetail::Tone(ToneDetail {
            engine: engine.into(),
            area: "MDLa".into(),
            index: 0,
            bank: None,
            address: None,
            category: None,
            model_id,
            requirements: Default::default(),
        }));
        asset.engine = engine.into();
        asset.kind = AssetKind::Tone;
        asset
    }

    fn asset(detail: AssetDetail) -> Asset {
        Asset {
            id: 1,
            kind: AssetKind::Scene,
            fantom_name: "x".into(),
            imported_name: "x".into(),
            note: String::new(),
            memo: String::new(),
            engine: "ZEN-Core".into(),
            detail,
            origin: Origin::User,
            created_at: 0,
            archived_at: None,
            tags: Vec::new(),
            sources: Vec::new(),
        }
    }

    #[test]
    fn a_scene_offers_the_banks_its_zones_point_at() {
        let scene = scene(
            &[],
            &[
                "MODEL JP8 PC 010",
                "MODEL JP8 PC 011",
                "ZEN-Core PR-A PC 060 \"Ac Pop Piano 1\"",
            ],
        );
        // One entry per bank, however many zones reach for it, in the order first played.
        assert_eq!(models_of(&scene), ["MODEL JP8", "ZEN-Core PR-A"]);
    }

    #[test]
    fn a_built_in_sound_is_named_by_its_bank() {
        let mut sound = tone("ZEN-Core", None);
        if let AssetDetail::Tone(detail) = &mut sound.detail {
            detail.bank = Some("PR-A".into());
        }
        sound.origin = Origin::Factory;
        assert_eq!(models_of(&sound), ["ZEN-Core PR-A"]);
    }

    #[test]
    fn a_tone_offers_the_model_its_record_names() {
        // A confirmed selector reads as the bank a scene would call it by.
        assert_eq!(models_of(&tone("MODEL", Some(7))), ["MODEL JP8"]);
        // An unconfirmed one still tells its model from another, by number.
        assert_eq!(models_of(&tone("MODEL", Some(9))), ["MODEL #9"]);
        // Every other engine is one model, and says nothing.
        assert!(models_of(&tone("ZEN-Core", None)).is_empty());
    }

    #[test]
    fn a_scene_plays_anywhere_only_when_it_asks_for_nothing() {
        // Preset banks alone, and nothing bundled.
        assert_eq!(
            plays_of(&scene(&[], &["ZEN-Core PR-A PC 060", "Drum CMN PC 001"])),
            Some(Plays::FactoryOnly)
        );
        // An expansion has to be installed, so the scene is not self-sufficient.
        assert_eq!(
            plays_of(&scene(&[], &["SN-AP EXSN01 PC 002"])),
            Some(Plays::NeedsYours)
        );
        // Neither is one that needs a tone out of the user bank.
        assert_eq!(
            plays_of(&scene(&["Sledgehammer Sha"], &["ZEN-Core PR-A PC 060"])),
            Some(Plays::NeedsYours)
        );
        // A tone is user memory whatever it plays; the question does not apply to it.
        assert_eq!(plays_of(&tone("MODEL", Some(7))), None);
    }

    #[test]
    fn an_unconfirmed_bank_keeps_its_raw_label() {
        // `BankRequirement::label` writes `LSB 72` where the mapping is unknown; that is a bank
        // like any other here, and must not be read as the factory's.
        let scene = scene(&[], &["EXSN LSB 72 PC 003"]);
        assert_eq!(models_of(&scene), ["EXSN LSB 72"]);
        assert_eq!(plays_of(&scene), Some(Plays::NeedsYours));
    }
}
