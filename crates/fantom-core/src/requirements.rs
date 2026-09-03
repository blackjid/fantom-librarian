//! What a file needs from wherever it is loaded, as data rather than prose.
//!
//! A bank names things it does not carry. A zone can point at a factory sound, at an installed
//! expansion, or at a user sample by *slot number* — and none of those travel in the file. The
//! failure that causes is the quiet kind: the bytes load without error and play a different
//! instrument than their author heard. Text in a terminal cannot prevent that; a value can.
//!
//! [`Requirements`] is the dependency closure of one piece of material, in one serialisable shape,
//! so a preflight check before an import, the facets on a browse page, an export's manifest, and
//! the "needs EXZ007" line of an install guide all read the same list instead of each
//! rediscovering it. Three scopes share the type — a whole file, one scene, one bundled tone — and
//! the two narrow ones are what a library shows per asset. Ask a [`Reader`] when the same file
//! answers many times, or the free functions ([`requirements`], [`scene_requirements`],
//! [`tone_requirements`]) for a one-shot answer.
//!
//! # What a destination can and cannot answer
//!
//! [`compare`] weighs a requirement list against an [`Inventory`], and is deliberately uneven about
//! it because its two halves come from different places. A backup names every occupied sample slot,
//! so those can be checked one by one. Nothing in any file has been found to list which expansions
//! an instrument has installed, so that half arrives as an [`ExpansionInventory`] the player keeps
//! by hand: with one, an expansion requirement reads as installed, owned but not loaded, or not
//! owned; without one it stays [`Verdict::Unknown`] — "here is what it needs, compare by hand" —
//! rather than becoming a guess. Silence is the dangerous answer here, so an address we cannot even
//! classify is reported too, as [`Requirements::unclassified`].

use std::collections::{BTreeMap, BTreeSet};

use crate::address::{self, AreaSpec};
use crate::codec;
use crate::container::{self, Raw, RecordTable, SampleBank, Svd};
use crate::expansions;
use crate::model::{Scene, ToneAddress, ToneRef, ToneType};
use crate::repackage::{MULTISAMPLE_AREAS, SAMPLE_REF_AREAS};
use crate::role::{self, Role};
use crate::{Error, Result};

/// Everything a file, a scene, or a tone asks of wherever it is loaded.
///
/// The lists are deduplicated and in a stable order: engines as first played, records and slots by
/// number. An empty [`Requirements`] is a genuine answer — material that plays only factory ROM
/// sounds it carries no reference to needs nothing at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Requirements {
    /// Sound engines the material plays, in the order first seen.
    pub engines: Vec<ToneType>,
    /// User tone records the scenes call for. `present` says whether this file carries each one;
    /// one that is missing is a hard error for an export, not a warning.
    pub user_tones: Vec<ToneRequirement>,
    /// Factory, expansion, and modelled banks a zone points at. These live in the instrument and
    /// are never substituted — they exist to be acknowledged and written into an install guide.
    pub banks: Vec<BankRequirement>,
    /// User sample slots the material plays, as 1-based panel numbers.
    pub samples: Vec<SlotRequirement>,
    /// User multisample slots the material plays, as 1-based panel numbers.
    pub multisamples: Vec<SlotRequirement>,
    /// Installed wave expansions a bundled tone's partials play waves from.
    ///
    /// The one requirement here that names a *product* rather than an address, which is why it is
    /// kept apart from `banks`: a wave group id travels in the tone and owes nothing to the bank
    /// slot the expansion happened to be installed at.
    pub wave_expansions: Vec<WaveExpansion>,
    /// Tone addresses belonging to no engine this version can name.
    ///
    /// Kept rather than dropped: an address we cannot classify may well need something, and
    /// omitting it silently is exactly the failure this type exists to prevent.
    pub unclassified: Vec<ToneAddress>,
    /// Whether the file carries user audio of its own — an `.svz` tone or sample bank does, a
    /// scene bank never does.
    pub carries_audio: bool,
}

impl Requirements {
    /// Whether nothing at all is required.
    pub fn is_empty(&self) -> bool {
        self.user_tones.is_empty()
            && self.banks.is_empty()
            && self.samples.is_empty()
            && self.multisamples.is_empty()
            && self.wave_expansions.is_empty()
            && self.unclassified.is_empty()
    }

    /// User tones the file does not carry: a zone points at a bank slot holding nothing, or
    /// holding the factory's `INITIAL` placeholder.
    pub fn missing_tones(&self) -> impl Iterator<Item = &ToneRequirement> {
        self.user_tones.iter().filter(|tone| !tone.present)
    }

    /// Sample slots whose audio this file does not carry, and which the destination must therefore
    /// already hold.
    pub fn missing_samples(&self) -> impl Iterator<Item = &SlotRequirement> {
        self.samples.iter().filter(|sample| !sample.carried)
    }

    /// Slots this file carries whose audio is silence.
    ///
    /// The material names a sample, the file holds it, and it plays nothing — the one way a
    /// package can be complete by every structural measure and still be empty.
    pub fn silent_samples(&self) -> impl Iterator<Item = &SlotRequirement> {
        self.samples.iter().filter(|sample| sample.silent)
    }

    /// Name the slots this material needs from a file that holds the directory naming them.
    ///
    /// A scene bank carries no slot table, so one rebuilt out of a backup knows the numbers it
    /// needs but not what was in them — while the backup it came from does. Passing that file's
    /// [`Inventory`] here is what turns "slot 22" into `"doh duh 2"`. Slots the other file cannot
    /// name are left unnamed rather than invented.
    pub fn named_from(mut self, held: &Inventory) -> Self {
        name_slots(&mut self.samples, &held.samples);
        name_slots(&mut self.multisamples, &held.multisamples);
        self
    }

