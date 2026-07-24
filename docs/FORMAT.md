# Roland Fantom file formats — reverse-engineering notebook

A living record of what we learn about the on-disk layout. Prefer confirmed facts with the sample
file and byte offsets that prove them; mark guesses clearly.

## ⚠️ Model naming — Roland uses two very similar names for different products

- **FANTOM-6 / FANTOM-7 / FANTOM-8** (2019, flagship workstation, no leading zero) — **this is the
  hardware everything in this doc was reverse-engineered and panel-verified against.**
- **FANTOM-06 / FANTOM-07 / FANTOM-08** (2022, "Fantom-0 series", *with* a leading zero) — a
  separate, cheaper product line. **Not tested.** The format may or may not match.
- **FANTOM-6 EX / 7 EX / 8 EX** — a newer expanded-memory revision of the flagship line. **Not
  tested**, though as a revision of the same tested line it's the more likely of the two to share
  this layout.

Earlier revisions of this doc (and the code) mistakenly called the tested hardware "Fantom-0" —
that name refers to the *other*, untested product line. Everything here has only been confirmed on
a plain FANTOM-6.

## File types

| Ext    | What it holds                          | Notes                                        |
|--------|----------------------------------------|----------------------------------------------|
| `.svd` | Full backup container                  | Roland backup format, model-specific layout  |
| `.svz` | ZEN-Core tone data                     | Individual tones; also used by Jupiter-X etc |
| `.sdz` | Sound pack                             | Roland Cloud sound content                   |

Scenes exported from a FANTOM-6 land in a `SOUND/` folder on the USB drive.

## Data model

ZEN-Core hierarchy: **Tones → Zones → Scenes**. A Scene has up to 16 Zones; each Zone references a
Tone plus performance settings.

## SVD5 container — CONFIRMED (FANTOM-6, from `fixtures/backup`)

Verified against `ROLAND/SOUND/PRISMA/FANTOM.SVD` (FANTOM-6 backup). All integers little-endian.

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
| +0x00 | tone MSB    | MIDI Bank Select MSB; selects the sound engine/area                |
| +0x01 | tone LSB    | MIDI Bank Select LSB; user page or factory bank                    |
| +0x02 | tone PC     | Zero-based program/index within the bank                           |
| +0x03 | zone index  | 0..15                                                             |
| +0x07 | `level`     | 0..127. TEST2→TEST3 set zone0 level `64`→`32` (100→50) at 0x19b   |

The tone reference is the three-byte **MSB / LSB / PC** tuple at table B `+0x00..+0x02`.
Earlier analysis treated LSB/PC alone as a big-endian `tone_id`; that representation remains useful
for ZEN-Core preset lookup, but the MSB is essential for distinguishing engines:

| MSB | User LSB | Bundled area | Engine/data |
|-----|----------|--------------|-------------|
| 86  | 0        | `RHYa` + same-index `INSa` | Rhythm kit and instruments |
| 87  | 0..63    | `PATa`       | ZEN-Core tone |
| 89  | 0        | `SNAa`       | SN-A (SuperNATURAL Acoustic) |
| 91  | 0        | `VTWa`       | Virtual ToneWheel organ |
| 105 | 0        | `ZAPa`       | SN-AP (SuperNATURAL Acoustic Piano) |
| 105 | 1        | `ZEPa`       | SN-EP (SuperNATURAL Electric Piano) |

Confirmed across NARF, TOP80, and PRISMA: every user tuple directly indexes its area record as
`(LSB - first_user_LSB) × 128 + PC`. Factory-bank LSBs are left unchanged.

The FANTOM EX MIDI Implementation makes the broader engine selector authoritative:

| MSB | Engine/group |
|-----|--------------|
| 86 | Drum Kit |
| 87 | ZEN-Core |
| 89 | SN-A |
| 90 | V-Piano |
| 91 | VTW |
| 92, 100 | EXZ Drum Kit |
| 93, 101 | EXZ Tone |
| 97 | MODEL |
| 103 | Expansion V-Piano |
| 105 | EXSN |
| 107 | ACB |

