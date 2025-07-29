#![allow(unused)]

use std::{
    fs::{self, File, metadata},
    io::{IsTerminal, Read, Write, stderr},
    ops::{Div, Mul},
    path::{Path, PathBuf},
    process::Command,
};

pub static mut CURRENT_DIR: Option<PathBuf> = None;
pub fn current_dir() -> &'static Path {
    unsafe {
        (&raw const CURRENT_DIR)
            .as_ref()
            .unwrap_unchecked()
            .as_ref()
            .unwrap_unchecked()
    }
}

pub fn t<T>(t: T) -> T {
    t
}

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
pub fn interject<T, I, F>(iter: I, fun: F) -> Interject<T, I, F>
where
    I: Iterator<Item = T>,
    F: FnMut(T, T) -> (Option<T>, Option<T>),
{
    Interject(iter, fun, None)
}

pub enum EitherIter<T, A, B>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    A(A),
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

#[cfg(unix)]
pub fn symlink_file(source: impl AsRef<Path>, dest: impl AsRef<Path>) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, dest)
}
// TODO: Implement for Windows

///
#[cfg(unix)]
pub fn gradle() -> PathBuf {
    use std::env::current_dir;

    current_dir().unwrap().join("gradlew")
}
#[cfg(windows)]
pub fn gradle() -> PathBuf {
    use std::env::current_dir;

    current_dir().unwrap().join("gradlew.bat")
}
