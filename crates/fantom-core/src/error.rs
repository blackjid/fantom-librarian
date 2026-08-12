use std::io;

/// Errors produced while reading or decoding Fantom data files.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    /// An I/O error that names the file it happened to. Worth the extra variant: without the path,
    /// a bare "Permission denied" gives no clue whether the source, the destination, or a
    /// mistyped mount point was at fault.
    #[error("cannot {action} {path}: {source}")]
    File {
        action: &'static str,
        path: String,
        source: io::Error,
    },

    #[error("malformed file: {0}")]
    Parse(#[from] binrw::Error),

    /// The bytes were read successfully but do not describe a structure we recognise yet.
    #[error("unrecognized structure: {0}")]
    Unrecognized(String),
}

pub type Result<T> = std::result::Result<T, Error>;
