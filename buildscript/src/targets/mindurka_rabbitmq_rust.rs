use std::{
    fs::{self, read_dir},
    process::Command,
};

use crate::targets::{Target, TargetEnabled, TargetImpl, TargetImplStatic};

pub struct Impl {
    build: bool,
}

impl Impl {
    fn new(build: bool) -> Self {
        Self { build }
    }
}

impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, params: &mut super::BuildParams) {
        if self.build {
            // TODO: Add --release flag for building in release mode.
            if !params
                .cargo()
                .args(["build", "--release", "-p", "mindurka-rabbitmq-rust"])
                .status()
                .unwrap()
                .success()
            {
                panic!("Building mindurka-rabbitmq-rust failed")
            }
        }
    }

    fn run_init(&mut self, _: super::Targets<'_>, _: &mut super::RunParams) {}
    fn run(&mut self, _: super::Targets<'_>, _: &mut super::RunParams) {}
}

impl TargetImplStatic for Impl {
    fn depends(list: &mut super::TargetList) {
        list.set_depend(Target::RabbitMq);
    }

    fn initialize_host(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        unimplemented!()
    }

    fn initialize_cached(
        enabled: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        if read_dir("mindurka-rabbitmq-rust").is_err() {
            return None;
        }

        Some(Self::new(enabled == TargetEnabled::Build))
    }

    fn initialize_local(
        enabled: super::TargetEnabled,
        _: super::Targets<'_>,
        params: &mut super::InitParams,
    ) -> Self {
        if !Command::new("git")
            .arg("clone")
            .arg(
                params
                    .git_backend
                    .repo_url("Darkdustry-Coders/mindurka-rabbitmq-rust"),
            )
            .arg(params.root.join("mindurka-rabbitmq-rust"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        Self::new(enabled == TargetEnabled::Build)
    }

    fn postinit(_: super::TargetEnabled, _: super::Targets<'_>, params: &mut super::InitParams) {
        if fs::read_dir("mindurka-rabbitmq-rust").is_ok() {
            params
                .rust_workspace_members
                .push("mindurka-rabbitmq-rust".into());
        }
    }
}
