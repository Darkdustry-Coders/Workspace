//! Forts plugin target.
//!
//! This module manages the Forts game mode plugin for Mindustry.
//! Forts is a strategic defense game mode.
//! Repository: https://github.com/Darkdustry-Coders/Forts

simple_server_target!(
    jar = "Forts",
    dir = "forts",
    target = "forts",
    prefix = "forts",
    server = "forts",
    startcommand = "host Forts_v1.5 attack",
    repo = "Darkdustry-Coders/Forts",

    fn setup_server(params) {
        params.run.link_global(
            params.root.join("forts/assets/testmap.msav"),
            "forts/config/maps/testmap.msav",
        );
    },
);

// /// Forts plugin target implementation.
// pub struct Impl {
//     /// Path to the plugin repository.
//     #[allow(unused)]
//     repo: PathBuf,
//     /// Path to the built JAR file.
//     #[allow(unused)]
//     path: PathBuf,
//     /// Command to run the server.
//     command: Option<Command>,
// }
//
// impl Impl {
//     /// Creates a new Forts target instance.
//     ///
//     /// # Arguments
//     /// * `path` - Path to the repository
//     fn new(path: PathBuf) -> Self {
//         Self {
//             repo: path,
//             path: current_dir().join(".bin/Forts.jar"),
//             command: None,
//         }
//     }
// }
//
// impl TargetImpl for Impl {
//     fn build(&mut self, deps: super::Targets<'_>, params: &mut super::BuildParams) {
//         // On Forts side it should copy resulting jar into `.bin/Forts.jar`.
//         if !params
//             .gradle()
//             .arg(":forts:build")
//             .status()
//             .unwrap()
//             .success()
//         {
//             panic!("building Forts failed");
//         }
//
//         if params.native_image {
//             println!("Merging jars! (will take a while!)");
//
//             let mut output = zip::ZipWriter::new(
//                 File::create(".cache/tools/buildscript/tmp.jar")
//                     .expect("failed to open '.cache/tools/buildscript/tmp.jar'"),
//             );
//
//             let mut buffer = vec![0; 1024 * 1024 * 16];
//
//             {
//                 let mut input = zip::ZipArchive::new(
//                     File::open(".bin/server-release.jar")
//                         .expect("failed to open '.bin/server-release.jar'"),
//                 )
//                 .expect("failed to open zip archive");
//                 for name in input
//                     .file_names()
//                     .map(|x| x.to_owned().into_boxed_str())
//                     .collect::<Vec<_>>()
//                 {
//                     if name.ends_with("/") {
//                         continue;
//                     }
//
//                     output
//                         .start_file(
//                             &name,
//                             FileOptions::DEFAULT
//                                 .compression_method(zip::CompressionMethod::Deflated),
//                         )
//                         .unwrap();
//                     let mut reader = Some(input.by_name(&name).unwrap());
//                     let mut pos = 0usize;
//
//                     while pos != 0 || reader.is_some() {
//                         if pos < buffer.len() / 2 {
//                             if let Some(x) = &mut reader {
//                                 match x.read(&mut buffer[pos..]) {
//                                     Ok(0) => _ = reader.take(),
//                                     Ok(l) => pos += l,
//                                     Err(why) => panic!("Reading of {name:?} failed: {why:#?}"),
//                                 }
//                             }
//
//                             if pos != 0 {
//                                 match output.write(&mut buffer[..pos]) {
//                                     Ok(0) => {
//                                         panic!("Could not write into {name:?}: unexpected EOF")
//                                     }
//                                     Ok(l) => {
//                                         buffer.copy_within(l..pos, 0);
//                                         pos -= l;
//                                     }
//                                     Err(why) => panic!("Could not write into {name:?}: {why:#?}"),
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//
//             for (name, prefix) in [
//                 (".bin/CorePlugin.jar", "coreplugin/"),
//                 (".bin/Forts.jar", "forts/"),
//             ] {
//                 let mut input = zip::ZipArchive::new(match File::open(name) {
//                     Ok(x) => x,
//                     Err(why) => panic!("failed to open {name:?}: {why:#?}"),
//                 })
//                 .expect("failed to open zip archive");
//                 for name in input
//                     .file_names()
//                     .map(|x| x.to_owned().into_boxed_str())
//                     .collect::<Vec<_>>()
//                 {
//                     if name.ends_with("/") {
//                         continue;
//                     }
//
//                     if &*name == "META-INF/MANIFEST.SF" {
//                         continue;
//                     }
//
//                     output
//                         .start_file(
//                             if name.ends_with(".class")
//                                 || name.contains("kotlin")
//                                 || name.contains("native-image")
//                                 || name.contains("jline")
//                             {
//                                 Cow::Borrowed(name.as_ref())
//                             } else {
//                                 Cow::Owned(String::from(prefix) + name.as_ref())
//                             }
//                             .as_ref(),
//                             FileOptions::DEFAULT
//                                 .compression_method(zip::CompressionMethod::Deflated),
//                         )
//                         .unwrap();
//                     let mut reader = Some(input.by_name(&name).unwrap());
//                     let mut pos = 0usize;
//
//                     while pos != 0 || reader.is_some() {
//                         if pos < buffer.len() / 2 {
//                             if let Some(x) = &mut reader {
//                                 match x.read(&mut buffer[pos..]) {
//                                     Ok(0) => _ = reader.take(),
//                                     Ok(l) => pos += l,
//                                     Err(why) => panic!("Reading of {name:?} failed: {why:#?}"),
//                                 }
//                             }
//
//                             if pos != 0 {
//                                 match output.write(&mut buffer[..pos]) {
//                                     Ok(0) => {
//                                         panic!("Could not write into {name:?}: unexpected EOF")
//                                     }
//                                     Ok(l) => {
//                                         buffer.copy_within(l..pos, 0);
//                                         pos -= l;
//                                     }
//                                     Err(why) => panic!("Could not write into {name:?}: {why:#?}"),
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//
//             output
//                 .start_file(
//                     "META-INF/mods",
//                     FileOptions::DEFAULT.compression_method(zip::CompressionMethod::Stored),
//                 )
//                 .unwrap();
//             output.write_all(b"coreplugin\nforts").unwrap();
//
//             output.finish().expect("failed to finish 'tmp.jar'");
//
//             fs::remove_dir_all(".cache/tools/buildscript/genenv").ok();
//             fs::create_dir_all(".cache/tools/buildscript/genenv/config").unwrap();
//
//             fs::write(
//                 ".cache/tools/buildscript/genenv/config/corePlugin.toml",
//                 format!(
//                     "serverName = \"forts\"\ngamemode = \"forts\"\nsharedConfigPath = {:?}",
//                     "config/sharedConfig.toml"
//                 ),
//             )
//             .unwrap();
//             fs::write(
//                 ".cache/tools/buildscript/genenv/config/sharedConfig.toml",
//                 "serverIp = \"127.0.0.1\"\nrabbitMqUrl=\"\"\nsurrealDbUrl=\"\"",
//             )
//             .unwrap();
//
//             let code = params
//                 .cmd(
//                     deps.java
//                         .as_ref()
//                         .unwrap()
//                         .home()
//                         .join(exe_path!("bin/java")),
//                 )
//                 .arg("-agentlib:native-image-agent=config-output-dir=.")
//                 .arg("-jar")
//                 .arg(current_dir().join(".cache/tools/buildscript/tmp.jar"))
//                 .current_dir(current_dir().join(".cache/tools/buildscript/genenv"))
//                 .spawn()
//                 .unwrap()
//                 .wait()
//                 .unwrap()
//                 .code()
//                 .unwrap_or(-1);
//             if code != 0 {
//                 panic!("'java' exited with error code {code}");
//             }
//
//             let code = params
//                 .cmd(
//                     deps.java
//                         .as_ref()
//                         .unwrap()
//                         .home()
//                         .join(exe_path!("bin/javac")),
//                 )
//                 .arg("buildscript/src/targets/NiMetadata.java")
//                 .arg("-d")
//                 .arg(".cache/tools/buildscript")
//                 .spawn()
//                 .unwrap()
//                 .wait()
//                 .unwrap()
//                 .code()
//                 .unwrap_or(-1);
//             if code != 0 {
//                 panic!("'java' exited with error code {code}");
//             }
//
//             {
//                 // Because zip is fucking ass
//                 println!("Re-merging the archive! (will while take a)");
//
//                 let mut reader =
//                     zip::ZipArchive::new(File::open(".cache/tools/buildscript/tmp.jar").unwrap())
//                         .unwrap();
//                 let mut writer =
//                     zip::ZipWriter::new(File::create(".cache/tools/buildscript/tmp2.jar").unwrap());
//
//                 let names: Vec<_> = reader.file_names().map(String::from).collect();
//                 for name in names {
//                     if name.ends_with("/") {
//                         continue;
//                     }
//
//                     if name != "mindustry/NiMetadata.class" {
//                         writer
//                             .raw_copy_file(reader.by_name(&name).unwrap())
//                             .unwrap();
//                         continue;
//                     }
//
//                     let mut reader = Some(
//                         File::open(".cache/tools/buildscript/mindustry/NiMetadata.class").unwrap(),
//                     );
//
//                     writer
//                         .start_file(
//                             &name,
//                             FileOptions::DEFAULT.compression_method(zip::CompressionMethod::Stored),
//                         )
//                         .unwrap();
//
//                     let mut pos = 0usize;
//
//                     while pos != 0 || reader.is_some() {
//                         if pos < buffer.len() / 2 {
//                             if let Some(x) = &mut reader {
//                                 match x.read(&mut buffer[pos..]) {
//                                     Ok(0) => _ = reader.take(),
//                                     Ok(l) => pos += l,
//                                     Err(why) => panic!("Reading of {name:?} failed: {why:#?}"),
//                                 }
//                             }
//
//                             if pos != 0 {
//                                 match writer.write(&mut buffer[..pos]) {
//                                     Ok(0) => {
//                                         panic!("Could not write into {name:?}: unexpected EOF")
//                                     }
//                                     Ok(l) => {
//                                         buffer.copy_within(l..pos, 0);
//                                         pos -= l;
//                                     }
//                                     Err(why) => panic!("Could not write into {name:?}: {why:#?}"),
//                                 }
//                             }
//                         }
//                     }
//                 }
//
//                 writer.finish().unwrap();
//             }
//
//             let code = params
//
//     val player = player ?: return
//     if (player.con == null) return
//     if (!player.con.hasConnected) return
//     if (!player.isAdded) return
//     if (Time.timeSinceMillis(player.con.connectTime) < 500) return
//
//     if (!player.con.chatRate.allow(2000, Administration.Config.chatSpamLimit.num())) {
//         player.con.kick(Packets.KickReason.kick)
//         Vars.netServer.admins.blacklistDos(player.con.address)
//     }
//
//     var message: String? = message ?: return
//
//     if (notnull(message).length > 150) {
//         throw ValidateException(player, "Player sent a message above the text limit.")
//     }
//
//                 .cmd(
//                     deps.java
//                         .as_ref()
//                         .unwrap()
//                         .home()
//                         .join(exe_path!("bin/native-image")),
//                 )
//                 .arg("-jar")
//                 .arg(".cache/tools/buildscript/tmp2.jar")
//                 .arg("-H:IncludeResources=.*/lang/.*\\.l")
//                 .arg(format!(
//                     "-H:JNIConfigurationFiles={}",
//                     current_dir()
//                         .join(".cache/tools/buildscript/genenv/jni-config.json")
//                         .display()
//                 ))
//                 .arg(format!(
//                     "-H:ResourceConfigurationFiles={}",
//                     current_dir()
//                         .join(".cache/tools/buildscript/genenv/resource-config.json")
//                         .display()
//                 ))
//                 .arg(format!(
//                     "-H:ReflectionConfigurationFiles={},{}",
//                     current_dir()
//                         .join(".cache/tools/buildscript/genenv/reflect-config.json")
//                         .display(),
//                     current_dir()
//                         .join("coreplugin/assets/reflect-config.json")
//                         .display()
//                 ))
//                 .arg(format!(
//                     "-H:SerializationConfigurationFiles={}",
//                     current_dir()
//                         .join(".cache/tools/buildscript/genenv/serialization-config.json")
//                         .display()
//                 ))
//                 .arg("--trace-class-initialization=kotlin.DeprecationLevel")
//                 .arg("--initialize-at-build-time=kotlin.DeprecationLevel")
//                 .arg("--no-fallback")
//                 .arg("--enable-url-protocols=http")
//                 .arg("-o")
//                 .arg(exe_path!(".bin/Forts"))
//                 .spawn()
//                 .unwrap()
//                 .wait()
//                 .unwrap()
//                 .code()
//                 .unwrap_or(-1);
//             if code != 0 {
//                 panic!("'native-image' exited with error code {code}");
//             }
//         }
//     }
//
//     fn run_init(&mut self, deps: super::Targets<'_>, params: &mut super::RunParams) {
//         let root = Path::new(".run/forts");
//
//         params.run.link_global(
//             params.root.join(".bin/CorePlugin.jar"),
//             "forts/config/mods/CorePlugin.jar",
//         );
//         params.run.link_global(
//             params.root.join(".bin/Forts.jar"),
//             "forts/config/mods/Forts.jar",
//         );
//         params.run.link_global(
//             params.root.join("forts/assets/testmap.msav"),
//             "forts/config/maps/testmap.msav",
//         );
//         params.run.write(
//             "forts/config/corePlugin.toml",
//             format!(
//                 "serverName = \"forts\"\ngamemode = \"forts\"\nsharedConfigPath = {:?}",
//                 params.root.join(".run/sharedConfig.toml")
//             ),
//         );
//
//         let port = params.next_port();
//
//         {
//             let mut contents = vec![];
//             contents.extend_from_slice(&3i32.to_be_bytes());
//
//             let option = "servername";
//             contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
//             contents.extend_from_slice(option.as_bytes());
//
//             let name = "[scarlet]Workspace [accent]| [white]Forts";
//             contents.push(4);
//             contents.extend_from_slice(&(name.len() as u16).to_be_bytes());
//             contents.extend_from_slice(name.as_bytes());
//
//             let option = "port";
//             contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
//             contents.extend_from_slice(option.as_bytes());
//
//             contents.push(1);
//             contents.extend_from_slice(&(port as i32).to_be_bytes());
//
//             let option = "startCommands";
//             contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
//             contents.extend_from_slice(option.as_bytes());
//
//             let commands = "host Forts_v1.5 attack";
//             contents.push(4);
//             contents.extend_from_slice(&(commands.len() as u16).to_be_bytes());
//             contents.extend_from_slice(commands.as_bytes());
//
//             params.run.write("forts/config/settings.bin", contents);
//         }
//
//         if params.native_image {
//             let mut cmd = params.cmd(fs::canonicalize(exe_path!(".bin/Forts")).unwrap());
//             cmd.current_dir(root);
//             self.command = Some(cmd);
//         } else {
//             let java = deps.java.as_ref().unwrap().home().join("bin/java");
//             let mindustry = deps.mindustry.as_ref().unwrap().path();
//
//             let mut cmd = params.cmd(java);
//             cmd.arg("-jar").arg(mindustry).current_dir(root);
//             self.command = Some(cmd);
//         }
//     }
//
//     fn run(&mut self, deps: super::Targets<'_>, params: &mut super::RunParams) {
//         deps.mprocs.as_ref().unwrap().spawn_task(
//             params,
//             &mut self.command.take().unwrap(),
//             "forts",
//         );
//     }
// }
//
// impl TargetImplStatic for Impl {
//     fn depends(list: &mut super::TargetList) {
//         list.set_depend(Target::Java);
//         list.set_depend(Target::CorePlugin);
//     }
//
//     fn initialize_host(
//         _: super::TargetEnabled,
//         _: super::Targets<'_>,
//         _: &mut super::InitParams,
//     ) -> Option<Self> {
//         unimplemented!()
//     }
//
//     fn initialize_cached(
//         _: super::TargetEnabled,
//         _: super::Targets<'_>,
//         _: &mut super::InitParams,
//     ) -> Option<Self> {
//         if read_dir("forts").is_err() {
//             return None;
//             awdsawwdaaaa
//         }
//
//         Some(Self::new(fs::canonicalize("forts").unwrap()))
//     }
//
//     fn initialize_local(
//         _: super::TargetEnabled,
//         _: super::Targets<'_>,
//         params: &mut super::InitParams,
//     ) -> Self {
//         if !Command::new("git")
//             .arg("clone")
//             .arg(params.git_backend.repo_url("Darkdustry-Coders/Forts"))
//             .arg(params.root.join("forts"))
//             .status()
//             .unwrap()
//             .success()
//         {
//             panic!("failed to fetch repo");
//         }
//
//         Self::new(fs::canonicalize("forts").unwrap())
//     }
//
//     fn postinit(_: super::TargetEnabled, _: super::Targets<'_>, params: &mut super::InitParams) {
//         if fs::read_dir("forts").is_ok() {
//             params.java_workspace_members.push("forts".into());
//         }
//     }
// }