It also confirms `89/65 = SN-A PRST`, `90/64 = V-Piano PRST`, and `91/65 = VTW PRST`.
The EX Sound List further confirms `86/64 = Drum PR-A`, `86/65 = Drum CMN`,
`105/64 = EXSN01 (SN-AP)`, `105/65 = EXSN02 (SN-EP)`,
`105/66 = EXSN03 (SN-AP)`, and `107/64 = ACB JP8`. Manual PC values are one-based;
the scene's stored PC byte is zero-based. These facts identify references for display but do not
by themselves prove how USER records for the newer engines are bundled in an SVD export.

The `fixtures/TONEMAP/FANTOM.SVD` export adds these observed bank mappings:

| Engine | MSB/LSB | Bank |
|--------|---------|------|
| VPiano | 90/0 | USER |
| VPiano | 90/64 | PRST |
| Expansion VPiano | 103/64 | M09X01 |
| MODEL | 97/64 | USER |
| MODEL | 97/66 | JP8 |
| MODEL | 97/68 | JU106 |
| MODEL | 97/70 | JX8P |
| MODEL | 97/72 | n/zyme |
| MODEL | 97/79 | SH101 |
| ACB | 107/0 | USER |
| ACB | 107/64 | JP8 |
| ACB | 107/66 | SH101 |
| ACB | 107/70 | JU106 |
| ACB | 107/76 | JX3P |

The export did not contain a JD-800 MODEL zone, so its bank address remains unknown.

For historical comparison, the old LSB/PC-as-BE16 representation looked like this:
| Tone (panel)        | tone_id (BE16) | hi   | lo   |
|---------------------|----------------|------|------|
| USER 448 (Brass)    | 827  (0x033b)  | 0x03 | 0x3b |
| USER 449 (Kalimba)  | 828  (0x033c)  | 0x03 | 0x3c |
| PR-AA 61 (JX Cream) | 23612 (0x5c3c) | 0x5c | 0x3c |
| INIT default (TEST) | 23868 (0x5d3c) | 0x5d | 0x3c |

**Resolved (mechanism confirmed via TONEMAP controlled capture):** the reference is NOT a global
tone number. When a scene is saved, the Fantom **bundles the USER tones it references into the
corresponding area and renumbers them**. For ZEN-Core, `(LSB × 128) + PC` indexes `PATa`. Verified:
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

- **Scene exports** (`SOUND/…`, single/multi-scene): `PATa` holds exactly the referenced user tones,
  directly indexed by 7-bit PC pages (`LSB × 128 + PC`; NARF's 348 tones end at LSB 2 / PC 91).
  The old BE16 interpretation made those same references appear sparse. Verified
  end-to-end: NARF scene 44 "Sledgehammer" → `Sledgehammer Sha / Sledge + Hammer / …` matching the
  panel; TONEMAP (gids 0/1/2) still direct.
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

  **Revisited with the export rank rule — does not transfer.** The rank trick works for exports
  because their `PATa` *is* gid-sorted and equals the referenced set. A backup's `PATa` holds all
  ~2048 user tones and is **not** gid-sorted (panel-confirmed `Africa Brass` is gid 827 yet index 1),
  so `rank(gid) == index` fails on every one of 349 oracle-matched pairs, and the gids are reassigned
  on import (export gid 565 ≠ backup gid 1058 for the same tone). Combined with the heavy duplication
  from user editing, no derivable rule emerged.

  **Investigated and set aside.** Cross-referencing the backup against the NARF scene export shows
  `gid → PATa index` *is* a well-defined many-to-one function, piecewise-linear in blocks (offsets
  361 / 489 / 255 / 127 over contiguous gid ranges). But it is **not reliably shippable**: (a) the
  block offsets/boundaries can only be recovered with a matching export as an oracle (they aren't
  derivable from the backup alone), and (b) after this user renamed imported tones, the backup holds
  **content-identical duplicates under different names** (`Africa Brass` @1 vs `Uptown Brass 3` @572),
  so a "correct" index can still show a different name than the panel. Resolving backups faithfully
  would need the synth's actual gid→tone resolution rule (likely tied to how the USER bank is
  loaded), which isn't in the file in a form we've found. The reader therefore shows the raw
  MSB/LSB/PC address for unresolved backup tones.

