use std::{
    fs::{self, read_dir},
    path::PathBuf,
    process::Command,
};

use crate::{
    exe_path,
    targets::{Target, TargetImpl, TargetImplStatic},
    util::current_dir,
};

pub struct Impl {
    #[allow(unused)]
    repo: PathBuf,
    #[allow(unused)]
    path: PathBuf,
    port: u16,
}

impl Impl {
    fn new(path: PathBuf) -> Self {
        Self {
            repo: path,
            path: current_dir().join(exe_path!(".bin/web")),
            port: 0,
        }
    }
}

impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, params: &mut super::BuildParams) {
        // TODO: Add --release flag for building in release mode.
        if !params
            .cargo()
            .args(["build", "--release", "-p", "web"])
            .status()
            .unwrap()
            .success()
        {
            panic!("Building mindurka-bot failed")
        }

        fs::rename(
            exe_path!(".cache/rust/release/web"),
            exe_path!(".bin/web"),
        )
        .expect("failed to move bot binary");
    }

    fn run_init(&mut self, _: super::Targets<'_>, params: &mut super::RunParams) {
        self.port = params.next_port();
        params.run.write("web/.keep", "");
    }

    fn run(&mut self, deps: super::Targets<'_>, params: &mut super::RunParams) {
        let mut cmd = params.cmd(&self.path);
        deps.mprocs.as_ref().unwrap().spawn_task(
            &params,
            cmd.current_dir(".run/web")
                .arg("--shared")
                .arg(params.root.join(".run/sharedConfig.toml"))
                .arg("--listen").arg(format!("0.0.0.0:{}", self.port)),
            "web",
        );
    }
}

impl TargetImplStatic for Impl {
    fn depends(list: &mut super::TargetList) {
        list.set_depend(Target::SurrealDb);
        list.set_depend(Target::MindurkaRabbitMqRust);
    }

    fn initialize_host(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        unimplemented!()
    }

    fn initialize_cached(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        if read_dir("web").is_err() {
            return None;
        }

        Some(Self::new(fs::canonicalize("web").unwrap()))
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        params: &mut super::InitParams,
    ) -> Self {
        if !Command::new("git")
            .arg("clone")
            .arg(params.git_backend.repo_url("Darkdustry-Coders/Website"))
            .arg(params.root.join("web"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        Self::new(fs::canonicalize("web").unwrap())
    }

    fn postinit(_: super::TargetEnabled, _: super::Targets<'_>, params: &mut super::InitParams) {
        if fs::read_dir("web").is_ok() {
            params.rust_workspace_members.push("web".into());
        }
    }
}
