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

Area tags seen (in order): `PRFa PATa RHYa INSa DCWa VTWa SNAa ZAPa ZEPa MDLa SMPa MLSa SYSa DIFa
USDa`; a scene export carries only the subset its scenes need, plus `SYSa`.

| Tag | Holds |
|-----|-------|
| `PRFa` | Scenes (performances) |
| `PATa` | ZEN-Core user tones |
| `RHYa` | Drum kits; `INSa` holds each kit's 88 instruments |
| `INSa` | Drum-kit instrument sets, indexed in lockstep with `RHYa` |
| `DCWa` | V-Piano user tones |
| `VTWa` | Virtual ToneWheel user tones |
| `SNAa` | SN-A user tones |
| `ZAPa` / `ZEPa` | SN-AP / SN-EP user tones |
| `MDLa` | MODEL user tones |
| `SMPa` | User sample slots (8000) |
| `MLSa` | User multisamples (128) |
| `USDa` | User sample waveform data (`SMPd` sections) |
| `ACBa` | ACB user tones (scene exports; absent from the backups seen) |
| `SYSa` | System settings — one 904-byte record, undecoded |
| `DIFa` | 32-byte checksum-like value, not enforced by the instrument |

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

`TONEMAP2` confirms that native ACB factory references are address-only in a scene export. Its
JP8 PC 0 and PC 1 scenes have no `ACBa` area, and their `PRFa` records differ only in the scene
name and PC byte; no factory tone name is serialized.

**Factory preset tones** instead encode a fixed ROM reference: `tone_id = (LSB << 8) | (PC - 1)`,
with MSB always 87 for ZEN-Core tones (so the `0x4000` bit is always set — preset LSBs are ≥ 64).
Verified against Roland's *FANTOM Sound List*: `JX Cream` = PR-A 0061 (LSB 92, PC 61) = `0x5c3c`;
PR-A 0001 = `0x5c00`; PR-B 0001 = `0x4000`. The 3667 ZEN-Core preset tones are bundled in
`crates/fantom-core/src/preset_tones.tsv` and resolved by [`presets::lookup`]. (Drum kits use MSB 86
and share the same 16-bit id space, so they are omitted to avoid mislabelling.)

`PATa` layout mirrors `PRFa`: 16-byte header (`count`, `record_size`, `data_start=0x10`), then
`count` records of `record_size` bytes; the tone **name** is the first 16 ASCII bytes, and byte
`+0x10` is the tone **category** (`0x23` = brass).

### Exports and backups index their user banks identically — CONFIRMED

The same rule reads both file kinds:

```
index = (LSB - first_lsb) * 128 + PC        for LSB < 64
```

- **Scene exports** (`SOUND/…`) bundle only the tones their scenes reference and renumber them
  densely from zero. NARF's 348 tones end at LSB 2 / PC 91.
- **Full backups** carry the whole USER bank at fixed slots — `PATa` 2048, `SNAa` 256, `RHYa`,
  `VTWa`, `ZAPa`, `ZEPa`, `DCWa` 128 each, `MDLa` 1024 — matching the slot counts Roland documents
  for the USER banks. Most records are simply unreferenced.

**Evidence.** Matching scenes *by name* between each export and the backup it came from, then
comparing the resolved tone name on both sides: NARF ↔ `Black NARFSOUNDS` **464 of 464** ZEN-Core
zones agree; TOP80 ↔ `2023.4.8+topandprisma` `PATa` 51/51, `RHYa` 3/3, `SNAa` 1/1, `ZAPa` 3/3;
PRISMA ↔ same backup `PATa` 16/16, `VTWa` 9/9, `ZEPa` 3/3, `ZAPa` 8/8, `RHYa` 1/1, `SNAa` 2/2. This
is not circular: the two files hold *different addresses* for the same sound (`Africa Brass` is
`PATa[207]` in the export and `PATa[443]` in the backup), so agreement can only come from reading
both correctly. Pinned by `crates/fantom-core/tests/fixtures.rs`.

> **Correction — where the earlier "unresolvable backup mapping" came from.** Previous revisions of
> this document recorded backup user tones as unresolvable, after a rank rule and a piecewise-linear
> block mapping both failed. Those attempts were fitting an artifact: the "gids" in the analysis
> (827, 1058, offsets 826 / 512) were **front-panel USER numbers**, not the bytes in the file.
> `Africa Main` zone 1 stores `lsb 3 / pc 59` → index **443** → `Africa Brass`, exactly as the panel
> shows. The wrong anchor looked plausible because `PATa[1]` happens to be a duplicate of the same
> tone. There was never a second mapping to find. When an oracle disagrees with a simple rule this
> consistently, check the oracle.

