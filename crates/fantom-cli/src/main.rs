//! `fantom` — command-line librarian for Roland Fantom data files.

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
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Inspect { file, len, offset } => run_inspect(&file, offset, len),
        Command::Areas { file } => run_areas(&file),
        Command::Scenes { file } => run_scenes(&file),
        Command::Show { file, scene, all } => run_show(&file, scene, all),
        Command::Tones { file } => run_tones(&file),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_inspect(file: &PathBuf, offset: usize, len: usize) -> fantom_core::Result<()> {
    let raw = Raw::open(file)?;

    println!("file:  {}", file.display());
    println!("size:  {} bytes", raw.len());
    match raw.ascii_magic(4) {
        Some(magic) => println!("magic: {magic:?} (printable)"),
        None => println!("magic: <non-printable>"),
    }
    println!();
    print!("{}", raw.hexdump(offset, len));

    Ok(())
}

fn run_areas(file: &PathBuf) -> fantom_core::Result<()> {
    let raw = Raw::open(file)?;
    let svd = fantom_core::container::Svd::parse(&raw)?;
    println!("{:<6} {:<6} {:>10} {:>10}", "TAG", "FORMAT", "OFFSET", "SIZE");
    for area in &svd.areas {
        println!(
            "{:<6} {:<6} {:>10} {:>10}",
            area.tag_str(),
            area.format_str(),
            area.offset,
            area.size,
        );
    }
    Ok(())
}

fn run_scenes(file: &PathBuf) -> fantom_core::Result<()> {
    let raw = Raw::open(file)?;
    let scenes = fantom_core::codec::read_scenes(&raw)?;
    println!("{} scenes:", scenes.len());
    for (i, scene) in scenes.iter().enumerate() {
        println!("{:>4}  {}", i + 1, scene.name);
    }
    Ok(())
}

fn run_show(file: &PathBuf, scene: usize, all: bool) -> fantom_core::Result<()> {
    let raw = Raw::open(file)?;
    let scenes = fantom_core::codec::read_scenes(&raw)?;
    let s = scenes.get(scene.wrapping_sub(1)).ok_or_else(|| {
        fantom_core::Error::Unrecognized(format!(
            "scene {scene} out of range (file has {})",
            scenes.len()
        ))
    })?;

    println!("Scene {scene}: {}", s.name);
    println!(
        "{:>4}  {:<3}  {:<22}  {:>10}  {:>5}",
        "zone", "on", "tone", "range", "level"
    );
    for z in &s.zones {
        if !z.enabled && !all {
            continue;
        }
        let range = format!("{}..{}", note_name(z.key_low), note_name(z.key_high));
        println!(
            "{:>4}  {:<3}  {:<22}  {:>10}  {:>5}",
            z.number + 1,
            if z.enabled { "on" } else { "off" },
            tone_label(&z.tone),
            range,
            z.level,
        );
    }
    Ok(())
}

fn run_tones(file: &PathBuf) -> fantom_core::Result<()> {
    let raw = Raw::open(file)?;
    let svd = fantom_core::container::Svd::parse(&raw)?;
    let pat = fantom_core::container::PatArea::from_svd(&raw, &svd)?;
    println!("{} tones:", pat.tones().len());
    for (i, tone) in pat.tones().iter().enumerate() {
        println!("{i:>5}  {}", tone.name);
    }
    Ok(())
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
