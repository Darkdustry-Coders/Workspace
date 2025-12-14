use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use crate::util::download;

use super::{TargetImpl, TargetImplStatic};

pub struct Impl {
    path: PathBuf,
}
impl Impl {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, params: &mut super::BuildParams) {
        params
            .env
            .insert("MINDUSTRY_PATH".into(), self.path.clone().into_os_string());
    }
}
impl TargetImplStatic for Impl {
    fn flags() -> super::TargetFlags {
        super::TargetFlags::new().always_local()
    }

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
        params: &mut super::InitParams,
    ) -> Option<Self> {
        let file = Path::new(".cache/tools/mindustry").join(match params.mindustry_version {
            crate::args::MindustryVersion::V146 => "server-v146.jar",
            crate::args::MindustryVersion::V149 => "server-v149.jar",
            crate::args::MindustryVersion::V150 => "server-v150.jar",
            crate::args::MindustryVersion::V153 => "server-v153.jar",
            crate::args::MindustryVersion::BleedingEdge => "server-be.jar",
        });

        if File::open(&file).is_ok() {
            Some(Self::new(fs::canonicalize(file).unwrap()))
        } else {
            None
        }
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        params: &mut super::InitParams,
    ) -> Self {
        match params.mindustry_version {
            crate::args::MindustryVersion::V146 => {
                let file = Path::new(".cache/tools/mindustry/server-v146.jar");
                eprintln!("Downloading Mindustry (v146)");
                download(
                    "https://github.com/5GameMaker/MindustryHotfixv7/releases/download/v146.8/server-release.jar",
                    file,
                );
                Self::new(fs::canonicalize(file).unwrap())
            }
            crate::args::MindustryVersion::V153 => {
                let file = Path::new(".cache/tools/mindustry/server-v153.jar");
                eprintln!("Downloading Mindustry (v153)");
                download(
                    "https://github.com/Anuken/Mindustry/releases/download/v153/server-release.jar",
                    file,
                );
                Self::new(fs::canonicalize(file).unwrap())
            }
            _ => todo!(),
        }
    }
}
