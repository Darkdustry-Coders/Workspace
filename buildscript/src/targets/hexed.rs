//! Hexed plugin target.
//!
//! This module manages the Hexed game mode plugin for Mindustry.
//! Hexed is a strategic hex-based territory control game mode.
//! Repository: https://github.com/Darkdustry-Coders/HexedPlugin

use std::{
    fs::{self, read_dir},
    path::PathBuf,
    process::Command,
};

use crate::util::current_dir;

use super::{Target, TargetImpl, TargetImplStatic};

// TODO: Download if enabled status is `Depend` instead of `Build`.

/// Hexed plugin target implementation.
pub struct Impl {
    /// Path to the plugin repository.
    #[allow(unused)]
    repo: PathBuf,
    /// Path to the built JAR file.
    #[allow(unused)]
    path: PathBuf,
    /// Command to run the server.
    command: Option<Command>,
}

impl Impl {
    /// Creates a new Hexed target instance.
    ///
    /// # Arguments
    /// * `path` - Path to the repository
    fn new(path: PathBuf) -> Self {
        Self {
            repo: path,
            path: current_dir().join(".bin/Hexed.jar"),
            command: None,
        }
    }
}

impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, params: &mut super::BuildParams) {
        // On Hexed side it should copy resulting jar into `.bin/Hexed.jar`.
        if !params
            .gradle()
            .arg(":hexed:build")
            .status()
            .unwrap()
            .success()
        {
            panic!("building Hexed failed");
        }
    }

    fn run_init(&mut self, deps: super::Targets<'_>, params: &mut super::RunParams) {
        let root = params.root.join(".run/hexed");

        params.run.link_global(
            params.root.join(".bin/CorePlugin.jar"),
            "hexed/config/mods/CorePlugin.jar",
        );
        params.run.link_global(
            params.root.join(".bin/Hexed.jar"),
            "hexed/config/mods/Hexed.jar",
        );

        params.run.link_global(
            params.root.join("hexed/assets/patch.hjson"),
            "hexed/config/patches/patch.hjson",
        );

        params.run.write(
            "hexed/config/corePlugin.toml",
            format!(
                r#"
                serverName = "hexed-pvp"
                gamemode = "hexed"
                sharedConfigPath = {:?}
                "#,
                params.root.join(".run/sharedConfig.toml")
            ),
        );

        let port = params.next_port();

        {
            let mut contents = vec![];
            contents.extend_from_slice(&3i32.to_be_bytes());

            let option = "servername";
            contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
            contents.extend_from_slice(option.as_bytes());

            let name = "[scarlet]Workspace [accent]| [white]Hexed";
            contents.push(4);
            contents.extend_from_slice(&(name.len() as u16).to_be_bytes());
            contents.extend_from_slice(name.as_bytes());

            let option = "port";
            contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
            contents.extend_from_slice(option.as_bytes());

            contents.push(1);
            contents.extend_from_slice(&(port as i32).to_be_bytes());

            let option = "startCommands";
            contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
            contents.extend_from_slice(option.as_bytes());

            let commands = "host";
            contents.push(4);
            contents.extend_from_slice(&(commands.len() as u16).to_be_bytes());
            contents.extend_from_slice(commands.as_bytes());

            params.run.write("hexed/config/settings.bin", contents);
        }

        let java = deps.java.as_ref().unwrap().home().join("bin/java");
        let mindustry = deps.mindustry.as_ref().unwrap().path();

        let mut cmd = params.cmd(java);
        cmd.arg("-jar").arg(mindustry).current_dir(root);
        self.command = Some(cmd);
    }

    fn run(&mut self, deps: super::Targets<'_>, params: &mut super::RunParams) {
        deps.mprocs.as_ref().unwrap().spawn_task(
            params,
            &mut self.command.take().unwrap(),
            "hexed",
        );
    }
}

impl TargetImplStatic for Impl {
    fn depends(list: &mut super::TargetList) {
        list.set_depend(Target::Java);
        list.set_depend(Target::CorePlugin);
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
        if read_dir("hexed").is_err() {
            return None;
        }

        Some(Self::new(fs::canonicalize("hexed").unwrap()))
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        params: &mut super::InitParams,
    ) -> Self {
        if !Command::new("git")
            .arg("clone")
            .arg(params.git_backend.repo_url("Darkdustry-Coders/Hexed-V8"))
            .arg(params.root.join("hexed"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        Self::new(fs::canonicalize("hexed").unwrap())
    }

    fn postinit(_: super::TargetEnabled, _: super::Targets<'_>, params: &mut super::InitParams) {
        if fs::read_dir("hexed").is_ok() {
            params.java_workspace_members.push("hexed".into());
        }
    }
}
