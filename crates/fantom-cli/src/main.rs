//! `fantom` — command-line librarian for Roland Fantom data files.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use fantom_core::container::Raw;

#[derive(Parser)]
#[command(name = "fantom", version, about = "Librarian for Roland Fantom data files")]
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

    /// List the tones bundled in a file's PATa area.
    Tones {
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
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Inspect { file, len, offset } => run_inspect(&file, offset, len),
        Command::Areas { file } => run_areas(&file),
        Command::Scenes { file } => run_scenes(&file),
        Command::Show { file, scene, all } => run_show(&file, scene, all),
        Command::Tones { file } => run_tones(&file),
        Command::Rename { file, scene, name, output } => {
            run_edit(&file, output.as_ref(), &format!("renamed scene {scene} to {name:?}"), |raw| {
                fantom_core::codec::set_scene_name(raw, scene, &name)
            })
        }
        Command::Comment { file, scene, text, output } => {
            run_edit(&file, output.as_ref(), &format!("set comment on scene {scene}"), |raw| {
                fantom_core::codec::set_scene_comment(raw, scene, &text)
            })
        }
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
    let _ = writeln!(out, "{:<6} {:<6} {:>10} {:>10}", "TAG", "FORMAT", "OFFSET", "SIZE");
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

fn run_scenes(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let scenes = fantom_core::codec::read_scenes(&raw)?;
    let mut out = String::new();
    let _ = writeln!(out, "{} scenes:", scenes.len());
    for (i, scene) in scenes.iter().enumerate() {
        let _ = writeln!(out, "{:>4}  {}", i + 1, scene.name);
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
        "{:>4}  {:<3}  {:<22}  {:>10}  {:>5}",
        "zone", "on", "tone", "range", "level"
    );
    for z in &s.zones {
        if !z.enabled && !all {
            continue;
        }
        let range = format!("{}..{}", note_name(z.key_low), note_name(z.key_high));
        let _ = writeln!(
            out,
            "{:>4}  {:<3}  {:<22}  {:>10}  {:>5}",
            z.number + 1,
            if z.enabled { "on" } else { "off" },
            tone_label(&z.tone),
            range,
            z.level,
        );
    }
    Ok(out)
}

fn run_tones(file: &PathBuf) -> fantom_core::Result<String> {
    let raw = Raw::open(file)?;
    let svd = fantom_core::container::Svd::parse(&raw)?;
    let pat = fantom_core::container::PatArea::from_svd(&raw, &svd)?;
    let mut out = String::new();
    let _ = writeln!(out, "{} tones:", pat.tones().len());
    for (i, tone) in pat.tones().iter().enumerate() {
        let _ = writeln!(out, "{i:>5}  {}", tone.name);
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

/// Render a zone's tone reference for display.
fn tone_label(tone: &fantom_core::model::ToneRef) -> String {
    use fantom_core::model::ToneRef;
    match tone {
        ToneRef::User { name: Some(n), .. } => n.clone(),
        ToneRef::User { id, name: None } => format!("user #{id}"),
        ToneRef::Preset { id } => match tone.preset() {
            Some(p) => format!("{} {:04} {}", p.bank, p.number, p.name),
            None => format!("preset {id:#06x}"),
        },
    }
}

/// Render a MIDI note number as a name, e.g. 60 -> `C4` (Roland convention: middle C = C4).
fn note_name(n: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NAMES[(n % 12) as usize], (n / 12) as i16 - 1)
}
