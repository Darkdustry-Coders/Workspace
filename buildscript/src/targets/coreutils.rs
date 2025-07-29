use std::{
    fs::{self, read_dir},
    path::{Path, PathBuf},
    process::Command,
};

use crate::util::{self, download, find_executable, is_executable, untar_gz};

use super::{TargetFlags, TargetImpl, TargetImplStatic};

const LINUX_BIN: &str = "https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox";

pub struct Impl(PathBuf);
impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, params: &mut super::BuildParams) {
        params.path.push(self.0.clone());
    }
}
impl TargetImplStatic for Impl {
    fn flags() -> TargetFlags {
        TargetFlags {
            always_local: false,
            ..Default::default()
        }
    }

    fn initialize_host(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        if cfg!(unix) {
            let path = find_executable("xargs")?;
            let path = path.parent()?;
            // Surely that's enough
            for x in ["uname", "yes", "[", "cat", "touch"] {
                if !is_executable(path.join(x)) {
                    return None;
                }
            }
            Some(Self(path.to_path_buf()))
        } else {
            unimplemented!()
        }
    }

    fn initialize_cached(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        if read_dir(".cache/tools/coreutils").is_ok() {
            Some(Self(fs::canonicalize(".cache/tools/coreutils").unwrap()))
        } else {
            None
        }
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Self {
        eprintln!("Downloading coreutils...");
        fs::create_dir_all(".cache/tools/coreutils").unwrap();
        let path = fs::canonicalize(".cache/tools/coreutils").unwrap();
        download(LINUX_BIN, ".cache/tools/coreutils/busybox");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(".cache/tools/coreutils/busybox").unwrap();
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(".cache/tools/coreutils/busybox", permissions).unwrap();
        }

        let commands = String::from_utf8(
            Command::new(".cache/tools/coreutils/busybox")
                .env("LANG", "C")
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        let commands = commands
            .lines()
            .map(|x| x.trim())
            .skip_while(|x| !x.starts_with("Currently defined functions"))
            .skip(1)
            .flat_map(|x| x.split(',').map(str::trim).filter(|x| !x.is_empty()));

        let coreutils = path.join("busybox");
        for x in commands {
            util::symlink_file(&coreutils, Path::new(".cache/tools/coreutils").join(x)).unwrap();
        }

        Self(fs::canonicalize(".cache/tools/coreutils").unwrap())
    }
}
