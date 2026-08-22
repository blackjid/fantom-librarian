//! `fantom` — command-line librarian for Roland Fantom data files.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use fantom_core::container::Raw;
use fantom_core::params;

mod cli;
mod render;
use cli::{AreasCommand, Cli, Command, SamplesCommand, ScenesCommand, TonesCommand, WriteOptions};
use render::{Align, Table};

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Inspect {
            file,
            offset,
            length,
        } => run_inspect(&file, offset, length),
        Command::Diff {
            left,
            right,
            area,
            context,
        } => run_diff(&left, &right, &area, context),
        Command::Verify { file } => run_verify(&file),
        Command::Areas { command } => match command {
            AreasCommand::List { file } => run_areas(&file),
        },
        Command::Check { file, against } => run_check(&file, against.as_ref()),
        Command::Tones { command } => match command {
            TonesCommand::List { file } => run_tones(&file),
            TonesCommand::Extract {
                file,
                tones,
                area,
                write,
            } => run_tone_extract(&file, &tones, &area, &write),
        },
        Command::Samples { command } => match command {
            SamplesCommand::List { file } => run_samples(&file),
        },
        Command::Scenes { command } => match command {
            ScenesCommand::List { file } => run_scenes(&file),
            ScenesCommand::Show {
                file,
                scene,
                include_disabled,
            } => run_show(&file, scene, include_disabled),
            ScenesCommand::Rename {
                file,
                scene,
                name,
                write,
            } => run_edit(
                &file,
                &write,
                &format!("renamed scene {scene} to {name:?}"),
                |raw| fantom_core::codec::set_scene_name(raw, scene, &name),
            ),
            ScenesCommand::Comment {
                file,
                scene,
                text,
                write,
            } => run_edit(
                &file,
                &write,
                &format!("set comment on scene {scene}"),
                |raw| fantom_core::codec::set_scene_comment(raw, scene, &text),
            ),
            ScenesCommand::Extract {
                file,
                scenes,
                write,
                samples,
                samples_at,
            } => run_extract(&file, &scenes, &write, samples.as_ref(), samples_at),
            ScenesCommand::Canary { file, scene, write } => run_canary(&file, scene, &write),
            ScenesCommand::Merge {
                target,
                source,
                write,
            } => run_merge(&target, &source, &write),
        },
    };
    match result {
        Ok(text) => print_output(&text),
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Write built output to stdout, treating a closed pipe (e.g. `| head`) as a clean exit.
fn print_output(text: &str) -> ExitCode {
    use std::io::Write;
    match std::io::stdout().write_all(text.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_inspect(file: &PathBuf, offset: usize, len: usize) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let mut out = String::new();
    let _ = writeln!(out, "file:  {}", file.display());
    let _ = writeln!(out, "size:  {} bytes", raw.len());
    match raw.ascii_magic(4) {
        Some(magic) => writeln!(out, "magic: {magic:?} (printable)"),
        None => writeln!(out, "magic: <non-printable>"),
    }
    .ok();
    let _ = writeln!(out);
    out.push_str(&raw.hexdump(offset, len));
    Ok(out)
}

fn run_areas(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let svd = fantom_core::container::Svd::parse(&raw)?;
    let mut table = Table::new(vec![
        ("TAG", Align::Left),
        ("FORMAT", Align::Left),
        ("OFFSET", Align::Right),
        ("SIZE", Align::Right),
    ]);
    for area in &svd.areas {
        table.row(vec![
            area.tag_str(),
            area.format_str(),
            area.offset.to_string(),
            area.size.to_string(),
        ]);
    }
    Ok(table.render())
}

fn run_diff(
    left: &PathBuf,
    right: &PathBuf,
    areas: &[String],
    context: usize,
) -> fantom_core::Result<String> {
    use fantom_core::diff::Finding;

    let left_raw = Raw::open(left)?;
    let right_raw = Raw::open(right)?;
    let findings = fantom_core::diff::compare(&left_raw, &right_raw)?;

    let mut out = String::new();
    let _ = writeln!(out, "left:  {} ({} bytes)", left.display(), left_raw.len());
    let _ = writeln!(
        out,
        "right: {} ({} bytes)",
        right.display(),
        right_raw.len()
    );
    let _ = writeln!(out);

    let selected: Vec<&Finding> = findings
        .iter()
        .filter(|f| areas.is_empty() || areas.iter().any(|a| a == f.tag()))
        .collect();

    if selected.is_empty() {
        let _ = writeln!(out, "no differences");
        return Ok(out);
    }

    let mut changed = 0;
    for finding in &selected {
        changed += finding.changed_bytes();
        match finding {
            Finding::AreaOnlyIn {
                tag,
                side,
                size,
                records,
            } => {
                let _ = writeln!(
                    out,
                    "{tag}  present in {} only ({size} bytes, {records} records)",
                    side.label()
                );
            }
            Finding::RecordSizeDiffers { tag, left, right } => {
                let _ = writeln!(
                    out,
                    "{tag}  record size differs: {left} vs {right} — records not comparable"
                );
            }
            Finding::RecordCountDiffers { tag, left, right } => {
                let _ = writeln!(out, "{tag}  record count differs: {left} vs {right}");
            }
            Finding::RecordOnlyIn {
                tag,
                side,
                record,
                name,
            } => {
                let _ = writeln!(
                    out,
                    "{tag}[{record}]  present in {} only  {name:?}",
                    side.label()
                );
            }
            Finding::AreaHeader { tag, runs } => {
                for run in runs {
                    let _ = writeln!(
                        out,
                        "{}",
                        render_run(
                            &format!("{tag}.header"),
                            run,
                            &left_raw,
                            &right_raw,
                            context
                        )
                    );
                }
            }
            Finding::AreaBytes { tag, runs } => {
                if runs.is_empty() {
                    let _ = writeln!(out, "{tag}  sizes differ (not a record table)");
                }
                for run in runs {
                    let _ = writeln!(
                        out,
                        "{}",
                        render_run(&format!("{tag}.bytes"), run, &left_raw, &right_raw, context)
                    );
                }
            }
            Finding::Record { tag, record, runs } => {
                for run in runs {
                    let _ = writeln!(
                        out,
                        "{}",
                        render_run(
                            &format!("{tag}[{record}]"),
                            run,
                            &left_raw,
                            &right_raw,
                            context
                        )
                    );
                }
            }
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{} finding{}, {changed} changed byte{}",
        selected.len(),
        if selected.len() == 1 { "" } else { "s" },
        if changed == 1 { "" } else { "s" }
    );
    Ok(out)
}

/// Render one differing run as `TAG[record]+0xoffset  @file-offset  old -> new`.
fn render_run(
    where_: &str,
    run: &fantom_core::diff::ByteRun,
    left: &Raw,
    right: &Raw,
    context: usize,
) -> String {
    let (left_bytes, right_bytes, offset) = if context == 0 {
        (run.left.clone(), run.right.clone(), run.offset)
    } else {
        // Clamp against the record-relative offset as well as the two file offsets: a run at the
        // start of a record (a scene rename lands at offset 0) has less context available before
        // it than the caller asked for.
        let before = context.min(run.offset).min(run.left_at).min(run.right_at);
        let after = context;
        (
            window(left, run.left_at - before, before + run.left.len() + after),
            window(
                right,
                run.right_at - before,
                before + run.right.len() + after,
            ),
            run.offset - before,
        )
    };
    format!(
        "{where_}+0x{offset:04x}  @0x{:06x}  {} -> {}  |{}| -> |{}|",
        run.left_at,
        render::hex(&left_bytes),
        render::hex(&right_bytes),
        render::ascii(&left_bytes),
        render::ascii(&right_bytes),
    )
}

fn window(raw: &Raw, at: usize, len: usize) -> Vec<u8> {
    raw.bytes()
        .get(at..(at + len).min(raw.len()))
        .unwrap_or_default()
        .to_vec()
}

/// Report what a file needs, and — given a destination — how much of it that destination has.
///
/// The dependency closure is the single most important fact about a file somebody hands you: a
/// bank referencing EXZ007 or sample slot 7 loads on any instrument and quietly plays something
/// else. Without `--against` this names the requirements; with it, each one is weighed against
/// what the other file shows it holds, and anything unmet exits non-zero so it works as a gate.
fn run_check(file: &PathBuf, against: Option<&PathBuf>) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let needs = fantom_core::requirements::requirements(&raw)?;

    let mut out = String::new();
    let _ = writeln!(
        out,
        "file:  {} ({})",
        file.display(),
        fantom_core::role::of(&raw).as_str()
    );
    if !needs.engines.is_empty() {
        let _ = writeln!(
            out,
            "plays: {}",
            needs
                .engines
                .iter()
                .map(|engine| engine.label())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let _ = writeln!(out);

    let Some(destination) = against else {
        match render::requirements(&needs) {
            report if report.is_empty() => {
                let _ = writeln!(out, "needs nothing from its destination");
            }
            report => out.push_str(&report),
        }
        return Ok(out);
    };

    let held = fantom_core::requirements::inventory(&Raw::open(destination)?)?;
    let _ = writeln!(
        out,
        "against: {} ({})\n",
        destination.display(),
        held.role.as_str()
    );

    let findings = fantom_core::requirements::compare(&needs, &held);
    if findings.is_empty() {
        let _ = writeln!(
            out,
            "everything this file needs, that one has — as far as a file can say"
        );
        return Ok(out);
    }

    let mut table = Table::new(vec![
        ("VERDICT", Align::Left),
        ("REQUIREMENT", Align::Left),
        ("THE DESTINATION", Align::Left),
    ]);
    for finding in &findings {
        table.row(vec![
            finding.verdict.as_str().to_string(),
            finding.requirement.clone(),
            finding.detail.clone(),
        ]);
    }
    out.push_str(&table.render());

    let problems = findings.iter().filter(|f| f.verdict.is_problem()).count();
    let unknown = findings
        .iter()
        .filter(|f| f.verdict == fantom_core::requirements::Verdict::Unknown)
        .count();
    let _ = writeln!(out);
    if unknown > 0 {
        let _ = writeln!(
            out,
            "{unknown} requirement{} no file can answer: nothing in one lists the expansions an\n\
             instrument has installed, and a bank naming no samples of its own cannot be matched\n\
             by name against the slots it points at. Check those on the panel.",
            render::plural(unknown)
        );
    }
    if problems > 0 {
        // Exit non-zero so this is usable as a preflight gate in a script.
        return Err(fantom_core::Error::Unrecognized(format!(
            "{out}{problems} requirement{} unmet",
            render::plural(problems)
        )));
    }
    Ok(out)
}

fn run_scenes(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let scenes = fantom_core::codec::read_scenes(&raw)?;
    let mut out = String::new();
    let _ = writeln!(out, "{} scenes:", scenes.len());
    let mut table = Table::new(vec![
        ("NO.", Align::Right),
        ("NAME", Align::Left),
        ("REFERENCES", Align::Left),
    ]);
    for (i, scene) in scenes.iter().enumerate() {
        let mut references = Vec::new();
        for zone in scene.zones.iter().filter(|zone| zone.enabled) {
            let tone = &zone.tone;
            let bank = tone.bank().unwrap_or("raw");
            let reference = format!(
                "{} {} PC {:03}{}",
                tone.tone_type().label(),
                bank,
                tone.address.pc,
                tone.name()
                    .map(|name| format!(" \"{name}\""))
                    .unwrap_or_default()
            );
            if !references.contains(&reference) {
                references.push(reference);
            }
        }
        let summary = if references.is_empty() {
            "(no enabled zones)".to_string()
        } else {
            references.join(", ")
        };
        table.row(vec![(i + 1).to_string(), scene.name.clone(), summary]);
    }
    out.push_str(&table.render());
    Ok(out)
}

fn run_show(file: &PathBuf, scene: usize, all: bool) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let scenes = fantom_core::codec::read_scenes(&raw)?;
    let s = scenes.get(scene.wrapping_sub(1)).ok_or_else(|| {
        fantom_core::Error::Unrecognized(format!(
            "scene {scene} out of range (file has {})",
            scenes.len()
        ))
    })?;

    let mut out = String::new();
    let _ = writeln!(
        out,
        "Scene {scene}: {}   {:.2} BPM   level {}",
        s.name,
        s.bpm(),
        s.level
    );
    if !s.comment.is_empty() {
        let _ = writeln!(out, "note: {}", s.comment);
    }
    let shown = |z: &&fantom_core::model::Zone| z.enabled || all;
    let mut table = Table::new(vec![
        ("zone", Align::Right),
        ("on", Align::Left),
        ("type", Align::Left),
        ("bank", Align::Left),
        ("tone", Align::Left),
        ("range", Align::Right),
        ("vel", Align::Right),
        ("level", Align::Right),
        ("pan", Align::Right),
        ("trans", Align::Right),
        ("oct", Align::Right),
        ("ch", Align::Right),
    ]);
    for z in s.zones.iter().filter(shown) {
        table.row(vec![
            (z.number + 1).to_string(),
            render::zone_state(z).to_string(),
            render::tone_type(&z.tone),
            render::bank(&z.tone),
            render::tone(&z.tone),
            render::range(render::note(z.key_low), render::note(z.key_high)),
            render::range(z.velocity_low, z.velocity_high),
            z.level.to_string(),
            pan(z.pan),
            render::signed(z.transpose),
            render::signed(z.octave),
            (z.midi_channel + 1).to_string(),
        ]);
    }
    out.push_str(&table.render());

    let arps: Vec<String> = s
        .zones
        .iter()
        .filter(shown)
        .filter(|z| z.arpeggio)
        .map(|z| (z.number + 1).to_string())
        .collect();
    if !arps.is_empty() {
        let _ = writeln!(out, "arpeggio: zone {}", arps.join(", "));
    }

    // What this one scene needs, rather than what its whole file does: the zones above are only
    // half the story if the sounds they play are not where this scene is going.
    let needs = fantom_core::requirements::scene_requirements(&raw, scene)?;
    let report =
        render::requirements(&needs.named_from(&fantom_core::requirements::inventory(&raw)?));
    if !report.is_empty() {
        let _ = writeln!(out);
        out.push_str(&report);
    }
    Ok(out)
}

/// Pan, formatted by the parameter table itself rather than by a rule restated here.
///
/// `L64`/`C`/`63R` is Roland's own notation for the field, and the table carries it — so this is
/// the one zone column the CLI does not get to have an opinion about.
fn pan(value: i8) -> String {
    let p = params::scene::SCENE_ZONE
        .param("Zone_Pan")
        .expect("the scene table has Zone Pan");
    params::render(p, value as i32)
}

fn run_tones(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let tones = fantom_core::codec::read_bundled_tones(&raw)?;
    let mut out = String::new();
    let _ = writeln!(out, "{} bundled tones:", tones.len());
    let mut table = Table::new(vec![
        ("AREA", Align::Left),
        ("TYPE", Align::Left),
        ("INDEX", Align::Right),
        ("NAME", Align::Left),
    ]);
    for tone in tones {
        table.row(vec![
            render::area_tag(&tone.area),
            tone.tone_type.label().to_string(),
            tone.index.to_string(),
            tone.name,
        ]);
    }
    out.push_str(&table.render());
    Ok(out)
}

/// Lift tones out of a file into an `.svz`, the one envelope that carries user audio.
///
/// An `.svz` source is repackaged in place by [`fantom_core::tonebank`]; an SVD is converted, which
/// is the only way a sampled user tone can leave a backup at all.
fn run_tone_extract(
    file: &PathBuf,
    tones: &[usize],
    area: &str,
    write: &WriteOptions,
) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let exported = if is_tone_bank(&raw) {
        fantom_core::tonebank::extract_tones(&raw, tones)?
    } else {
        fantom_core::convert::export_tones(&raw, &area_tag(area)?, tones)?
    };
    if write.should_write() {
        exported.save(write.output())?;
    }

    let needs = fantom_core::requirements::requirements(&exported)?;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "extracted {} tone{}{}{}",
        tones.len(),
        render::plural(tones.len()),
        match needs.samples.len() {
            0 => String::new(),
            n => format!(
                " with {n} sample{}{}",
                render::plural(n),
                match needs.multisamples.len() {
                    0 => String::new(),
                    m => format!(" and {m} multisample{}", render::plural(m)),
                }
            ),
        },
        write_destination(write),
    );
    // Whatever the audio could not cover: an expansion the destination has to have installed.
    out.push_str(&render::requirements(&needs));
    Ok(out)
}

/// A four-byte area tag from what the user typed.
fn area_tag(area: &str) -> fantom_core::Result<[u8; 4]> {
    area.as_bytes().try_into().map_err(|_| {
        fantom_core::Error::Unrecognized(format!(
            "{area:?} is not a four-character area tag (try PATa or RHYa)"
        ))
    })
}

fn run_samples(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let svd = fantom_core::container::Svd::parse(&raw)?;
    let bank = fantom_core::container::read_samples(&raw, &svd)?;

    let mut out = String::new();
    if bank.is_empty() {
        let _ = writeln!(out, "no user samples in this file");
        return Ok(out);
    }

    let _ = writeln!(out, "{} user samples:", bank.slots.len());
    let mut table = Table::new(vec![
        ("SLOT", Align::Right),
        ("NAME", Align::Left),
        ("FRAMES", Align::Right),
        ("SECONDS", Align::Right),
        ("KEY", Align::Right),
        ("WAVEFORM", Align::Left),
    ]);
    for slot in &bank.slots {
        let data = bank.data.iter().find(|d| d.slot as usize == slot.index);
        table.row(vec![
            // Panel numbering: the instrument's first user sample is 1, and every other number
            // this tool prints or takes — a tone's reference, `--samples-at` — counts the same way.
            (slot.index + 1).to_string(),
            slot.name.clone(),
            slot.end.to_string(),
            format!("{:.2}", data.map(|d| d.seconds()).unwrap_or_default()),
            render::note(slot.original_key),
            data.map(|d| d.name.clone())
                .unwrap_or_else(|| "<no waveform>".to_string()),
        ]);
    }
    out.push_str(&table.render());

    if !bank.multisamples.is_empty() {
        let _ = writeln!(out, "\n{} multisamples:", bank.multisamples.len());
        let mut table = Table::new(vec![("SLOT", Align::Right), ("NAME", Align::Left)]);
        for ms in &bank.multisamples {
            table.row(vec![(ms.index + 1).to_string(), ms.name.clone()]);
        }
        out.push_str(&table.render());
    }

    for orphan in bank.orphans() {
        let _ = writeln!(out, "warning: {orphan}");
    }
    Ok(out)
}

/// Apply an in-place edit, then either write it or report its explicit dry run.
fn run_edit(
    file: &PathBuf,
    write: &WriteOptions,
    what: &str,
    edit: impl FnOnce(&mut Raw) -> fantom_core::Result<()>,
) -> fantom_core::Result<String> {
    let mut raw = Raw::open(file)?;
    edit(&mut raw)?;
    let mut out = String::new();
    let _ = writeln!(out, "{what}");
    if write.should_write() {
        raw.save(write.output())?;
        let _ = writeln!(out, "wrote {}", write.output().display());
    } else {
        let _ = writeln!(out, "{}", write.dry_run_notice());
    }
    Ok(out)
}

/// What a rebuilt bank still needs from wherever it is loaded.
///
/// The output is the file the user will carry across, so it is the one whose requirements matter —
/// but a scene bank has no slot table, so the source it was built from is where the sample names
/// still live. A failure here is deliberately silent: a warning that cannot be computed must not
/// stop a rebuild that succeeded.
fn requirements_note(output: &Raw, source: &Raw) -> String {
    let Ok(needs) = fantom_core::requirements::requirements(output) else {
        return String::new();
    };
    let named = match fantom_core::requirements::inventory(source) {
        Ok(held) => needs.named_from(&held),
        Err(_) => needs,
    };
    render::requirements(&named)
}

fn run_verify(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let report = fantom_core::verify::check(&raw)?;

    let mut out = String::new();
    let _ = writeln!(out, "file:  {}", file.display());
    let _ = writeln!(
        out,
        "checked {} record checksum{} across {} area{}",
        report.checked,
        if report.checked == 1 { "" } else { "s" },
        report.areas_with_checksums,
        if report.areas_with_checksums == 1 {
            ""
        } else {
            "s"
        },
    );
    if report.areas_with_checksums == 0 {
        let _ = writeln!(out, "(this container stores no per-record checksums)");
    }
    for problem in &report.problems {
        let _ = writeln!(out, "  problem: {problem}");
    }
    if !report.is_ok() {
        // Exit non-zero so this is usable as a gate in a script.
        return Err(fantom_core::Error::Unrecognized(format!(
            "{out}{} problem{} found",
            report.problems.len(),
            if report.problems.len() == 1 { "" } else { "s" }
        )));
    }
    let _ = writeln!(out, "OK");
    Ok(out)
}

/// How many user samples a file carries, for reporting what an extract left behind.
fn sample_count(raw: &Raw) -> usize {
    fantom_core::container::Svd::parse(raw)
        .and_then(|svd| fantom_core::container::read_samples(raw, &svd))
        .map(|bank| bank.slots.len())
        .unwrap_or(0)
}

/// Explain a tone-bank rebuild that carried every sample instead of the referenced ones.
///
/// Only a `PATa` tone says which user samples it plays. A drum kit's waves live in its paired
/// `INSa`, where the field that would mark one as a user sample has never been seen set — so the
/// samples a kit plays cannot be told apart from the ones it does not, and all of them travel. The
/// output is correct but larger than it needs to be, which is worth saying out loud.
fn carried_all_samples_note(source: &Raw, output: &Raw) -> String {
    let Ok(spec) = fantom_core::tonebank::engine(source) else {
        return String::new();
    };
    if spec.sample_refs_decoded {
        return String::new();
    }
    match sample_count(output) {
        0 => String::new(),
        n => format!(
            "note: carried all {n} user sample{} — a {} record's sample references are not\n\
             \x20     decoded, so the ones it plays cannot be told from the ones it does not.\n",
            if n == 1 { "" } else { "s" },
            spec.tag_str(),
        ),
    }
}

/// Whether a file is an SVZ tone bank rather than a scene bank.
fn is_tone_bank(raw: &Raw) -> bool {
    fantom_core::container::Svd::parse(raw)
        .map(|svd| svd.kind == fantom_core::container::Kind::Svz)
        .unwrap_or(false)
}

/// Write a companion sample file and/or repoint the bank's references at where it will land.
///
/// A scene bank names user samples by absolute panel slot, so on any other instrument it plays
/// only if that audio sits at those exact numbers. Two files fix that between them: an `.svz` the
/// destination imports as one contiguous run, and this bank rewritten to point at that run.
fn carry_scene_samples(
    source: &Raw,
    extracted: &Raw,
    companion: Option<&PathBuf>,
    base: Option<u16>,
    should_write: bool,
) -> fantom_core::Result<(Raw, String)> {
    let slots = fantom_core::repackage::referenced_sample_slots(extracted)?;
    if slots.is_empty() {
        return Ok((
            extracted.clone(),
            "note: these scenes play no user samples, so there is nothing to carry\n".to_string(),
        ));
    }

    let base = base.unwrap_or(1);
    let remap = fantom_core::repackage::contiguous_remap(&slots, base)?;
    let rebased = fantom_core::repackage::rebase_sample_slots(extracted, &remap)?;

    let mut out = String::new();
    if let Some(path) = companion {
        let indexes: Vec<usize> = slots.iter().map(|&slot| slot as usize - 1).collect();
        let bank = fantom_core::samplebank::export_samples(source, &indexes)?;
        if should_write {
            bank.save(path)?;
        }
        let _ = writeln!(
            out,
            "{} {} sample{} to {}",
            if should_write { "wrote" } else { "would write" },
            slots.len(),
            if slots.len() == 1 { "" } else { "s" },
            path.display()
        );
    }

    // IMPORT SAMPLE asks for a destination *per sample*, confirmed on a FANTOM-6 — it does not
    // fill a run from one starting slot. So a range is not an instruction: name each sample and
    // the slot it has to go to, in the order the dialog lists them, because the bank has already
    // been rewritten to expect exactly that.
    let names = fantom_core::container::Svd::parse(source)
        .and_then(|svd| fantom_core::container::read_samples(source, &svd))
        .map(|bank| bank.slots)
        .unwrap_or_default();

    let _ = writeln!(
        out,
        "the instrument asks where each sample goes — assign them exactly like this, or the\n\
         bank will not find them:\n"
    );
    for (position, &was) in slots.iter().enumerate() {
        let name = names
            .iter()
            .find(|s| s.index + 1 == was as usize)
            .map(|s| s.name.as_str())
            .unwrap_or("<not in this file>");
        let now = base as usize + position;
        let _ = writeln!(
            out,
            "    {:>2}. {name:<20} → slot {now:<5} (was {was})",
            position + 1
        );
    }
    let _ = writeln!(out);

    Ok((rebased, out))
}

fn run_extract(
    file: &PathBuf,
    scenes: &[usize],
    write: &WriteOptions,
    samples: Option<&PathBuf>,
    samples_at: Option<u16>,
) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    if is_tone_bank(&raw) {
        let extracted = fantom_core::tonebank::extract_tones(&raw, scenes)?;
        if write.should_write() {
            extracted.save(write.output())?;
        }
        let mut out = carried_all_samples_note(&raw, &extracted);
        // Samples travel with the tones that play them; anything unreferenced is left behind, so
        // say so rather than silently shrinking the file.
        let (before, after) = (sample_count(&raw), sample_count(&extracted));
        if before > after {
            let _ = writeln!(
                out,
                "note: left behind {} sample{} no selected tone references",
                before - after,
                if before - after == 1 { "" } else { "s" }
            );
        }
        let _ = writeln!(
            out,
            "extracted {} tone{}{}{}",
            scenes.len(),
            if scenes.len() == 1 { "" } else { "s" },
            match after {
                0 => String::new(),
                n => format!(" with {n} sample{}", if n == 1 { "" } else { "s" }),
            },
            write_destination(write),
        );
        return Ok(out);
    }
    let extracted = fantom_core::repackage::extract_scenes(&raw, scenes)?;

    // Without --samples/--samples-at the bank keeps its original slot references, and the warning
    // names what the destination must already hold. With either, the samples are made to travel.
    let (final_bank, note) = match (samples, samples_at) {
        (None, None) => (extracted, String::new()),
        (companion, base) => {
            carry_scene_samples(&raw, &extracted, companion, base, write.should_write())?
        }
    };
    let note = if note.is_empty() {
        requirements_note(&final_bank, &raw)
    } else {
        note
    };

    if write.should_write() {
        final_bank.save(write.output())?;
    }
    Ok(format!(
        "{note}extracted {} scene{}{}\n",
        scenes.len(),
        if scenes.len() == 1 { "" } else { "s" },
        write_destination(write),
    ))
}

