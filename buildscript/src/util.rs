//! Utility functions module.
//!
//! This module provides various helper functions for file operations,
//! downloads, and system interactions.

#![allow(unused)]

use std::{
    fs::{self, File, metadata},
    io::{self, IsTerminal, Read, Write, stderr},
    mem::transmute,
    ops::{Div, Mul},
    path::{Path, PathBuf},
    process::Command,
};

/// Global current directory storage (initialized at startup).
pub static mut CURRENT_DIR: Option<PathBuf> = None;

/// Returns the current working directory.
///
/// # Safety
/// This function is unsafe as it accesses a mutable static.
/// It should only be called after initialization.
pub fn current_dir() -> &'static Path {
    unsafe {
        (&raw const CURRENT_DIR)
            .as_ref()
            .unwrap_unchecked()
            .as_ref()
            .unwrap_unchecked()
    }
}

/// Identity function - returns its argument unchanged.
///
/// Useful for explicit type annotations in closures.
pub fn t<T>(t: T) -> T {
    t
}

/// Iterator adapter that allows interjecting items between iterations.
///
/// The closure receives consecutive items and can produce both
/// a stored value and an output value.
pub struct Interject<T, I, F>(I, F, Option<T>)
where
    I: Iterator<Item = T>,
    F: FnMut(T, T) -> (Option<T>, Option<T>);
impl<T, I, F> Iterator for Interject<T, I, F>
where
    I: Iterator<Item = T>,
    F: FnMut(T, T) -> (Option<T>, Option<T>),
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match (self.2.take(), self.0.next()) {
                (None, None) => break None,
                (None, Some(x)) => drop(self.2.replace(x)),
                (Some(x), None) => break Some(x),
                (Some(x), Some(y)) => {
                    let (hoard, send) = (self.1)(x, y);
                    self.2 = hoard;
                    if let Some(x) = send {
                        break Some(x);
                    }
                }
            }
        }
    }
}
/// Creates an Interject iterator adapter.
///
/// # Arguments
/// * `iter` - The source iterator
/// * `fun` - Closure receiving consecutive items
///
/// # Returns
/// An Interject iterator
pub fn interject<T, I, F>(iter: I, fun: F) -> Interject<T, I, F>
where
    I: Iterator<Item = T>,
    F: FnMut(T, T) -> (Option<T>, Option<T>),
{
    Interject(iter, fun, None)
}

/// Iterator that can be either of two iterator types.
///
/// Useful when you need to return different iterator types
/// from a function based on runtime conditions.
pub enum EitherIter<T, A, B>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    /// First iterator variant.
    A(A),
    /// Second iterator variant.
    B(B),
}
impl<T, A, B> Iterator for EitherIter<T, A, B>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::A(a) => a.next(),
            Self::B(b) => b.next(),
        }
    }
}

/// Obtain an executable path.
///
/// This will insert a `.exe` on Windows.
#[cfg(unix)]
#[macro_export]
macro_rules! exe_path {
    ($expr:expr) => {
        $expr
    };
}
/// Obtain an executable path.
///
/// This will insert a `.exe` on Windows.
#[cfg(windows)]
#[macro_export]
macro_rules! exe_path {
    ($expr:expr) => {
        concat!($expr, ".exe")
    };
}

