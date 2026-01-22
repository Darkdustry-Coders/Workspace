use std::{
    fs::{self, read_dir},
    path::PathBuf,
    process::Command,
};

use crate::util::{self, current_dir};

use super::{Target, TargetImpl, TargetImplStatic};

// TODO: Download if enabled status is `Depend` instead of `Build`.

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
            path: current_dir().join(".bin/Forts.jar"),
            command: None,
        }
    }
}
impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, params: &mut super::BuildParams) {
        // On Forts side it should copy resulting jar into `.bin/Forts.jar`.
        if !params
            .gradle()
            .arg(":forts:build")
            .status()
            .unwrap()
            .success()
        {
            panic!("building Forts failed");
        }
    }

    fn run_init(&mut self, deps: super::Targets<'_>, params: &mut super::RunParams) {
        let root = params.root.join(".run/forts");
        fs::create_dir_all(root.join("config/mods")).unwrap();
        fs::create_dir_all(root.join("config/maps")).unwrap();

        util::symlink_file(
            params.root.join(".bin/CorePlugin.jar"),
            root.join("config/mods/CorePlugin.jar"),
        )
        .unwrap();
        util::symlink_file(
            params.root.join(".bin/Forts.jar"),
            root.join("config/mods/Forts.jar"),
        )
        .unwrap();
        fs::copy(
            params.root.join("forts/assets/testmap.msav"),
            root.join("config/maps/testmap.msav"),
        )
        .unwrap();
        fs::write(
            root.join("config/corePlugin.toml"),
            format!(
                "serverName = \"forts\"\ngamemode = \"forts\"\nsharedConfigPath = {:?}",
                params.root.join(".run/sharedConfig.toml")
            ),
        )
        .unwrap();

        let port = params.next_port();

        {
            let mut contents = vec![];
            contents.extend_from_slice(&3i32.to_be_bytes());

            let option = "name";
            contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
            contents.extend_from_slice(option.as_bytes());

            let name = "Template Server";
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

            let commands = "host Forts_v1.5 attack";
            contents.push(4);
            contents.extend_from_slice(&(commands.len() as u16).to_be_bytes());
            contents.extend_from_slice(commands.as_bytes());

            fs::write(root.join("config/settings.bin"), contents).unwrap();
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
            "forts",
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
        params: &mut super::InitParams,
    ) -> Option<Self> {
        if read_dir("forts").is_err() {
            return None;
        }

        params.java_workspace_members.push("forts".into());
        Some(Self::new(fs::canonicalize("forts").unwrap()))
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        params: &mut super::InitParams,
    ) -> Self {
        if !Command::new("git")
            .arg("clone")
            .arg(params.git_backend.repo_url("Darkdustry-Coders/Forts"))
            .arg(params.root.join("forts"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        Self::new(fs::canonicalize("forts").unwrap())
    }
}