### The user sample bank — `SMPa`, `USDa`, `MLSa` (CONFIRMED)

Present in full backups only; a backup whose user never sampled has an 8-byte `USDa` and no named
`SMPa` records.

**`SMPa`** — 8000 slots × 84 bytes, one per panel sample slot:

| Off | Size | Field | Notes |
|-----|------|-------|-------|
| `0x00` | 16 | name | ASCII, space-padded |
| `0x40` | 1 | in-use | 1 when the slot holds a sample |
| `0x41` | 1 | level | `0x7f` typical |
| `0x44` | 1 | loop mode | 0/1/2 seen; meanings unconfirmed |
| `0x45` | 1 | original key | MIDI note, 60 = C4 default |
| `0x48` | 4 | start | frames, LE |
| `0x4c` | 4 | loop point | frames, LE |
| `0x50` | 4 | end | frames, LE |

Bytes `0x10`–`0x3f` are zero in every populated record across three backups; meaning unknown.

**`USDa`** — a single record holding the audio. It opens with a directory of 8-byte
`{u32 slot, u32 offset}` entries terminated by `slot = 0xFFFFFFFF` (whose offset is the body
length), each offset pointing at an `SMPd` section relative to the `USDa` body:

| Off | Size | Field |
|-----|------|-------|
| `0x00` | 4 | `SMPd` magic |
| `0x08` | 4 | section size in bytes |
| `0x0c` | 4 | count of 16-bit words (two per frame → stereo) |
| `0x10` | 16 | name as imported |
| `0x20` | 4 | sample rate (48000 in every section seen) |
| `0x80` | … | 16-bit PCM |

Verified on `2023.4.8+topandprisma`: 50 directory entries, 50 `SMPd` magics in the file, 50 named
`SMPa` slots, agreeing by position and name. `words / 2 == SMPa.end` exactly for 48 of 50 (the two
exceptions are trimmed samples). Slot names can differ from section names when a slot was renamed
after import.

**`MLSa`** — 128 multisamples × 1040 bytes: a 16-byte name followed by 128 eight-byte entries
(16 + 128×8 = 1040), one per MIDI key. Every record in all three backups is still the factory
default (`INITIAL MSMPL`, entries `00 00 7f 00 80 00 00 00`), so the entry fields are **not
decoded**. Needs a capture with a real user multisample.

### How a tone references a sample — CONFIRMED

A ZEN-Core `PATa` record holds **four partials at stride 124 (`0x7c`)**. Each partial selects where
its wave comes from:

| Record offset | Size | Field |
|---------------|------|-------|
| `0xdf + p*124` | 1 | wave **group**: `0` = internal ROM wave, `2` = **user sample**, `1`/`3` also seen |
| `0xe2 + p*124` | 2 | wave **number**, LE — a ROM wave index, or a **1-based `SMPa` slot** when the group is 2 |

Evidence, from `Black NARFSOUNDS` (2048 tones × 4 partials = 8192 partials):

- 93 partials have group 2, and **every one** of them names a slot in `1..50` — exactly the number
  of populated `SMPa` slots. Not one falls outside.
- The names corroborate independently: `IML Whoa 1` → slot 3 `3 IML Whoa 1`; `Relax Bass`
  partials 1 and 2 → slots 7 and 8 (`7 Relax Bass 1 E`, `8 Relax Bass 2 E`); `LOAP Voc 1`
  partials 1–4 → slots 14–17 (`PAD 1`…`PAD 4`); `MJ Samp` → slot 2 `2 that way`.
- Control: `Africa Brass` is `(group 0, wave 383)` on all four partials — a ROM wave, not a slot.
  The group byte is what separates the two; the wave number alone is ambiguous (ROM indexes run to
  at least 895 and overlap the slot range).

**The instrument does not bundle samples into a scene export.** The FANTOM-written NARF export
carries **68 group-2 partials and no `SMPa`/`USDa` area at all** — it keeps the slot references and
leaves the audio behind. So dropping those areas when repackaging matches Roland's own behavior; a
sampled tone is complete only where the destination already holds that sample in that slot. The
CLI now names the required slots rather than warning generically.

> **This is untestable by ear on the source instrument.** Re-importing a canary there sounds
> correct because its slots still hold the audio the tone points at. Confirmed on a FANTOM-6:
> `CNY Levitating`'s sampled zones played normally despite the bank carrying no audio. Only the
> byte-level reference above, or an import onto an instrument with different sample content,
> distinguishes "the samples travelled" from "they were already there".

