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
(e.g. `DSOTM Breathe`, `Africa Main`). It holds two parallel 16-entry per-zone tables. Offsets
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

The **tone reference** also lives in table B but is **encoded** — the displayed tone numbers
(448/449/61) do NOT appear as plain 16-bit values, so it is bank/type/number packed. Deferred.

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
