# fantom-librarian

A modular librarian for the Roland Fantom synthesizer — read Fantom data files, manage Scenes,
edit metadata, and package/organize. It does **not** create or edit synthesis parameters.

Written in Rust as a Cargo workspace so the parsing core stays reusable across a CLI, a desktop
app, and a WASM web UI.

## Layout

```
crates/
  fantom-core/    # pure library: bytes -> typed model (no I/O policy)
    container/    # SVD/SVZ framing (area table, record tables, zone/tone/sample areas)
    model/        # domain types: Scene, Zone, ToneRef, metadata
    address.rs    # the one table: which area a tone address indexes, and at which record
    codec/        # maps container bytes onto the model
    repackage.rs  # extract / canary / merge — rebundling and renumbering dependencies
    requirements.rs # the dependency closure as a value: what material needs where it lands
    convert.rs    # SVD -> SVZ: a user tone leaves a backup with the audio it plays
    diff.rs       # compares two files by area and record; how new offsets get found
    presets.rs    # factory ZEN-Core preset tone name lookup (bundled sound list)
    params/       # Roland's parameter map — file bytes against SysEx addresses
                  #   tone.rs  a 1632-byte ZEN-Core tone; scene.rs  a 3572-byte scene
    tests/        # tests against real files; each skips when its fixture is absent
  fantom-library/ # the librarian above the parser: workspace, catalog, imports
    workspace.rs  # the portable folder: marker, managed originals, exports, catalog
    import.rs     # validate, copy, catalogue, report — one source group per import
    catalog.rs    # browse, search, tag, relate to songs
    schema.sql    # the catalog's tables, with why each one is shaped that way
  fantom-cli/     # the `fantom` binary — first consumer of the library
    render.rs     # value labels and table layout; presentation only, no I/O
  fantom-midi/    # SysEx transport; reads the parameter map from fantom-core
desktop/          # the FANTOM Librarian app: Tauri v2 + React
  src-tauri/      # the command layer over fantom-library; decides nothing itself
  src/            # React front end — lib/api.ts is the only place that calls invoke
tools/            # gen_params.py / gen_scene_params.py — regenerate the parameter tables
docs/FORMAT.md    # reverse-engineering notebook for the on-disk layout
docs/LIBRARIAN_PLAN.md # what the app is for, and what v1 deliberately leaves out
fixtures/         # committed test files — see fixtures/README.md for what may go here
fixtures-local/   # private corpus: backups, docs, purchased packs (gitignored)
```

## The desktop app

```sh
cd desktop
pnpm install
pnpm app          # dev: vite + a debug build of the Rust side
pnpm app:build    # a bundled .app / .dmg
```

The app manages one **workspace** folder — the library, and ordinary user data you can copy and
back up:

```
My FANTOM Library/
  fantom-library.json   # the marker that makes this folder a workspace
  library.db            # the catalog
  originals/            # content-addressed copies of everything imported
  exports/              # generated deployment folders
```

Imports are copied, never moved or edited. Identical records consolidate into one library item
that remembers every source it came from, so re-importing an overlapping pack grows provenance
rather than duplicates.

## Status

Reads the SVD5 container envelope (verified on a Roland **FANTOM-6**), lists **scene names**, and
shows each scene's **tempo, level and comment/memo** with its **16 zones** (type, bank, tone,
switch, key and velocity range, level, pan, transpose, octave, MIDI channel, arpeggio). The whole
3572-byte scene record is mapped block by block in `params::scene`, so the fields above are a
selection rather than the limit. It retains every zone's raw MSB/LSB/PC address and resolves
bundled names for ZEN-Core,
Drum, SN-A, SN-AP, SN-EP, VTW, V-Piano, MODEL, and ACB USER sounds. Factory references whose
names are not serialized remain visible by engine, bank, and program number.

> Only tested on a FANTOM-6. Not yet verified on the FANTOM-06/07/08 ("Fantom-0" series — a
> different, cheaper product line despite the similar name) or FANTOM-6/7/8 EX.

