use directories::ProjectDirs;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The tree ids kept on this machine, one per line, in the directory the
/// platform sets aside for this program's configuration.
pub struct Store {
    path: PathBuf,
}

pub enum Error {
    NoHome,
    Io { path: PathBuf, cause: io::Error },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoHome => f.write_str(
                "no configuration directory: this platform reports no home directory to keep one in",
            ),
            Error::Io { path, cause } => write!(f, "{}: {cause}", path.display()),
        }
    }
}

impl Store {
    pub fn open() -> Result<Self, Error> {
        let directories = ProjectDirs::from("", "", "jatabag").ok_or(Error::NoHome)?;

        Ok(Self {
            path: directories.config_dir().join("trees.txt"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read(&self) -> Result<Vec<Uuid>, Error> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(cause) => return Err(self.blame(cause)),
        };

        Ok(parse(&contents))
    }

    /// Keep `ids`, and answer with the ones that were not held already.
    pub fn add(&self, ids: &[Uuid]) -> Result<Vec<Uuid>, Error> {
        let mut kept = self.read()?;
        let mut added = Vec::new();

        for id in ids {
            if !kept.contains(id) {
                kept.push(*id);
                added.push(*id);
            }
        }

        if !added.is_empty() {
            self.write(&kept)?;
        }

        Ok(added)
    }

    fn write(&self, ids: &[Uuid]) -> Result<(), Error> {
        let directory = self.path.parent().ok_or(Error::NoHome)?;

        fs::create_dir_all(directory).map_err(|cause| self.blame(cause))?;

        let mut lines: String = ids.iter().fold(String::new(), |mut lines, id| {
            lines.push_str(&id.hyphenated().to_string());
            lines.push('\n');
            lines
        });
        lines.shrink_to_fit();

        let pending = self.path.with_extension("txt.pending");

        fs::write(&pending, lines).map_err(|cause| self.blame(cause))?;
        fs::rename(&pending, &self.path).map_err(|cause| self.blame(cause))
    }

    fn blame(&self, cause: io::Error) -> Error {
        Error::Io {
            path: self.path.clone(),
            cause,
        }
    }
}

fn parse(contents: &str) -> Vec<Uuid> {
    contents
        .lines()
        .filter_map(|line| Uuid::try_parse(line.trim()).ok())
        .collect()
}

#[cfg(test)]
mod add {
    use super::*;

    const ONE: Uuid = Uuid::from_u128(0x2e4e68639bfe4c37b64faa6da5ba51eb);
    const TWO: Uuid = Uuid::from_u128(0x0193a1b2c3d47e5f8a9b0c1d2e3f4a5b);

    fn store(name: &str) -> Store {
        let path = std::env::temp_dir()
            .join(format!("jatabag-{name}"))
            .join("trees.txt");

        let _ = fs::remove_dir_all(path.parent().unwrap());

        Store { path }
    }

    #[test]
    fn keeps_an_id_that_was_not_held() {
        let store = store("keeps");

        assert_eq!(vec![ONE], store.add(&[ONE]).ok().unwrap());
        assert_eq!(vec![ONE], store.read().ok().unwrap());
    }

    #[test]
    fn answers_with_only_the_ids_it_had_not_seen() {
        let store = store("answers");

        store.add(&[ONE]).ok().unwrap();

        assert_eq!(vec![TWO], store.add(&[ONE, TWO]).ok().unwrap());
        assert_eq!(vec![ONE, TWO], store.read().ok().unwrap());
    }

    #[test]
    fn reads_nothing_before_anything_is_kept() {
        assert!(store("empty").read().ok().unwrap().is_empty());
    }

    #[test]
    fn steps_over_a_line_that_is_not_an_id() {
        assert_eq!(
            vec![ONE],
            parse(&format!("\n  {}  \nnot an id\n", ONE.hyphenated()))
        );
    }
}
