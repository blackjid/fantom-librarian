# FANTOM Librarian product plan

## Purpose

Build a local-first librarian for the flagship Roland FANTOM that consolidates
user-created and downloaded sound material into a searchable, safe library.
The first product goal is not synthesis editing or live control. It is to make
it easy to assemble a FANTOM-loadable package without slot collisions,
especially for packs which assume fixed user-sample locations (for example,
NARF-style packs that require samples in slots 1–50).

The product starts as a personal consolidation tool. Reliable, portable package
generation is the foundation for later sharing.

## Product principles

- **Local first.** No account, cloud service, upload, or marketplace is needed.
- **Original imports are immutable.** The app never edits an imported pack in
  place. It produces new FANTOM-loadable files only when exporting.
- **Safety beats cleverness.** Never silently overwrite, substitute, discard,
  or claim a package is complete when its dependencies are missing.
- **The library is not the synth.** The library is unlimited, searchable, and
  historical. Hardware slots are a limited deployment target.
- **FANTOM compatibility is explicit.** All device-facing names obey FANTOM
  constraints. Model compatibility is labelled by confidence rather than
  guessed or rejected.

## Users and core job

The first user is a FANTOM owner with a mixture of personal work, full backups,
free downloads, and purchased sound packs. They need to:

1. Preserve what they own and where it came from.
2. Find a useful Scene or Tone without repeatedly loading packs on the device.
3. Relate sounds to cover songs without turning the app into a setlist manager.
4. Combine material safely.
5. Put required samples in safe locations rather than the source pack's fixed
   locations.
6. Export a folder that can be loaded from USB with clear instructions.

## Canonical library and workspace

The app manages one chosen, portable workspace folder. It is the canonical
local library and can be copied or backed up as normal user data.

```text
My FANTOM Library/
  library database
  managed original imports/
  generated exports/
  slot-allocation ledger
```

- Imports are copied into managed storage by default; their original files are
  not changed.
- The original path is retained as optional provenance.
- Managed data may be content-deduplicated internally so repeat imports do not
  double storage unnecessarily.
- The workspace has one default sample-slot allocation map in v1.
- There is no device backup mirror and no expectation that the user refresh a
  backup after every hardware change. The allocation map is an app-managed
  plan/history, not a real-time statement of device state.

## Source imports and provenance

Every import creates a browsable **source group**, such as `NARF Sounds`,
`My 2025 backup`, or a downloaded pack. A unified library view also shows the
contents across all sources.

- One import may contain a folder or several selected files, treating them as
  one source pack. This lets a Scene/Tone bank and its supplied sample material
  retain their relationship.
- A single file is simply a one-file source group.
- Import fields are all optional: source/pack name, author or vendor, URL,
  licence/ownership note, and import date. The app should retain any information
  that is supplied but never block an import for incomplete provenance.
- Sources and assets are archived/hidden rather than destructively deleted in
  v1. Their files, provenance, duplicate links, and allocation history remain
  recoverable.
- Direct archive (`.zip`) import is deferred. Users unzip a pack and import its
  files/folder.

### Input scope

v1 accepts FANTOM `.svd` and `.svz` material, including Scene exports, tone
banks, sample material, and full backups. Roland Cloud `.sdz` content is
explicitly out of scope.

Every source is structurally/checksum validated before it contributes usable
assets. An invalid or unsupported source can remain visible with an error
report, but cannot be exported.

When importing a full backup, catalogue all non-empty user Scenes and user
Tones, including unreferenced standalone Tones. Skip known blank/INIT records
by default.

## Library model

### Main Library

The main Library is one unified view for **Scenes** and **Tones**, with a type
filter. Searching `Rhodes` should surface both a standalone Rhodes Tone and
Scenes which use it.

- Exact/normalised-identical assets consolidate into one canonical library item
  while retaining every source association.
- Source groups continue to display their own copy/occurrence.
- Similar items are not merged automatically: renamed sounds, small parameter
  changes, or different dependency layouts remain distinct until comparison
  tooling exists.
- Different assets with the same FANTOM name remain as imported. The app
  disambiguates them using source, type, tags, and metadata, and warns when an
  export contains duplicate names. It never auto-renames them.

### Names and notes

Each asset has one editable **FANTOM-compatible name**. It is validated for
the device's constraints and is the name written to a generated export. The
imported name remains part of the import history.

- Scene name editing is in scope: it is already hardware-verified by the core.
- Tone rename is deferred until a safe, independently verified write path
  exists.
- Preserve the imported FANTOM Scene memo/comment in v1; do not edit it.
- Use a separate, longer library note for app metadata.

### Tags

v1 includes simple flat, user-defined tags for Scenes and Tones. They can be
added, removed, searched, and filtered. No hierarchy, smart-folder rules, or
automatic tag generation is needed initially.

## Songs

Song metadata is for cover-band context, not for setlist management.

A Song is a first-class record with:

- title;
- original artist;
- performance key;
- notes;
- optional tags.

A Song can be associated with a Scene or a Tone. This supports, for example,
a Rhodes Tone that suits multiple songs even before a dedicated Scene exists.

- v1 links are simple associations with an optional note. Do not distinguish
  candidate/in-use status, sections, arrangements, or arbitrary relationship
  tags yet; leave the data model extensible for them.
- Provide a lightweight Songs view where users can create/search Songs and see
  linked Scenes/Tones.
- Setlists, song ordering, live recall, and export-by-Song are out of scope.
- Song metadata is not written into FANTOM files or package manifests in v1.

## Samples and slots

Samples are first-class assets internally, but are not shown in the main
Library. A separate **Samples & Slots** view shows:

