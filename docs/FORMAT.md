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

**Each scene record (`record_size` bytes)** is Roland's Scene parameter blocks packed end to end,
by the file/wire rule below. Zone order is 1:1 with the panel (zone 0 = Zone 1).

| File   | Block              | Len | ×  | Holds                                          |
|--------|--------------------|-----|----|------------------------------------------------|
| `0x000`| `[Scene Common]`   | 144 | 1  | name, level, tempo, memo, colour, rating       |
| `0x090`| `[Chorus]`         | 48  | 1  | scene chorus                                   |
| `0x0c0`| `[Reverb]`         | 44  | 1  | scene reverb                                   |
| `0x0ec`| `[MFX]`            | 84  | 2  | scene IFX 1 and 2                              |
| `0x194`| `[Scene Zone]`     | 72  | 16 | tone reference, level, pan, tune, scale, Rx    |
| `0x614`| `[Zone EQ]`        | 12  | 16 | three-band EQ per zone                         |
| `0x6d4`| `[Zone Control]`   | 96  | 16 | key/velocity range, transpose, arp, external   |
| `0xcd4`| `[Scene Controller]`| 256| 1  | pedal, knob, slider and wheel assigns          |
| `0xdd4`| `[Analog Filter]`  | 32  | 1  | FANTOM-8 analog filter                         |

These tile `0xdf4` = 3572 exactly, with no gap. Block lengths are the packed parameter length plus
padding that the wire map does not predict, so they are measured, not derived; `params::scene`
asserts the tiling. Selected fields:

| Block | Off | Field | Evidence |
|-------|-----|-------|----------|
| Scene Common | +0x10 | Scene Level | 100 in TEST 1 |
| Scene Common | +0x38 | Scene Tempo, u16 ×100 | `12000` = 120.00 BPM |
| Scene Common | +0x40 | Scene Memo, 64 ASCII | wire `+0x42`; see the packing rule |
| Scene Zone | +0x00..02 | tone MSB / LSB / PC | |
| Scene Zone | +0x03 | Receive Channel | 0..15, *not* the zone index |
| Scene Zone | +0x07 | Zone Level | TEST2→TEST3 set zone0 `64`→`32` at 0x19b |
| Scene Zone | +0x08 | Zone Pan | zero-centred |
| Zone Control | +0x00 | Keyboard Switch | TEST1→TEST2 flipped zone1's 0x738 0→1 |
| Zone Control | +0x04 | Keyboard Range Lower | TEST3 set zone0 `3c` (C4) at 0x6d8 |
| Zone Control | +0x05 | Keyboard Range Upper | TEST3 set zone0 `48` (C5) at 0x6d9 |
| Zone Control | +0x08 | Zone Transpose | zero-centred, ±48 |

> **The zone table starts at `0x6d4`, not `0x6d0`.** `container::zone::RawZone` frames it four bytes
> early, so its fields read four higher than Zone Control's (`enable` = `+0x04` is Keyboard Switch
> `+0x00`) and its `cf cd` marker at `+0x3e` is Zone Control `+0x3a`. The four bytes are the tail of
> the last `[Zone EQ]` entry. The framing is consistent, so the decoder is correct; only the
> boundary was misplaced, which is why the 188 bytes before it looked like an unexplained gap.

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
| `0x04` | 4 | flags — `0x02010020`, or `0x42010020` on 4 of 50 |
| `0x08` | 4 | total audio size, both channels including padding |
| `0x0c` | 4 | bytes of real PCM **per channel** |
| `0x10` | 16 | name as imported |
| `0x20` | 4 | sample rate (48000 in every section seen) |
| `0x24` | 4 | per-sample word, carried into an SVZ's `USDa` directory |
| `0x80` | … | 16-bit PCM, **left channel block then right**, each padded |

**The audio is not interleaved.** Each channel is a contiguous block of 16-bit mono, padded up to a
multiple of 512 — with another 512 added when that would leave under 128 bytes of slack — and the
two blocks sit back to back. So `size == 2 × pad512(channel_bytes)`, which holds for **all 50**
samples of `2023.4.8+topandprisma`, and `channel_bytes == SMPa.end × 2` for 48 of them (the two
exceptions are the trimmed samples). The padding at the end of the left block is zero-filled.

> An earlier revision read `+0x0c` as "count of 16-bit words, two per frame", which gives the right
> frame count by coincidence — `words / 2` and `channel_bytes / 2` are the same number — but implies
> interleaved stereo, which this is not. The field name and the block layout come from an ImHex
> pattern for the related MC/MV format (see the attribution note below); the padding rule and the
> `size == 2 × pad512(…)` identity were then confirmed here.

Verified on `2023.4.8+topandprisma`: 50 directory entries, 50 `SMPd` magics in the file, 50 named
`SMPa` slots, agreeing by position and name. `words / 2 == SMPa.end` exactly for 48 of 50 (the two
exceptions are trimmed samples). Slot names can differ from section names when a slot was renamed
after import.

**`MLSa`** — 128 multisamples × 1040 bytes: a 16-byte name followed by 128 eight-byte entries
(16 + 128×8 = 1040), one per MIDI key. **The entry is now decoded**, from a capture:

| Off | Size | Field |
|-----|------|-------|
| `0x00` | 2 | user sample slot, 1-based; `0` = this key plays nothing |
| `0x02` | 2 | level (`127` in every entry seen) |
| `0x04` | 2 | pan (`128` = centre) |
| `0x06` | 2 | unknown, `0` everywhere |

