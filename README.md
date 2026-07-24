# fantom-librarian

A modular librarian for the Roland Fantom synthesizer — read Fantom data files, manage Scenes,
edit metadata, and package/organize. It does **not** create or edit synthesis parameters.

Written in Rust as a Cargo workspace so the parsing core stays reusable across a CLI now and a
GUI or WASM web UI later.

## Layout

```
crates/
  fantom-core/   # pure library: bytes -> typed model (no I/O policy)
    container/   # SVD/SVZ framing (size prefix, area table, zone headers)
    model/       # domain types: Scene, Zone, Tone, metadata
    codec/       # maps container bytes onto the model (read now, write later)
    device/      # per-model quirks (Fantom-0 vs 6/7/8) behind a trait
  fantom-cli/    # the `fantom` binary — first consumer of the library
docs/FORMAT.md   # reverse-engineering notebook for the on-disk layout
fixtures/        # sample files (gitignored by default) + golden snapshots
```

## Status

Reads the SVD5 container envelope (verified on Fantom-0 backups), lists **scene names**, decodes
each scene's **16 zones** (tone, switch, key range, level), and resolves per-zone **tone names**
from the `PATa` tone area. Confirmed against real backups and controlled sample scenes.

**Factory preset** tones are named from a bundled copy of Roland's FANTOM Sound List
(`crates/fantom-core/src/preset_tones.tsv`, ~3.7k ZEN-Core tones). **User** tone names resolve
directly for **scene exports** (`SOUND/…`); in **full backups** user tones use a global address that
isn't fully mapped yet, so they show as `user #id`. See [`docs/FORMAT.md`](docs/FORMAT.md).

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

cargo build            # build everything
cargo test             # run tests
```
