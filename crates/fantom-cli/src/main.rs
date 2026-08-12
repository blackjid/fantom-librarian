//! `fantom` — command-line librarian for Roland Fantom data files.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use fantom_core::container::Raw;

#[derive(Parser)]
#[command(
    name = "fantom",
    version,
    about = "Librarian for Roland Fantom data files"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Inspect a file's envelope: size, magic, and a hexdump of its head.
    ///
    /// This is the reverse-engineering microscope — record what you learn in `docs/FORMAT.md`.
    Inspect {
        /// Path to a `.svd` / `.svz` / `.sdz` file.
        file: PathBuf,

        /// How many bytes to hexdump from the given offset.
        #[arg(long, default_value_t = 256)]
        len: usize,

        /// Byte offset to start the hexdump at.
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },

    /// List the memory areas in an SVD container.
    Areas {
        /// Path to a `.svd` file.
        file: PathBuf,
    },

    /// Compare two SVD files, reporting each difference as AREA[record]+offset.
    ///
    /// The counterpart to `inspect`: export two files that differ by one deliberate change and
    /// this shows exactly which bytes carry it. Record the result in `docs/FORMAT.md`.
    Diff {
        /// Baseline file.
        left: PathBuf,
        /// File to compare against the baseline.
        right: PathBuf,
        /// Only report this area (e.g. `DCWa`); repeatable.
        #[arg(long, value_name = "TAG")]
        area: Vec<String>,
        /// Unchanged bytes to show on either side of each run.
        #[arg(long, default_value_t = 0)]
        context: usize,
    },

    /// Report bundled ACB, V-Piano, and Model dependency areas.
    Dependencies {
        /// Path to a `.svd` file.
        file: PathBuf,
    },

    /// List the scene names in an SVD backup.
    Scenes {
        /// Path to a `.svd` file.
        file: PathBuf,
    },

    /// Show one scene with its 16 zones (tone, switch, key range, level).
    Show {
        /// Path to a `.svd` file.
        file: PathBuf,
        /// Scene number, 1-based (as printed by `scenes`).
        scene: usize,
        /// Include zones that are switched off.
        #[arg(long)]
        all: bool,
    },

    /// List every named user tone bundled in an SVD.
    Tones {
        /// Path to a `.svd` file.
        file: PathBuf,
    },

    /// List the user samples and multisamples a file carries.
    Samples {
        /// Path to a `.svd` file.
        file: PathBuf,
    },

    /// Rename a scene. Without --output this is a dry run.
    Rename {
        /// Path to a `.svd` file.
        file: PathBuf,
        /// Scene number, 1-based (as printed by `scenes`).
        scene: usize,
        /// New scene name (max 16 chars; longer is truncated).
        name: String,
        /// Write the edited file here (omit for a dry run; pass the input path to edit in place).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Set a scene's comment/memo. Without --output this is a dry run.
    Comment {
        /// Path to a `.svd` file.
        file: PathBuf,
        /// Scene number, 1-based.
        scene: usize,
        /// New comment (max 64 chars; longer is truncated).
        text: String,
        /// Write the edited file here (omit for a dry run).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Build a new bank from a self-contained scene export and selected scenes.
    Extract {
        /// Self-contained scene-export SVD file.
        file: PathBuf,
        /// Scene numbers to include, 1-based and in the desired output order.
        #[arg(required = true, num_args = 1..)]
        scenes: Vec<usize>,
        /// Write the extracted bank here.
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Extract one scene with visible canary names for hardware tone-bundle validation.
    Canary {
        /// Self-contained scene-export SVD file.
        file: PathBuf,
        /// Scene number to extract, 1-based.
        scene: usize,
        /// Write the canary bank here.
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Merge two self-contained scene-export banks and rebundle their user tones.
    Merge {
        /// Bank whose scenes and non-scene areas form the base of the output.
        target: PathBuf,
        /// Bank whose scenes will be appended.
        source: PathBuf,
        /// Write the merged bank here.
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Inspect { file, len, offset } => run_inspect(&file, offset, len),
        Command::Areas { file } => run_areas(&file),
        Command::Diff {
            left,
            right,
            area,
            context,
        } => run_diff(&left, &right, &area, context),
        Command::Dependencies { file } => run_dependencies(&file),
        Command::Scenes { file } => run_scenes(&file),
        Command::Show { file, scene, all } => run_show(&file, scene, all),
        Command::Tones { file } => run_tones(&file),
        Command::Samples { file } => run_samples(&file),
        Command::Rename {
            file,
            scene,
            name,
            output,
        } => run_edit(
            &file,
            output.as_ref(),
            &format!("renamed scene {scene} to {name:?}"),
            |raw| fantom_core::codec::set_scene_name(raw, scene, &name),
        ),
        Command::Comment {
            file,
            scene,
            text,
            output,
        } => run_edit(
            &file,
            output.as_ref(),
            &format!("set comment on scene {scene}"),
            |raw| fantom_core::codec::set_scene_comment(raw, scene, &text),
        ),
        Command::Extract {
            file,
            scenes,
            output,
        } => run_extract(&file, &scenes, &output),
        Command::Canary {
            file,
            scene,
            output,
        } => run_canary(&file, scene, &output),
        Command::Merge {
            target,
            source,
            output,
        } => run_merge(&target, &source, &output),
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
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<6} {:<6} {:>10} {:>10}",
        "TAG", "FORMAT", "OFFSET", "SIZE"
    );
    for area in &svd.areas {
        let _ = writeln!(
            out,
            "{:<6} {:<6} {:>10} {:>10}",
            area.tag_str(),
            area.format_str(),
            area.offset,
            area.size,
        );
    }
    Ok(out)
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
    let _ = writeln!(out, "right: {} ({} bytes)", right.display(), right_raw.len());
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
                        render_run(&format!("{tag}.header"), run, &left_raw, &right_raw, context)
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
        let before = context
            .min(run.offset)
            .min(run.left_at)
            .min(run.right_at);
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
        hex(&left_bytes),
        hex(&right_bytes),
        ascii(&left_bytes),
        ascii(&right_bytes),
    )
}

fn window(raw: &Raw, at: usize, len: usize) -> Vec<u8> {
    raw.bytes()
        .get(at..(at + len).min(raw.len()))
        .unwrap_or_default()
        .to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

fn run_dependencies(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let svd = fantom_core::container::Svd::parse(&raw)?;
    let tones = fantom_core::codec::read_bundled_tones(&raw)?;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<6} {:<6} {:>10} {:<24}",
        "TAG", "FORMAT", "SIZE", "STATUS"
    );
    for tag in [b"ACBa", b"DCWa", b"MDLa"] {
        if let Some(area) = svd.area(tag) {
            let count = tones.iter().filter(|tone| tone.area == *tag).count();
            let _ = writeln!(
                out,
                "{:<6} {:<6} {:>10} {:<24}",
                area.tag_str(),
                String::from_utf8_lossy(&area.format),
                area.size,
                format!("{count} tones; names decoded")
            );
        } else {
            let _ = writeln!(
                out,
                "{:<6} {:<6} {:>10} {:<24}",
                area_tag(tag),
                "-",
                "-",
                "absent"
            );
        }
    }
    Ok(out)
}