#[cfg(unix)]
pub fn is_executable(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    use std::ffi::{c_char, c_int};
    unsafe extern "C" {
        fn access(pathname: *const c_char, mode: c_int) -> c_int;
    }
    metadata(path).is_ok_and(|x| {
        x.is_file()
            && unsafe {
                let path = path.as_os_str();
                let mut npath = vec![0 as c_char; path.len() + 1];
                npath[0..path.len()].copy_from_slice(std::slice::from_raw_parts(
                    path.as_encoded_bytes().as_ptr() as *const _ as *const c_char,
                    path.as_encoded_bytes().len(),
                ));
                access(npath.as_ptr(), 1) == 0
            }
    })
}
/// Finds an executable in the system PATH.
///
/// # Arguments
/// * `cmd` - Command name to find
///
/// # Returns
/// Full path to the executable if found, None otherwise
#[cfg(unix)]
pub fn find_executable(cmd: impl AsRef<std::ffi::OsStr>) -> Option<PathBuf> {
    let path = std::env::var("PATH").unwrap();
    let path = interject(path.split(':').map(|x| x.to_string()), |x, y| {
        if x.chars().rev().take_while(|x| x == &'\\').count() % 2 == 1 {
            let x = &x[0..x.len() - 1];
            (Some(format!("{x}:{y}")), None)
        } else {
            (Some(y), Some(x))
        }
    });

    for path in path.map(|x| PathBuf::from(x.as_str()).join(cmd.as_ref())) {
        if is_executable(&path) {
            return Some(path);
        }
    }

    None
}

/// Writes data to a file only if it differs from existing content.
///
/// This prevents unnecessary file modifications and timestamp updates.
///
/// # Arguments
/// * `path` - Path to the file
/// * `data` - Data to write
///
/// # Returns
/// io::Result indicating success or failure
pub fn write_if_diff<P: AsRef<Path>, S: AsRef<[u8]>>(path: P, data: S) -> io::Result<()> {
    let path = path.as_ref();
    let mut data = data.as_ref();

    if match File::open(path) {
        Ok(mut file) => 'a: loop {
            let mut buf = [0; 8192];
            match file.read(&mut buf) {
                Ok(0) => break buf.is_empty(),
                Err(_) => break false,
                Ok(l) => {
                    if l >= data.len() {
                        break 'a false;
                    }
                    if &buf[0..l] != &data[0..l] {
                        break 'a false;
                    }
                    data = &data[l..];
                }
            }
        },
        Err(_) => false,
    } {
        return Ok((()));
    }

    fs::write(path, data)
}

/// Downloads a file from a URL with progress display.
///
/// Displays a progress bar when stderr is a terminal.
///
/// # Arguments
/// * `url` - URL to download from
/// * `path` - Local path to save the file
pub fn download(url: &str, path: impl AsRef<Path>) {
    let path = path.as_ref();

    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let mut resp = ureq::get(url).call().unwrap();
    let max_len: usize = if stderr().is_terminal() {
        resp.headers()
            .get("content-length")
            .map(|x| x.to_str().unwrap().parse().unwrap())
            .unwrap()
    } else {
        0
    };
    let mut buf = [0; 16384];
    let mut body = resp.body_mut().as_reader();
    let mut file = File::create(path).unwrap();

    if stderr().is_terminal() {
        eprint!("[          ] 0% (0/{max_len})");
        stderr().flush().unwrap();
    }

    let mut total_len = 0usize;
    loop {
        let len = body.read(&mut buf).unwrap();
        if len == 0 {
            break;
        }
        file.write_all(&buf[0..len]).unwrap();
        if stderr().is_terminal() {
            total_len += len;
            let perc = total_len.mul(100) / max_len;
            eprint!(
                "\r\x1b[K[{}{}] {perc}% ({:.02}/{:.02}MiB)",
                "#".repeat(perc.div(10)),
                " ".repeat(10 - perc.div(10)),
                total_len as f32 / 1024.0 / 1024.0,
                max_len as f32 / 1024.0 / 1024.0,
            );
            stderr().flush().unwrap();
        }
    }

    if stderr().is_terminal() {
        println!(
            "\r\x1b[K[##########] 100% ({:.02}MiB)",
            max_len as f32 / 1024.0 / 1024.0
        );
    }

    file.flush().unwrap();
}

