//! Mindustry server.
//!
//! This module manages Mindustry server JAR downloads and configuration.

use std::{
    fs::{self, read_dir},
    path::{Path, PathBuf},
    process::Command,
};

use crate::util::current_dir;

use super::{TargetImpl, TargetImplStatic};

/// Mindustry server target implementation.
pub struct Impl {
    path: PathBuf,
}
impl Impl {
    /// Returns the path to the server JAR.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, params: &mut super::BuildParams) {
        // If it works, it works. Just you wait till you learn how you UPDATE this thing.
        if !params
            .gradle()
            .current_dir(fs::canonicalize("mindustry").unwrap())
            .arg(":server:dist")
            .arg("-Pbuildversion=157")
            .arg(format!("-Pnativeimage={}", params.native_image))
            .status()
            .unwrap()
            .success()
        {
            panic!("building Mindustry failed");
        }

        if !params
            .gradle()
            .current_dir(fs::canonicalize("arc").unwrap())
            .arg("publishAllPublicationsToMavenRepository")
            .status()
            .unwrap()
            .success()
        {
            panic!("publishing Arc failed");
        }

        fs::copy(
            "mindustry/server/build/libs/server-release.jar",
            ".bin/server-release.jar",
        )
        .unwrap();

        // Build so nice I'll do it twice (otherwise server-release.jar has no shit).
        if !params
            .gradle()
            .current_dir(fs::canonicalize("mindustry").unwrap())
            .arg(":core:publishAllPublicationsToMavenRepository")
            .arg(":server:publishAllPublicationsToMavenRepository")
            .arg("-Pbuildversion=157")
            .arg(format!("-Pnativeimage={}", params.native_image))
            .status()
            .unwrap()
            .success()
        {
            panic!("building Mindustry failed");
        }
    }
}

impl TargetImplStatic for Impl {
    fn depends(list: &mut super::TargetList) {
        list.set_depend(super::Target::Java);
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
        if read_dir("mindustry").is_err() {
            return None;
        }

        Some(Self {
            path: current_dir().join(".bin/server-release.jar"),
        })
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
                    .repo_url("Darkdustry-Coders/MindustryServer"),
            )
            .arg(params.root.join("mindustry"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        if !Command::new("git")
            .arg("clone")
            .arg(params.git_backend.repo_url("Darkdustry-Coders/Arc"))
            .arg(params.root.join("arc"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        Self {
            path: current_dir().join(".bin/server-release.jar"),
        }
    }

    fn postinit(_: super::TargetEnabled, _: super::Targets<'_>, params: &mut super::InitParams) {
        if fs::read_dir("mindustry").is_ok() {
            params.java_masked_members.push("mindustry".into());
        }
        if fs::read_dir("arc").is_ok() {
            params.java_masked_members.push("arc".into());
        }
    }
}