**Factory preset** ZEN-Core tones are named from a bundled copy of Roland's FANTOM Sound List
(`crates/fantom-core/src/preset_tones.tsv`, ~3.7k tones). **User** tones resolve by direct index in
**scene exports and full backups alike** — both address their user banks the same way. Unresolved
types/banks are shown as their raw `MSB`, `LSB`, and `PC` instead of being mislabeled. See
[`docs/FORMAT.md`](docs/FORMAT.md).

**Write path:** `rename` and `comment` overwrite only the scene's name/comment field and nothing
else (verified by byte diff and confirmed on a FANTOM-6). `extract`, `canary`, and `merge` rebuild
scene-export banks with exact per-engine dependency mappings, rebundling `PATa`, paired
`RHYa`/`INSa`, `VTWa`, `SNAa`, `ZAPa`, `ZEPa`, `DCWa`, `MDLa`, and `ACBa` records and rewriting
their zone references.

**Full backups work as a source**, hardware-confirmed on a FANTOM-6: four canary banks extracted
from full backups all imported with their `CNY` tone names visible, covering ZEN-Core, drum kits
(with their paired instrument sets), SN-A, SN-AP, SN-EP, VTW, and V-Piano USER. One was then
re-imported after overwriting every USER slot it referenced — and its scene slot — with INIT data:
all its tones were recreated correctly, proving the instrument wrote them from the rebuilt bundle
rather than resolving sounds it already had. Extracting scene
385 from a 35 MB backup produces a 7.9 KB self-contained bank. Also verified by extracting all 149
scenes shared between the export/backup fixture pairs from both sources and asserting the results
decode identically. *Writing* a full backup is a separate problem and is not supported.

Extraction and merging were hardware-confirmed on a FANTOM-6 using NARF and a cross-bank
NARF/PRISMA canary: zones, keyboard groups, tones, and samples continued to work, including the
PRISMA scene's SN-A and SN-EP dependencies. V-Piano, MODEL/ABM, and ACB USER records are also
rebundled and rebased, with their multi-record indexing confirmed on hardware.

**SVZ tone banks** are read and repackaged too. An `.svz` holds tones rather than scenes — `PATa`,
or `RHYa` with its paired `INSa` — and unlike a scene export it **carries the audio of any user
sample its tones play**. `extract` and `merge` work on them with the same verbs, selecting by the
indexes `tones` prints, carrying each selected tone's samples and renumbering the references;
samples nothing references are left behind, and the CLI says so.

Selection follows every kind of reference a tone can make: a ZEN-Core partial'''s own sample slots, a
drum kit'''s (in its paired `INSa`), and the samples a **user multisample** maps across the keyboard,
which a tone reaches only indirectly. Multisamples travel too — an `.svz` carries the `MSPa` record
and the whole chain is renumbered together.