/// Extracts a gzip-compressed tar archive.
///
/// # Arguments
/// * `archive` - Path to the tar.gz archive
/// * `path` - Destination directory
/// * `skip_segments` - Number of path segments to skip when extracting
#[cfg(unix)]
pub fn untar_gz(archive: impl AsRef<Path>, path: impl AsRef<Path>, skip_segments: usize) {
    use std::{io::BufReader, os::unix::fs::PermissionsExt};

    let archive = archive.as_ref();
    let untar_path = path.as_ref();

    let mut buf = [0; 16384];
    let file = BufReader::new(File::open(archive).unwrap());
    let file = flate2::bufread::GzDecoder::new(file);
    let mut file = tar::Archive::new(file);
    for x in file.entries().unwrap() {
        let mut x = x.unwrap();
        let path = x.path_bytes();
        let mut path = str::from_utf8(path.as_ref()).unwrap();
        if path.ends_with('/') {
            fs::create_dir_all(untar_path.join(path)).unwrap();
            continue;
        }
        for _ in 0..skip_segments {
            let Some(i) = path.find('/') else {
                continue;
            };
            path = &path[i + 1..];
        }
        let path = untar_path.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(&path).unwrap();
        loop {
            let len = x.read(&mut buf).unwrap();
            if len == 0 {
                break;
            }
            file.write_all(&buf[0..len]).unwrap();
        }
        file.flush().unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        if let Ok(x) = x.header().mode() {
            perms.set_mode(x);
        }
        file.set_permissions(perms).unwrap();
    }
}

/// Extracts an xz-compressed tar archive.
///
/// # Arguments
/// * `archive` - Path to the tar.xz archive
/// * `path` - Destination directory
/// * `skip_segments` - Number of path segments to skip when extracting
#[cfg(unix)]
pub fn untar_xz(archive: impl AsRef<Path>, path: impl AsRef<Path>, skip_segments: usize) {
    use std::{io::BufReader, os::unix::fs::PermissionsExt};

    let archive = archive.as_ref();
    let untar_path = path.as_ref();

    let mut buf = [0; 16384];
    let file = BufReader::new(File::open(archive).unwrap());
    let file = xz::bufread::XzDecoder::new(file);
    let mut file = tar::Archive::new(file);
    for x in file.entries().unwrap() {
        let mut x = x.unwrap();
        let path = x.path_bytes();
        let mut path = str::from_utf8(path.as_ref()).unwrap();
        if path.ends_with('/') {
            fs::create_dir_all(untar_path.join(path)).unwrap();
            continue;
        }
        for _ in 0..skip_segments {
            let Some(i) = path.find('/') else {
                continue;
            };
            path = &path[i + 1..];
        }
        let path = untar_path.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(&path).unwrap();
        loop {
            let len = x.read(&mut buf).unwrap();
            if len == 0 {
                break;
            }
            file.write_all(&buf[0..len]).unwrap();
        }
        file.flush().unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        if let Ok(x) = x.header().mode() {
            perms.set_mode(x);
        }
        file.set_permissions(perms).unwrap();
    }
}

/// Creates a symbolic link to a file.
///
/// # Arguments
/// * `source` - Path to the source file
/// * `dest` - Path for the symbolic link
///
/// # Returns
/// io::Result indicating success or failure
#[cfg(unix)]
pub fn symlink_file(source: impl AsRef<Path>, dest: impl AsRef<Path>) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, dest)
}

#[cfg(unix)]
pub fn symlink_dir(source: impl AsRef<Path>, dest: impl AsRef<Path>) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, dest)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopChildError;

pub trait PathBufExt {
    fn pop_child(&mut self) -> Result<(), PopChildError>;
}
impl PathBufExt for PathBuf {
    fn pop_child(&mut self) -> Result<(), PopChildError> {
        let len = match self.parent() {
            Some(x) => x,
            None => return Err(PopChildError),
        }
        .as_os_str()
        .len();

        unsafe {
            let buf: &mut Vec<u8> = transmute(self.as_mut_os_string());
            buf.set_len(len);
        }

        Ok(())
    }
}

// TODO: Implement for Windows