A FANTOM-6 multisample built from three samples across three key ranges reads back exactly as its
panel showed:

```
T8_MSAMP   keys   0..45  -> slot 2003
           keys  46..76  -> slot 2005
           keys  77..127 -> slot 2018
```

Which also explains the factory default that made this look undecodable for so long: `INITIAL
MSMPL`'s `00 00 7f 00 80 00 00 00` is this same structure with **slot 0 — no sample** — at level 127
and centre pan. It was never an opaque blob; it was an empty one.

**`MSPa` — a multisample travels in a tone export.** The same capture exported the tone that plays
it, and the file gained an `MSPa` area: one 1040-byte record, identical in layout to `MLSa`, holding
the multisample with its slots renumbered densely (2003/2005/2018 → 3/4/5) beside a `USPa` and
`USDa` carrying those three samples. So the whole chain travels for a tone:

```
tone --(wave group 3)--> multisample --(per key)--> sample slots --> USPa/USDa audio
```

**This is a transitive dependency, and missing it is invisible.** A tone that plays a multisample
names no sample directly, so a reader that stops at the partials reports *no sample dependency at
all*. Computing the closure from `T8_MSMP_TONE.svz`'s two tones yields slots 1–5, which is exactly
the five the instrument put in the file: two named by `Beat It Gong` and three reachable only
through `T8_MSAMP`.

### A multisample installs on import, and the reference is repointed — HARDWARE-CONFIRMED

Importing that tone export on a FANTOM-6 with **[x] with sample**:

- the import list labels each tone by what it depends on — `Beat it gong (US)` for user samples,
  `T8_MSAMP (MS)` for a multisample;
- a **new multisample is created** in the first free slot. The instrument already held
  `01 T8_MSAMP`, and after import the list reads `02 T8_MSAMP`;
- the imported tone's reference is **repointed at the new slot**: tone 565 reads wave
  `02.T8_MSAMP`, while the original tone 955 still reads `01.T8_MSAMP`;
- it plays correctly, key ranges intact.

So a multisample is not merely written into an export, it is read back out of one — and its number
behaves exactly as a sample slot does, dense in the file and repointed to a panel slot on import.

> **The repackager is byte-identical to Roland's exporter for this case.** Exporting that
> re-imported tone gives `T9_BACK.svz`; extracting the same tone from `T8_MSMP_TONE.svz` with this
> tool gives **the same 5,824,680 bytes** — tone record, renumbered multisample reference, `MSPa`
> with remapped key ranges, `USPa`, all audio, every CRC-32. Pinned by
> `extracting_a_multisampled_tone_matches_the_instrument_byte_for_byte`.

### How a tone references a sample — CONFIRMED

A ZEN-Core `PATa` record holds **four partials at stride 124 (`0x7c`), the first at `0xc8`**. Its
wave selection is four consecutive fields 23 bytes in:

| Partial offset | Absolute (p=0) | Size | Field |
|----------------|----------------|------|-------|
| `+23` | `0xdf` | 1 | wave **group type** — see the table below |
| `+24` | `0xe0` | 2 | wave group id |
| `+26` | `0xe2` | 2 | wave **number L**, LE — a ROM wave index, or a **1-based `SMPa` slot** when the group is 2 |
| `+28` | `0xe4` | 2 | wave **number R**, same encoding; `0` means none |

> Earlier revisions listed only the group at `0xdf` and the number at `0xe2`. Those offsets were
> right, but stopping there hid a field: **there are two wave numbers, one per channel**, and only
> the left was being read.

### A partial's oscillator type — and when the wave fields mean nothing

A ZEN-Core partial is not always a PCM oscillator. `OSC_TYPE` sits at **`1488 + 36 × partial`**
(`PCMS_PTL.OSC_TYPE`, the last block of the 1632-byte record) and takes five values, of which three
are confirmed against the panel:

| Value | Panel | Confirmed by | Plays a wave? |
|-------|-------|--------------|---------------|
| `0` | PCM | most partials | yes, via `WAV_NUM_L` |
| `1` | VA | `Africa Brass` partial 1 | no — synthesised |
| `2` | PCM Sync | `Human Pad 1` partial 1 | yes, via **`SYNC_WAV_NUM`** |
| `3` | SuperSAW | `Break Free` zone 1; `SuperSawPluckEMA` partial 1 | no — synthesised |
| `4` | Noise | `LovesMikeJagger` zone 1 → `MAROON Whistle` partial 1 | no — synthesised |

All five read off the panel, except `3`, which is corroborated two ways: `Break Free`'s zone 1 shows
SuperSAW but plays a *factory* tone whose bytes are in ROM rather than any file, while
`SuperSawPluckEMA` is a USER tone whose `OSC_TYPE 3` partial is self-evidencing. Across one backup
the split is 7348 / 640 / 5 / 136 / 62.

`MyPolySyn1` partial 3 is SuperSAW carrying `group 2` wave numbers it cannot play — a tone left
half-edited. The instrument exported those samples anyway, so its dependency scan does not consult
`OSC_TYPE`, and neither does this tool.

