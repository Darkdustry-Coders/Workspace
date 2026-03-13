use std::{
    borrow::Cow,
    collections::HashMap,
    fs::{self, ReadDir},
    io,
    path::PathBuf,
};

use crate::util::{self, PathBufExt};

pub struct SyncFs {
    root: PathBuf,
    keep_paths: Vec<PathBuf>,
    restore_paths: HashMap<PathBuf, PathBuf>,
    modified: HashMap<PathBuf, Vec<u8>>,
    links: HashMap<PathBuf, PathBuf>,
}
impl SyncFs {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            keep_paths: vec![],
            restore_paths: HashMap::new(),
            modified: HashMap::new(),
            links: HashMap::new(),
        }
    }

    pub fn keep_path(&mut self, path: PathBuf) {
        self.keep_paths.push(path);
    }

    pub fn restore<S: Into<PathBuf>, D: Into<PathBuf>>(&mut self, original: S, dest: D) {
        self.restore_paths.insert(dest.into(), original.into());
    }

    pub fn write<P: Into<PathBuf>, B: Into<Vec<u8>>>(&mut self, path: P, data: B) {
        let path = path.into();
        self.links.remove(&path);
        self.modified.insert(path, data.into());
    }

    /// Link `dest` to point to `original`.
    pub fn link_global<S: Into<PathBuf>, D: Into<PathBuf>>(&mut self, original: S, dest: D) {
        let source = original.into();
        let dest = dest.into();

        self.modified.remove(&dest);
        self.links.insert(dest, source);
    }

    pub fn finalize(&self) -> io::Result<()> {
        struct StackFrame {
            pub readdir: ReadDir,
            pub keep: bool,
        }

        'part1: {
            let readdir = match fs::read_dir(&self.root) {
                Ok(x) => x,
                Err(why) if why.kind() == io::ErrorKind::NotFound => break 'part1,
                Err(why) => return Err(why),
            };

            let mut rpath = PathBuf::new();
            let mut stack = vec![StackFrame {
                readdir,
                keep: false,
            }];

            while let Some(frame) = stack.last_mut() {
                let file = match frame.readdir.next() {
                    Some(Ok(x)) => x,
                    Some(Err(why)) => return Err(why),
                    None => {
                        let done_frame = stack.pop().unwrap();
                        if let Some(prev_frame) = stack.last_mut() {
                            if done_frame.keep {
                                prev_frame.keep = true;
                            }
                            assert!(rpath.pop_child().is_ok());
                        }
                        continue;
                    }
                };

                rpath.push(file.file_name());

                if !frame.keep && self.keep_paths.iter().any(|path| rpath.starts_with(path)) {
                    assert!(rpath.pop_child().is_ok());
                    frame.keep = true;
                    continue;
                }

                if !frame.keep && self.modified.keys().any(|path| rpath == path.as_path()) {
                    assert!(rpath.pop_child().is_ok());
                    frame.keep = true;
                    continue;
                }

                if !frame.keep && self.links.contains_key(&rpath) {
                    assert!(rpath.pop_child().is_ok());
                    frame.keep = true;
                    continue;
                }

                if !frame.keep
                    && self.restore_paths.iter().any(|(dest, original)| {
                        rpath.starts_with(dest.as_path())
                            && original
                                .join(rpath.strip_prefix(dest.as_path()).unwrap())
                                .exists()
                    })
                {
                    frame.keep = true;
                }

                let fpath = file.path();
                match fs::read_dir(&fpath) {
                    Ok(readdir) => {
                        stack.push(StackFrame {
                            readdir,
                            keep: false,
                        });
                    }
                    Err(why) if why.kind() == io::ErrorKind::NotADirectory => {
                        fs::remove_file(fpath)?;
                        assert!(rpath.pop_child().is_ok());
                    }
                    Err(why) => return Err(why),
                }
            }
        }

        for (path, value) in &self.modified {
            if self.keep_paths.iter().any(|x| path.starts_with(x)) {
                continue;
            }

            let path = self.root.join(path);
            if fs::read(&path).is_ok_and(|x| x == &**value) {
                continue;
            }

            match fs::write(&path, value) {
                Ok(x) => x,
                Err(why) if why.kind() == io::ErrorKind::NotFound => {
                    fs::create_dir_all(path.parent().unwrap())?;
                    fs::write(&path, value)?;
                }
                Err(why) => return Err(why),
            }
        }

        for (dest, source) in &self.links {
            if self.keep_paths.iter().any(|x| dest.starts_with(x)) {
                continue;
            }

            let dest = self.root.join(dest);

            let realdest = {
                let mut path = Cow::Borrowed(&dest);
                loop {
                    match fs::read_link(path.as_ref()) {
                        Ok(x) => path = Cow::Owned(x),
                        Err(why) if why.kind() == io::ErrorKind::InvalidInput => break path,
                        Err(why) if why.kind() == io::ErrorKind::NotFound => break path,
                        Err(why) => return Err(why),
                    }
                }
            };
            let realsource = {
                let mut path = Cow::Borrowed(source);
                loop {
                    match fs::read_link(path.as_ref()) {
                        Ok(x) => path = Cow::Owned(x),
                        Err(why) if why.kind() == io::ErrorKind::InvalidInput => break path,
                        Err(why) if why.kind() == io::ErrorKind::NotFound => break path,
                        Err(why) => return Err(why),
                    }
                }
            };

            if realdest.as_ref() == realsource.as_ref() {
                continue;
            }

            match fs::metadata(realdest.as_ref()) {
                Ok(dest_meta) => match fs::metadata(realsource.as_ref()) {
                    Ok(source_meta) => {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::MetadataExt;

                            if dest_meta.dev() == source_meta.dev()
                                && dest_meta.ino() == source_meta.ino()
                            {
                                continue;
                            }
                        }
                    }
                    Err(why) if why.kind() == io::ErrorKind::NotFound => (),
                    Err(why) => return Err(why),
                },
                Err(why) if why.kind() == io::ErrorKind::NotFound => (),
                Err(why) => return Err(why),
            }

            match fs::remove_file(&dest) {
                Ok(_) => (),
                Err(why) if why.kind() == io::ErrorKind::NotFound => (),
                Err(why) if why.kind() == io::ErrorKind::IsADirectory => fs::remove_dir_all(&dest)?,
                Err(why) => return Err(why),
            }

            match fs::hard_link(source, &dest) {
                Ok(_) => continue,
                Err(why) if why.kind() == io::ErrorKind::NotFound => {
                    fs::create_dir_all(dest.parent().unwrap())?;
                    match fs::hard_link(source, &dest) {
                        Ok(_) => continue,
                        Err(why) if why.kind() == io::ErrorKind::IsADirectory => (),
                        Err(why) => return Err(why),
                    }
                }
                Err(why) if why.kind() == io::ErrorKind::IsADirectory => (),
                Err(why) => return Err(why),
            }

            match util::symlink_file(source, &dest) {
                Ok(_) => (),
                Err(why) if why.kind() == io::ErrorKind::IsADirectory => {}
                Err(why) => return Err(why),
            }

            match util::symlink_dir(source, &dest) {
                Ok(_) => (),
                Err(why) => return Err(why),
            }
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.modified.clear();
        self.keep_paths.clear();
    }
}
