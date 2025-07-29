use std::{
    env::current_dir,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::util::{download, is_executable, untar_gz};

use super::{TargetFlags, TargetImpl, TargetImplStatic};

pub struct Impl {
    java_home: PathBuf,
}
impl Impl {
    fn new(java_home: PathBuf) -> Self {
        eprintln!("java: {java_home:?}");
        Self { java_home }
    }

    pub fn home(&self) -> &Path {
        &self.java_home
    }
}
impl TargetImpl for Impl {
    fn build(&mut self, _: super::Targets<'_>, params: &mut super::BuildParams) {
        params.env.insert(
            "JAVA_HOME".into(),
            self.java_home.as_os_str().to_os_string(),
        );
        params.path.push(self.java_home.join("bin"));
    }
}
impl TargetImplStatic for Impl {
    fn depends(list: &mut super::TargetList) {
        list.set_depend(super::Target::CoreUtils);
    }

    fn flags() -> TargetFlags {
        TargetFlags {
            always_local: false,
            ..Default::default()
        }
    }

    fn initialize_host(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        'a: {
            if let Ok(java_home) = std::env::var("JAVA_HOME") {
                let java_home = PathBuf::from(java_home);

                if !is_executable(java_home.join("bin/javac")) {
                    break 'a;
                }

                let Ok(out) = Command::new(java_home.join("bin/java"))
                    .arg(
                        current_dir()
                            .unwrap()
                            .join("buildscript/src/targets/java-version-check.java"),
                    )
                    .output()
                else {
                    break 'a;
                };
                if !out.status.success() {
                    break 'a;
                }
                let Ok(out) = str::from_utf8(&out.stdout) else {
                    break 'a;
                };
                let Ok(version): Result<u8, _> = out.trim().parse() else {
                    break 'a;
                };
                if version < 17 {
                    break 'a;
                }

                return Some(Self::new(java_home));
            }
        }

        if cfg!(unix) {
            if let Ok(x) = fs::read_dir("/usr/lib/jvm") {
                for x in x {
                    let Ok(x) = x else { continue };

                    let java_home = x.path();

                    if !is_executable(java_home.join("bin/javac")) {
                        continue;
                    }

                    let Ok(out) = Command::new(java_home.join("bin/java"))
                        .arg(
                            current_dir()
                                .unwrap()
                                .join("buildscript/src/targets/java-version-check.java"),
                        )
                        .output()
                    else {
                        continue;
                    };
                    if !out.status.success() {
                        continue;
                    }
                    let Ok(out) = str::from_utf8(&out.stdout) else {
                        continue;
                    };
                    let Ok(version): Result<u8, _> = out.trim().parse() else {
                        continue;
                    };
                    if version < 17 {
                        continue;
                    }

                    return Some(Self::new(java_home));
                }
            }
        }

        // TODO: Implement for Windows

        None
    }

    fn initialize_cached(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        if is_executable(".cache/tools/java/bin/javac")
            && is_executable(".cache/tools/java/bin/java")
        {
            Some(Self::new(fs::canonicalize(".cache/tools/java").unwrap()))
        } else {
            None
        }
    }

    #[allow(unreachable_code)]
    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        _: &mut super::InitParams,
    ) -> Self {
        #[cfg(unix)]
        {
            eprintln!("Downloading JDK21");

            let url = "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz";
            let archive = ".cache/tools/java/archive.tar.gz";

            fs::create_dir_all(".cache/tools/java").unwrap();

            download(url, archive);
            untar_gz(archive, ".cache/tools/java", 1);

            return Self::new(fs::canonicalize(".cache/tools/java").unwrap());
        }

        // TODO: Implement for Windows

        todo!()
    }
}