**A PCM-Sync partial names its wave somewhere else.** `Human Pad 1` reads on the panel as partial 1
= PCM Sync wave **32**, partial 2 = Int bank a wave **324**. In the bytes, partial 2 is
`group 0, L=324` — an exact match — while partial 1 is `SYNC_WAV_NUM=31` with `WAV_NUM_L=1`, a
placeholder. So for `OSC_TYPE 2` the panel's wave number is `SYNC_WAV_NUM`, displayed **one higher
than stored**, and the `WMT` wave number is not the wave.

Confirmed twice, on two different instruments' worth of data: `Human Pad 1` panel 32 ↔ stored 31,
and `PCMSYNC SAMP` — built for this — panel 20 ↔ stored 19. A ROM wave selected the ordinary way
shows no such offset (`324` reads as `324`), so the shift belongs to the sync field, not to the
display.

**None of this changes what a sample dependency is, so far.** Every group-2 partial in the corpus is
`OSC_TYPE 0` — 93 of 93 in `Black NARFSOUNDS`, 68 of 68 in the FFC bank — so no VA or Noise partial
is contributing a phantom sample reference to any list this tool prints. The one exception is `MyPolySyn1`, noted above.

**A sync wave can never be a user sample — CONFIRMED at the panel.** The obvious worry was that a
PCM-Sync partial might point `SYNC_WAV_NUM` at a user sample, in which case this tool would miss the
dependency entirely, since it reads only `WAV_NUM_L` and `WAV_NUM_R`. It cannot: the editor offers
only a **fixed set of waves specific to PCM Sync** for that field — not user samples, and not even
the ordinary internal or expansion waves. The small stored values seen across the corpus (`17`,
`19`, `31`) are consistent with a short dedicated list.

So `SYNC_WAV_NUM` never carries a dependency, and reading it would only manufacture phantom ones.
That closes the last route by which a sample reference could hide from this tool: everything a tone
can depend on is a `PATa` partial's own wave numbers, the multisample those numbers can reach, an
`INSa` instrument's wave numbers, or an installed expansion — all of which are read.

**All four group types — CONFIRMED against the panel.** Tones whose bytes were known were opened in
the FANTOM-6's tone editor and the wave group field read off directly:

| Value | Panel | The numbers mean | Travels? |
|-------|-------|------------------|----------|
| `0` | — | an internal ROM wave | in the instrument |
| `1` | `EXP` | a wave in an installed expansion; the **group id** picks the bank | must be installed |
| `2` | `SAMP` | a 1-based **user sample** slot | via a companion `.svz` |
| `3` | `MSAMP` | a 1-based **user multisample** slot | **not yet — `MLSa` is undecoded** |

Evidence, all from one session: `Sledge + Hammer` partials 1 and 2 read `SAMP` waves `0030` and
`0031`, matching the bytes' slots 30 and 31 — an independent confirmation of the sample decode
against the instrument's own display. `Money Bass Stab` partial 1 reads `EXP`, bank `EXZ006`, wave
`355 MG Fat Bs`, right `0 Off` — confirming both the left number and that zero displays as "off".
`Beat It EP` partial 1 reads `EXP`, bank `EXZ005`, wave `13 Dyn EP 2`. And `Finesse Rise` partial 1
reads **`MSAMP`**, which is the finding: group 3 is a multisample.

The group id is *not* the displayed bank number — id 1005 shows as `EXZ005` but id 1008 shows as
`EXZ006`, so the mapping is something else and is reported raw rather than guessed.

> **A multisample's samples are followed; the multisample itself cannot travel in a scene bank.**
> The record is decoded (see `MLSa` above), so the samples it maps across the keyboard are found and
> carried into a companion file with everything else. What a scene export has no room for is the
> definition — there is no `MLSa` in one, and the instrument writes none — so the destination must
> already hold the multisample, or rebuild it over the slots the companion lands on. A tone export
> is different: it carries `MSPa` and the whole chain travels.
>
> One reference in the older fixtures is dangling. `Finesse Rise` partial 1 is switched on and names
> multisample 1, but every `MLSa` record in that backup is still the factory default — the
> multisample it wants does not exist, which is presumably why the zone is switched off. That is a
> property of that fixture, not of the format.

**Both numbers must be followed — though not for the reason first assumed.** 25 of
`Black NARFSOUNDS`'s 93 sampled partials name a right slot, often a *different* sample:
`Beat It Gong` holds left 1 `1 Beat It - C2` and right 22 `doh duh 2`.

> **Correction.** An earlier revision called that a stereo split and said the tone "plays slot 1 on
> the left and slot 22 on the right". The panel says otherwise: a group-2 partial offers **no
> `Wave No. R` field at all**, only `Wave No. L/Mono`. A player cannot set a right slot for a user
> sample, and would have no reason to — a sample slot already holds both channels (see the `SMPd`
> layout above, two channel blocks per section). The right number on a sampled partial is therefore
> not a second channel; it is most likely left over from when the partial selected a ROM wave, where
> `L`/`R` really are two waves and a stereo pair is consecutive (`481`/`482` in a drum kit).

It is still a **transfer dependency**, which is why it must be read and renumbered — and that is now
confirmed directly rather than inferred. A tone this tool had rebased to `L=2001, R=2002` was
imported to a FANTOM-6 and then **exported back by the instrument**: the export carries **two**
samples, `1 Beat It - C2` and `doh duh 2`, with the tone renumbered to `L=1, R=2`. Roland's own
dependency scan reads the right number even though its editor never shows it.