    /// Banks that have to be installed before the material plays as its author heard it — the
    /// expansions, minus the factory presets every instrument already has.
    pub fn expansions(&self) -> impl Iterator<Item = &BankRequirement> {
        self.banks.iter().filter(|bank| !bank.is_factory())
    }

    /// Whether anything here can only be satisfied by content already installed on the
    /// instrument — the requirements an export has to make the user acknowledge.
    pub fn needs_installed_content(&self) -> bool {
        self.expansions().next().is_some()
            || !self.wave_expansions.is_empty()
            || !self.unclassified.is_empty()
    }
}

/// One user tone record the material needs, and whether the file carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ToneRequirement {
    /// Four-byte area tag the record lives in, as text.
    pub area: String,
    /// Zero-based record index within that area.
    pub index: usize,
    pub engine: ToneType,
    /// The record's name, when the file holds it.
    pub name: Option<String>,
    /// The address the zone stores, kept so a missing tone can still be named precisely.
    pub address: ToneAddress,
    /// Whether the file bundles the record — a placeholder `INITIAL` slot counts as missing.
    pub present: bool,
}

/// A factory, expansion, or modelled bank a zone points at. Content that lives in the instrument.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BankRequirement {
    pub engine: ToneType,
    /// Bank label when its byte mapping is confirmed — `EXZ007`, `EXSN01`, `JP8`, `PR-A`.
    pub bank: Option<String>,
    /// The sound's name, when the bundled sound list can name it.
    pub tone: Option<String>,
    pub address: ToneAddress,
}

impl BankRequirement {
    /// Whether this is factory content every FANTOM ships with, rather than something that has to
    /// be installed before the material plays correctly.
    ///
    /// The preset banks — `PR-A`..`PR-D`, `PRST`, `CMN` — are in every instrument, so listing them
    /// as install requirements is noise that hides the ones that matter. Everything else is
    /// treated as needing installation: the wave and SuperNATURAL expansions (`EXZ*`, `EXSN*`)
    /// plainly do, and the modelled banks (`JP8`, `JU106`, …) are downloads rather than shipped
    /// content. A bank whose mapping is unconfirmed is never assumed to be factory — the safe
    /// error is to mention something the destination turns out to have.
    pub fn is_factory(&self) -> bool {
        self.bank.as_deref().is_some_and(is_factory_bank)
    }

    /// The expansion product this asks for, when the bank names one.
    ///
    /// The bank label *is* the product code for everything that has to be installed, which is what
    /// lets a requirement be weighed against an [`ExpansionInventory`]. Factory banks name no
    /// product because nobody installs them, and `USER` names the instrument's own memory.
    pub fn product(&self) -> Option<&str> {
        self.bank
            .as_deref()
            .filter(|bank| !is_factory_bank(bank) && *bank != "USER")
    }

    /// How the requirement reads in a list. An unconfirmed bank shows its raw `LSB` rather than an
    /// invented name.
    pub fn label(&self) -> String {
        let bank = self
            .bank
            .clone()
            .unwrap_or_else(|| format!("LSB {}", self.address.lsb));
        let tone = self
            .tone
            .as_ref()
            .map(|name| format!(" \"{name}\""))
            .unwrap_or_default();
        format!(
            "{} {bank} PC {:03}{tone}",
            self.engine.label(),
            self.address.pc
        )
    }
}

/// Whether a bank label names content every FANTOM ships with.
///
/// Split out from [`BankRequirement::is_factory`] so a caller holding only the label — a catalog
/// filtering on stored text, say — decides it the same way. See that method for the reasoning.
pub fn is_factory_bank(bank: &str) -> bool {
    bank.starts_with("PR-") || bank == "PRST" || bank == "CMN"
}

/// An installed wave expansion a bundled tone's partials play waves from.
///
/// Two instruments can hold the same expansion at different bank slots, so a slot number says
/// nothing portable. The group id does: it is the product, and [`expansions::wave_group_product`]
/// turns it into the code an install guide names.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WaveExpansion {
    /// The wave group id exactly as the tone record stores it.
    pub id: u16,
    /// The product it decodes to — `EXZ005`. `None` for an id outside the decoded range, which is
    /// reported as itself rather than guessed at.
    pub product: Option<String>,
}

/// Read one back from a stored catalog, whichever shape it was written in.
///
/// A library stores this type as JSON, and rows written before the id was decoded hold a bare
/// number. Those rows must keep parsing: a catalog whose detail fails to deserialise falls back to
/// an empty one, so a shape change here would quietly empty every asset already imported. The
/// product is decoded from the id either way rather than trusted from the row, so a stored catalog
/// answers with this build's mapping instead of the one it was written with.
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for WaveExpansion {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Stored {
            Id(u16),
            Decoded { id: u16 },
        }
        Ok(Self::new(match Stored::deserialize(deserializer)? {
            Stored::Id(id) | Stored::Decoded { id } => id,
        }))
    }
}

impl WaveExpansion {
    /// Decode one stored wave group id.
    pub fn new(id: u16) -> Self {
        Self {
            id,
            product: expansions::wave_group_product(id).map(str::to_string),
        }
    }

    /// How it reads in a list: the product where the id decodes, the raw id where it does not.
    pub fn label(&self) -> String {
        match &self.product {
            Some(product) => format!("wave expansion {product}"),
            None => format!("wave expansion, group id {}", self.id),
        }
    }
}

/// One user sample or multisample slot the material plays.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlotRequirement {
    /// 1-based panel slot number, as a tone stores it.
    pub slot: u16,
    /// The slot's name, when this file carries the directory naming it.
    pub name: Option<String>,
    /// Whether this file carries the content for the slot. A scene bank never does; an `.svz`
    /// tone export carries the audio of every sample its tones play.
    pub carried: bool,
    /// The file holds this slot's audio and it is nothing but zeros.
    ///
    /// Deleting a sample on the instrument wipes the waveform and keeps the slot's directory
    /// entry, so a bank can carry a full-length, correctly named sample that plays nothing. Only
    /// the bytes say so, which is why it is read here rather than trusted to a flag.
    #[cfg_attr(feature = "serde", serde(default))]
    pub silent: bool,
    /// Names of the bundled tones that play it, for a report that says which sound goes silent.
    pub played_by: Vec<String>,
}

