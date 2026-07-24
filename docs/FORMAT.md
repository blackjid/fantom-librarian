# Roland Fantom file formats — reverse-engineering notebook

A living record of what we learn about the on-disk layout. Prefer confirmed facts with the sample
file and byte offsets that prove them; mark guesses clearly.

## File types

| Ext    | What it holds                          | Notes                                        |
|--------|----------------------------------------|----------------------------------------------|
| `.svd` | Full backup container                  | Roland backup format, model-specific layout  |
| `.svz` | ZEN-Core tone data                     | Individual tones; also used by Jupiter-X etc |
| `.sdz` | Sound pack                             | Roland Cloud sound content                   |

Scenes exported from a Fantom-0 land in a `SOUND/` folder on the USB drive.

## Data model

ZEN-Core hierarchy: **Tones → Zones → Scenes**. A Scene has up to 16 Zones; each Zone references a
Tone plus performance settings.

## SVD5 container — CONFIRMED (Fantom-0, from `fixtures/backup`)

Verified against `ROLAND/SOUND/PRISMA/FANTOM.SVD` (Fantom-0 backup). All integers little-endian.

### File header (offset 0x00)
| Off  | Size | Field         | Example        | Meaning                                        |
|------|------|---------------|----------------|------------------------------------------------|
| 0x00 | 2    | `header_size` | `ae 00` = 174  | Bytes from 0x02 to first data area (0x02+174 = 0xb0) |
| 0x02 | 4    | `magic`       | `SVD5`         | Container identifier                            |
| 0x06 | 10   | reserved      | zeros          | Pads to 0x10                                    |
| 0x10 | 16×N | area table    | see below      | `N = (header_size - 14) / 16`; here N = 10      |

### Area-table entry (16 bytes)
| Off | Size | Field    | Example      | Meaning                              |
|-----|------|----------|--------------|--------------------------------------|
| +0  | 4    | `tag`    | `PRFa`       | Area kind (ASCII)                    |
| +4  | 4    | `format` | `KY19`       | Format/version stamp (const in file) |
| +8  | 4    | `offset` | `b0 00 00 00`| Absolute byte offset of the area     |
| +12 | 4    | `size`   | `50 df 00 00`| Area length in bytes                 |

Area tags seen (in order): `PRFa PATa RHYa INSa VTWa SNAa ZAPa ZEPa SYSa DIFa`.
Meanings inferred: **PRFa = Performances/Scenes**, PATa = Patches/Tones, RHYa = Rhythm/drum,
SYSa = System, DIFa = manifest/diff. Others TBD.

### PRFa area = Scenes: 16-byte header, then fixed-stride records
The `PRFa` area opens with a 16-byte header, then an array of fixed-stride scene records:

**Area header (16 bytes at area start):**
| Off | Size | Field         | Example       | Meaning                                   |
|-----|------|---------------|---------------|-------------------------------------------|
| +0  | 4    | `count`       | `32 00 00 00` | Number of scenes (NARF = 50; PRISMA = 16) |
| +4  | 4    | `record_size` | `f4 0d 00 00` | Stride per record (3572 bytes)            |
| +8  | 8    | (unknown)     |               | TBD                                        |

**Each scene record (`record_size` bytes):** starts with a 16-byte ASCII `name`, space-padded
(e.g. `DSOTM Breathe`, `Africa Main`), followed at **`+0x40`** by a longer ASCII **comment/memo**
field (e.g. `"KEY SPLIT[SPLIT POINT B4] C5 …"`). It also holds two parallel 16-entry per-zone tables. Offsets
below were **confirmed by controlled single-variable edits** (`fixtures/tests/TEST 1..3`) and
cross-checked against the "Africa Main" panel: zone order is 1:1 with the panel (zone 0 = Zone 1).

**A) Zone table — record-relative `0x6d0`, 16 × 0x60 (96) bytes** (CONFIRMED):
| Off   | Field       | Evidence                                                          |
|-------|-------------|-------------------------------------------------------------------|
| +0x04 | `enable`    | 0/1. TEST1→TEST2 flipped zone1's byte 0x734 (`0x6d0+0x60+4`) 0→1  |
| +0x08 | `key_low`   | MIDI note. TEST3 set zone0 to `3c` (C4=60) at 0x6d8               |
| +0x09 | `key_high`  | MIDI note. TEST3 set zone0 to `48` (C5=72) at 0x6d9               |
| +0x3e | `marker`    | Constant `cf cd` (16 of them at stride 0x60, from 0x70e)          |

**B) Zone settings table — record-relative `0x194`, 16 × 0x48 (72) bytes** (partly decoded):
| Off   | Field       | Evidence                                                          |
|-------|-------------|-------------------------------------------------------------------|
| +0x00 | marker      | Constant `0x57`                                                   |
| +0x03 | zone index  | 0..15                                                             |
| +0x07 | `level`     | 0..127. TEST2→TEST3 set zone0 level `64`→`32` (100→50) at 0x19b   |

The **tone reference** is a **16-bit big-endian value at table A `+0x01`** (bytes `+0x01/+0x02`).
It uniquely identifies the zone's tone but is **bank-relative** — not the plain display number:
| Tone (panel)        | tone_id (BE16) | hi   | lo   |
|---------------------|----------------|------|------|
| USER 448 (Brass)    | 827  (0x033b)  | 0x03 | 0x3b |
| USER 449 (Kalimba)  | 828  (0x033c)  | 0x03 | 0x3c |
| PR-AA 61 (JX Cream) | 23612 (0x5c3c) | 0x5c | 0x3c |
| INIT default (TEST) | 23868 (0x5d3c) | 0x5d | 0x3c |

