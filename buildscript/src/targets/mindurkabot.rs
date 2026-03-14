use std::{
    fs::{self, read_dir},
    path::PathBuf,
    process::Command,
};

use serde::Serialize;
use tera::{Context, Tera};

use crate::{
    exe_path,
    targets::{Target, TargetImpl, TargetImplStatic},
    util::current_dir,
};

#[derive(Serialize)]
pub struct BotTemplateParams {
    shared_config_path: PathBuf,
}

pub struct Impl {
    #[allow(unused)]
    repo: PathBuf,
    #[allow(unused)]
    path: PathBuf,
}

impl Impl {
    fn new(path: PathBuf) -> Self {
        Self {
            repo: path,
            path: current_dir().join(exe_path!(".bin/mindurka-bot")),
        }
    }
}

impl TargetImpl for Impl {
    fn build(&mut self, _deps: super::Targets<'_>, params: &mut super::BuildParams) {
        // TODO: Add --release flag for building in release mode.
        if !params
            .cargo()
            .args(["build", "--release", "-p", "mindurka-bot"])
            .status()
            .unwrap()
            .success()
        {
            panic!("Building mindurka-bot failed")
        }

        fs::rename(
            exe_path!(".cache/rust/release/mindurka-bot"),
            exe_path!(".bin/mindurka-bot"),
        )
        .expect("failed to move bot binary");
    }

    fn run_init(&mut self, _deps: super::Targets<'_>, params: &mut super::RunParams) {
        if let Some(template) = params.templates.get("mindurka-bot") {
            let mut tera = Tera::default();
            tera.add_template_files([(template, Some("mindurka-bot"))])
                .expect("Failed to add template to tera");

            let context = Context::from_serialize(BotTemplateParams {
                shared_config_path: params.root.join(".run/sharedConfig.toml"),
            })
            .unwrap();

            let config_content = tera
                .render("mindurka-bot", &context)
                .expect("Failed to render template");
            params.run.write("mindurka-bot/config.toml", config_content);
        } else {
            params.run.write(
                "mindurka-bot/config.toml",
                format!(
                    r#"
                    shared_config_path = {:?}
                    "#,
                    params.root.join("mindurka-bot/config.toml")
                ),
            );
        }
    }

    fn run(&mut self, deps: super::Targets<'_>, params: &mut super::RunParams) {
        let mut cmd = params.cmd(&self.path);
        deps.mprocs.as_ref().unwrap().spawn_task(
            &params,
            cmd.current_dir(".run/mindurka-bot")
                .arg("-c")
                .arg("config.toml"),
            "mindurka-bot",
        );
    }
}

impl TargetImplStatic for Impl {
    fn depends(list: &mut super::TargetList) {
        list.set_depend(Target::RabbitMq);
        list.set_depend(Target::SurrealDb);
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
        if read_dir("mindurka-bot").is_err() {
            return None;
        }

        Some(Self::new(fs::canonicalize("mindurka-bot").unwrap()))
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        params: &mut super::InitParams,
    ) -> Self {
        if !Command::new("git")
            .arg("clone")
            .arg(params.git_backend.repo_url("Darkdustry-Coders/MindurkaBot"))
            .arg(params.root.join("mindurka-bot"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        Self::new(fs::canonicalize("mindurka-bot").unwrap())
    }

    fn postinit(_: super::TargetEnabled, _: super::Targets<'_>, params: &mut super::InitParams) {
        if fs::read_dir("mindurka-bot").is_ok() {
            params.rust_workspace_members.push("mindurka-bot".into());
        }
    }
}
