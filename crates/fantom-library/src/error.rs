use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("catalog: {0}")]
    Db(#[from] rusqlite::Error),

    #[error(transparent)]
    Core(#[from] fantom_core::Error),

    #[error("{0} is not a Fantom library workspace")]
    NotAWorkspace(PathBuf),

    #[error("{0} already contains a Fantom library workspace")]
    AlreadyAWorkspace(PathBuf),

    #[error("this workspace was written by a newer version of the app (format {0})")]
    WorkspaceTooNew(u32),

    #[error("no {kind} with id {id}")]
    NotFound { kind: &'static str, id: i64 },

    #[error("{0}")]
    Rejected(String),
}

impl Error {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
