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
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Inspect { file, len, offset } => run_inspect(&file, offset, len),
        Command::Areas { file } => run_areas(&file),
        Command::Scenes { file } => run_scenes(&file),
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
