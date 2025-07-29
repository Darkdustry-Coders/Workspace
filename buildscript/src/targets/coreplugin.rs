use std::{
    fs::{self, read_dir},
    path::{Path, PathBuf},
    process::Command,
};

use crate::util::{current_dir, gradle};

use super::{Target, TargetImpl, TargetImplStatic};

// TODO: Download if enabled status is `Depend` instead of `Build`.

pub struct Impl {
    repo: PathBuf,
    path: PathBuf,
}
impl Impl {
    fn new(path: PathBuf) -> Self {
        Self {
            repo: path,
            path: current_dir().join(".bin/CorePlugin.jar"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, params: &mut super::BuildParams) {
        // On CorePlugin side it should copy resulting jar into `.bin/CorePlugin.jar`.
        if !params
            .cmd(gradle())
            .arg(":coreplugin:build")
            .arg(":coreplugin:publishAllPublicationsToMavenRepository")
            .status()
            .unwrap()
            .success()
        {
            panic!("building CorePlugin failed");
        }
    }
}
impl TargetImplStatic for Impl {
    fn depends(list: &mut super::TargetList) {
        list.set_depend(Target::Java);
        list.set_depend(Target::RabbitMq);
        list.set_depend(Target::SurrealDb);
        list.set_depend(Target::Mindustry);
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
        if read_dir("coreplugin").is_err() {
            return None;
        }

        params.java_workspace_members.push("coreplugin".into());
        Some(Self::new(fs::canonicalize("coreplugin").unwrap()))
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        params: &mut super::InitParams,
    ) -> Self {
        if !Command::new("git")
            .arg("clone")
            .arg(params.git_backend.repo_url("Darkdustry-Coders/CorePlugin"))
            .arg(params.root.join("coreplugin"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        Self::new(fs::canonicalize("coreplugin").unwrap())
    }
}
