# Capturing the names the librarian cannot resolve

Everything the librarian can name comes from three bundled tables:

| Table | Covers | Built by |
|-------|--------|----------|
| `crates/fantom-core/src/preset_tones.tsv` | ZEN-Core presets | (no generator; extracted once) |
| `crates/fantom-core/src/factory_sounds.tsv` | drum kits, SN-A, V-Piano, VTW, ACB JP8 | `tools/gen_sound_list.py` |
| `crates/fantom-core/src/factory_scenes.tsv` | factory scene names | `tools/gen_sound_list.py` |
| `crates/fantom-core/src/expansion_sounds.tsv` | 3065 sounds, 26 expansions | `tools/gen_expansion_catalog.py` |

A zone whose address none of them covers shows its bank and program and no name.

## What is still unnamed, and why

- **`105/68`** — an EXSN expansion this instrument does not have. Selecting it echoes the
  previously selected name, which is how an absent bank answers. Needs the expansion installed, or
  its sound list PDF.
- **`107/66` (ACB SH101), `107/70` (ACB JU106)** — ACB is the one engine that cannot be read back:
  MSB 107 is a selectable bank with no temporary area, and sweeping twelve candidate addresses
  returned byte-identical answers. Only each expansion's own sound list can name these.

Both ACB labels are also **unverified**. They come from the same `TONEMAP` reading that got every
MODEL bank wrong (see `FORMAT.md`), and ACB cannot be swept to check them. Selecting `107/66` on
the panel and reading the bank name would settle it.

`93/4-6`, `93/18` and `93/25` are not banks at all — they are holes in the slot table, and answer
with a blank name.

## Capturing a bank

`dump-sounds` writes to the **temporary scene**, so audition on a scratch scene. Nothing is stored
unless you press Write.

```sh
cargo run -p fantom-midi --bin dump-sounds -- <msb> <lsb> > <msb>-<lsb>.tsv
```

`--first` and `--last` are 0-based; the output's `PC` column counts from one. Some engines only
answer in the zone that engine is routed to — VTW needs `--zone 2` on a FANTOM-6, and reads blank
in zone 1.

**Read the bank label off the panel before adding the rows.** The catalogs are keyed by product
code, and the address is not the product: the LSB records where the product was *placed*. Select
the bank on the instrument and take the name it shows — that is how `EXSN04` was identified, and
how six wrongly labelled MODEL banks were caught. Then add the address to `PRODUCTS` in
`tools/gen_expansion_catalog.py` and to `ToneRef::bank`; `expansions::tests` fails if they disagree.

`dump-wave-groups` reads the same banks a different way: the wave **group id** each sound's
partials play from, which for an expansion wave is the product (`1005` is `EXZ005`). It is the
second opinion on a bank's label, and it needs no panel.

```sh
cargo run -p fantom-midi --bin dump-wave-groups -- <msb> <lsb>
```

If the instrument stops answering — no reply even to a Universal Identity Request — check
`SYSTEM` -> `MIDI` -> `Rx SysEx`, then replug the USB cable. A power cycle alone does not
re-enumerate it on the host.

## After any capture

```sh
# an expansion bank
tools/gen_expansion_catalog.py sounds/*.tsv > crates/fantom-core/src/expansion_sounds.tsv

# a base-instrument bank. Both PDFs, minus the ZEN-Core presets `preset_tones.tsv` already
# holds — the ACB JUPITER-8 bank is only in the FANTOM EX list. Swept banks are extra arguments.
pdftotext -layout FANTOM_SoundList_multi02_W.pdf base.txt
pdftotext -layout FANTOM_EX_SoundList_multi01_W.pdf ex.txt
cat base.txt ex.txt | tools/gen_sound_list.py 91-65.tsv | awk -F'\t' 'NR==1 || $1!=87' \
    > crates/fantom-core/src/factory_sounds.tsv

# Factory scene names. The scene list's slot is intentionally discarded: a player can move a
# scene without changing its data. INITIAL SCENE is the blank template, not a factory scene.
pdftotext -layout FANTOM_SoundList_multi02_W.pdf - | tools/gen_sound_list.py \
    | awk -F'\t' 'BEGIN { print "name" } NR > 1 && $1 == 85 && $5 != "INITIAL SCENE" { print $5 }' \
    > crates/fantom-core/src/factory_scenes.tsv

cargo test -p fantom-core
```

`factory::tests::no_bank_is_missing_a_program_in_the_middle` fails on a bank with a hole in it,
which is what a dropped sound-list row looks like. Then bump
`fantom_library::rescan::NAMING_REVISION` so existing catalogs read their scenes again — without
it, a library already stamped with the current revision will never see the new names.