/// Reads requirements out of one open file.
///
/// Answering per asset means asking the same file hundreds of times — a backup holds 512 scenes
/// and thousands of tones — and every answer needs the area table and the sample directory. This
/// parses both once, which is the difference between an import that takes a second and one that
/// takes a minute. The free functions below are the one-shot form of the same thing.
pub struct Reader<'a> {
    raw: &'a Raw,
    svd: Svd,
    bank: SampleBank,
}

impl<'a> Reader<'a> {
    pub fn open(raw: &'a Raw) -> Result<Self> {
        let svd = Svd::parse(raw)?;
        // A file with no sample areas answers with an empty bank rather than an error: a scene
        // export has none by definition, and that is not a fault in it.
        let bank = container::read_samples(raw, &svd).unwrap_or_default();
        Ok(Self { raw, svd, bank })
    }

    /// Everything the file needs: the closure over its scenes and over every record it bundles.
    ///
    /// For a scene export this is exactly what its scenes reference, since an export bundles
    /// nothing else. For a full backup it widens to the whole user bank the file holds, which is
    /// the honest reading of "what does this file need" for a file that is the whole instrument.
    pub fn file(&self) -> Result<Requirements> {
        let mut scan = self.scan();

        // A tone bank has no scenes at all, and a missing PRFa is not a fault in one.
        if self.svd.area(b"PRFa").is_some() {
            for scene in codec::read_scenes(self.raw)? {
                scan.scene(&scene);
            }
        }
        // Bundled records are followed whether or not a scene named them: in a tone bank they
        // *are* the material, and following them is what turns a tone's partials into slots.
        for tone in codec::read_bundled_tones(self.raw)? {
            if codec::is_placeholder_name(&tone.name) {
                continue;
            }
            if let Some(spec) = address::spec_for_tag(&tone.area) {
                scan.engine(spec.tone_type);
                scan.follow(spec, tone.index, &tone.name);
            }
        }
        Ok(scan.finish())
    }

    /// What one decoded scene needs — the per-asset answer a library shows.
    pub fn scene(&self, scene: &Scene) -> Requirements {
        let mut scan = self.scan();
        scan.scene(scene);
        scan.finish()
    }

    /// What one bundled tone needs: the samples, multisamples, and expansions its partials play.
    ///
    /// `area` is the record's own area tag, as [`codec::BundledTone`] reports it; a drum kit's
    /// paired instrument set is followed with it.
    pub fn tone(&self, area: &[u8; 4], index: usize) -> Result<Requirements> {
        let spec = address::spec_for_tag(area).ok_or_else(|| {
            Error::Unrecognized(format!("{} is not a user tone area", tag_str(area)))
        })?;
        let mut scan = self.scan();
        scan.engine(spec.tone_type);
        let name = record_name(self.raw, &self.svd, spec, index).unwrap_or_default();
        scan.follow(spec, index, &name);
        Ok(scan.finish())
    }

    /// What this file shows it holds, for weighing another file's requirements against it.
    ///
    /// Samples only: the expansions are a fact about the instrument, not about the file, so they
    /// are left for [`Inventory::with_expansions`] to add.
    pub fn inventory(&self) -> Inventory {
        Inventory {
            role: role::of(self.raw),
            samples: held(self.bank.slots.iter().map(|s| (s.index, &s.name))),
            multisamples: held(self.bank.multisamples.iter().map(|m| (m.index, &m.name))),
            expansions: None,
        }
    }

    fn scan(&self) -> Scan<'_> {
        Scan::new(self.raw, &self.svd, &self.bank)
    }
}

/// Everything one file needs. See [`Reader::file`].
pub fn requirements(raw: &Raw) -> Result<Requirements> {
    Reader::open(raw)?.file()
}

/// What one scene needs, by its 1-based number. See [`Reader::scene`].
pub fn scene_requirements(raw: &Raw, scene_number: usize) -> Result<Requirements> {
    let scenes = codec::read_scenes(raw)?;
    let scene = scenes.get(scene_number.wrapping_sub(1)).ok_or_else(|| {
        Error::Unrecognized(format!(
            "scene {scene_number} out of range (file has {})",
            scenes.len()
        ))
    })?;
    Ok(Reader::open(raw)?.scene(scene))
}

/// What one bundled tone needs. See [`Reader::tone`].
pub fn tone_requirements(raw: &Raw, area: &[u8; 4], index: usize) -> Result<Requirements> {
    Reader::open(raw)?.tone(area, index)
}

/// Accumulates a closure while walking scenes and records, deduplicating as it goes.
struct Scan<'a> {
    raw: &'a Raw,
    svd: &'a Svd,
    /// The file's own sample bank: both what it already carries and the names of the slots it
    /// merely references, since a backup names slots its scenes only point at.
    bank: &'a SampleBank,
    engines: Vec<ToneType>,
    tones: BTreeMap<([u8; 4], usize), ToneRequirement>,
    banks: Vec<BankRequirement>,
    samples: BTreeMap<u16, SlotRequirement>,
    multisamples: BTreeMap<u16, SlotRequirement>,
    wave_expansions: Vec<WaveExpansion>,
    unclassified: Vec<ToneAddress>,
    followed: BTreeSet<([u8; 4], usize)>,
}

