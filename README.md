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

Reads the SVD5 container envelope (verified on a Roland **FANTOM-6**), lists **scene names**, shows
each scene's **comment/memo** and its **16 zones** (tone, switch, key range, level), and resolves
per-zone **tone names** from the `PATa` tone area. Confirmed against real backups and controlled
sample scenes.

> Only tested on a FANTOM-6. Not yet verified on the FANTOM-06/07/08 ("Fantom-0" series — a
> different, cheaper product line despite the similar name) or FANTOM-6/7/8 EX.

**Factory preset** tones are named from a bundled copy of Roland's FANTOM Sound List
(`crates/fantom-core/src/preset_tones.tsv`, ~3.7k ZEN-Core tones). **User** tone names resolve
directly for **scene exports** (`SOUND/…`); in **full backups** user tones use a global address that
isn't fully mapped yet, so they show as `user #id`. See [`docs/FORMAT.md`](docs/FORMAT.md).

**Write path:** `rename` and `comment` overwrite only the scene's name/comment field and nothing
else (verified by byte diff and confirmed on a FANTOM-6). `extract` and `merge` rebuild scene-export
banks with an exact referenced-user-tone to `PATa` mapping, copy complete opaque scene/tone records,
and rewrite only bundled user-tone references. Repackaging rejects files where that mapping cannot
be proven, including full backups and some older exports.

Extraction and a two-scene merge were hardware-confirmed on a FANTOM-6 using NARF: canary tone names
proved that the rebuilt `PATa` was imported, while zones, keyboard groups, tones, and samples
continued to work. Merge currently rebundles `PATa` only and retains every other area from the
target file. A source scene that depends on source-only rhythm, sample, or other engine data may
therefore need additional area-specific bundling; arbitrary cross-bank merges are not yet claimed.

## Usage

```sh
# Envelope + hexdump — the reverse-engineering microscope.
cargo run -p fantom-cli -- inspect path/to/FANTOM.SVD --len 512

# List the memory areas in an SVD container.
cargo run -p fantom-cli -- areas path/to/FANTOM.SVD

# List the scene names in an SVD backup.
cargo run -p fantom-cli -- scenes path/to/FANTOM.SVD

# Show one scene with its zones (tone, switch, key range, level). Use --all for off zones.
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