Following it is what keeps this tool's output matching the instrument's. Ignoring it would drop a
sample the FANTOM carries, and renumbering only the left one would leave the right pointing at
whatever takes over the old slot.

### A rebuilt scene bank survives the instrument unchanged — HARDWARE-CONFIRMED

The same round trip is the strongest statement available about the scene repackager. A bank this
tool built was imported, and the scene exported straight back out. Comparing the two files:

```
fantom diff T3_BANK/FANTOM.SVD T6_BACK/FANTOM.SVD
  SYSa[0]+0x00a6  07 -> 06
  SYSa[0]+0x00af  03 00 -> 01 01
  SYSa[0]+0x00fb  24 -> 25
1 finding, 12 changed bytes
```

**Every other byte is identical** — same file length, and `PRFa` and `PATa` come back exactly as
written, rebased sample references included. The only differences are twelve bytes of `SYSa`, where
the instrument stamps its own system settings on the way out.

> One of those bytes is worth noting: `SYSa[0]+0x00fb` goes `0x24` → `0x25`, and the SVZ preamble
> stamp this instrument writes is `KY019%` (`0x25`) where every older fixture reads `KY019$`
> (`0x24`). The last stamp byte appears to be an OS-era marker the instrument keeps in `SYSa` and
> copies into what it writes. This tool writes `$`, and files carrying it import and play correctly,
> so the difference is cosmetic — but it explains a discrepancy this document previously recorded as
> unexplained.

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

**The reference is absolute, and no import order works around it.** The slot number lives *inside*
the tone record, and a scene export bundles its own copies of those records — so importing a tone
`.svz` first cannot help: it creates different records elsewhere in USER memory and never touches the
bytes the scene bank carries. There is no indirection in the format for a tone to say "the sample
that travelled with me"; there is one field, and it names one of the panel's 8000 slots.

`FFC SAMPLES 1-50.svz` and its scene bank show a commercial author hitting exactly this. The bank's
`PATa` holds 348 tones, 29 of which carry group-2 references covering **all 50 slots 1..50 and
nothing outside**, so the pack's instructions have to say: delete whatever you have in slots 1–50,
import the pack's samples *specifically* to 0001–0050, and — for your own samples — "you will need
to re-reference your User samples to your User Tones". That last line is the author confirming from
the outside that the instrument does not fix references up on import.

Only two things can make the numbers agree: move the samples to where the tones point (what the pack
does), or rewrite the tones to point where the samples land. **The tool now does the second, and it
works on hardware.**

> **Hardware-confirmed on a FANTOM-6, end to end.** Scene 401 `Beat It` was extracted from a backup
> with `--samples-at 2001`, producing a 16 KB bank plus a 1.2 MB companion. The companion imported
> to slots 2001–2002; the bank imported as a scene; **the tone plays, and the panel shows its wave
> number as `2001`**. That last detail is the proof: the instrument stored the number this tool
> wrote, at a slot that had never held this audio on any machine. A sampled scene moved between
> sample banks without the destination having to surrender slots 1–50.
>
> Note also what the panel reading rules out — the scene did not sound because the instrument
> resolved something it already had, which is the coincidence that makes this whole area hard to
> test by ear.
`extract --samples out.svz --samples-at 101` writes the scene bank plus a sample-only `.svz` holding
exactly the samples those scenes play, and rewrites the bank's references onto the contiguous run
starting at 101 — the run the instrument produces when that file is imported there. The audio can
then land on free slots instead of overwriting whatever the destination keeps at 1–50.

Two caveats, both structural. A drum kit's sample references cannot be rebased, because they are not
decoded (see `INSa` above), so a bank bundling kits gets a warning. And the companion has to be
built from a **full backup** — a scene export has no audio to copy.

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

#### An instrument's four wave blocks — CONFIRMED shape, group field UNCONFIRMED

Each 216-byte instrument carries **four wave blocks at stride 28 (`0x1c`), starting at `+0x1c`** —
the same `WMT` structure a `PATa` partial uses:

| Block offset | Size | Field | Notes |
|--------------|------|-------|-------|
| `+0x00` | 1 | wave switch | **only ever 0 or 1** across 45056 blocks in four files |
| `+0x01` | 1 | wave **group type** | `0` in every block of every fixture — see below |
| `+0x02` | 2 | wave group id | 8 overwhelmingly; also 16, 2, 10, 11 |
| `+0x04` | 2 | wave number L | 1..963, matching the ROM wave range |
| `+0x06` | 2 | wave number R | 0..906; a stereo wave stores a consecutive pair here |

Evidence: `Starship` selects `(422, 0)`, `(481, 482)`, `(422, 0)`, `(806, 807)` — the pairs are the
stereo halves. `High Q` selects `(963, 0)` and leaves blocks 1–3 switched off, and its three off
blocks are all zero. The `+0x00` field takes no value but 0 and 1 in any fixture, which is what
fixes the block base and stride; at any other alignment it takes dozens.

**The group field is `+0x01` and the value is `2` — CONFIRMED by capture.** A FANTOM-6 drum kit was
exported, one key's instrument pointed at a user sample, and the kit exported again. The entire
difference is five bytes:

```
fantom diff DRUM_BEFORE.svz DRUM_AFTER.svz --area INSa
  INSa[0]+0x0cc5   00 08 00 4c 02 -> 02 14 27 01 00
```

