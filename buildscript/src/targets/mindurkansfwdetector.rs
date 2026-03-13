use std::{
    fs::{self, File, read_dir},
    path::PathBuf,
    process::Command,
};

use serde::Serialize;
use tera::{Context, Tera};

use crate::{
    targets::{Target, TargetImpl, TargetImplStatic},
    util::current_dir,
};

#[derive(Serialize)]
pub struct NsfwDetectorTemplateParams {
    shared_config_path: PathBuf,
}

pub struct Impl {
    #[allow(unused)]
    repo: PathBuf,
    #[allow(unused)]
    path: PathBuf,
    command: Option<Command>,
}

impl Impl {
    fn new(path: PathBuf) -> Self {
        Self {
            repo: path,
            path: current_dir().join(".bin/Newtd.jar"),
            command: None,
        }
    }
}

impl TargetImpl for Impl {
    fn build(&mut self, _deps: super::Targets<'_>, params: &mut super::BuildParams) {
        if !params
            .cargo()
            .args(["build", "--release", "-p", "mindurka-nsfw-detector"])
            .status()
            .unwrap()
            .success()
        {
            panic!("Building mindurka-nsfw-detector failed")
        }
    }

    fn run_init(&mut self, _deps: super::Targets<'_>, params: &mut super::RunParams) {
        let root = params.root.join(".run/mindurka-nsfw-detector");
        fs::create_dir_all(&root).unwrap();

        let config = root.join("config.toml");
        fs::File::create(&config).expect("Failed to create config file");
        if let Some(template) = params.templates.get("mindurka-nsfw-detector") {
            let mut tera = Tera::default();
            tera.add_template_files([(template, Some("mindurka-nsfw-detector"))])
                .expect("Failed to add template to tera");

            let context = Context::from_serialize(NsfwDetectorTemplateParams {
                shared_config_path: params.root.join(".run/sharedConfig.toml"),
            })
            .unwrap();

            let config_content = tera
                .render("mindurka-nsfw-detector", &context)
                .expect("Failed to render template");
            fs::write(&config, config_content).expect("Failed to write rendered config");
        } else {
            fs::write(
                &config,
                format!(
                    r#"
                shared_config_path = {:?}
                "#,
                    params.root.join(".run/sharedConfig.toml")
                ),
            )
            .unwrap();
        }

        let mut cmd = params.cargo();
        cmd.current_dir(root)
            .args([
                "run",
                "--release",
                "-p",
                "mindurka-nsfw-detector",
                "--",
                "-c",
                config.to_str().unwrap(),
            ])
            .env("RUST_LOG", "info");
        self.command = Some(cmd);
    }

    fn run(&mut self, deps: super::Targets<'_>, params: &mut super::RunParams) {
        deps.mprocs.as_ref().unwrap().spawn_task(
            &params,
            &mut self.command.as_mut().unwrap(),
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
        params: &mut super::InitParams,
    ) -> Option<Self> {
        if read_dir("mindurka-nsfw-detector").is_err() {
            return None;
        }

        params
            .rust_workspace_members
            .push("mindurka-nsfw-detector".into());
        Some(Self::new(
            fs::canonicalize("mindurka-nsfw-detector").unwrap(),
        ))
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        params: &mut super::InitParams,
    ) -> Self {
        if !Command::new("git")
            .arg("clone")
            .arg(
                params
                    .git_backend
                    .repo_url("Darkdustry-Coders/MindurkaNsfwDetector"),
            )
            .arg(params.root.join("mindurka-nsfw-detector"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        Self::new(fs::canonicalize("mindurka-nsfw-detector").unwrap())
    }
}