**Validation** — "Africa Main" (scene 385) decodes to exactly the panel's 4 zones:
Z1 Brass 0–71 · Z2 Kalimba 73–127 · Z3 Kalimba 72–72 · Z4 JX-Cream 0–71 (levels 107/107/100/82).

**Still TBD:** pan and other per-zone params; the scene-common block before `0x194`.

### Controlled user-tone parameter diffs (TONEMAP4/5)

Single-scene exports with one in-place parameter edit confirm the following offsets within the
first user-tone record of the opaque engine areas. These are read-only observations, not yet a
complete decoder; all other bytes must remain opaque.

| Area | Relative offset | Confirmed parameter | Observed change |
|------|-----------------|---------------------|-----------------|
| `DCWa` | `0x0025` | V-Piano tuning type | PRESET -> OFF (`01 -> 00`) |
| `DCWa` | `0x008D` | V-Piano lid | 3 -> 5 |
| `DCWa` | `0x0090` | V-Piano hammer noise (signed) | +2 -> -2 (`02 -> FE`) |
| `DCWa` | `0x0096` | V-Piano key-off noise | OFF -> 4 |
| `MDLa` | `0x05D8` | Model OSC cross-mod | 1143 -> 1068 |
| `MDLa` | `0x06B6` | Model filter cutoff (LE u16) | 962 -> 566 |
| `MDLa` | `0x06B8` | Model filter resonance (LE u16) | 863 -> 0 |
| `MDLa` | `0x06D0` | Model ENV2 attack (LE u16) | unknown -> 299 |
| `ACBa` | `0x1B70` | ACB VCO1 wave | TRI -> SAW |
| `ACBa` | `0x1BE4` | ACB ENV1 sustain | 0 -> 176 |
| `ACBa` | `0x1D40` | ACB effect type | Phaser -> Fuzz |
| `ACBa` | `0x1D54` | ACB LFO rate | 155 -> 142 |

The edits also change a small number of neighboring/derived fields. No scene or system bytes
changed in these captures. The offsets are relative to the area start (including its 16-byte
area header), and must not be applied to multi-record areas without first locating the record.

### Opaque user-tone areas are record tables (TONEMAP6/7)

The earlier conservative description of `ACBa`, `DCWa`, and `MDLa` as indivisible payloads is
superseded for scene exports. Each is an area with the standard 16-byte count/record-size header
and fixed-size user-tone records. `TONEMAP7V_ACB2` contains two ACB USER scenes and proves the
indexing rule: `ACBa` has count `2`, record size `9984`, and the scenes reference PC `0` and `1`.
The single-tone TONEMAP6 exports establish the corresponding record sizes:

| Area | Engine | Single-record area size | Record size |
|------|--------|-------------------------|-------------|
| `ACBa` | ACB USER | `10000` | `9984` |
| `DCWa` | V-Piano USER | `700` | `684` |
| `MDLa` | Model USER | `2064` | `2048` |

For scene exports, USER references use the record's zero-based PC index. This makes record-level
deduplication and index rebasing possible; multi-record V-Piano and Model captures are still
needed before enabling those two families in the merger.

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

## Writing / metadata edits (DIFa checksum)

The `DIFa` area holds a 32-byte value that looks like a checksum (high-entropy on freshly exported
files, e.g. TONEMAP `05 00 ac 0a 84 0a c4 1e …`; **all-zero** on older exports like PRISMA). It does
**not** match SHA-1/256/512, MD5, or BLAKE over any obvious region — likely keyed or proprietary.