That offset is instrument 15's first wave block: **group type `0` → `2`**, group id `8` → `10004`,
wave number `588` → `1`. The group value is the same `2` a tone uses. `DRUM_AFTER.svz` also gained
`USPa` and `USDa` holding `doh duh 2`, which settles a question that had been open in the other
direction too: **a drum kit can play a user sample, and the instrument carries it into a tone
export.**

So `crate::tonebank` now selects a drum bank's samples instead of carrying all of them, reading the
references out of the paired `INSa` and renumbering them there. Extracting the captured kit produces
a bank with exactly one sample, its reference rewritten to slot 1.

For the record, this is what the fixtures looked like before the capture — the reason the value had
to be guessed at rather than read:

| File | instruments | wave blocks | blocks with a user sample |
|------|-------------|-------------|---------------------------|
| `DRUM_20260623.svz` | 3344 | 13376 | none |
| `2023.4.8+topandprisma` | 11264 | 45056 | none |
| `Black NARFSOUNDS` | 11264 | 45056 | none |
| `Fantom-0 TOP80` | 11264 | 45056 | none |

Note also that the wave *group id* is not the marker, tempting as it looks: `PATa`'s 93 group-2
partials in `Black NARFSOUNDS` carry 1010, 8, 1001 and 1007 there, and plenty of group-0 partials
carry 8 too. Only the group type byte separates them.

**The capture that would settle it:** on the instrument, take one drum kit, point a single key's
instrument at a user sample, and export the kit as `.svz` twice — once before the change, once
after. `fantom diff before.svz after.svz --area INSa --context 4` then names the differing bytes
directly, exactly as the `TONEMAP*` pairs did for the opaque engines. Until then, `crate::tonebank`
treats a drum kit's samples as unselectable and carries all of them — the safe behaviour either way,
since selecting on an unobserved value would silently drop audio if the guess were wrong.

#### Where the `WMT` layout came from

The block base and stride here were found by byte-level survey (the switch field takes no value but
0 and 1 at this alignment and dozens at any other). The **field names and the `+0x01` group type**
came from two projects by the same author, which arrive at the structure from Roland's own editor
data for other ZEN-Core devices — Jupiter-X, Juno-X, MV-1 — rather than from file bytes:

