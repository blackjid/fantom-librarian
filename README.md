# fantom-librarian

A modular librarian for the Roland Fantom synthesizer — read Fantom data files, manage Scenes,
edit metadata, and package/organize. It does **not** create or edit synthesis parameters.

Written in Rust as a Cargo workspace so the parsing core stays reusable across a CLI now and a
GUI or WASM web UI later.

## Layout

```
crates/
  fantom-core/   # pure library: bytes -> typed model (no I/O policy)
    container/   # SVD/SVZ framing (size prefix, area table, zone/tone tables)
    model/       # domain types: Scene, Zone, ToneRef, metadata
    codec/       # maps container bytes onto the model (read now, write later)
    presets.rs   # factory ZEN-Core preset tone name lookup (bundled sound list)
  fantom-cli/    # the `fantom` binary — first consumer of the library
docs/FORMAT.md   # reverse-engineering notebook for the on-disk layout
fixtures/        # sample files (gitignored by default) + golden snapshots
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
(`crates/fantom-core/src/preset_tones.tsv`, ~3.7k tones). **User** names resolve directly for
**scene exports** (`SOUND/…`); unresolved types/banks are shown as their raw `MSB`, `LSB`, and `PC`
instead of being mislabeled. See [`docs/FORMAT.md`](docs/FORMAT.md).

**Write path:** `rename` and `comment` overwrite only the scene's name/comment field and nothing
else (verified by byte diff and confirmed on a FANTOM-6). `extract` and `merge` rebuild scene-export
banks with exact per-engine dependency mappings. They rebundle `PATa`, paired `RHYa`/`INSa`,
`VTWa`, `SNAa`, `ZAPa`, and `ZEPa` records and rewrite their zone references. Full backups remain
unsupported because their ZEN-Core mapping cannot be derived safely.

Extraction and merging were hardware-confirmed on a FANTOM-6 using NARF and a cross-bank
NARF/PRISMA canary: zones, keyboard groups, tones, and samples continued to work, including the
PRISMA scene's SN-A and SN-EP dependencies. V-Piano, MODEL/ABM, and ACB USER records are also
rebundled and rebased, with their multi-record indexing confirmed on hardware. Copying external
sample waveform files remains open; references to installed factory/model/expansion banks are
preserved but require the same content on the destination.

## Usage

```sh
# Envelope + hexdump — the reverse-engineering microscope.
cargo run -p fantom-cli -- inspect path/to/FANTOM.SVD --len 512

# List the memory areas in an SVD container.
cargo run -p fantom-cli -- areas path/to/FANTOM.SVD

# List the scene names in an SVD backup.
cargo run -p fantom-cli -- scenes path/to/FANTOM.SVD

# Show one scene with its zones (type, bank, tone, switch, key range, level).
# Use --all for off zones.
cargo run -p fantom-cli -- show path/to/FANTOM.SVD 385

# List the tones bundled in a file.
cargo run -p fantom-cli -- tones path/to/FANTOM.SVD

# Edit scene metadata (dry run without -o; pass -o to write a copy).
cargo run -p fantom-cli -- rename  path/to/FANTOM.SVD 44 "My Scene"   -o out.svd
cargo run -p fantom-cli -- comment path/to/FANTOM.SVD 44 "split at B4" -o out.svd

# Build a smaller bank from scenes 44 and 3, in that order, with their referenced user tones.
cargo run -p fantom-cli -- extract path/to/FANTOM.SVD 44 3 -o extracted/FANTOM.SVD

# Build a one-scene hardware-test bank with visible CNY scene/tone names.
cargo run -p fantom-cli -- canary path/to/FANTOM.SVD 44 -o canary/FANTOM.SVD

# Append all scenes from one scene-export bank to another, de-duplicating identical user tones.
cargo run -p fantom-cli -- merge base/FANTOM.SVD additions/FANTOM.SVD -o merged/FANTOM.SVD

cargo build            # build everything
cargo test             # run tests
```
