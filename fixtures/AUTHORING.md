# Authoring a public fixture corpus

The goal: every test in the suite runs against files that may be published, so CI exercises
the readers rather than skipping them, and a contributor can run the whole suite from a clean
clone.

That means re-making the captures with content you authored. It cannot be done by generating
files in code — most of these tests are worth having precisely because **the instrument wrote
the bytes**, and a synthetic stand-in would only check this tool against its own assumptions.
So the work is a bench session, not a refactor.

## Ground rules

- **Start from INIT.** Every tone, kit and scene should be an INIT patch you then edit, never
  a factory or purchased one you renamed. Referencing a factory tone by MIDI address is fine —
  no content travels with an address.
- **Keep samples very short.** Sample audio is the only thing here that does not compress: an
  11 MB backup becomes 0.2 MB in git, but a megabyte of audio stays a megabyte. Record
  ~0.2 s — a click, a short sine — and the entire corpus lands around 1 MB.
- **Name things distinctly.** Several tests assert on names to prove the right record was
  found. `PUB Tone A`, `PUB Kit A`, `PUB Scene One` are easier to assert on, and to spot in a
  diff, than `INIT TONE`.
- After each export, `cargo run -p fantom-cli -- areas <file>` and `tones <file>` tell you
  what the file actually carries. Check before filing it away.

## Session 1 — the source backup

Build one instrument state that everything else derives from.

1. Initialise. USER banks should hold nothing you did not make.
2. Create **three ZEN-Core USER tones** from INIT: `PUB Tone A`, `PUB Tone B`, `PUB Tone C`.
   Give them different partial counts and obviously different filter/level settings.
3. Record **three user samples**, ~0.2 s each: `PUB Smp 1`, `PUB Smp 2`, `PUB Smp 3`.
   Note which panel slots they land in.
4. Make **`PUB Tone S`** — a ZEN-Core tone whose partial 1 plays `PUB Smp 1` as its left wave
   and `PUB Smp 2` as its right. The stereo split is the point: a reader that follows only the
   left number must fail.
5. Build a **user multisample** `PUB Msmp` mapping `PUB Smp 1` and `PUB Smp 3` across two key
   ranges, and a tone `PUB Tone M` that plays it.
6. Make a **drum kit** `PUB Kit A` with at least one instrument playing `PUB Smp 3`.
7. Make a **PCM-Sync tone** `PUB Tone Y` — a partial using PCM Sync, so its wave lives in
   `SYNC_WAV_NUM` rather than being a sample reference.
8. Build **four scenes**, each with several enabled zones, deliberately varied so the scene
   reader has something to bite on:
   - `PUB Scene One` — 4 zones, distinct key ranges (a split), distinct levels, and **distinct
     pans including at least one left and one right**, plus one zone transposed and one with an
     octave shift.
   - `PUB Scene Two` — zones playing `PUB Tone S` and `PUB Tone M`, so the scene depends on
     samples and a multisample.
   - `PUB Scene Kit` — a zone playing `PUB Kit A`.
   - `PUB Scene Fac` — zones referencing **factory** tones by address only, no USER tones.
9. Take a full backup.

Save as `fixtures/backup/FANTOM.SVD`.

> Write down the panel truth for `PUB Scene One` as you build it — each zone's tone, key range,
> level, pan, transpose, octave. That table is what replaces the "Africa Main" panel assertions,
> and it is much easier to record now than to reconstruct later.

**Replaces:** `a_backup_names_the_user_tones_its_scenes_reference`,
`the_sample_bank_agrees_with_its_waveform_directory`,
`a_sampled_scene_travels_as_a_bank_plus_a_companion_sample_file`,
`a_multisample_maps_key_ranges_onto_panel_sample_slots`, and every `scene_params.rs` test that
currently reads the NARF backup.

## Session 2 — scene exports from that backup

Export scenes to the USB `SOUND/` folder. Two separate exports, because the strongest oracle
in the suite compares a scene as it sits in an export against the same scene in the backup —
the two store *different addresses* for the same sound, so agreement means the indexing rule
reads both correctly.

1. Export `PUB Scene One` + `PUB Scene Two` + `PUB Scene Kit` → `fixtures/export/A/FANTOM.SVD`
2. Export `PUB Scene One` + `PUB Scene Fac` → `fixtures/export/B/FANTOM.SVD`

**Replaces:** `exports_and_backups_resolve_the_same_tone_names`,
`extracting_a_scene_from_a_backup_matches_extracting_it_from_the_export`,
`a_scene_export_cannot_source_sample_audio`.

## Session 3 — SVZ tone banks written by the instrument

