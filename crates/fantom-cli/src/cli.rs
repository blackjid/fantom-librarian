use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "fantom",
    version,
    about = "Librarian for Roland Fantom data files"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Work with scenes in an SVD bank or scene export.
    Scenes {
        #[command(subcommand)]
        command: ScenesCommand,
    },
    /// List user tones bundled in a file.
    Tones {
        #[command(subcommand)]
        command: TonesCommand,
    },
    /// List user samples and multisamples in a file.
    Samples {
        #[command(subcommand)]
        command: SamplesCommand,
    },
    /// Inspect memory areas in an SVD container.
    Areas {
        #[command(subcommand)]
        command: AreasCommand,
    },
    /// Report what a file needs from its destination, and weigh it against another file.
    Check {
        /// Path to a `.svd` / `.svz` file.
        file: PathBuf,
        /// A destination to check against — a full backup of the instrument you will load onto.
        #[arg(long, value_name = "FILE")]
        against: Option<PathBuf>,
    },
    /// Inspect a file's envelope: size, magic, and a hexdump of its head.
    Inspect {
        /// Path to a `.svd` / `.svz` / `.sdz` file.
        file: PathBuf,
        /// Byte offset to start the hexdump at.
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// How many bytes to hexdump from the given offset.
        #[arg(long, default_value_t = 256)]
        length: usize,
    },
    /// Compare two SVD files, reporting each difference as AREA[record]+offset.
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
    /// Verify a file's structure and record checksums.
    Verify {
        /// Path to a `.svd` / `.svz` file.
        file: PathBuf,
    },
}

#[derive(Subcommand)]
pub(crate) enum ScenesCommand {
    /// List the scene names in an SVD backup.
    List { file: PathBuf },
    /// Show one scene: tempo, level, memo, and its zones.
    Show {
        file: PathBuf,
        /// Scene number, 1-based (as printed by `scenes list`).
        scene: usize,
        /// Include zones that are switched off.
        #[arg(long)]
        include_disabled: bool,
    },
    /// Rename a scene.
    Rename {
        file: PathBuf,
        scene: usize,
        name: String,
        #[command(flatten)]
        write: WriteOptions,
    },
    /// Set a scene's comment/memo.
    Comment {
        file: PathBuf,
        scene: usize,
        text: String,
        #[command(flatten)]
        write: WriteOptions,
    },
    /// Build a new bank from selected scenes in a self-contained scene export.
    Extract {
        file: PathBuf,
        /// Scene numbers to include, 1-based and in the desired output order.
        #[arg(required = true, num_args = 1..)]
        scenes: Vec<usize>,
        #[command(flatten)]
        write: WriteOptions,
        /// Write a companion `.svz` carrying the selected scenes' user samples.
        #[arg(long, value_name = "FILE")]
        samples: Option<PathBuf>,
        /// Panel slot for the companion's first imported sample; defaults to 1 with --samples.
        #[arg(long, value_name = "SLOT")]
        samples_at: Option<u16>,
    },
    /// Extract one scene with visible canary names for hardware validation.
    Canary {
        file: PathBuf,
        scene: usize,
        #[command(flatten)]
        write: WriteOptions,
    },
    /// Merge two self-contained scene-export banks and rebundle their user tones.
    Merge {
        /// Bank whose scenes and non-scene areas form the base of the output.
        target: PathBuf,
        /// Bank whose scenes will be appended.
        source: PathBuf,
        #[command(flatten)]
        write: WriteOptions,
    },
}

#[derive(Subcommand)]
pub(crate) enum TonesCommand {
    List { file: PathBuf },
}

#[derive(Subcommand)]
pub(crate) enum SamplesCommand {
    List { file: PathBuf },
}

#[derive(Subcommand)]
pub(crate) enum AreasCommand {
    List { file: PathBuf },
}

/// Shared execution policy for commands that can write files.
#[derive(Args)]
pub(crate) struct WriteOptions {
    /// Destination for the generated or edited file.
    #[arg(short, long, required_unless_present = "dry_run")]
    pub(crate) output: Option<PathBuf>,
    /// Validate and prepare the change without writing any files.
    #[arg(long, required_unless_present = "output")]
    pub(crate) dry_run: bool,
}

impl WriteOptions {
    pub(crate) fn should_write(&self) -> bool {
        !self.dry_run
    }

    pub(crate) fn output(&self) -> &PathBuf {
        self.output
            .as_ref()
            .expect("clap requires --output unless --dry-run is set")
    }

    pub(crate) fn dry_run_notice(&self) -> String {
        match &self.output {
            Some(path) => format!("dry run — would write {}", path.display()),
            None => "dry run — no output file selected".to_string(),
        }
    }
}