**It appears the FANTOM does not hard-enforce it.** [Smirnov75/svd5tool](https://github.com/Smirnov75/svd5tool)
unpacks and **repacks** SVD5 backups without recomputing `DIFa`, and that workflow is used
successfully — so an edited file with a stale (or zeroed) `DIFa` should still load. (Its `[xxxx]`
filename tag is **CRC-16/CCITT**, poly `0x1021`, init `0xFFFF`, over each area body from `+0x10` — a
convenience label, not the file checksum.) svd5tool also independently confirms this doc's structure:
header + record table of `id / signature / offset / length`, each area prefixed with
`count / record_length / info_length`.

Metadata edits are therefore done **in place**: a scene rename overwrites the 16-byte name field, a
comment overwrites the 64-byte field at `+0x40`, and nothing else changes (verified: renaming a
7-char scene touches exactly the differing name bytes). A renamed scene-export bank was loaded
successfully on a FANTOM-6 and displayed the edited name.

Scene-export repackaging uses the confirmed MSB/LSB/PC mapping above. Extract/merge copy complete
opaque records, de-duplicate identical dependency bundles, assign fresh dense per-engine indexes,
rewrite zone LSB/PC bytes, and rebuild the area table. `RHYa` and `INSa` records remain paired.
Source-only engine areas are added to the output. Full backups remain rejected because their
ZEN-Core mapping is unresolved.

The `canary` command strengthens that hardware test without changing synthesis parameters: it
extracts one scene, prefixes its name with `CNY`, and renames records in every recognized bundled
dependency area `CNY01…CNYNN`. Seeing those names in the imported scene's zones proves that the
FANTOM read the rebuilt bundle rather than merely resolving pre-existing tone names.

**Hardware-confirmed on a FANTOM-6:** extracting NARF scene 44 produced one `CNY Sledgehammer`
scene with its eight renamed bundled tones; the names appeared on the instrument and its zones,
keyboard groups, tones, and samples worked. Merging independently extracted `CNY Africa Main` and
`CNY Sledgehammer` produced a two-scene, ten-tone bank and both scenes worked on the instrument.
This confirms `PRFa` rebuilding, `PATa` rebuilding, tone-id rebasing, and same-origin multi-scene
merging.

**Cross-bank status:** `RHYa`/`INSa`, `VTWa`, `SNAa`, `ZAPa`, and `ZEPa` are now dependency-aware
and mergeable. NARF, TOP80, and PRISMA round-trip locally with identical decoded ZEN-Core
assignments; extracting PRISMA `Time` produces exactly two `PATa`, one `SNAa`, and one `ZEPa`
record. A NARF/PRISMA canary was hardware-confirmed with working keyboard groups and sounds,
including `Time`'s SN-A and SN-EP zones. External sample waveform files and any unknown future area
kinds are not copied.

The typed reader retains the complete MSB/LSB/PC tuple and reports the documented panel-facing
types `Drum`, `ZEN-Core`, `SN-A`, `SN-AP`, `SN-EP`, `EXSN`, `VTW`, `VPiano`, `MODEL`, `EXZ`,
and `ACB`. Confirmed USER dependency areas resolve their record names. Unknown types and
unconfirmed banks deliberately display their raw MSB/LSB/PC values.

**Unsupported newer/model families:** full backups contain `DCWa` (128 piano records, likely the
VPiano USER bank) and `MDLa` (1024 opaque model records). The manual confirms MSB 90 is VPiano and
MSB 97 is MODEL, but their SVD scene-export dependency mapping is not yet confirmed. VPiano USER,
MODEL/ABM USER, ACB USER, and newer EX-only model data are therefore not bundled.
Factory/installed expansion references are left unchanged, but only work on a destination
instrument that has the same engine/model/expansion installed.

## Prior art

- [kimsand/Jupiter80Librarian](https://github.com/kimsand/Jupiter80Librarian) — Swift; `Model/`
  split into `SVDFile` / `SVDType` / `SVDTone` / `SVDLiveSet` / `SVDRegistration`.
- [sagamusix/JDTools](https://github.com/sagamusix/JDTools) — C++20; reads **and writes** JD-08 SVD
  banks (round-trip reference).
- [Smirnov75/svd5tool](https://github.com/Smirnov75/svd5tool) — Pascal; unpack/repack SVD5 backups.
  Confirms the container structure and that repacking without touching `DIFa` is accepted.