Export tones, not scenes. These prove the `.svz` reader and the repackager's tone path.

1. `PUB Tone A`,`B`,`C` (no samples) → `fixtures/svz/tones.svz`
2. `PUB Tone S` + `PUB Tone M` (carries sample and multisample audio) →
   `fixtures/svz/tones-sampled.svz`
3. `PUB Kit A` (a drum-kit bank) → `fixtures/svz/drums.svz`

**Replaces:** `extracting_every_tone_of_an_svz_reproduces_it`,
`rebuilding_an_instrument_written_sampled_export_is_byte_identical`,
`an_instrument_export_numbers_its_sample_reference_from_one`,
`extracting_a_sampled_tone_carries_its_waveform`,
`extracting_drum_kits_keeps_their_instrument_sets_paired`,
`files_written_by_the_instrument_verify_clean`, `repackaged_tone_banks_verify_clean`.

## Session 4 — round trips

This is the part no amount of code can substitute: something this tool built, taken through
the instrument and back out. Each step is *build here, import there, export back*.

1. **Scene bank.** `fantom extract fixtures/backup/FANTOM.SVD <n> -o built.svd` for
   `PUB Scene One`. Save the input as `fixtures/roundtrip/built/FANTOM.SVD`. Import it, then
   export that scene straight back out → `fixtures/roundtrip/built-back/FANTOM.SVD`.
   *Replaces `a_bank_round_tripped_through_the_instrument_comes_back_unchanged`.*

2. **Sampled tone.** Export `PUB Tone S` from the instrument as
   `fixtures/roundtrip/tone-sampled.svz`.
   *Replaces `an_instrument_export_carries_the_right_wave_numbers_sample_too`.*

3. **Drum kit before/after.** Export `PUB Kit A` → `fixtures/roundtrip/drum-before.svz`.
   Re-import it to a fresh slot, then export again → `drum-after.svz`.
   *Replaces `a_drum_kit_that_plays_a_user_sample_is_read_from_the_capture`.*

4. **Multisample tone.** Export `PUB Tone M` → `fixtures/roundtrip/msmp-tone.svz`. Import it
   into a clean instrument, back that up → `fixtures/roundtrip/msmp-backup/FANTOM.SVD`, and
   export the tone again → `fixtures/roundtrip/msmp-back.svz`.
   *Replaces the three multisample tests, including the byte-identical one.*

5. **PCM-Sync.** Export `PUB Tone Y` → `fixtures/roundtrip/pcm-sync.svz`.
   *Replaces `a_pcm_sync_partials_internal_wave_is_not_a_sample`.*

## Session 5 — the opaque engines

Six single-scene exports, each holding USER patches on an engine whose records this tool
carries but does not decode. Build each patch from INIT.

| Export | Contents |
|---|---|
| `fixtures/engines/acb-1/FANTOM.SVD` | one scene, one ACB USER tone |
| `fixtures/engines/acb-2/FANTOM.SVD` | one scene, **two** ACB USER tones (proves PC indexing) |
| `fixtures/engines/vpiano-1/FANTOM.SVD` | one V-Piano USER tone |
| `fixtures/engines/vpiano-2/FANTOM.SVD` | two V-Piano USER tones |
| `fixtures/engines/model-1/FANTOM.SVD` | one MODEL USER tone |
| `fixtures/engines/model-2/FANTOM.SVD` | two MODEL USER tones |

**Replaces:** `extracting_every_scene_of_an_export_reproduces_it`,
`hardware_validated_banks_still_decode`.

## The one that does not survive

`building_a_sample_svz_from_a_backup_reproduces_a_shipped_one` byte-matches a **commercially
shipped** pack, so it has no self-authored equivalent. Its distinct value over the rest is
scale (50 samples) and an older OS revision; the sample-export path itself is already checked
against an instrument-written file by `rebuilding_an_instrument_written_sampled_export_is_byte_identical`.

Either drop it, or — if the instrument can export a **sample-only** `.svz` — record enough of
your own short samples to recreate the same oracle and keep it.

## After the captures

Hand the files over and the remaining work is on this side:

- Repoint every `private(...)` call at its public path.
- Rewrite the assertions that name specific content — `Africa Main`, `Beat It Gong`, `Sledgehammer`,
  particular slot numbers — against the panel truth recorded in Session 1.
- Delete `fixtures-local/` from the test path entirely, leaving it for reverse-engineering
  scratch work, and drop `FANTOM_FIXTURES=require` since there would be nothing left to require.

Estimated committed size: roughly **1 MB**, dominated by however much sample audio you record.
The 11 MB backup compresses to about 0.2 MB because its record tables are mostly INIT.
