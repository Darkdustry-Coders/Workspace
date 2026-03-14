use std::{
    fs::{self, DirEntry, Metadata},
    io,
    path::{Path, PathBuf},
};

pub struct ReadDir<S: AsRef<Path>> {
    readdir: fs::ReadDir,
    path: S,
}
impl<S: AsRef<Path>> Iterator for ReadDir<S> {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.readdir.next().map(|x| {
            x.map_err(|x| {
                io::Error::new(
                    x.kind(),
                    format!("readdir({:?}): {}", self.path.as_ref().display(), x),
                )
            })
        })
    }
}
/// Open a directory for reading.
///
/// ## Fs2
/// This function will use more memory to improve error reporting.
/// If you want a more lightweight version, use [fs::read_dir].
pub fn read_dir<S: AsRef<Path>>(path: S) -> io::Result<ReadDir<S>> {
    Ok(ReadDir {
        readdir: match fs::read_dir(path.as_ref()) {
            Ok(x) => x,
            Err(why) => {
                return Err(io::Error::new(
                    why.kind(),
                    format!("opendir({:?}): {}", path.as_ref().display(), why),
                ));
            }
        },
        path,
    })
}

/// Recursively create directories.
///
/// Does not fail if a directory exists.
///
/// ## Fs2
/// This function will use more memory to improve error reporting.
/// If you want a more lightweight version, use [fs::create_dir_all].
pub fn create_dir_all<S: AsRef<Path>>(path: S) -> io::Result<()> {
    fs::create_dir_all(path.as_ref()).map_err(|x| {
        io::Error::new(
            x.kind(),
            format!("create_dir_all({:?}): {}", path.as_ref().display(), x),
        )
    })
}

/// Write to a file.
///
/// ## Fs2
/// This function will use more memory to improve error reporting.
/// If you want a more lightweight version, use [fs::write].
pub fn write<S: AsRef<Path>, C: AsRef<[u8]>>(path: S, contents: C) -> io::Result<()> {
    fs::write(path.as_ref(), contents).map_err(|x| {
        io::Error::new(
            x.kind(),
            format!("write({:?}): {}", path.as_ref().display(), x),
        )
    })
}

/// Get the original path behind a symlink.
///
/// The path may be another symlink.
///
/// ## Fs2
/// This function will use more memory to improve error reporting.
/// If you want a more lightweight version, use [fs::read_link].
pub fn read_link<S: AsRef<Path>>(path: S) -> io::Result<PathBuf> {
    fs::read_link(path.as_ref()).map_err(|x| {
        io::Error::new(
            x.kind(),
            format!("read_link({:?}): {}", path.as_ref().display(), x),
        )
    })
}

/// Get the original path behind a symlink.
///
/// The path may be another symlink.
///
/// ## Fs2
/// This function will use more memory to improve error reporting.
/// If you want a more lightweight version, use [fs::read_link].
pub fn hard_link<O: AsRef<Path>, Q: AsRef<Path>>(original: O, link: Q) -> io::Result<()> {
    fs::hard_link(original.as_ref(), link.as_ref()).map_err(|x| {
        io::Error::new(
            x.kind(),
            format!(
                "hard_link({:?}, {:?}): {}",
                original.as_ref().display(),
                link.as_ref().display(),
                x
            ),
        )
    })
}

/// Get the file metadata.
///
/// ## Fs2
/// This function will use more memory to improve error reporting.
/// If you want a more lightweight version, use [fs::metadata].
pub fn metadata<S: AsRef<Path>>(path: S) -> io::Result<Metadata> {
    fs::metadata(path.as_ref()).map_err(|x| {
        io::Error::new(
            x.kind(),
            format!("metadata({:?}): {}", path.as_ref().display(), x),
        )
    })
}

/// Delete a file.
///
/// ## Fs2
/// This function will use more memory to improve error reporting.
/// If you want a more lightweight version, use [fs::remove_file].
pub fn remove_file<S: AsRef<Path>>(path: S) -> io::Result<()> {
    fs::remove_file(path.as_ref()).map_err(|x| {
        io::Error::new(
            x.kind(),
            format!("remove_file({:?}): {}", path.as_ref().display(), x),
        )
    })
}

/// Delete a directory recursively.
///
/// ## Fs2
/// This function will use more memory to improve error reporting.
/// If you want a more lightweight version, use [fs::remove_dir_all].
pub fn remove_dir_all<S: AsRef<Path>>(path: S) -> io::Result<()> {
    fs::remove_dir_all(path.as_ref()).map_err(|x| {
        io::Error::new(
            x.kind(),
            format!("remove_dir_all({:?}): {}", path.as_ref().display(), x),
        )
    })
}