**Still unknown:** the `MLSa` multisample entry fields, and whether a multisample is referenced by
the same group/number scheme (no fixture has a populated multisample).

### `INSa` is `RHYa`'s payload, not a separate bank

`count(INSa) == count(RHYa)` in every fixture, and each 19008-byte `INSa` record is **88 sub-records
of 216 bytes**, each starting with a 16-byte instrument name (88 × 216 = 19008 exactly):

```
INSa[0] +0x000 'TR-808 Rimshot P'
        +0x0d8 'Elec Stick 5'
        +0x1b0 'TR-909 Rimshot P'   (stride 0xd8 = 216)
```

So `INSa[i]` holds the 88 key instruments of drum kit `RHYa[i]`. A scene's MSB-86 reference selects
the pair, which is why the two areas must always be copied and renumbered together.

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

> **Offset convention.** `fantom diff` reports **record-relative** offsets, which is 0x10 less than
> the area-relative offsets in the table above (`DCWa 0x0025` = `DCWa[0]+0x0015`). Record-relative is
> the useful form — it is the same in every record of a multi-record area. Prefer it in new entries.

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

ACB USER tone names occupy 16 bytes at record offset `0x1c44`. Each four-byte word is stored
byte-reversed: for example, `tfoS S & ltbu  2e` decodes to `Soft & Subtle2`. Confirmed by the
controlled `TONEMAP9_ACB` / `TONEMAP9_ACB2` rename pair, whose files differ only at the final
name character. The scene's `107/0/PC` reference indexes these `ACBa` records directly.

For scene exports, USER references use the record's zero-based PC index. This makes record-level
deduplication and index rebasing possible. `TONEMAP10_MOD` hardware-confirms the multi-record
Model case: two `MDLa` records named `Berlin Night  4` and `Berlin Night  6` are referenced as
PC 0 and PC 1. `TONEMAP10_VP` confirms the same rule for V-Piano: two `DCWa` records named
`Stage Grand4` and `Stage Grand4 3` are referenced as PC 0 and PC 1.

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

**Finding new offsets.** Export two files that differ by exactly one deliberate change, then let
`diff` locate it — it aligns the files by area and record, so a size change in one area does not
smear the report:

```sh
cargo run -p fantom-cli -- diff fixtures/TONEMAP9_VP/FANTOM.SVD fixtures/TONEMAP9_VP2/FANTOM.SVD
# DCWa[0]+0x000b  @0x000e6f  33 -> 34  |3| -> |4|

cargo run -p fantom-cli -- diff a.svd b.svd --area MDLa --context 4
```

Every offset in the tables above reproduces through this command. Record what you learn here, with
the fixture pair that proves it.

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

Repackaging uses the confirmed MSB/LSB/PC mapping above. Extract/merge copy complete records,
de-duplicate identical dependency bundles, assign fresh dense per-engine indexes, rewrite zone
LSB *and* PC bytes, and rebuild the area table. `RHYa` and `INSa` records remain paired.
Source-only engine areas are added to the output.

**Both a scene export and a full backup can be the source.** Since the two index their user banks
identically, the only difference is how much is left behind: a backup's areas are mostly
unreferenced, and only the records the selected scenes actually use are carried over. Extracting
scene 385 from a 35 MB backup produces a 7.9 KB bank. Verified locally by extracting each of the
149 scenes shared between the three export/backup fixture pairs from *both* sources and asserting
the results decode identically.

The output is always a self-contained scene-export bank. Writing a full backup — placing records
back into fixed USER slots — is a separate problem and is not supported.

`SYSa` and `DIFa` are copied verbatim. `SMPa`, `MLSa`, and `USDa` are recognised and deliberately
**dropped**, because no decoded field links a tone to a sample; the CLI warns when a source carries
user samples. Any area outside those sets stops the rebuild rather than being silently lost.

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

**Hardware-confirmed from a full backup (2026-08-12).** Four canary banks were extracted straight
out of full backups and imported on a FANTOM-6. All four loaded, and every zone showed its `CNYnn`
prefix, proving the instrument read the rebuilt bundle rather than resolving a tone it already had:

