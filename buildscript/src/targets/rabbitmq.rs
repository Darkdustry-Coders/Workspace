use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::util::{download, find_executable, is_executable, untar_xz};

use super::{Target, TargetImpl, TargetImplStatic};

static URL: &str = "https://github.com/rabbitmq/rabbitmq-server/releases/download/v4.1.2/rabbitmq-server-generic-unix-4.1.2.tar.xz";

pub struct Impl {
    rabbitmq_home: PathBuf,
    port: u16,
}
impl Impl {
    fn new(rabbitmq_home: PathBuf) -> Self {
        Self {
            rabbitmq_home,
            port: 0,
        }
    }

    pub fn url(&self) -> String {
        format!("amqp://guest:guest@localhost:{}/%2F", self.port)
    }
}
impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, _: &mut super::BuildParams) {}

    fn run_init(&mut self, _: super::Targets<'_>, params: &mut super::RunParams) {
        self.port = params.next_port();

        let rabbitmq_root = params.root.join(".run/rabbitmq");
        fs::create_dir_all(&rabbitmq_root).unwrap();

        let mut config = BufWriter::new(File::create(rabbitmq_root.join("rabbitmq.conf")).unwrap());
        config
            .write_all(format!("listeners.tcp.default = 127.0.0.1:{}\n", self.port).as_bytes())
            .unwrap();
        config.flush().unwrap();
    }

    fn run(&mut self, mut deps: super::Targets<'_>, params: &mut super::RunParams) {
        let rabbitmq_root = params.root.join(".run/rabbitmq");

        let mut command = Command::new(self.rabbitmq_home.join("sbin").join("rabbitmq-server"));
        command.arg("start");
        command.env("RABBITMQ_CONFIG_FILE", rabbitmq_root.join("rabbitmq.conf"));
        command.env("RABBITMQ_PID_FILE", rabbitmq_root.join("rabbitmq.pid"));
        command.env("RABBITMQ_LOG_BASE", rabbitmq_root.join("log"));
        command.env("RABBITMQ_MNESIA_BASE", rabbitmq_root.join("db"));
        command.env("RABBITMQ_MNESIA_DIR", rabbitmq_root.join("db"));

        deps.mprocs
            .as_mut()
            .unwrap()
            .spawn_task(params, &mut command, "rabbitmq");
    }
}
impl TargetImplStatic for Impl {
    fn depends(list: &mut super::TargetList) {
        list.set_depend(Target::CoreUtils);
    }

    fn flags() -> super::TargetFlags {
        super::TargetFlags {
            always_local: false,
            ..Default::default()
        }
    }

    fn initialize_host(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        find_executable("rabbitmq-server")
            .map(|x| fs::canonicalize(x).unwrap())
            .map(|x| x.parent().unwrap().parent().unwrap().to_path_buf())
            .map(Impl::new)
    }

    fn initialize_cached(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        if is_executable(".cache/tools/rabbitmq/sbin/rabbitmq-server") {
            Some(Self::new(
                fs::canonicalize(".cache/tools/rabbitmq").unwrap(),
            ))
        } else {
            None
        }
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Self {
        let archive = ".cache/tools/rabbitmq/archive.tar.xz";
        let dir = Path::new(archive).parent().unwrap();
        fs::create_dir_all(dir).unwrap();
        download(URL, archive);
        untar_xz(archive, dir, 1);
        fs::remove_file(archive).ok();

        Self::new(fs::canonicalize(dir).unwrap())
    }
}
