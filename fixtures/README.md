# Fixtures

Test files come in two tiers, split by what may be published.

## `fixtures/` — public, committed

Files authored by this repository's owner on their own instrument, carrying no third-party
content. They are committed, so **the tests that use them always run** — including in CI,
which never sees the private tier.

| Path | Size | Contents |
|------|------|----------|
| `tests/TEST 1..3/FANTOM.SVD` | 4620 B each | One scene, areas `PRFa`/`SYSa`/`DIFa` |

`TEST 1..3` are a controlled series: three exports of the same scene differing by one
deliberate panel edit each — the zone switch, then the key range, then the zone level. They
differ by 2 and 4 bytes respectively, which is what makes them able to pin an offset. They
carry no engine or sample areas at all, so no bundled tone travels with them; the one tone
they name is a factory preset referenced by MIDI address, which carries no content.

### Adding to this tier

A file belongs here only if you made it yourself and it holds nothing you did not author.
In practice that means building it from INIT tones on the instrument and exporting it. Before
committing one, check what it actually carries:

```sh
cargo run -p fantom-cli -- areas <file>     # engine areas mean bundled tone records
cargo run -p fantom-cli -- tones <file>     # anything named here travels with the file
cargo run -p fantom-cli -- samples <file>   # sample audio, which is never publishable
```

An area list of `PRFa`/`SYSa`/`DIFa` alone is the safe shape. Anything else — `PATa`, `ACBa`,
`DCWa`, `MDLa`, `RHYa`/`INSa`, `SNAa`, `ZAPa`, `ZEPa`, `VTWa`, or any `SMPa`/`SMPd`/`USPa`
sample data — means the file embeds sound content, and whether that content is yours to
publish is a judgement only you can make. Keep this tier small and obviously clean; the
private tier exists for everything else.

## `fixtures-local/` — private, gitignored

Full instrument backups, Roland's MIDI Implementation and Sound List PDFs, and content from
purchased sound packs. **None of it is redistributable**, so it is never committed and never
reaches CI.

Tests needing it **skip when it is absent**, so a fresh clone runs green. That is a
convenience for contributors, not a licence to let the suite rot — on a machine that has the
corpus, run:

```sh
FANTOM_FIXTURES=require cargo test --workspace
```

which turns a missing private fixture into a failure instead of a silent skip. Use it before
pushing anything that touches the readers.

The location is overridable, so a 1.2 GB corpus can live outside the repo:

```sh
FANTOM_FIXTURES_DIR=/Volumes/Audio/fantom-fixtures cargo test --workspace
```

### What the private tier holds

Paths the suite looks for, relative to the private root:

- `backup/ROLAND/SOUND/{NARF,TOP80,PRISMA}/FANTOM.SVD` — scene-export banks
- `backup/ROLAND/FANTOM/BACKUP/*/FANTOM.SVD` — the full backups those were exported from
- `hwtest/`, `hwtest_back/` — before/after captures from hardware round trips
- `*.svz` tone and sample banks, `TONEMAP*/` controlled parameter-edit exports

These back the strongest checks in the suite — a scene resolved from an export and from the
backup it came from must name the same tones, and a bank this tool built must survive the
instrument byte for byte. They cannot be published, which is precisely why the public tier
above must never be allowed to quietly become the only thing running.
