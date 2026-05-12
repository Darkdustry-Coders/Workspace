use std::{fs::File, io::Write, path::PathBuf};

use zip::ZipArchive;

/// Nil plugin.
///
/// There's no run target as nil is not meant to be used on its own.
pub struct Impl {
    /// Path to the plugin repository.
    #[allow(unused)]
    repo: PathBuf,
    /// Path to the built JAR file.
    #[allow(unused)]
    path: PathBuf,
}
impl Impl {
    fn new(path: PathBuf) -> Self {
        Self {
            repo: path,
            path: crate::current_dir().join(".bin/Nil.jar"),
        }
    }
}
impl super::TargetImpl for Impl {
    fn build(&mut self, deps: super::Targets<'_>, params: &mut super::BuildParams) {
        // On NilPlugin side it should copy resulting jar into `.bin/Nil.jar`.
        if !params
            .gradle()
            .arg(concat!(":nil:build"))
            .status()
            .unwrap()
            .success()
        {
            panic!(concat!("building Nil failed"));
        }

        if params.native_image {
            println!("Merging jars! (will take a while!)");

            let mut output = zip::ZipWriter::new(
                ::std::fs::File::create(".cache/tools/buildscript/tmp.jar")
                    .expect("failed to open '.cache/tools/buildscript/tmp.jar'"),
            );

            let mut buffer = vec![0; 1024 * 1024 * 16];

            {
                let mut input = zip::ZipArchive::new(
                    ::std::fs::File::open(".bin/server-release.jar")
                        .expect("failed to open '.bin/server-release.jar'"),
                )
                .expect("failed to open zip archive");
                for name in input
                    .file_names()
                    .map(|x| x.to_owned().into_boxed_str())
                    .collect::<Vec<_>>()
                {
                    if name.ends_with("/") {
                        continue;
                    }

                    output
                        .start_file(
                            &name,
                            ::zip::write::FileOptions::DEFAULT
                                .compression_method(zip::CompressionMethod::Deflated),
                        )
                        .unwrap();
                    let mut reader = Some(input.by_name(&name).unwrap());
                    let mut pos = 0usize;

                    while pos != 0 || reader.is_some() {
                        if pos < buffer.len() / 2 {
                            if let Some(x) = &mut reader {
                                match ::std::io::Read::read(x, &mut buffer[pos..]) {
                                    Ok(0) => _ = reader.take(),
                                    Ok(l) => pos += l,
                                    Err(why) => {
                                        panic!("Reading of {name:?} failed: {why:#?}")
                                    }
                                }
                            }

                            if pos != 0 {
                                match Write::write(&mut output, &mut buffer[..pos]) {
                                    Ok(0) => {
                                        panic!("Could not write into {name:?}: unexpected EOF")
                                    }
                                    Ok(l) => {
                                        buffer.copy_within(l..pos, 0);
                                        pos -= l;
                                    }
                                    Err(why) => {
                                        panic!("Could not write into {name:?}: {why:#?}")
                                    }
                                }
                            }
                        }
                    }
                }
            }

            for (name, prefix) in [
                (".bin/CorePlugin.jar", "coreplugin/"),
                (".bin/Nil.jar", "nil/"),
            ] {
                let mut input = ZipArchive::new(match File::open(name) {
                    Ok(x) => x,
                    Err(why) => panic!("failed to open {name:?}: {why:#?}"),
                })
                .expect("failed to open zip archive");
                for name in input
                    .file_names()
                    .map(|x| x.to_owned().into_boxed_str())
                    .collect::<Vec<_>>()
                {
                    if name.ends_with("/") {
                        continue;
                    }

                    if &*name == "META-INF/MANIFEST.SF" {
                        continue;
                    }

                    output
                        .start_file(
                            if name.ends_with(".class")
                                || name.contains("kotlin")
                                || name.contains("native-image")
                                || name.contains("jline")
                            {
                                ::std::borrow::Cow::Borrowed(name.as_ref())
                            } else {
                                ::std::borrow::Cow::Owned(String::from(prefix) + name.as_ref())
                            }
                            .as_ref(),
                            ::zip::write::FileOptions::DEFAULT
                                .compression_method(zip::CompressionMethod::Deflated),
                        )
                        .unwrap();
                    let mut reader = Some(input.by_name(&name).unwrap());
                    let mut pos = 0usize;

                    while pos != 0 || reader.is_some() {
                        if pos < buffer.len() / 2 {
                            if let Some(x) = &mut reader {
                                match ::std::io::Read::read(x, &mut buffer[pos..]) {
                                    Ok(0) => _ = reader.take(),
                                    Ok(l) => pos += l,
                                    Err(why) => {
                                        panic!("Reading of {name:?} failed: {why:#?}")
                                    }
                                }
                            }

                            if pos != 0 {
                                match ::std::io::Write::write(&mut output, &mut buffer[..pos]) {
                                    Ok(0) => {
                                        panic!("Could not write into {name:?}: unexpected EOF")
                                    }
                                    Ok(l) => {
                                        buffer.copy_within(l..pos, 0);
                                        pos -= l;
                                    }
                                    Err(why) => {
                                        panic!("Could not write into {name:?}: {why:#?}")
                                    }
                                }
                            }
                        }
                    }
                }
            }

            output
                .start_file(
                    "META-INF/mods",
                    ::zip::write::FileOptions::DEFAULT
                        .compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
            ::std::io::Write::write_all(&mut output, concat!("coreplugin\nnil").as_ref()).unwrap();

            output.finish().expect("failed to finish 'tmp.jar'");

            ::std::fs::remove_dir_all(".cache/tools/buildscript/genenv").ok();
            crate::fs2::create_dir_all(".cache/tools/buildscript/genenv/config").unwrap();

            crate::fs2::write(
                ".cache/tools/buildscript/genenv/config/corePlugin.toml",
                format!(
                    "serverName = \"unknown\"\ngamemode = \"unknown\"\nsharedConfigPath = {:?}",
                    "config/sharedConfig.toml"
                ),
            )
            .unwrap();
            crate::fs2::write(
                ".cache/tools/buildscript/genenv/config/sharedConfig.toml",
                "serverIp = \"127.0.0.1\"\nrabbitMqUrl=\"\"\nsurrealDbUrl=\"\"",
            )
            .unwrap();
            crate::fs2::write(
                ".cache/tools/buildscript/genenv/config/nilConfig.toml",
                concat!("hasStats = true\nunlockSpecialBlocks = true\nrestoreTeams = true\n",
                "enableRtv = true\nenableVnw = true\nenableSurrender = true\nenableSpectate = true")
            )
            .unwrap();

            let code = params
                .cmd(
                    deps.java
                        .as_ref()
                        .unwrap()
                        .home()
                        .join(crate::exe_path!("bin/java")),
                )
                .arg("-agentlib:native-image-agent=config-output-dir=.")
                .arg("-jar")
                .arg(crate::current_dir().join(".cache/tools/buildscript/tmp.jar"))
                .current_dir(crate::current_dir().join(".cache/tools/buildscript/genenv"))
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
                .code()
                .unwrap_or(-1);
            if code != 0 {
                panic!("'java' exited with error code {code}");
            }

            let code = params
                .cmd(
                    deps.java
                        .as_ref()
                        .unwrap()
                        .home()
                        .join(crate::exe_path!("bin/javac")),
                )
                .arg("buildscript/src/targets/NiMetadata.java")
                .arg("-d")
                .arg(".cache/tools/buildscript")
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
                .code()
                .unwrap_or(-1);
            if code != 0 {
                panic!("'java' exited with error code {code}");
            }

            {
                // Because zip is fucking ass
                println!("Re-merging the archive! (will while take a)");

                let mut reader = zip::ZipArchive::new(
                    ::std::fs::File::open(".cache/tools/buildscript/tmp.jar").unwrap(),
                )
                .unwrap();
                let mut writer = zip::ZipWriter::new(
                    ::std::fs::File::create(".cache/tools/buildscript/tmp2.jar").unwrap(),
                );

                let names: Vec<_> = reader.file_names().map(String::from).collect();
                for name in names {
                    if name.ends_with("/") {
                        continue;
                    }

                    if name != "mindustry/NiMetadata.class" {
                        writer
                            .raw_copy_file(reader.by_name(&name).unwrap())
                            .unwrap();
                        continue;
                    }

                    let mut reader = Some(
                        ::std::fs::File::open(
                            ".cache/tools/buildscript/mindustry/NiMetadata.class",
                        )
                        .unwrap(),
                    );

                    writer
                        .start_file(
                            &name,
                            ::zip::write::FileOptions::DEFAULT
                                .compression_method(zip::CompressionMethod::Stored),
                        )
                        .unwrap();

                    let mut pos = 0usize;

                    while pos != 0 || reader.is_some() {
                        if pos < buffer.len() / 2 {
                            if let Some(x) = &mut reader {
                                match ::std::io::Read::read(x, &mut buffer[pos..]) {
                                    Ok(0) => _ = reader.take(),
                                    Ok(l) => pos += l,
                                    Err(why) => {
                                        panic!("Reading of {name:?} failed: {why:#?}")
                                    }
                                }
                            }

                            if pos != 0 {
                                match ::std::io::Write::write(&mut writer, &mut buffer[..pos]) {
                                    Ok(0) => {
                                        panic!("Could not write into {name:?}: unexpected EOF")
                                    }
                                    Ok(l) => {
                                        buffer.copy_within(l..pos, 0);
                                        pos -= l;
                                    }
                                    Err(why) => {
                                        panic!("Could not write into {name:?}: {why:#?}")
                                    }
                                }
                            }
                        }
                    }
                }

                if let Ok(x) = ::std::fs::read_to_string(
                    crate::current_dir()
                        .join(".cache/tools/buildscript/genenv/reachability-metadata.json"),
                ) {
                    writer
                        .start_file(
                            "META-INF/native-image/mindurka/workspace/reachability-metadata.json",
                            ::zip::write::FileOptions::DEFAULT
                                .compression_method(zip::CompressionMethod::Stored),
                        )
                        .unwrap();

                    ::std::io::Write::write_all(&mut writer, x.as_bytes()).unwrap();
                }

                writer.finish().unwrap();
            }

            let code = {
                let mut cmd = params.cmd(
                    deps.java
                        .as_ref()
                        .unwrap()
                        .home()
                        .join(crate::exe_path!("bin/native-image")),
                );
                cmd.arg("-jar")
                    .arg(".cache/tools/buildscript/tmp2.jar")
                    .arg("-H:IncludeResources=.*/lang/.*\\.l");
                if ::std::fs::File::open(
                    crate::current_dir()
                        .join(".cache/tools/buildscript/genenv/reachability-metadata.json"),
                )
                .is_err()
                {
                    cmd.arg(format!(
                        "-H:JNIConfigurationFiles={}",
                        crate::current_dir()
                            .join(".cache/tools/buildscript/genenv/jni-config.json")
                            .display()
                    ))
                    .arg(format!(
                        "-H:ResourceConfigurationFiles={}",
                        crate::current_dir()
                            .join(".cache/tools/buildscript/genenv/resource-config.json")
                            .display()
                    ))
                    .arg(format!(
                        "-H:ReflectionConfigurationFiles={},{}",
                        crate::current_dir()
                            .join(".cache/tools/buildscript/genenv/reflect-config.json")
                            .display(),
                        crate::current_dir()
                            .join("coreplugin/assets/reflect-config.json")
                            .display()
                    ))
                    .arg(format!(
                        "-H:SerializationConfigurationFiles={}",
                        crate::current_dir()
                            .join(".cache/tools/buildscript/genenv/serialization-config.json")
                            .display()
                    ));
                } else {
                    println!(
                        "\x1b[33m[WARN] 'reachability-metadata.json' is buggy af and thus not supported. Please use GraalVM 17\x1b[0m"
                    );
                }
                cmd.arg("--trace-class-initialization=kotlin.DeprecationLevel")
                    .arg("-H:CStandard=C11")
                    .arg("--initialize-at-build-time=kotlin.DeprecationLevel")
                    .arg("--no-fallback")
                    .arg("--enable-url-protocols=http")
                    .arg("-o")
                    .arg(crate::exe_path!(concat!(".bin/Nil")))
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap()
                    .code()
                    .unwrap_or(-1)
            };
            if code != 0 {
                panic!("'native-image' exited with error code {code}");
            }
        }
    }

    fn run_init(&mut self, _: super::Targets<'_>, _: &mut super::RunParams) {}
    fn run(&mut self, _: super::Targets<'_>, _: &mut super::RunParams) {}
}

impl super::TargetImplStatic for Impl {
    fn depends(list: &mut super::TargetList) {
        list.set_depend(super::Target::Java);
        list.set_depend(super::Target::CorePlugin);
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
        if ::std::fs::read_dir("nil").is_err() {
            return None;
        }

        Some(Self::new(::std::fs::canonicalize("nil").unwrap()))
    }

    fn initialize_local(
        _: super::TargetEnabled,
        _: super::Targets<'_>,
        params: &mut super::InitParams,
    ) -> Self {
        if !::std::process::Command::new("git")
            .arg("clone")
            .arg(params.git_backend.repo_url("Darkdustry-Coders/NilPlugin"))
            .arg(params.root.join("nil"))
            .status()
            .unwrap()
            .success()
        {
            panic!("failed to fetch repo");
        }

        Self::new(::std::fs::canonicalize("nil").unwrap())
    }

    fn postinit(_: super::TargetEnabled, _: super::Targets<'_>, params: &mut super::InitParams) {
        if ::std::fs::read_dir("nil").is_ok() {
            params.java_workspace_members.push("nil".into());
        }
    }
}