impl<'a> Scan<'a> {
    fn new(raw: &'a Raw, svd: &'a Svd, bank: &'a SampleBank) -> Self {
        Self {
            raw,
            svd,
            bank,
            engines: Vec::new(),
            tones: BTreeMap::new(),
            banks: Vec::new(),
            samples: BTreeMap::new(),
            multisamples: BTreeMap::new(),
            wave_expansions: Vec::new(),
            unclassified: Vec::new(),
            followed: BTreeSet::new(),
        }
    }

    /// Add everything a scene needs.
    ///
    /// Zone state decides what counts, exactly as it does for packaging: a muted or switched-off
    /// zone is one control away from sounding, so its tone is still a dependency, while a zone
    /// nobody ever configured still points at the factory default it was born with and is not.
    fn scene(&mut self, scene: &Scene) {
        for zone in &scene.zones {
            if scene.zone_state(zone).is_played() {
                self.zone(&zone.tone);
            }
        }
    }

    fn zone(&mut self, tone: &ToneRef) {
        self.engine(tone.tone_type());
        let address = tone.address;
        match address::resolve(address.msb, address.lsb, address.pc) {
            Some((spec, index)) => {
                // A record named `INITIAL TONE` is an empty slot: the file resolves the address
                // but carries no sound there, which is a missing dependency, not a present one.
                let name = tone.name().filter(|name| !codec::is_placeholder_name(name));
                self.tones
                    .entry((spec.tag, index))
                    .or_insert_with(|| ToneRequirement {
                        area: spec.tag_str(),
                        index,
                        engine: spec.tone_type,
                        name: name.map(str::to_owned),
                        address,
                        present: name.is_some(),
                    });
                if let Some(name) = name {
                    self.follow(spec, index, name);
                }
            }
            // An unplaceable address is reported as itself; naming it a bank would be a guess.
            None if tone.tone_type() == ToneType::Unknown => {
                if !self.unclassified.contains(&address) {
                    self.unclassified.push(address);
                }
            }
            None => {
                let bank = BankRequirement {
                    engine: tone.tone_type(),
                    bank: tone.bank().map(str::to_owned),
                    tone: tone.name().map(str::to_owned),
                    address,
                };
                if !self.banks.contains(&bank) {
                    self.banks.push(bank);
                }
            }
        }
    }

    fn engine(&mut self, engine: ToneType) {
        if !self.engines.contains(&engine) {
            self.engines.push(engine);
        }
    }

    /// Follow one bundled record's own references, and those of the areas indexed in lockstep with
    /// it — a drum kit names its samples in the paired `INSa`, never in `RHYa`.
    fn follow(&mut self, spec: &AreaSpec, index: usize, played_by: &str) {
        if !self.followed.insert((spec.tag, index)) {
            return;
        }
        for tag in spec.paired {
            if SAMPLE_REF_AREAS.contains(tag) {
                self.record(tag, index, played_by);
            }
        }
    }

    fn record(&mut self, tag: &[u8; 4], index: usize, played_by: &str) {
        let (raw, svd) = (self.raw, self.svd);
        let Some(record) = RecordTable::from_svd(raw, svd, tag)
            .ok()
            .flatten()
            .and_then(|table| table.record(index))
        else {
            return;
        };
        for slot in container::sample_slots_of(tag, record) {
            self.need_sample(slot, played_by);
        }
        // Only a ZEN-Core tone stores the other two kinds of reference inline; a drum kit's
        // instrument set has no field for either.
        if tag == b"PATa" {
            for number in container::multisample_slots(record) {
                self.need_multisample(number, played_by);
            }
            for id in container::expansion_banks(record) {
                if !self.wave_expansions.iter().any(|played| played.id == id) {
                    self.wave_expansions.push(WaveExpansion::new(id));
                }
            }
        }
    }

    fn need_sample(&mut self, slot: u16, played_by: &str) {
        let bank = self.bank;
        let audio = bank
            .data
            .iter()
            .find(|audio| audio.slot as usize + 1 == slot as usize);
        let entry = self.samples.entry(slot).or_insert_with(|| SlotRequirement {
            slot,
            name: bank
                .slots
                .iter()
                .find(|held| held.index + 1 == slot as usize)
                .map(|held| held.name.clone()),
            carried: audio.is_some(),
            silent: audio.is_some_and(|audio| audio.silent),
            played_by: Vec::new(),
        });
        note_player(&mut entry.played_by, played_by);
    }

    /// A multisample is a dependency with dependencies of its own: a tone names it by number, and
    /// it names a sample per key. Those samples are needed as directly as the ones a partial
    /// points at, and nothing else in the file says so.
    fn need_multisample(&mut self, number: u16, played_by: &str) {
        let fresh = !self.multisamples.contains_key(&number);
        let held = self.held_multisample(number);
        let entry = self
            .multisamples
            .entry(number)
            .or_insert_with(|| SlotRequirement {
                slot: number,
                name: held.clone(),
                carried: held.is_some(),
                silent: false,
                played_by: Vec::new(),
            });
        note_player(&mut entry.played_by, played_by);

        if fresh {
            let (raw, svd) = (self.raw, self.svd);
            for tag in MULTISAMPLE_AREAS {
                let Some(record) = RecordTable::from_svd(raw, svd, tag)
                    .ok()
                    .flatten()
                    .and_then(|table| table.record(number as usize - 1))
                else {
                    continue;
                };
                for slot in container::sample_slots_of(tag, record) {
                    self.need_sample(slot, played_by);
                }
            }
        }
    }

    /// The name of a multisample this file actually defines, if it does.
    ///
    /// A backup's `MLSa` holds 128 records of which the untouched ones are the factory
    /// `INITIAL MSMPL`; a companion's `MSPa` holds only what it carries. Both are filtered and
    /// numbered by [`container::read_samples`], so this is a lookup rather than a second reader.
    fn held_multisample(&self, number: u16) -> Option<String> {
        self.bank
            .multisamples
            .iter()
            .find(|held| held.index + 1 == number as usize)
            .map(|held| held.name.clone())
    }

