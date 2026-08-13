# fantom-librarian

A modular librarian for the Roland Fantom synthesizer — read Fantom data files, manage Scenes,
edit metadata, and package/organize. It does **not** create or edit synthesis parameters.

Written in Rust as a Cargo workspace so the parsing core stays reusable across a CLI now and a
GUI or WASM web UI later.

## Layout

```
crates/
  fantom-core/   # pure library: bytes -> typed model (no I/O policy)
    container/   # SVD/SVZ framing (area table, record tables, zone/tone/sample areas)
    model/       # domain types: Scene, Zone, ToneRef, metadata
    address.rs   # the one table: which area a tone address indexes, and at which record
    codec/       # maps container bytes onto the model
    repackage.rs # extract / canary / merge — rebundling and renumbering dependencies
    diff.rs      # compares two files by area and record; how new offsets get found
    presets.rs   # factory ZEN-Core preset tone name lookup (bundled sound list)
    tests/       # tests against real files; each skips when its fixture is absent
  fantom-cli/    # the `fantom` binary — first consumer of the library
docs/FORMAT.md   # reverse-engineering notebook for the on-disk layout
fixtures/        # real Fantom files and hardware captures (gitignored)
```

## Status

Reads the SVD5 container envelope (verified on a Roland **FANTOM-6**), lists **scene names**, and
shows each scene's **comment/memo** and its **16 zones** (type, bank, tone, switch, key range,
level). It retains every zone's raw MSB/LSB/PC address and resolves bundled names for ZEN-Core,
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

That per-tone selection needs a decoded tone→sample link, which exists for ZEN-Core tones only. A
drum kit's waves live in its paired `INSa`, where the field marking one as a user sample has never
been observed set — no fixture has a kit that plays one. So a drum bank **carries every sample it
holds, at unchanged slot numbers**, rather than selecting: the alternative was dropping all of them
silently. The CLI says which happened, and merging drum banks with conflicting samples is refused.
`docs/FORMAT.md` records the `INSa` wave-block layout and names the two-file capture that would
settle the group field.

**In a scene bank, user samples do not travel** — and that is Roland's own behaviour, not a
limitation here: the instrument's scene exports carry sample *slot references* and no audio. A tone
references a sample by slot (wave group 2 on a partial, see `docs/FORMAT.md`), so `extract` names
the exact slots a destination must already hold.

**`--samples` makes them travel anyway**, as a second file, because that is the only shape the
instrument will read audio from:

```sh
cargo run -p fantom-cli -- extract backup/FANTOM.SVD 401 -o out/FANTOM.SVD \
    --samples out/samples.svz --samples-at 101
```

That writes the scene bank *and* a sample-only `.svz` holding just the samples those scenes play —
one sample, 1 MB, not the backup's 23 — then rewrites the bank's references so they point wherever
you import it. Load the `.svz` through **MENU → IMPORT SAMPLE** at slot 101 and the numbers agree.
Without `--samples-at` the run starts at slot 1. The alternative, which is what commercial packs
have to tell their buyers, is to delete whatever you keep in slots 1–50 and load theirs there.

The companion is built from a full backup, since only a backup holds the audio, and the builder is
tested by reproducing a commercially shipped sample pack from one: all 23,427,900 bytes, differing
only in a format-revision byte. Drum kits are the exception — their sample references are not
decoded, so they cannot be rebased, and the CLI says so when a bank bundles any.

References to installed factory/model/expansion banks are preserved but require the same content on
the destination.

## Usage

```sh
# Envelope + hexdump — the reverse-engineering microscope.
cargo run -p fantom-cli -- inspect path/to/FANTOM.SVD --len 512

# List the memory areas in an SVD container.
cargo run -p fantom-cli -- areas path/to/FANTOM.SVD

# Compare two files, reporting each difference as AREA[record]+offset.
# Export two files differing by one deliberate change and this finds the bytes that carry it.
cargo run -p fantom-cli -- diff before.SVD after.SVD --area DCWa --context 4

# List the scene names in an SVD backup.
cargo run -p fantom-cli -- scenes path/to/FANTOM.SVD

# Show one scene with its zones (type, bank, tone, switch, key range, level).
# Use --all for off zones.
cargo run -p fantom-cli -- show path/to/FANTOM.SVD 385

# List the tones bundled in a file.
cargo run -p fantom-cli -- tones path/to/FANTOM.SVD

# List the user samples and multisamples a file carries.
cargo run -p fantom-cli -- samples path/to/FANTOM.SVD

# Check structure and record checksums. Exits non-zero on a problem, so it works as a gate.
cargo run -p fantom-cli -- verify path/to/FANTOM.SVD

# SVZ tone banks use the same verbs. Numbers are the indexes `tones` prints.
cargo run -p fantom-cli -- tones   path/to/Z-Core.svz
cargo run -p fantom-cli -- extract path/to/Z-Core.svz 0 12 3 -o subset.svz
cargo run -p fantom-cli -- merge   a.svz b.svz -o combined.svz

# Edit scene metadata (dry run without -o; pass -o to write a copy).
cargo run -p fantom-cli -- rename  path/to/FANTOM.SVD 44 "My Scene"   -o out.svd
cargo run -p fantom-cli -- comment path/to/FANTOM.SVD 44 "split at B4" -o out.svd

# Build a smaller bank from scenes 44 and 3, in that order, with their referenced user tones.
cargo run -p fantom-cli -- extract path/to/FANTOM.SVD 44 3 -o extracted/FANTOM.SVD

# The same, plus a companion sample file, with the bank repointed at slots 101+.
cargo run -p fantom-cli -- extract path/to/BACKUP.SVD 401 -o out/FANTOM.SVD \
    --samples out/samples.svz --samples-at 101

# Build a one-scene hardware-test bank with visible CNY scene/tone names.
cargo run -p fantom-cli -- canary path/to/FANTOM.SVD 44 -o canary/FANTOM.SVD

# Append all scenes from one scene-export bank to another, de-duplicating identical user tones.
cargo run -p fantom-cli -- merge base/FANTOM.SVD additions/FANTOM.SVD -o merged/FANTOM.SVD

cargo build            # build everything
cargo test             # run tests
```