fn write_destination(write: &WriteOptions) -> String {
    if write.should_write() {
        format!(" to {}", write.output().display())
    } else {
        format!(" ({})", write.dry_run_notice())
    }
}

fn run_canary(file: &PathBuf, scene: usize, write: &WriteOptions) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let canary = fantom_core::repackage::canary_scene(&raw, scene)?;
    if write.should_write() {
        canary.save(write.output())?;
    }
    Ok(format!(
        "{}{} scene {scene} canary with marked dependencies{}\n",
        requirements_note(&canary, &raw),
        if write.should_write() {
            "wrote"
        } else {
            "prepared"
        },
        write_destination(write),
    ))
}

fn run_merge(
    target: &PathBuf,
    source: &PathBuf,
    write: &WriteOptions,
) -> fantom_core::Result<String> {
    let target_raw = Raw::open(target)?;
    let source_raw = Raw::open(source)?;
    if is_tone_bank(&target_raw) {
        let before = fantom_core::codec::read_bundled_tones(&target_raw)?.len();
        let merged = fantom_core::tonebank::merge_tones(&target_raw, &source_raw)?;
        let after = fantom_core::codec::read_bundled_tones(&merged)?.len();
        if write.should_write() {
            merged.save(write.output())?;
        }
        return Ok(format!(
            "{}merged to {} tones ({} new){}\n",
            carried_all_samples_note(&target_raw, &merged),
            after,
            after - before,
            write_destination(write),
        ));
    }
    let source_count = fantom_core::codec::read_scenes(&source_raw)?.len();
    let merged = fantom_core::repackage::merge_scenes(&target_raw, &source_raw)?;
    if write.should_write() {
        merged.save(write.output())?;
    }
    Ok(format!(
        "{}appended {source_count} scene{} from {}{}\n",
        requirements_note(&merged, &source_raw),
        if source_count == 1 { "" } else { "s" },
        source.display(),
        write_destination(write),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use fantom_core::diff::ByteRun;

    #[test]
    fn scenes_list_is_accepted() {
        let cli = Cli::try_parse_from(["fantom", "scenes", "list", "bank.svd"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn every_canonical_command_path_is_accepted() {
        let commands: &[&[&str]] = &[
            &["fantom", "scenes", "list", "bank.svd"],
            &[
                "fantom",
                "scenes",
                "show",
                "bank.svd",
                "1",
                "--include-disabled",
            ],
            &[
                "fantom",
                "scenes",
                "rename",
                "bank.svd",
                "1",
                "New name",
                "--dry-run",
            ],
            &[
                "fantom",
                "scenes",
                "comment",
                "bank.svd",
                "1",
                "A comment",
                "--output",
                "out.svd",
            ],
            &[
                "fantom",
                "scenes",
                "extract",
                "bank.svd",
                "1",
                "2",
                "--dry-run",
            ],
            &[
                "fantom", "scenes", "canary", "bank.svd", "1", "--output", "out.svd",
            ],
            &[
                "fantom",
                "scenes",
                "merge",
                "base.svd",
                "source.svd",
                "--dry-run",
            ],
            &["fantom", "tones", "list", "bank.svd"],
            &[
                "fantom",
                "tones",
                "extract",
                "backup.SVD",
                "954",
                "--dry-run",
            ],
            &[
                "fantom",
                "tones",
                "extract",
                "backup.SVD",
                "1",
                "--area",
                "RHYa",
                "-o",
                "kit.svz",
            ],
            &["fantom", "samples", "list", "bank.svd"],
            &["fantom", "areas", "list", "bank.svd"],
            &["fantom", "check", "bank.svd"],
            &["fantom", "check", "theirs.svd", "--against", "mine.SVD"],
            &["fantom", "inspect", "bank.svd", "--length", "512"],
            &[
                "fantom",
                "diff",
                "before.svd",
                "after.svd",
                "--area",
                "DCWa",
            ],
            &["fantom", "verify", "bank.svd"],
        ];

        for argv in commands {
            assert!(
                Cli::try_parse_from(*argv).is_ok(),
                "failed to parse {argv:?}"
            );
        }
    }

    #[test]
    fn write_commands_require_output_or_dry_run() {
        for argv in [
            ["fantom", "scenes", "rename", "bank.svd", "1", "New name"].as_slice(),
            ["fantom", "scenes", "comment", "bank.svd", "1", "A comment"].as_slice(),
            ["fantom", "scenes", "extract", "bank.svd", "1"].as_slice(),
            ["fantom", "scenes", "canary", "bank.svd", "1"].as_slice(),
            ["fantom", "scenes", "merge", "base.svd", "source.svd"].as_slice(),
            ["fantom", "tones", "extract", "backup.SVD", "954"].as_slice(),
        ] {
            assert!(
                Cli::try_parse_from(argv).is_err(),
                "unexpectedly parsed {argv:?}"
            );
        }

        assert!(Cli::try_parse_from([
            "fantom",
            "scenes",
            "rename",
            "bank.svd",
            "1",
            "New name",
            "--dry-run",
            "--output",
            "out.svd",
        ])
        .is_ok());
    }

    #[test]
    fn removed_flat_commands_and_options_are_rejected() {
        for argv in [
            ["fantom", "scenes", "bank.svd"].as_slice(),
            ["fantom", "show", "bank.svd", "1"].as_slice(),
            ["fantom", "rename", "bank.svd", "1", "New name", "--dry-run"].as_slice(),
            ["fantom", "inspect", "bank.svd", "--len", "512"].as_slice(),
            ["fantom", "scenes", "show", "bank.svd", "1", "--all"].as_slice(),
            // `dependencies list` reported a fraction of the closure as prose; `check` reports
            // all of it, from `fantom_core::requirements`.
            ["fantom", "dependencies", "list", "bank.svd"].as_slice(),
        ] {
            assert!(
                Cli::try_parse_from(argv).is_err(),
                "unexpectedly parsed {argv:?}"
            );
        }
    }

    #[test]
    fn root_and_scenes_help_are_discoverable() {
        let mut root = Cli::command();
        let root_help = root.render_help().to_string();
        assert!(root_help.contains("scenes"));
        assert!(root_help.contains("inspect"));

        let mut scenes = Cli::command();
        let scenes_command = scenes.find_subcommand_mut("scenes").unwrap();
        let scenes_help = scenes_command.render_help().to_string();
        for command in [
            "list", "show", "rename", "comment", "extract", "canary", "merge",
        ] {
            assert!(
                scenes_help.contains(command),
                "missing {command} from scenes help"
            );
        }
    }

    fn run_at(offset: usize, file_offset: usize) -> ByteRun {
        ByteRun {
            offset,
            left_at: file_offset,
            right_at: file_offset,
            left: vec![b'S'],
            right: vec![b'X'],
        }
    }

    /// A scene rename changes the first byte of a record, so the run sits at record offset 0 with
    /// no context available before it — while its *file* offset is thousands of bytes in. Clamping
    /// against the file offset alone underflowed and panicked.
    #[test]
    fn context_is_clamped_to_what_the_record_actually_has() {
        let raw = Raw::from_bytes(vec![b'.'; 4096]);
        for context in [1, 4, 12, 512] {
            let rendered = render_run("PRFa[0]", &run_at(0, 0x50), &raw, &raw, context);
            assert!(
                rendered.starts_with("PRFa[0]+0x0000"),
                "context {context} moved the reported offset: {rendered}"
            );
        }
        // With room to spare, context still widens the window as intended.
        let rendered = render_run("DCWa[0]", &run_at(11, 0x100), &raw, &raw, 4);
        assert!(rendered.starts_with("DCWa[0]+0x0007"), "{rendered}");
    }
}