**Resolved (mechanism confirmed via TONEMAP controlled capture):** `tone_id` is NOT a global tone
number. When a scene is saved, the Fantom **bundles the USER tones it references into the file's
`PATa` area and renumbers them**, so for user tones `tone_id` is an **index into `PATa`**. Verified:
TONEMAP `PATa` holds exactly its 3 referenced USER tones, and `tone_id` 0/1/2 → `Strings Fall` /
`Thriller trillo` / `Jump Brass EmA` (= panel USER 1/2/129).

**Factory preset tones** instead encode a fixed ROM reference: `tone_id = (LSB << 8) | (PC - 1)`,
with MSB always 87 for ZEN-Core tones (so the `0x4000` bit is always set — preset LSBs are ≥ 64).
Verified against Roland's *FANTOM Sound List*: `JX Cream` = PR-A 0061 (LSB 92, PC 61) = `0x5c3c`;
PR-A 0001 = `0x5c00`; PR-B 0001 = `0x4000`. The 3667 ZEN-Core preset tones are bundled in
`crates/fantom-core/src/preset_tones.tsv` and resolved by [`presets::lookup`]. (Drum kits use MSB 86
and share the same 16-bit id space, so they are omitted to avoid mislabelling.)

`PATa` layout mirrors `PRFa`: 16-byte header (`count`, `record_size`, `data_start=0x10`), then
`count` records of `record_size` bytes; the tone **name** is the first 16 ASCII bytes, and byte
`+0x10` is the tone **category** (`0x23` = brass).

- **Scene exports** (`SOUND/…`, single/multi-scene): `tone_id` indexes `PATa` directly (offset 0).
  Verified across TONEMAP and the NARF export.
- **Full backups**: the reference is **per-scene**, not global. Each scene's user tones are a
  **contiguous bundle** in `PATa`, and its zone gids equal `bundle_base_index + per_scene_offset`.
  Confirmed against panel truth: `Africa Main` offset 826 (gid 827/828 → `PATa[1,2]` = Africa
  Brass/Kalimba); `Sledgehammer` offset 512 (gid 1058.. → `PATa[546..]` = Sledgehammer Sha…). Proof
  it is per-scene: the *same* tone `Sledge Syn Vox` (`PATa[551]`) is gid 806 in one scene and 1063
  in another. `PATa` holds many **duplicate** tones, records don't embed their gid, and the
  per-scene offset is **not** stored in the scene record. `USDa` was the prime suspect but turned
  out to be **user sample waveform data** — an 8-byte directory of `SMPd` (sample) sub-sections,
  not a tone directory. The gid appears to index a **de-duplicated** user-tone list while `PATa`
  stores per-scene bundles *with* duplicates (so gid ≈ raw index + a per-scene offset), and the
  mapping between the two is still unlocated.

  **Investigated and set aside.** Cross-referencing the backup against the NARF scene export shows
  `gid → PATa index` *is* a well-defined many-to-one function, piecewise-linear in blocks (offsets
  361 / 489 / 255 / 127 over contiguous gid ranges). But it is **not reliably shippable**: (a) the
  block offsets/boundaries can only be recovered with a matching export as an oracle (they aren't
  derivable from the backup alone), and (b) after this user renamed imported tones, the backup holds
  **content-identical duplicates under different names** (`Africa Brass` @1 vs `Uptown Brass 3` @572),
  so a "correct" index can still show a different name than the panel. Resolving backups faithfully
  would need the synth's actual gid→tone resolution rule (likely tied to how the USER bank is
  loaded), which isn't in the file in a form we've found. Backups keep showing `user #id`.

**Validation** — "Africa Main" (scene 385) decodes to exactly the panel's 4 zones:
Z1 Brass 0–71 · Z2 Kalimba 73–127 · Z3 Kalimba 72–72 · Z4 JX-Cream 0–71 (levels 107/107/100/82).

**Still TBD:** the tone-reference encoding (table B); pan and other per-zone params; the
scene-common block before `0x194`.

Confirmed across banks: PRISMA 16, NARF 50, TOP80 83, full backup **512** scenes
(PRFa size 1828880 = 16-byte header + 512 × 3572). Two adjacent names sit 0xdf4 apart
(`DSOTM Breathe` @0xc0, `On The Run` @0xeb4).

> Note: the `count` field at +0 was initially mistaken for a "data offset" because PRISMA happens
> to have exactly 16 scenes and a 16-byte header. NARF (count 0x32) disproved that — records always
> begin at a fixed 16-byte header, and +0 is the scene count.

### SVZ tone export (`EXPORT_Z-Core.svz`)
Same area-table shape, but the preamble differs: magic `SVZa` at 0x00, then a 12-byte preamble
(`02 02` looks like an area count = 2), area table at 0x10 with `format` = `ZCOR` (ZEN-Core).
Areas: `DIFa`, `PATa`. A 16-char tone name (`ACYL Lead`) sits ~0x9c.

## How to inspect

```sh
cargo run -p fantom-cli -- inspect fixtures/your-file.svd --len 512
cargo run -p fantom-cli -- inspect fixtures/your-file.svd --offset 0x40 --len 128
```

## Prior art

- [kimsand/Jupiter80Librarian](https://github.com/kimsand/Jupiter80Librarian) — Swift; `Model/`
  split into `SVDFile` / `SVDType` / `SVDTone` / `SVDLiveSet` / `SVDRegistration`.
- [sagamusix/JDTools](https://github.com/sagamusix/JDTools) — C++20; reads **and writes** JD-08 SVD
  banks (round-trip reference).
