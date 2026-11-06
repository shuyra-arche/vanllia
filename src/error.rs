use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    Workflow(String),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Declaration {
        path: PathBuf,
        message: String,
    },
    Cargo {
        manifest: PathBuf,
        message: String,
    },
    Duplicate {
        kind: &'static str,
        id: String,
        first: PathBuf,
        second: PathBuf,
    },
    UnknownUnit {
        owner: String,
        id: String,
    },
    UnknownImage {
        unit: String,
        image: String,
    },
    UnknownResource {
        resource: String,
    },
    EmptySelection,
    Cycle(Vec<String>),
    RootMissing(PathBuf),
    SourceMissing(PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Workflow(message) => f.write_str(message),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Declaration { path, message } => write!(f, "{}: {message}", path.display()),
            Self::Cargo { manifest, message } => {
                write!(f, "Cargo metadata for {}: {message}", manifest.display())
            }
            Self::Duplicate {
                kind,
                id,
                first,
                second,
            } => write!(
                f,
                "duplicate {kind} `{id}`: {} and {}",
                first.display(),
                second.display()
            ),
            Self::UnknownUnit { owner, id } => write!(f, "{owner} references unknown Unit `{id}`"),
            Self::UnknownImage { unit, image } => {
                write!(f, "Unit `{unit}` references unknown Cargo bin `{image}`")
            }
            Self::UnknownResource { resource } => write!(
                f,
                "initrd resource `{resource}` has no image, file or System runtime declaration"
            ),
            Self::EmptySelection => {
                f.write_str("no Unit selected; set `units` in vanllia.toml or pass --unit")
            }
            Self::Cycle(units) => {
                write!(f, "selected after graph cannot start: {}", units.join(", "))
            }
            Self::RootMissing(path) => write!(f, "no vanllia.toml found above {}", path.display()),
            Self::SourceMissing(path) => write!(
                f,
                "source {} is unavailable; prepare this checkout before check/plan",
                path.display()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub(crate) fn io(path: &std::path::Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}