- sample assets and their dependents;
- sample requirements which are present or missing;
- the default allocation map;
- reserved ranges;
- active/retired allocations;
- generated-export usage.

### Missing sample requirements

If a Scene/Tone references samples but the audio was not imported with its
source group, show it as a **missing sample requirement**, for example:

> Requires 4 samples: 1 available, 3 missing.

Cross-source sample matching and manual linking of possible audio are deferred.
In v1, sample audio satisfies a requirement only when it arrived in the same
source import on strong/unambiguous evidence.

### Default slot allocation map

The user manually reserves slots or ranges which must never be used by exports.
For example, slots `1–500` may remain available for samples made on the device
or loaded outside the app.

The app records export allocations immediately. There is no install-status
feedback loop: an allocation is a single recorded state, not `planned` versus
`installed`.

- An allocation is not silently freed when an asset/source is archived.
- It can be marked **retired**, which preserves history and keeps the slots
  unavailable.
- A separate explicit **reclaim** action makes a retired allocation available
  again.
- If an asset has already been assigned slots, later exports reuse those slots
  by default, with an explicit override.

### Placement policy

At export time, choose a starting slot. Required samples are assigned as a
contiguous range from that point, after checking reservations and existing
allocations. If the requested location is unavailable, the app suggests a
safe next free block.

This mirrors FANTOM's familiar sample-placement workflow. For example, a
50-sample pack can be rebased from slots `1–50` to `501–550`, avoiding a
reserved `1–500` range.

## Package definitions and exports

An export starts from a saved **package definition**. It contains the selected
Scenes/Tones, package name, sample mode, and relevant allocation decisions.

- A definition is a living recipe: rebuilding uses the current library state
  (for example, current device-compatible Scene names).
- Each generated deployment folder is immutable and versioned/timestamped;
  rebuilding never overwrites an earlier export.
- Generated folders are always stored in the workspace's `exports/` directory
  first. Users can then reveal/copy them to USB.
- A previous export remains an auditable record of the exact content generated
  at that time through its manifest.

### Selection and dependencies

Users explicitly add any Scenes and/or standalone Tones to a package.

- Required user Tones of a selected Scene are included by default and shown as
  dependency-derived inclusions.
- Users may deliberately add extra Tones.
- A missing required user Tone is a hard export error in v1.
- Factory/model/expansion dependencies are not substituted. The export is
  allowed only after explicit acknowledgement, and the requirement appears in
  its install guide.

### Sample modes

The user chooses the mode at export time, for that specific export:

1. **Include samples** — create a sample companion, rebase references to the
   chosen slots, and include the audio. This is the self-contained option.
2. **References only** — rebase Scene/Tone sample references to chosen slots,
   but do not include the audio. The user can load the original sample material
   manually at those slots on the FANTOM.

References-only is deliberately not limited to the source's original slot
numbers. For example, an imported NARF Scene can point to `101–150` while the
user manually imports the original audio into `101–150`.

Every references-only export includes a placement map showing sample
name/original slot to destination slot.

If **Include samples** is selected but any required audio is missing, block the
export and identify the missing requirements. Offer references-only as the
safe alternative; never label an incomplete package self-contained.

### Deployment folder

Every export produces a deployment folder containing:

- FANTOM-readable Scene/Tone bank files;
- the sample companion file when samples are included;
- a human-readable install guide describing order, locations, required
  expansions, and unresolved external requirements;
- a machine-readable manifest for validation, history, and future rebuilds;
- a placement map when samples are references-only.

The generated package is validated structurally and by checksum before it is
presented as ready.

## Hardware interaction and platform scope

- Deployment is USB-file based in v1. Direct USB-MIDI/SysEx transfer is
  deferred.
- Hardware auditioning is deferred.
- The product remains a librarian/package manager. It does not edit synthesis
  parameters, Zones, effects, or sequencer data.
- Build with cross-platform architecture, but validate and release macOS first.
- Target the flagship FANTOM-6/7/8 format, with FANTOM-6 hardware-confirmed.
  FANTOM-7/8 and EX revisions should be labelled as likely/unverified where
  applicable. FANTOM-06/07/08 is a separate family and must be clearly labelled
  rather than assumed compatible.

## Sharing boundary

Sharing is a later layer over the same deployment-folder format.

- The app remains file-based and does not host or upload content.
- Provenance/licence notes can warn when material is marked purchased or
  personal-use-only.
- The user remains responsible for rights; the app makes risk visible rather
  than attempting legal enforcement.

## Explicit v1 exclusions

- `.sdz` / Roland Cloud pack support;
- archive (`.zip`) import;
- setlists and song ordering;
- export selection by Song;
- direct hardware transfer and audition;
- sound-design/synthesis parameter editing;
- Tone renaming until write support is verified;
- editing FANTOM Scene memos/comments;
- cross-source/manual sample matching;
- tag hierarchies, smart folders, relationship state/role systems;
- physical-device backup synchronisation or allocation install status;
- multiple named FANTOM installation maps;
- automatic content substitution or automatic renaming.

## v1 acceptance scenario

v1 is useful when this complete workflow succeeds:

1. Create or open a portable workspace.
2. Reserve slots `1–500` in the default allocation map.
3. Import a NARF pack and its supplied sample material together as one source.
4. Browse its Scenes/Tones with existing imports; add tags, an optional Song
   association, and provenance where useful.
5. Select a Scene and create a saved package definition.
6. Export with samples starting at a safe location, such as `501`.
7. Receive a validated, versioned USB deployment folder with rewritten
   FANTOM files, sample companion, placement map, and install guide.
8. Reopen the package definition/export record and inspect its allocations and
   dependencies.