    fn finish(self) -> Requirements {
        Requirements {
            engines: self.engines,
            user_tones: self.tones.into_values().collect(),
            banks: self.banks,
            samples: self.samples.into_values().collect(),
            multisamples: self.multisamples.into_values().collect(),
            wave_expansions: {
                let mut played = self.wave_expansions;
                played.sort_unstable_by_key(|expansion| expansion.id);
                played
            },
            unclassified: self.unclassified,
            carries_audio: !self.bank.data.is_empty(),
        }
    }
}

fn name_slots(slots: &mut [SlotRequirement], held: &[HeldSlot]) {
    for slot in slots.iter_mut().filter(|slot| slot.name.is_none()) {
        slot.name = held
            .iter()
            .find(|candidate| candidate.slot == slot.slot)
            .map(|candidate| candidate.name.clone());
    }
}

fn note_player(players: &mut Vec<String>, name: &str) {
    if !name.is_empty() && !players.iter().any(|player| player == name) {
        players.push(name.to_string());
    }
}

fn record_name(raw: &Raw, svd: &Svd, spec: &AreaSpec, index: usize) -> Option<String> {
    let record = RecordTable::from_svd(raw, svd, &spec.tag)
        .ok()
        .flatten()
        .and_then(|table| table.record(index))?;
    let name = spec.decode_name(record.get(spec.name_offset..)?);
    (!codec::is_placeholder_name(&name)).then_some(name)
}

fn tag_str(tag: &[u8; 4]) -> String {
    String::from_utf8_lossy(tag).into_owned()
}

/// What a destination file shows it holds — the other half of a compatibility check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Inventory {
    /// What the file is for. A scene export carries no sample bank, so it can only say what it
    /// needs, never what an instrument holds.
    pub role: Role,
    /// Occupied user sample slots, 1-based.
    pub samples: Vec<HeldSlot>,
    /// Multisample slots the file defines, 1-based.
    pub multisamples: Vec<HeldSlot>,
    /// The expansions the destination has, when anyone has said. `None` is not "owns nothing": no
    /// file lists them, so an inventory only exists where a player kept one.
    pub expansions: Option<ExpansionInventory>,
}

/// One slot a destination holds, and what is in it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HeldSlot {
    pub slot: u16,
    pub name: String,
}

impl Inventory {
    /// Whether this file can speak for an instrument's sample bank at all.
    ///
    /// Only a backup dumps the whole memory, so only a backup's *silence* about a slot means the
    /// slot is empty. A scene export holding no sample areas says nothing, and reporting every
    /// requirement as missing against one would be a confident wrong answer.
    pub fn knows_samples(&self) -> bool {
        self.role == Role::Backup || !self.samples.is_empty()
    }

    /// Add what the player says their instrument has, which no file can say for them.
    pub fn with_expansions(mut self, expansions: ExpansionInventory) -> Self {
        self.expansions = Some(expansions);
        self
    }
}

/// Read what a destination file holds. See [`Reader::inventory`].
pub fn inventory(raw: &Raw) -> Result<Inventory> {
    Ok(Reader::open(raw)?.inventory())
}

fn held<'a>(slots: impl Iterator<Item = (usize, &'a String)>) -> Vec<HeldSlot> {
    slots
        .map(|(index, name)| HeldSlot {
            slot: index as u16 + 1,
            name: name.clone(),
        })
        .collect()
}

/// Which expansions a player owns, and which the instrument currently holds.
///
/// The one thing about a destination that no file states, so this arrives from wherever that
/// hand-kept note lives rather than from bytes. The two flags are independent; see
/// `fantom_library::catalog::expansions` for why.
///
/// Codes are addressed case-insensitively: a catalogued product keeps its own case (`n/zyme`) while
/// a hand-recorded one is whatever was typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ExpansionInventory {
    /// Product codes the player has bought, loaded or not.
    pub owned: BTreeSet<String>,
    /// Product codes the instrument currently holds.
    pub installed: BTreeSet<String>,
}

impl ExpansionInventory {
    /// Record one product, as the two flags a workspace stores.
    pub fn record(&mut self, product: &str, owned: bool, installed: bool) {
        let code = product.to_ascii_uppercase();
        if owned {
            self.owned.insert(code.clone());
        }
        if installed {
            self.installed.insert(code);
        }
    }

    /// Whether the instrument currently holds this product.
    ///
    /// The question a browse filter asks, and the first thing a verdict asks: what plays is what is
    /// loaded, whether or not anyone recorded buying it.
    pub fn is_installed(&self, product: &str) -> bool {
        self.installed.contains(&product.to_ascii_uppercase())
    }

    /// Whether nothing at all has been recorded — a note nobody has written in yet.
    pub fn is_empty(&self) -> bool {
        self.owned.is_empty() && self.installed.is_empty()
    }

    /// How one product stands: [`Verdict::Met`] when the instrument holds it,
    /// [`Verdict::NotLoaded`] when it is owned and has to be loaded, [`Verdict::Missing`] when it
    /// has to be bought. A lookup, so an unrecorded product reads as unowned; whether the note is
    /// worth believing is [`compare`]'s question.
    pub fn verdict(&self, product: &str) -> Verdict {
        match () {
            _ if self.is_installed(product) => Verdict::Met,
            _ if self.owned.contains(&product.to_ascii_uppercase()) => Verdict::NotLoaded,
            _ => Verdict::Missing,
        }
    }
}