- [Roland-Zen-Decode-XML](https://github.com/DrKnackeratorStrikesAgain/Roland-Zen-Decode-XML)
  (no license file) generates parameter maps from the editors' XML. Its `PCMRInst` is **216 bytes**
  and `PCMR` **3328**, matching this format's `INSa` sub-record and `RHYa` record exactly, and its
  `INST_CMN.WMT[4] @ 0x1c` lands precisely where the survey put the blocks.
- [roland-structured-storage-pattern](https://github.com/DrKnackeratorStrikesAgain/roland-structured-storage-pattern)
  (MIT) is an ImHex pattern for these containers. Its `PCMEX` is **1632** bytes with
  `PCMT_PTL[4]` at stride 124, and inside that partial `WAV_GTYPE`, `WAV_GID`, `WAV_NUM_L` and
  `WAV_NUM_R` fall at +23, +24, +26 and +28 — i.e. `0xdf`, `0xe0`, `0xe2`, `0xe4`, exactly the
  offsets confirmed here. It also models an area header as `count / elementSize / dataStartOffset`
  whose extra words are "CRC32 checksums, one per element — or offsets, if the elements are variable
  length like in an SVZ user samples chunk", which is independently the two shapes documented above.

- [Roland-Structured-Storage](https://github.com/DrKnackeratorStrikesAgain/Roland-Structured-Storage)
  (MIT) is a JS library whose parameter tables carry `byteOffset` **and** `sysexOffset` per field,
  plus the bias for signed ones. That correspondence is the SysEx mapping documented above, and
  `tools/gen_params.py` generates `params::tone` from it. It has no scene group.

The first two are used as sources of *hypotheses*, not of facts: every field was checked against the
fixtures in this repository before being written down, which is how the second wave number turned
from a name in someone else's schema into a confirmed dependency with a test. The third is checked
against the hardware itself by `validate-params`, which found two places it disagrees with a
FANTOM-6. Nothing from any of them is vendored.

Two things they do **not** answer, for the avoidance of a second look: their `DIFa` is an empty
struct, and their ZenCore sample data is an unmapped stub — `USPa` is explicitly an "unknown
chapter" there. The `USPa`/`SMPa`/`SMPd` mapping in this document has no counterpart in them.

> **The MC/MV XOR does not apply here.** That pattern documents MC/MV looper and pad sample data as
> 16-bit PCM "encoded with an XOR algorithm" against a 20-byte key. ZenCore sample data is *not*
> encoded: measured over 200 kB of a FANTOM-6 backup's `SMPd`, the raw bytes average 1897 in
> absolute delta between consecutive 16-bit samples — smooth, like real audio — and XOR-decoding
> leaves that unchanged, while a fresh export's silent lead-in is a constant `0xffff` raw and
> becomes non-constant once decoded. Copying sections verbatim is right either way, but a future
> WAV export would have needed to know.

**Validation** — "Africa Main" (scene 385) decodes to exactly the panel's 4 zones:
Z1 Brass 0–71 · Z2 Kalimba 73–127 · Z3 Kalimba 72–72 · Z4 JX-Cream 0–71 (levels 107/107/100/82,
pans L16/17R/C/25R).

> **A biased field's file bytes are two's-complement.** Pan `0xf0` is −16, and goes out as
> −16 + 64 = `0x30`. Read one unsigned and L16 shows as 240. In the generated table a non-zero
> `bias` is the flag that says to sign-extend, and it is set only where Roland's displayed range
> starts *negative* — a display that merely counts from 1, as Receive Channel's `1 - 16` does, is a
> label for the player and biasing by it would corrupt the wire value.

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

## SVZ tone export — CONFIRMED

An `.svz` is the tone-level counterpart to a scene bank: no `PRFa`, just one engine area plus its
dependencies. Verified against `EXPORT_Z-Core.svz` (10 ZEN-Core tones), `Z-Core_20260623.svz`
(274 tones, 2 samples) and `DRUM_20260623.svz` (38 kits).

### Preamble (16 bytes, then the area table at 0x10 as usual)

| Off | Size | Field | Notes |
|-----|------|-------|-------|
| 0x00 | 4 | magic | `SVZa` — no u16 length prefix, unlike SVD5 |
| 0x04 | 1 | area count | 2, 3, 4 across the three fixtures, matching their area tables |
| 0x05 | 1 | revision | 2 on the older export, 3 on both 2026 ones |
| 0x06 | 6 | stamp | `KY019$` |
| 0x0c | 4 | reserved | zeros |

The area table entries and the `format` stamp work as in SVD5, with `ZCOR` (ZEN-Core) in place of
`KY19`. Areas seen: `DIFa`, `PATa`, `RHYa`+`INSa`, `USPa`, `USDa`.

### `info_length` is the area header size, and SVZ proves it varies

Every area body starts with `count`, `record_size`, `info_length`. **Records begin at
`info_length`, not at a fixed 16.** Every area of every SVD5 file declares 16, which is why
treating it as constant worked; SVZ declares 20, 24, 56, 168, 1112 — always `16 + 4 × count`:

| Area | count | record_size | info_length | `16 + 4×count` |
|------|-------|-------------|-------------|----------------|
| `PATa` (old export) | 10 | 1632 | 56 | 56 ✓ |
| `PATa` (2026) | 274 | 1632 | 1112 | 1112 ✓ |
| `RHYa` / `INSa` | 38 | 3328 / 19008 | 168 | 168 ✓ |
| `USPa` | 2 | 64 | 24 | 24 ✓ |
| `DIFa` | 1 | 32 | 20 | 20 ✓ |

So an SVZ area carries a **four-byte word per record** between the header and the records, and
that word is a **CRC-32 of the record** — the standard reflected polynomial `0xEDB88320`, init and
final xor `0xFFFFFFFF`, over the record's bytes. Confirmed on **366 of 366 records**: every record
of every area across all three SVZ fixtures, including `DIFa`, `USPa`, `RHYa` and `INSa`.
`info_length + count × record_size` equals the area size exactly in every case.

This is a real integrity check, and the tool now uses it both ways: a record copied through keeps
its checksum, a record edited on the way gets a fresh one, and `fantom verify` recomputes every
one. The `USDa` directory's fourth word is *not* a CRC-32 of its section — still unknown, but those
sections are copied verbatim so it travels correctly.

Tone records are byte-identical in layout to an SVD's: 1632 bytes, name at `+0x00`, category at
`+0x10`, the four partials at stride 124. Everything the reader knows about tones applies unchanged.

### `USPa` and `USDa` — an SVZ carries its samples

This is the format's most useful property, and the opposite of a scene export:

- **`USPa`** — the sample slot table, 64 bytes per record: name at `+0x00`, in-use at `+0x14`,
  level `+0x15`, original key `+0x19`, end point in frames at `+0x24`. Slot 0 of
  `Z-Core_20260623.svz` is named `Sample003;F#3-F#` and stores key `0x36` = 54 = F#3.
- **`USDa`** — variable-size records (`record_size = 0`). A 16-byte header, then one 16-byte
  directory entry per section — `{u32 slot, u32 offset, u32 size, u32 word}`, offsets relative to
  the area body — then the `SMPd` sections themselves. In an SVZ's `SMPd`, the 16-bit sample count
  sits at `+0x04` and the rate at `+0x0c`, with the name still at `+0x10`. **The audio begins at
  `+0x60`.**

> **Correction — there is no 384-byte `SMPd` header.** A previous revision read `384 + frames × 4 ==
> section size` off `Z-Core_20260623.svz` and concluded the header was 384 bytes. It is 96 (`0x60`),
> and that file's two sections merely happen to hold 288 bytes past their playable end. `FFC SAMPLES
> 1-50.svz` disproves the old rule outright: its 50 sections leave 204–1080 bytes over, no two alike.
> The count at `+0x04` is the *playable* length — it equals `USPa.end × 2` exactly — and a section
> can carry audio beyond it.

A tone references a sample exactly as in an SVD — wave group 2, 1-based `USPa` slot — so the same
decode drives both. `Z-Core_20260623.svz` has one sampled tone, `MyPolySyn1`, pointing at slot 2.

### A tone exported *with* its sample — CONFIRMED

`EXPORT_Z-Core2.svz` is a FANTOM-6 export of one ZEN-Core tone plus the user sample it plays:
`DIFa` + `PATa` (1 record) + `USPa` (1 record) + `USDa` (1 section). It is the smallest complete
example of what this tool builds, and it settles two things.

**The instrument renumbers the exported reference to a dense 1-based index.** The tone's partial 0
is `group 2, wave 1` and the file carries exactly one `USPa` record — while **on the panel, that
tone's wave reads group `SAMP`, sample `0029`**. So slot 29 became slot 1 on the way out: an SVZ
addresses samples by position within its own `USPa`, not by the panel slot they came from. That is
the same renumbering `crate::tonebank` applies when it selects tones, now confirmed against the
instrument rather than merely self-consistent.

This has a consequence worth stating plainly. A reference renumbered on the way out is only
meaningful if it is renumbered again on the way in: an SVZ that says "wave 1" must be repointed at
whatever panel slot its sample lands in, or the imported tone would play whatever happens to sit in
panel slot 1. **So the two containers behave in opposite ways, and for a structural reason:**

| | sample areas | what a tone's reference means | fixed up on import |
|---|---|---|---|
| SVZ tone export | `USPa` + `USDa` | position within this file's `USPa` | must be — the slot is chosen at import |
| SVD scene export | none | an absolute panel slot | **cannot be** — nothing to renumber against |

A scene export carries no sample area, so there is no table for a dense index to point into and
nothing the import could rewrite a reference to. Its numbers are panel slots and they stay panel
slots — which is exactly what `FFC 3PCK BUNDLE` shows, its bundled tones naming slots 1..50 and its
instructions demanding the destination put the audio there.

**Rebuilding it reproduces the instrument's file byte for byte.** Selecting its only tone makes the
repackager lay the whole file out from parts, and the result `cmp`s equal to Roland's: preamble,
area order and offsets, every `info_length`, every record CRC-32, the `USPa` record, and the `USDa`
directory including its carried per-section word. Pinned by
`rebuilding_an_instrument_written_sampled_export_is_byte_identical`.

> The preamble stamp is **not** the constant this document claimed. This file reads `KY019%` where
> every other fixture reads `KY019$` (`0x25` vs `0x24`), same revision 3. Meaning unknown; the
> preamble is copied verbatim, so nothing depends on it.

> **This export also exposed a reader bug.** `PatArea` started its records at a fixed 16 bytes, so it
> misread every SVZ `PATa` — whose records begin at `16 + 4 × count`. It went unnoticed because the
> CLI's tone listing goes through `RecordTable`, which honours the declared length, and every other
> `PatArea` caller reads SVD5 areas, which all declare 16. Fixed to use the declared start with the
> same clamping rule `RecordTable` uses.

### A sample-only SVZ, and how it maps onto a backup — CONFIRMED

An `.svz` need not hold tones at all. `FFC SAMPLES 1-50.svz` (a commercial pack) is `DIFa` + `USPa` +
`USDa` and nothing else — the format Roland's **MENU → IMPORT SAMPLE** consumes, which lets the user
choose the destination slot range. It is the only container that moves user audio between
instruments without moving tones with it.

That pack is a Rosetta Stone: its 50 samples are the *same recordings* as the ones in the
`2023.4.8+topandprisma` backup, so the two container shapes can be diffed directly. **All 50 agree
on every field**:

| SVZ `SMPd` | Backup `SMPd` | Field |
|------------|---------------|-------|
| `+0x04` | `+0x0c` | playable length in 16-bit words |
| `+0x0c` | `+0x20` | sample rate |
| `+0x10` | `+0x10` | name (same offset in both) |
| `+0x60` | `+0x80` | **audio — byte-identical, all 50** |
| `USDa` directory `word` | `+0x24` | see below |
| `+0x08` (`0x1002`/`0x11002`) | `+0x04` (`0x02010020`) | flags, undecoded |

**The `USDa` directory's fourth word is not a checksum — it is carried.** Earlier revisions recorded
it as "not a CRC-32 of its section — still unknown". It is the backup `SMPd`'s own `+0x24` word,
copied across unchanged: **50 of 50**. Nothing needs computing to emit one, only preserving. (Tested
and ruled out for the record: CRC-32 over the section, the audio alone, the header alone, two offset
variants, adler32, and a byte sum.)

So converting a backup's sample into SVZ form is a **header rewrite around an unchanged audio
payload**, not a re-encode. The remaining unknown is the flags word, which takes two values across
the pack (`0x1002` and `0x11002`) and one in the backup.

### A tool-built sample file imports — HARDWARE-CONFIRMED

A sample-only SVZ built by [`crate::samplebank`] from a backup was imported on a **FANTOM-6** via
`MENU → IMPORT SAMPLE`. The dialog listed exactly the two samples the file carries, numbered 1 and
2; both were written to user-chosen slots and **play correctly there, under their own names**.

That one import validates every construction rule at once — preamble revision `3` and the `KY019$`
stamp, `USPa` records synthesized from `SMPa` by the `-0x2c` shift, the rewritten `SMPd` headers,
the per-section word carried rather than computed, every record CRC-32, and the area geometry. Until
then all of it was inference from a byte-identical reproduction of someone else's file.

**The destination is chosen by the user, not assigned by the instrument.** The import runs in two
steps: pick which samples, then pick where they go. Nothing in the file proposes a slot, and the
instrument does not fill from the first free one on its own.

> **The import dialog's preview plays nothing — for any `.svz`, including Roland's own.** Pressing
> preview on a sample in the import list produced no sound from a file this tool built, which looked
> like a defect until the same was tried on commercial and instrument-written files: none of them
> preview either. So preview does not work in this flow at all, and it says nothing about the file.
>
> Worth recording because the obvious reading was the wrong one. A single observation of "our output
> behaves oddly" is not evidence until the same is asked of a file known to be good — which is the
> whole reason a Roland-authored fixture is worth keeping around.

**Repackaging** (`crate::tonebank`) selects tones by index, carries the paired `INSa` for drum kits,
carries the `MSPa` record of any multisample a selected tone plays — renumbering both the tone'''s
reference to it and its own per-key sample slots —
and carries the `USPa` slots and `USDa` sections the selected tones play, renumbering the tone's
references to match. Samples nothing references are left behind, and the CLI says so. Area order is
preserved from the source.

That selection only works where the tone→sample link is decoded, which means `PATa` alone. For a
drum bank, "no tone references this sample" and "we cannot see the reference" are the same
observation, so **every sample travels, at its original slot number** — the numbers must not move,
because a reference that cannot be found cannot be rewritten. The CLI reports it. Merging two drum
banks that carry *different* samples is refused for the same reason: their slots collide and nothing
can be repointed. The rule lives in `AreaSpec::sample_refs_decoded`, so decoding the `INSa` group
field later is a one-line change to what the repackager is allowed to do.

## SysEx — CONFIRMED (FANTOM-6)

Model ID `00 00 00 5B`, device ID `0x10`, RQ1 `0x11`, DT1 `0x12`. Identity Reply gives family code
`5B 03`, family number `00 00` — the same ID the FANTOM EX and FANTOM-06/07/08 manuals document, so
both apply to a plain FANTOM-6.

```
F0 41 10 00 00 00 5B <cmd> <aa bb cc dd> <data…> <sum> F7
sum = (128 - (Σ address+data mod 128)) & 0x7f
```

Addresses **and sizes** carry 7 bits per byte: 144 is `00 00 01 10`, not `00 00 00 90`.

| Address | Area |
|---------|------|
| `02 00 00 00` | Temporary Scene; Common `+00 00 00`, Zone *n* `+00 1n 00`, Zone Control `+00 3n 00` |
| `02 1n 00 00` | Temporary Z-Core Tone, zone *n* |

The temporary tone address only applies while the zone holds a tone of that engine; write the
zone's own block (`02 00 1n 00`, MSB/LSB/PC) to change it, rather than sending bank-select on
channel *n*, which assumes a receive-channel mapping a scene is free to remap.

**The panel caches the tone name.** Writing temporary memory sets the edited asterisk but does not
redraw it. Writing Scene Common `+0x12` (Current Zone) away and back does — verified on a FANTOM-6.

### A record on disk is its SysEx parameter blocks, packed

The same map describes both sides. Converting file → wire:

- multi-nibble wire fields are one little-endian integer in the file (`0x2ee0` → nibbles `2 e e 0`);
- signed fields are stored zero-centred and biased on the wire (pan `0xf0` = −16 → `0x30`);
- the instrument **clamps** out-of-range values silently, so an unbiased byte lands wrong, not rejected.

`PRFa` is exactly this: Scene Common is 150 addresses and 144 bytes, the difference being the
4-nibble Scene Tempo at wire `+0x38` and two other multi-nibble fields, which is why the memo is at
file `+0x40` and wire `+0x42`. See the scene record layout above for the full block map.

`PATa` is the same parameter *sequence* with file-only padding interleaved, so its offsets must be
looked up rather than computed. `crates/fantom-core/src/params/` holds both tables; `PCMEX` totals
1632 bytes across 33 blocks and a scene 3572 across 55.

**The table carries display data too.** Roland prints beside each parameter how its value is meant
to be read — an enumeration's member names, a decimal scale, a unit — and the generator keeps it, so
a field formats correctly wherever it is read rather than at each call site. An enumeration is
emitted only when its label count matches the declared range, which rejects both a misparse and a
range that merely reserves its top value (`Zone Portamento Time`, shown `0 - 127, TONE`).

**The two tables have different sources, because the editor data has no scene.** Its groups are
`PCMEX`, `PCMR`, `MdlSynPrm0` and `INST_CMN_GROUP` — tones, drum kits, MODEL — and none of its 26
blocks is a Scene Common, Scene Zone, Zone EQ, Zone Control or Scene Controller. `params::scene` is
therefore generated from the FANTOM EX MIDI Implementation by `tools/gen_scene_params.py`, which
derives file offsets from wire addresses by the packing rule above. Both tables contain `[MFX]` and
independently agree it is 84 bytes in the file against 80 of parameters — the check that the
derivation is right, asserted in `params`.

Two corrections to the generated data, both confirmed by RQ1 and by the FANTOM MIDI Implementation:
`PCMS_PTL` (`[Tone Synth Partial]`) is **29** bytes, not 30, with `0x1b` onward Reserved. And
`PCMT_CMN.PHRASE` / `PHRASE_VEL_RATE` hold real values in a file that the instrument never reports
over SysEx, so RQ1 is not a complete tone reader.

**Validation** — `validate-params` drives the instrument's own USER bank by bank-select, reads every
block back, and compares: 40 tones, 12 blocks, 293 fields confirmed holding non-default values, zero
mismatches. `send-tone` writes a complete tone from a file into temporary memory and reads it back —
1166 fields, zero differences, for a tone the instrument's USER bank does not contain.

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