fn area_tag(tag: &[u8; 4]) -> String {
    String::from_utf8_lossy(tag).into_owned()
}

fn run_scenes(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let scenes = fantom_core::codec::read_scenes(&raw)?;
    let mut out = String::new();
    let _ = writeln!(out, "{} scenes:", scenes.len());
    let _ = writeln!(out, "{:<4} {:<20} REFERENCES", "NO.", "NAME");
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
        let _ = writeln!(out, "{:>4} {:<20} {}", i + 1, scene.name, summary);
    }
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
    let _ = writeln!(out, "Scene {scene}: {}", s.name);
    if !s.comment.is_empty() {
        let _ = writeln!(out, "note: {}", s.comment);
    }
    let _ = writeln!(
        out,
        "{:>4}  {:<3}  {:<9}  {:<8}  {:<22}  {:>10}  {:>5}",
        "zone", "on", "type", "bank", "tone", "range", "level"
    );
    for z in &s.zones {
        if !z.enabled && !all {
            continue;
        }
        let range = format!("{}..{}", note_name(z.key_low), note_name(z.key_high));
        let _ = writeln!(
            out,
            "{:>4}  {:<3}  {:<9}  {:<8}  {:<22}  {:>10}  {:>5}",
            z.number + 1,
            if z.enabled { "on" } else { "off" },
            tone_type_label(&z.tone),
            z.tone
                .bank()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("LSB {}", z.tone.address.lsb)),
            tone_label(&z.tone),
            range,
            z.level,
        );
    }
    Ok(out)
}

fn run_tones(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let tones = fantom_core::codec::read_bundled_tones(&raw)?;
    let mut out = String::new();
    let _ = writeln!(out, "{} bundled tones:", tones.len());
    let _ = writeln!(out, "{:<6} {:<9} {:>5}  NAME", "AREA", "TYPE", "INDEX");
    for tone in tones {
        let _ = writeln!(
            out,
            "{:<6} {:<9} {:>5}  {}",
            area_tag(&tone.area),
            tone.tone_type.label(),
            tone.index,
            tone.name
        );
    }
    Ok(out)
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
    let _ = writeln!(
        out,
        "{:>5}  {:<18} {:>10} {:>8}  {:>4}  NAME (waveform)",
        "SLOT", "NAME", "FRAMES", "SECONDS", "KEY"
    );
    for slot in &bank.slots {
        let data = bank.data.iter().find(|d| d.slot as usize == slot.index);
        let _ = writeln!(
            out,
            "{:>5}  {:<18} {:>10} {:>8.2}  {:>4}  {}",
            slot.index,
            slot.name,
            slot.end,
            data.map(|d| d.seconds()).unwrap_or_default(),
            note_name(slot.original_key),
            data.map(|d| d.name.as_str()).unwrap_or("<no waveform>"),
        );
    }

    if !bank.multisamples.is_empty() {
        let _ = writeln!(out, "\n{} multisamples:", bank.multisamples.len());
        for ms in &bank.multisamples {
            let _ = writeln!(out, "{:>5}  {}", ms.index, ms.name);
        }
    }

    for orphan in bank.orphans() {
        let _ = writeln!(out, "warning: {orphan}");
    }
    Ok(out)
}