/// How one requirement stands against a destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum Verdict {
    /// The destination provides it.
    Met,
    /// The destination does not have it.
    Missing,
    /// The player owns the expansion, but the instrument does not currently hold it.
    NotLoaded,
    /// Something is at that slot, but not this.
    Differs,
    /// The format cannot answer: compare by hand.
    Unknown,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Met => "met",
            Self::Missing => "missing",
            Self::NotLoaded => "not loaded",
            Self::Differs => "differs",
            Self::Unknown => "unknown",
        }
    }

    /// Whether this verdict should stop an import or fail a gate.
    ///
    /// An expansion sitting unloaded on the shelf is as unmet as one nobody owns: the material
    /// plays the wrong sound until someone acts. What differs is the instruction, not the verdict.
    pub fn is_problem(self) -> bool {
        matches!(self, Self::Missing | Self::Differs | Self::NotLoaded)
    }
}

/// One requirement weighed against a destination.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Finding {
    /// The requirement, as it reads in a report.
    pub requirement: String,
    pub verdict: Verdict,
    /// What the destination has to say about it, if anything.
    pub detail: String,
}

/// Weigh a requirement list against what a destination file holds.
///
/// Only what the destination has to *provide* is reported: a tone the file bundles and audio it
/// carries are already satisfied and would be noise. An empty result therefore means the
/// destination can play this material as far as its file can tell — which is not the same as
/// certainty, and is why unverifiable requirements are still listed as [`Verdict::Unknown`].
pub fn compare(needs: &Requirements, held: &Inventory) -> Vec<Finding> {
    let mut findings = Vec::new();

    for tone in needs.missing_tones() {
        findings.push(Finding {
            requirement: format!(
                "user tone {}[{}] (MSB {} LSB {} PC {:03})",
                tone.area, tone.index, tone.address.msb, tone.address.lsb, tone.address.pc
            ),
            verdict: Verdict::Missing,
            detail: "the file points at this slot but bundles no sound there".into(),
        });
    }

    for sample in needs.missing_samples() {
        let named = sample.name.as_deref();
        let there = held.samples.iter().find(|slot| slot.slot == sample.slot);
        let (verdict, detail) = match (held.knows_samples(), there, named) {
            (false, _, _) => (
                Verdict::Unknown,
                "that file carries no sample bank, so it cannot say what is in the slot".into(),
            ),
            (true, None, _) => (Verdict::Missing, "the slot is empty there".into()),
            (true, Some(slot), Some(name)) if slot.name == name => {
                (Verdict::Met, format!("holds {:?}", slot.name))
            }
            (true, Some(slot), Some(_)) => (Verdict::Differs, format!("holds {:?}", slot.name)),
            // Without a slot directory of its own the file cannot name what it needs, so a
            // populated slot on the other side proves nothing either way.
            (true, Some(slot), None) => (Verdict::Unknown, format!("holds {:?}", slot.name)),
        };
        findings.push(Finding {
            requirement: sample_label("sample", sample),
            verdict,
            detail,
        });
    }

    for multisample in needs.multisamples.iter().filter(|slot| !slot.carried) {
        let there = held
            .multisamples
            .iter()
            .find(|slot| slot.slot == multisample.slot);
        let (verdict, detail) = match there {
            Some(slot) => (Verdict::Met, format!("holds {:?}", slot.name)),
            None if held.role == Role::Backup => (
                Verdict::Missing,
                "that instrument defines no multisample there".into(),
            ),
            None => (
                Verdict::Unknown,
                "that file carries no multisamples, so it cannot say".into(),
            ),
        };
        findings.push(Finding {
            requirement: sample_label("multisample", multisample),
            verdict,
            detail,
        });
    }

    // Everything below lives in the instrument rather than in any file, so only the player's
    // inventory can confirm it — except the factory banks, which are in every instrument by
    // definition.
    for bank in &needs.banks {
        let (verdict, detail) = if bank.is_factory() {
            (Verdict::Met, "factory content, in every FANTOM".into())
        } else {
            weigh_expansion(bank.product(), held)
        };
        findings.push(Finding {
            requirement: bank.label(),
            verdict,
            detail,
        });
    }
    for expansion in &needs.wave_expansions {
        let (verdict, detail) = weigh_expansion(expansion.product.as_deref(), held);
        findings.push(Finding {
            requirement: expansion.label(),
            verdict,
            detail,
        });
    }
    for address in &needs.unclassified {
        findings.push(Finding {
            requirement: format!(
                "unclassified tone address MSB {} LSB {} PC {:03}",
                address.msb, address.lsb, address.pc
            ),
            verdict: Verdict::Unknown,
            detail: "no engine or bank this version can name".into(),
        });
    }

    findings
}

/// Weigh one installed-content requirement against the inventory, and say what to do about it.
///
/// Three ways to answer nothing but honestly, all reported as [`Verdict::Unknown`] rather than
/// guessed at: no inventory at all; an inventory nobody has written in yet, which is not a
/// statement that nothing is owned; and a requirement naming no product to look up — an address
/// whose bank is unconfirmed, or a wave group id past what this build decodes.
fn weigh_expansion(product: Option<&str>, held: &Inventory) -> (Verdict, String) {
    let (Some(product), Some(inventory)) = (product, held.expansions.as_ref()) else {
        return (Verdict::Unknown, INSTALLED_CONTENT.to_string());
    };
    if inventory.is_empty() {
        return (Verdict::Unknown, INSTALLED_CONTENT.to_string());
    }
    let verdict = inventory.verdict(product);
    let detail = match verdict {
        Verdict::Met => format!("{product} is installed"),
        Verdict::NotLoaded => format!("{product} is owned: load it into a slot"),
        Verdict::Missing | Verdict::Differs | Verdict::Unknown => {
            format!("{product} is not in the inventory")
        }
    };
    (verdict, detail)
}

/// Why an installed-content requirement is unanswerable: no area listing what an instrument has
/// installed has been identified, and no inventory was given to make up for it. Kept short because
/// a report repeats it per requirement.
const INSTALLED_CONTENT: &str = "no file lists this, and no inventory says";

