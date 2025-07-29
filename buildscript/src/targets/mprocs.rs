use std::{
    fs,
    num::NonZeroU16,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread::sleep,
    time::Duration,
};

use crate::util::{download, find_executable, is_executable, untar_gz};

use super::{RunParams, TargetEnabled, TargetFlags, TargetImpl, TargetImplStatic, Targets};

static BASE_URL: &str = "https://github.com/pvolok/mprocs/releases/download/v0.7.3/mprocs-0.7.3";

pub struct Impl {
    mprocs: PathBuf,
    port: u16,
    process: Option<Child>,
}
impl Impl {
    pub fn port(&self) -> Option<NonZeroU16> {
        NonZeroU16::new(self.port)
    }

    pub fn spawn_task(&self, _: &RunParams, command: &mut Command, name: &str) {
        let mut cmd = String::new();
        if let Some(x) = command.get_current_dir() {
            cmd.push_str(&format!("cd {x:?} && "));
        }
        for (k, v) in command.get_envs() {
            let Some(v) = v else {
                continue;
            };
            let k = k.to_str().unwrap();
            cmd.push_str(&format!("{k}={v:?} "));
        }

        cmd.push_str(&format!("{:?}", command.get_program().to_str().unwrap()));

        for x in command.get_args() {
            cmd.push_str(&format!(" {x:?}"));
        }

        if !Command::new(&self.mprocs)
            .arg("--server")
            .arg(format!("127.0.0.1:{}", self.port))
            .arg("--ctl")
            .arg(format!("{{c: add-proc, cmd: {cmd:?}, name: {name:?}}}"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed starting a task");
        }
    }

    pub fn wait(&mut self) -> bool {
        if let Some(mut x) = self.process.take() {
            if !x.wait().unwrap().success() {
                return false;
            }
        }
        true
    }
}
impl TargetImpl for Impl {
    fn build(&mut self, _: Targets<'_>, _: &mut super::BuildParams) {
        // STUB: This target is not compiled from source.
    }

    fn run_init(&mut self, _: Targets<'_>, _: &mut super::RunParams) {
        // TODO: Windows
    }

    fn run(&mut self, _: Targets<'_>, params: &mut super::RunParams) {
        let port = params.next_port();
        self.process = Some(
            params
                .cmd(&self.mprocs)
                .arg("--server")
                .arg(format!("127.0.0.1:{port}"))
                .spawn()
                .unwrap(),
        );
        self.port = port;
        sleep(Duration::from_millis(10));
    }
}
impl TargetImplStatic for Impl {
    fn flags() -> TargetFlags {
        TargetFlags {
            always_local: true,
            ..Default::default()
        }
    }

    fn depends(_: &mut super::TargetList) {}

    fn initialize_host(
        _: TargetEnabled,
        _: Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        find_executable("mprocs")
            .map(|x| fs::canonicalize(x).unwrap())
            .map(|mprocs| Impl {
                mprocs,
                port: 0,
                process: None,
            })
    }
    fn initialize_cached(
        _: TargetEnabled,
        _: Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        if is_executable(".cache/tools/mprocs/mprocs") {
            Some(Self {
                mprocs: PathBuf::from(".cache/tools/mprocs/mprocs"),
                port: 0,
                process: None,
            })
        } else {
            None
        }
    }
    #[allow(unreachable_code)]
    fn initialize_local(_: TargetEnabled, _: Targets<'_>, _: &mut super::InitParams) -> Self {
        eprintln!("Downloading mprocs");

        let exe = ".cache/tools/mprocs/mprocs";
        let archive = ".cache/tools/mprocs/archive";
        let dir = Path::new(exe).parent().unwrap();
        fs::create_dir_all(dir).unwrap();
        download(
            &(BASE_URL.to_string()
                + if cfg!(unix) {
                    "-linux-x86_64-musl.tar.gz"
                } else {
                    "-windows-x86_64.zip"
                }),
            archive,
        );
        #[cfg(unix)]
        {
            untar_gz(archive, ".cache/tools/mprocs", 1);
        }
        fs::remove_file(archive).ok();
        Self {
            mprocs: PathBuf::from(".cache/tools/mprocs/mprocs"),
            port: 0,
            process: None,
        }
    }
}