| Bank | Source | Engines exercised | Result |
|------|--------|-------------------|--------|
| `CNY Deep V-Grand` | `2023.4.8+topandprisma` scene 155 | `DCWa` (V-Piano USER), `PATa` | names correct — record 124 of a 128-slot bank rebased to 0 |
| `CNY Time` | same, scene 358 | `PATa`, `SNAa`, `ZEPa` | names correct |
| `CNY Us And Them` | same, scene 361 | `VTWa`, `ZAPa` | names correct |
| `CNY Levitating` | `Black NARFSOUNDS` scene 409 | `PATa`, `RHYa`+`INSa`, `SNAa` | names correct, kit pairing intact |

This confirms the direct index rule end to end on hardware, the opaque-engine path encoding an
index across LSB pages, and `RHYa`/`INSa` staying paired through a rebase.

**Wiped-memory re-import — the decisive run.** `CNY Levitating` was then re-tested after clearing
everything it depends on: an INIT tone was written over every USER slot it referenced (ZEN-Core,
drum kit, SN-A) *and* an INIT scene over its scene slot. Re-importing the same bank recreated all
of them, correctly, and the scene played. Because the destination slots held nothing but INIT data
beforehand, the instrument can only have written these records out of the bank's own bundle — this
rules out the alternative explanation that a canary's names merely resolve against tones already in
memory. `PRFa`, `PATa`, `RHYa`+`INSa`, and `SNAa` rebuilding are therefore confirmed on hardware,
not just inferred.

The sampled zones still sounded, as the slot model predicts: the *tones* were wiped, the **sample
memory was not**, so the restored tones' group-2 references still pointed at populated slots 5 and
6. That is consistent with — but not a test of — sample portability; see the note below.

> **The sample gap cannot be tested on the source instrument.** The sampled zones of
> `CNY Levitating` *did* sound, even though the bank carries no `SMPa`/`USDa`. That is not evidence
> the samples travelled: the import went back onto the same instrument the backup came from, whose
> sample memory still holds `5 Lev Loopx` and `6 Levitating Yey` at the same slots, so a tone
> referencing those slots plays correctly by coincidence. It does suggest the tone→sample reference
> is a **slot number** rather than embedded audio. Proving it needs either a byte-level decode
> (the `PATa` diff capture) or an import onto an instrument with different sample content —
> listening on this one cannot answer it.

**Cross-bank status:** `RHYa`/`INSa`, `VTWa`, `SNAa`, `ZAPa`, and `ZEPa` are now dependency-aware
and mergeable. NARF, TOP80, and PRISMA round-trip locally with identical decoded ZEN-Core
assignments; extracting PRISMA `Time` produces exactly two `PATa`, one `SNAa`, and one `ZEPa`
record. A NARF/PRISMA canary was hardware-confirmed with working keyboard groups and sounds,
including `Time`'s SN-A and SN-EP zones. External sample waveform files and any unknown future area
kinds are not copied.

The typed reader retains the complete MSB/LSB/PC tuple and reports the documented panel-facing
types `Drum`, `ZEN-Core`, `SN-A`, `SN-AP`, `SN-EP`, `EXSN`, `VTW`, `VPiano`, `MODEL`, `EXZ`,
and `ACB`. Confirmed USER dependency areas resolve their record names in exports and backups alike.
Unknown types and unconfirmed banks deliberately display their raw MSB/LSB/PC values.

The address→area table lives in one place, `crates/fantom-core/src/address.rs`, and both the reader
and the repackager consult it, so they cannot drift apart.

**Modelled engines** (`DCWa` V-Piano, `MDLa` MODEL, `ACBa` ACB) are handled as record tables whose
*internals* stay opaque: records are copied verbatim, de-duplicated by byte equality, and
renumbered. That works from a backup too — a backup's `MDLa` holds 1024 records, which needs the
index encoded across LSB pages rather than in PC alone. Confirmed end-to-end by extracting a
backup's `VPiano USER` scene: `DCWa[124]` → a single-record `DCWa` in the output.

Factory/installed expansion references are left unchanged, but only work on a destination
instrument that has the same engine/model/expansion installed.

## Prior art

- [kimsand/Jupiter80Librarian](https://github.com/kimsand/Jupiter80Librarian) — Swift; `Model/`
  split into `SVDFile` / `SVDType` / `SVDTone` / `SVDLiveSet` / `SVDRegistration`.
- [sagamusix/JDTools](https://github.com/sagamusix/JDTools) — C++20; reads **and writes** JD-08 SVD
  banks (round-trip reference).
- [Smirnov75/svd5tool](https://github.com/Smirnov75/svd5tool) — Pascal; unpack/repack SVD5 backups.
  Confirms the container structure and that repacking without touching `DIFa` is accepted.