fn sample_label(kind: &str, slot: &SlotRequirement) -> String {
    match &slot.name {
        Some(name) => format!("{kind} slot {} {name:?}", slot.slot),
        None => format!("{kind} slot {}", slot.slot),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_verdict_that_stops_a_gate_is_only_a_definite_one() {
        assert!(Verdict::Missing.is_problem());
        assert!(Verdict::Differs.is_problem());
        assert!(!Verdict::Met.is_problem());
        // Owned but not loaded is unmet: the sound does not play until someone loads it.
        assert!(Verdict::NotLoaded.is_problem());
        // The whole point of the honest answer: it informs without pretending to decide.
        assert!(!Verdict::Unknown.is_problem());
    }

    #[test]
    fn an_unconfirmed_bank_shows_its_raw_lsb_rather_than_a_name() {
        let named = BankRequirement {
            engine: ToneType::Exz,
            bank: Some("EXZ007".into()),
            tone: Some("Big Brass".into()),
            address: ToneAddress {
                msb: 93,
                lsb: 7,
                pc: 3,
            },
        };
        assert_eq!(named.label(), "EXZ EXZ007 PC 003 \"Big Brass\"");

        let unknown = BankRequirement {
            engine: ToneType::Acb,
            bank: None,
            tone: None,
            address: ToneAddress {
                msb: 107,
                lsb: 72,
                pc: 3,
            },
        };
        assert_eq!(unknown.label(), "ACB LSB 72 PC 003");
    }

    /// A destination that cannot speak must not be read as speaking. A scene export holds no
    /// sample bank, so every slot would otherwise come back "missing" — a confident wrong answer.
    #[test]
    fn a_file_with_no_sample_bank_answers_unknown_rather_than_missing() {
        let needs = Requirements {
            samples: vec![SlotRequirement {
                slot: 7,
                name: Some("Whoa".into()),
                carried: false,
                silent: false,
                played_by: vec!["Africa Brass".into()],
            }],
            ..Requirements::default()
        };

        let silent = Inventory {
            role: Role::SceneBank,
            ..Inventory::default()
        };
        let findings = compare(&needs, &silent);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].verdict, Verdict::Unknown);

        // An instrument backup with nothing in slot 7 *is* saying the slot is empty.
        let backup = Inventory {
            role: Role::Backup,
            ..Inventory::default()
        };
        assert_eq!(compare(&needs, &backup)[0].verdict, Verdict::Missing);
    }

    #[test]
    fn a_slot_holding_other_audio_differs_rather_than_being_met() {
        let needs = Requirements {
            samples: vec![SlotRequirement {
                slot: 3,
                name: Some("IML Whoa 1".into()),
                carried: false,
                silent: false,
                played_by: Vec::new(),
            }],
            ..Requirements::default()
        };
        let holds = |name: &str| Inventory {
            role: Role::Backup,
            samples: vec![HeldSlot {
                slot: 3,
                name: name.into(),
            }],
            ..Inventory::default()
        };

        assert_eq!(
            compare(&needs, &holds("IML Whoa 1"))[0].verdict,
            Verdict::Met
        );
        assert_eq!(
            compare(&needs, &holds("Kick 2"))[0].verdict,
            Verdict::Differs
        );
    }

    /// Audio the file brings with it is not something the destination has to provide.
    #[test]
    fn carried_samples_are_not_asked_of_the_destination() {
        let needs = Requirements {
            samples: vec![SlotRequirement {
                slot: 1,
                name: Some("Kick".into()),
                carried: true,
                silent: false,
                played_by: Vec::new(),
            }],
            carries_audio: true,
            ..Requirements::default()
        };
        assert!(compare(&needs, &Inventory::default()).is_empty());
    }

    /// The numbers survive a rebuild; the names do not, because a scene bank has no slot table.
    #[test]
    fn slots_can_be_named_from_the_file_the_material_came_out_of() {
        let needs = Requirements {
            samples: vec![SlotRequirement {
                slot: 22,
                name: None,
                carried: false,
                silent: false,
                played_by: Vec::new(),
            }],
            ..Requirements::default()
        };
        let source = Inventory {
            role: Role::Backup,
            samples: vec![HeldSlot {
                slot: 22,
                name: "doh duh 2".into(),
            }],
            ..Inventory::default()
        };
        assert_eq!(
            needs.named_from(&source).samples[0].name.as_deref(),
            Some("doh duh 2")
        );
    }

    /// The install guide has to name the expansions and not the presets: a FANTOM that cannot play
    /// `PR-A` does not exist, while one without `EXZ007` is the common case.
    #[test]
    fn factory_banks_are_told_from_expansions() {
        let bank = |label: &str| BankRequirement {
            engine: ToneType::ZenCore,
            bank: Some(label.into()),
            tone: None,
            address: ToneAddress {
                msb: 87,
                lsb: 64,
                pc: 0,
            },
        };
        for factory in ["PR-A", "PR-D", "PRST", "CMN"] {
            assert!(bank(factory).is_factory(), "{factory}");
        }
        for installed in ["EXZ007", "EXSN01", "JP8", "M09X01"] {
            assert!(!bank(installed).is_factory(), "{installed}");
        }
        // An unnamed bank is not assumed to be one the destination already has.
        assert!(!BankRequirement {
            engine: ToneType::Acb,
            bank: None,
            tone: None,
            address: ToneAddress {
                msb: 107,
                lsb: 72,
                pc: 0,
            },
        }
        .is_factory());
    }

    /// Installed content cannot be confirmed from any file — and saying so is the point, so it is
    /// reported rather than dropped.
    #[test]
    fn installed_content_is_reported_as_unverifiable_not_omitted() {
        let needs = Requirements {
            banks: vec![BankRequirement {
                engine: ToneType::Exz,
                bank: Some("EXZ007".into()),
                tone: None,
                address: ToneAddress {
                    msb: 93,
                    lsb: 7,
                    pc: 0,
                },
            }],
            wave_expansions: vec![WaveExpansion::new(1005), WaveExpansion::new(4)],
            unclassified: vec![ToneAddress {
                msb: 120,
                lsb: 3,
                pc: 9,
            }],
            ..Requirements::default()
        };
        let findings = compare(&needs, &Inventory::default());
        assert_eq!(findings.len(), 4);
        assert!(findings.iter().all(|f| f.verdict == Verdict::Unknown));
        assert!(needs.needs_installed_content());

        // A decoded id reads as the product an install guide has to name; an undecoded one still
        // reads, as the number it is.
        let labels: Vec<&str> = findings.iter().map(|f| f.requirement.as_str()).collect();
        assert!(labels.contains(&"wave expansion EXZ005"), "{labels:?}");
        assert!(labels.contains(&"wave expansion, group id 4"), "{labels:?}");

        // A scene that only plays factory presets asks nothing of the user.
        let factory = Requirements {
            banks: vec![BankRequirement {
                engine: ToneType::ZenCore,
                bank: Some("PR-A".into()),
                tone: Some("Ac Pop Piano 1".into()),
                address: ToneAddress {
                    msb: 87,
                    lsb: 64,
                    pc: 60,
                },
            }],
            ..Requirements::default()
        };
        assert!(!factory.needs_installed_content());
        assert_eq!(
            compare(&factory, &Inventory::default())[0].verdict,
            Verdict::Met
        );
    }

    /// Three answers, not two: owning an expansion and having it loaded are different facts, so
    /// "buy it" and "load it" are different instructions.
    #[test]
    fn an_expansion_inventory_answers_installed_owned_and_neither() {
        let mut held = ExpansionInventory::default();
        held.record("EXZ007", true, true);
        held.record("EXSN03", true, false);
        // What plays is what the instrument holds, not what was paid for.
        held.record("JP8", false, true);
        held.record("n/zyme", true, false);

        assert_eq!(held.verdict("EXZ007"), Verdict::Met);
        assert_eq!(held.verdict("EXSN03"), Verdict::NotLoaded);
        assert_eq!(held.verdict("JP8"), Verdict::Met);
        assert_eq!(held.verdict("EXZ001"), Verdict::Missing);
        // A catalogued code keeps its own case, so codes are addressed case-insensitively.
        assert_eq!(held.verdict("N/ZYME"), Verdict::NotLoaded);
    }

    /// The inventory is the input that turns "compare by hand" into an instruction.
    #[test]
    fn expansions_weighed_against_an_inventory_stop_answering_unknown() {
        let exz = |code: &str, lsb: u8| BankRequirement {
            engine: ToneType::Exz,
            bank: Some(code.into()),
            tone: None,
            address: ToneAddress {
                msb: 93,
                lsb,
                pc: 3,
            },
        };
        let needs = Requirements {
            banks: vec![
                exz("EXZ007", 7),
                exz("EXZ008", 11),
                exz("EXZ012", 15),
                BankRequirement {
                    engine: ToneType::Acb,
                    bank: None,
                    tone: None,
                    address: ToneAddress {
                        msb: 107,
                        lsb: 72,
                        pc: 3,
                    },
                },
            ],
            wave_expansions: vec![WaveExpansion::new(1005), WaveExpansion::new(4)],
            ..Requirements::default()
        };

        let mut inventory = ExpansionInventory::default();
        inventory.record("EXZ007", true, true);
        inventory.record("EXZ008", true, false);
        inventory.record("EXZ005", true, true);
        let held = Inventory::default().with_expansions(inventory);

        let findings = compare(&needs, &held);
        let finding = |needle: &str| {
            findings
                .iter()
                .find(|finding| finding.requirement.contains(needle))
                .unwrap_or_else(|| panic!("nothing reported for {needle}"))
        };
        assert_eq!(finding("EXZ007").verdict, Verdict::Met);
        assert_eq!(finding("EXZ008").verdict, Verdict::NotLoaded);
        assert_eq!(finding("EXZ012").verdict, Verdict::Missing);
        assert_eq!(finding("wave expansion EXZ005").verdict, Verdict::Met);
        // Unknown survives only where it is the honest answer: an address no bank name covers, and
        // a wave group id this build cannot decode into a product.
        assert_eq!(finding("ACB LSB 72").verdict, Verdict::Unknown);
        assert_eq!(finding("group id 4").verdict, Verdict::Unknown);

        // Each answer says what to do about it, and names the product to act on.
        assert!(finding("EXZ008").detail.contains("EXZ008"));
        assert!(finding("EXZ012").detail.contains("EXZ012"));
    }

    /// A note nobody has written in is not a statement that nothing is owned.
    ///
    /// The inventory is kept by hand, so an empty one is the same silence a file gives — and
    /// answering "buy all of these" to a player who simply never filled it in would be the
    /// confident wrong answer this type exists to avoid.
    #[test]
    fn an_inventory_with_nothing_recorded_answers_unknown_rather_than_unowned() {
        let needs = Requirements {
            wave_expansions: vec![WaveExpansion::new(1008)],
            ..Requirements::default()
        };

        let blank = Inventory::default().with_expansions(ExpansionInventory::default());
        assert_eq!(compare(&needs, &blank)[0].verdict, Verdict::Unknown);

        // One product recorded is a note, and it speaks for the products it does not name.
        let mut kept = ExpansionInventory::default();
        kept.record("EXZ007", true, true);
        let held = Inventory::default().with_expansions(kept);
        assert_eq!(compare(&needs, &held)[0].verdict, Verdict::Missing);
    }
}