/// Apply an in-place edit to a file, then either write it (with `--output`) or report a dry run.
fn run_edit(
    file: &PathBuf,
    output: Option<&PathBuf>,
    what: &str,
    edit: impl FnOnce(&mut Raw) -> fantom_core::Result<()>,
) -> fantom_core::Result<String> {
    let mut raw = Raw::open(file)?;
    edit(&mut raw)?;
    let mut out = String::new();
    let _ = writeln!(out, "{what}");
    match output {
        Some(path) => {
            raw.save(path)?;
            let _ = writeln!(out, "wrote {}", path.display());
        }
        None => {
            let _ = writeln!(out, "(dry run — pass --output <file> to write)");
        }
    }
    Ok(out)
}

/// Name the user samples an output bank needs but cannot carry.
///
/// A tone references a sample by *slot number*, so the audio never travels with it — the
/// instrument's own scene exports work the same way. The output is therefore complete only if the
/// destination already holds these samples in these slots, which is worth spelling out precisely:
/// a bank that sounds right on the machine it came from can be silent everywhere else.
fn sample_warning(output: &Raw, source: &Raw) -> String {
    let (Ok(out_svd), Ok(src_svd)) = (
        fantom_core::container::Svd::parse(output),
        fantom_core::container::Svd::parse(source),
    ) else {
        return String::new();
    };

    let Ok(tones) = fantom_core::container::PatArea::from_svd(output, &out_svd) else {
        return String::new();
    };
    let mut needed: Vec<(u16, String)> = Vec::new();
    for tone in tones.tones() {
        for &slot in &tone.samples {
            if !needed.iter().any(|(s, _)| *s == slot) {
                needed.push((slot, tone.name.clone()));
            }
        }
    }
    if needed.is_empty() {
        return String::new();
    }
    needed.sort_by_key(|(slot, _)| *slot);

    // Name the slots from the source, which is where the audio still lives.
    let names = fantom_core::container::read_samples(source, &src_svd)
        .map(|bank| bank.slots)
        .unwrap_or_default();

    let mut out = format!(
        "warning: the extracted tones play {} user sample{}, which no scene export carries —\n\
         \x20        a tone references a sample *slot*, so the audio stays on the instrument.\n\
         \x20        The destination needs these samples in these slots:\n",
        needed.len(),
        if needed.len() == 1 { "" } else { "s" },
    );
    for (slot, tone) in &needed {
        let name = names
            .iter()
            .find(|s| s.index + 1 == *slot as usize)
            .map(|s| s.name.as_str())
            .unwrap_or("<not in this file>");
        let _ = writeln!(out, "           slot {slot:>3}  {name:<20} (played by {tone:?})");
    }
    out
}

fn run_extract(file: &PathBuf, scenes: &[usize], output: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let extracted = fantom_core::repackage::extract_scenes(&raw, scenes)?;
    extracted.save(output)?;
    Ok(format!(
        "{}extracted {} scene{} to {}\n",
        sample_warning(&extracted, &raw),
        scenes.len(),
        if scenes.len() == 1 { "" } else { "s" },
        output.display()
    ))
}

fn run_canary(file: &PathBuf, scene: usize, output: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let canary = fantom_core::repackage::canary_scene(&raw, scene)?;
    canary.save(output)?;
    Ok(format!(
        "{}wrote scene {scene} canary with marked dependencies to {}\n",
        sample_warning(&canary, &raw),
        output.display()
    ))
}

fn run_merge(target: &PathBuf, source: &PathBuf, output: &PathBuf) -> fantom_core::Result<String> {
    let target_raw = Raw::open(target)?;
    let source_raw = Raw::open(source)?;
    let source_count = fantom_core::codec::read_scenes(&source_raw)?.len();
    let merged = fantom_core::repackage::merge_scenes(&target_raw, &source_raw)?;
    merged.save(output)?;
    Ok(format!(
        "{}appended {source_count} scene{} from {} to {}\n",
        sample_warning(&merged, &source_raw),
        if source_count == 1 { "" } else { "s" },
        source.display(),
        output.display()
    ))
}

/// Render a zone's tone reference for display.
fn tone_label(tone: &fantom_core::model::ToneRef) -> String {
    match (tone.preset(), tone.name()) {
        (Some(p), _) => format!("{:04} {}", p.number, p.name),
        (_, Some(name)) => name.to_owned(),
        _ => format!("PC {:03}", tone.address.pc),
    }
}

fn tone_type_label(tone: &fantom_core::model::ToneRef) -> String {
    use fantom_core::model::ToneType;
    match tone.tone_type() {
        ToneType::Unknown => format!("MSB {}", tone.address.msb),
        known => known.label().to_owned(),
    }
}

/// Render a MIDI note number as a name, e.g. 60 -> `C4` (Roland convention: middle C = C4).
fn note_name(n: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NAMES[(n % 12) as usize], (n / 12) as i16 - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fantom_core::diff::ByteRun;

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
