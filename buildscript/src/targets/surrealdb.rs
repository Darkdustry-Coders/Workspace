//! SurrealDB database.
//!
//! This module manages SurrealDB installation.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    exe_path,
    util::{download, find_executable, is_executable},
};

use super::{RunParams, TargetEnabled, TargetFlags, TargetImpl, TargetImplStatic, Targets};

/// URL for SurrealDB binary release.
static URL: &str = if cfg!(target_os = "linux") {
    "https://github.com/surrealdb/surrealdb/releases/download/v3.0.1/surreal-v3.0.1.linux-amd64.tgz"
} else {
    "https://github.com/surrealdb/surrealdb/releases/download/v3.0.1/surreal-v3.0.1.windows-amd64.exe"
};

/// SurrealDB target implementation.
pub struct Impl {
    /// Path to SurrealDB binary directory.
    surreal: PathBuf,
    /// Server port number.
    port: u16,
}

impl Impl {
    /// Creates a new SurrealDB instance.
    ///
    /// # Arguments
    /// * `surreal` - Path to SurrealDB directory
    fn new(surreal: PathBuf) -> Self {
        Self { surreal, port: 0 }
    }

    /// Returns the WebSocket connection URL.
    pub fn url(&self) -> String {
        format!("ws://admin:password@localhost:{}/main/mindustry", self.port)
    }
}

impl TargetImpl for Impl {
    fn build(&mut self, _: Targets<'_>, _: &mut super::BuildParams) {
        // STUB: This target is not compiled from source.
    }

    fn run_init(&mut self, _: Targets<'_>, params: &mut RunParams) {
        if params.host_surrealdb {
            return;
        }

        self.port = params.next_port();
        params.env.insert("SURREAL_USER".into(), "admin".into());
        params.env.insert("SURREAL_PASS".into(), "password".into());
        params.env.insert(
            "SURREAL_BIND".into(),
            format!("127.0.0.1:{}", self.port).into(),
        );
    }

    fn run(&mut self, mut deps: Targets<'_>, params: &mut RunParams) {
        if params.host_surrealdb {
            return;
        }

        deps.mprocs.as_mut().unwrap().spawn_task(
            params,
            Command::new(self.surreal.join(exe_path!("surreal")))
                .arg("start")
                .arg("--import-file")
                .arg(fs::canonicalize("sql/init.surrealql").unwrap())
                .arg(format!(
                    "surrealkv://{}",
                    params.root.join(".run/surrealdb").to_str().unwrap()
                )),
            "surreal",
        );
    }
}

impl TargetImplStatic for Impl {
    fn flags() -> TargetFlags {
        TargetFlags {
            always_local: true,
            ..Default::default()
        }
    }

    fn initialize_host(
        _: TargetEnabled,
        _: Targets<'_>,
        params: &mut super::InitParams,
    ) -> Option<Self> {
        if params.host_surrealdb {
            return Some(Impl {
                surreal: PathBuf::new(),
                port: 0,
            });
        }

        let surreal = find_executable("surreal").map(|x| x.parent().unwrap().to_path_buf());
        surreal.map(|surreal| Impl { surreal, port: 0 })
    }

    fn initialize_cached(
        _: TargetEnabled,
        _: Targets<'_>,
        params: &mut super::InitParams,
    ) -> Option<Self> {
        if params.host_surrealdb {
            return Some(Impl {
                surreal: PathBuf::new(),
                port: 0,
            });
        }

        if is_executable(exe_path!(".cache/tools/surrealdb/surreal")) {
            Some(Self {
                surreal: fs::canonicalize(".cache/tools/surrealdb").unwrap(),
                port: 0,
            })
        } else {
            None
        }
    }

    #[allow(unreachable_code)]
    fn initialize_local(_: TargetEnabled, _: Targets<'_>, params: &mut super::InitParams) -> Self {
        if params.host_surrealdb {
            return Impl {
                surreal: PathBuf::new(),
                port: 0,
            };
        }

        #[cfg(target_os = "linux")]
        {
            use crate::util::untar_gz;

            let archive = ".cache/tools/surrealdb/archive.tar.gz";
            let dir = Path::new(archive).parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            download(URL, archive);
            untar_gz(archive, dir, 1);
            fs::remove_file(archive).ok();

            return Self::new(fs::canonicalize(dir).unwrap());
        }

        #[cfg(target_os = "windows")]
        {
            let exe = ".cache/tools/surrealdb/surreal.exe";
            let dir = Path::new(archive).parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            download(URL, archive);

            return Self::new(fs::canonicalize(dir).unwrap());
        }

        todo!();
    }
}