Extracting a multisampled tone produces a file **byte-identical to the instrument'''s own export** of
that tone: 5,824,680 bytes, tone record, renumbered multisample reference, remapped key ranges,
audio and checksums alike. Engines whose references are still unreadable fall back to carrying every
sample the source holds, at unchanged slot numbers, rather than dropping them silently.

**A user tone can leave a backup.** `.svz` is the only envelope that carries sample audio, and until
now it was unreachable from the format people actually have: a sampled sound sat in a backup with no
way out but re-exporting it by hand on the instrument. `tones extract` builds one — the tone records
copied across, everything they play carried with them (samples, the samples a multisample maps
across the keyboard, and the multisample itself), all renumbered to their new positions.

The evidence is byte-level. From one FANTOM-6 backup, four exports that same instrument wrote are
reproduced **byte for byte**, differing only in the OS-era stamp byte documented in
[`docs/FORMAT.md`](docs/FORMAT.md): a two-tone file with a multisample and 7 MB of audio, a single
multisampled tone, a drum kit with no sample areas at all, and one that plays a sample — every shape
the instrument writes. Engines whose sample references are
undecoded are refused rather than exported without their audio.

**Hardware-confirmed on a FANTOM-6.** A multisampled tone converted out of a backup — one tone, one
multisample, three samples — imported and plays correctly; four tones selected into one file
imported together under their own names. A tone whose multisample number *moves* (2 in the backup,
1 in the file) plays too, and so does a converted drum kit with its sample. The instrument repoints
each tone at wherever its samples land, so the positions a carried file uses are rewritten on the
way in.

**The round trip is byte-identical.** One of those files, imported and then exported back off the
instrument, returns all 5,824,680 bytes unchanged apart from the stamp byte the instrument makes its
own — so every field is read as intended, not just the ones you can hear.

Drum kits load from **IMPORT DRUM**, not IMPORT TONE, which lists no kit at all — Roland's own kit
exports included.

That session also found the failure no structural check can see, and it belongs to the instrument
rather than to this tool: **a FANTOM can play a sample that its own files do not carry.** For the
affected slots a backup stores the record in full and the waveform as zeros. So did a tone export
the instrument itself wrote, asked for the same sounds, on a day when its export of another tone
carried 5.6 MB of real audio. A tone built from such a backup imports perfectly and plays nothing,
and re-importing the sample is the only recovery known. Nothing in a file marks the state, so
`samples list` reads the audio to tell `<silent>` from `<no waveform>`, and anything built from
such a slot says so before it is written. See [`docs/FORMAT.md`](docs/FORMAT.md), which also says
what is still unexplained.

**In a scene bank, user samples do not travel** — and that is Roland's own behaviour, not a
limitation here: the instrument's scene exports carry sample *slot references* and no audio. A tone
references a sample by slot (wave group 2 on a partial, see `docs/FORMAT.md`), so `extract` names
the exact slots a destination must already hold.

**`--samples` makes them travel anyway**, as a second file, because that is the only shape the
instrument will read audio from:

```sh
cargo run -p fantom-cli -- scenes extract backup/FANTOM.SVD 401 -o out/FANTOM.SVD \
    --samples out/samples.svz --samples-at 101
```

That writes the scene bank *and* a sample-only `.svz` holding just the samples those scenes play —
two samples, 1.2 MB, not the backup's 23 — then rewrites the bank's references so they point
wherever you import it. Load the `.svz` through **MENU → IMPORT SAMPLE**, put each sample in the
slot the tool names, and the numbers agree. The alternative, which is what commercial packs have to
tell their buyers, is to delete whatever you keep in slots 1–50 and load theirs there.

**Hardware-confirmed on a FANTOM-6.** The companion imported to slots 2001–2002, the bank imported
as a scene, and the tone plays with the panel showing its wave number as `2001` — the number this
tool wrote, at a slot that had never held that audio. `IMPORT SAMPLE` asks for a destination per
sample, so the tool prints each sample's name against its required slot rather than a range.

The companion is built from a full backup, since only a backup holds the audio, and the builder is
tested by reproducing a commercially shipped sample pack from one: all 23,427,900 bytes, differing
only in a format-revision byte. Drum kits are the exception — their sample references are not
decoded, so they cannot be rebased, and the CLI says so when a bank bundles any.

References to installed factory/model/expansion banks are preserved but require the same content on
the destination.

**The dependency closure is a type, not a printout.** `fantom_core::requirements` answers what one
file, one scene, or one tone needs from wherever it lands — user sample and multisample slots (with
the tone that goes silent without each one), user tones a bank references but does not bundle,
factory banks told apart from expansions that must be installed, and any address this version
cannot classify, which is reported rather than dropped. It is serde-serialisable, `check`,
`scenes show`, `extract`, `canary`, and `merge` all report from it, and the library stores it per
asset at import.

Against a second file it goes as far as the format allows and no further. Sample slots are checked
one by one — reading NARF against the backup it came from resolves all 50 by name — while nothing
in any file has been found to list an instrument's installed expansions, so those come back
`unknown` with the requirement named, rather than as a guess. Derived from the bytes alone, NARF's
closure is exactly user sample slots 1–50: the placement its printed instructions demand.

## Usage

```sh
# Envelope + hexdump — the reverse-engineering microscope.
cargo run -p fantom-cli -- inspect path/to/FANTOM.SVD --length 512

# List the memory areas in an SVD container.
cargo run -p fantom-cli -- areas list path/to/FANTOM.SVD

# Compare two files, reporting each difference as AREA[record]+offset.
# Export two files differing by one deliberate change and this finds the bytes that carry it.
cargo run -p fantom-cli -- diff before.SVD after.SVD --area DCWa --context 4

# List the scene names in an SVD backup.
cargo run -p fantom-cli -- scenes list path/to/FANTOM.SVD

# Show one scene with its zones (type, bank, tone, switch, key range, level).
# Use --include-disabled for off zones.
cargo run -p fantom-cli -- scenes show path/to/FANTOM.SVD 385

# List the tones bundled in a file.
cargo run -p fantom-cli -- tones list path/to/FANTOM.SVD

# List the user samples and multisamples a file carries.
cargo run -p fantom-cli -- samples list path/to/FANTOM.SVD

# Check structure and record checksums. Exits non-zero on a problem, so it works as a gate.
cargo run -p fantom-cli -- verify path/to/FANTOM.SVD

# What a file needs from wherever it is loaded: user samples, multisamples, missing user
# tones, and the expansions that must already be installed.
cargo run -p fantom-cli -- check theirs/FANTOM.SVD

# Weigh those requirements against a destination — a backup of the instrument you will load
# onto. Unmet requirements exit non-zero, so this works as a preflight gate too.
cargo run -p fantom-cli -- check theirs/FANTOM.SVD --against mine/FANTOM.SVD

# Lift a user tone out of a backup as a self-contained .svz, carrying the samples it plays.
# --area RHYa takes a drum kit instead. Works on an .svz source too, repackaging in place.
cargo run -p fantom-cli -- tones extract path/to/BACKUP.SVD 954 -o pad.svz

# SVZ tone banks use the same scene operations. Numbers are the indexes `tones list` prints.
cargo run -p fantom-cli -- tones list path/to/Z-Core.svz
cargo run -p fantom-cli -- scenes extract path/to/Z-Core.svz 0 12 3 -o subset.svz
cargo run -p fantom-cli -- scenes merge a.svz b.svz -o combined.svz

# Edit scene metadata (pass --dry-run to preview; --output is required to write).
cargo run -p fantom-cli -- scenes rename  path/to/FANTOM.SVD 44 "My Scene"   --dry-run
cargo run -p fantom-cli -- scenes rename  path/to/FANTOM.SVD 44 "My Scene"   -o out.svd
cargo run -p fantom-cli -- scenes comment path/to/FANTOM.SVD 44 "split at B4" -o out.svd

# Build a smaller bank from scenes 44 and 3, in that order, with their referenced user tones.
cargo run -p fantom-cli -- scenes extract path/to/FANTOM.SVD 44 3 -o extracted/FANTOM.SVD

# The same, plus a companion sample file, with the bank repointed at slots 101+.
cargo run -p fantom-cli -- scenes extract path/to/BACKUP.SVD 401 -o out/FANTOM.SVD \
    --samples out/samples.svz --samples-at 101

# Build a one-scene hardware-test bank with visible CNY scene/tone names.
cargo run -p fantom-cli -- scenes canary path/to/FANTOM.SVD 44 -o canary/FANTOM.SVD

# Append all scenes from one scene-export bank to another, de-duplicating identical user tones.
cargo run -p fantom-cli -- scenes merge base/FANTOM.SVD additions/FANTOM.SVD -o merged/FANTOM.SVD

cargo build            # build everything
cargo test             # run tests
```
