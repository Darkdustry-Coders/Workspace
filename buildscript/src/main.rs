mod args;
mod targets;
mod util;

use std::{
    env::current_dir,
    fs, io,
    path::PathBuf,
    process::{Command, Stdio, exit},
    str::FromStr,
};

use args::{Args, EnvTy};
use targets::{BuildParams, InitParams, RunParams, TARGET_NAMES, Target, TargetList, Targets};
use util::CURRENT_DIR;

fn main() {
    unsafe {
        CURRENT_DIR = Some(current_dir().unwrap());
    }

    dotenvy::dotenv().ok();

    let args = args::args();

    if match &args {
        Args::Help => true,
        Args::Build { build, .. } => build.targets.is_empty(),
        _ => false,
    } {
        args::print_help();
        exit(1);
    }

    unsafe {
        std::env::set_var("WORKSPACE", current_dir().unwrap());
        std::env::set_var("MINDURKA_WORKSPACE", current_dir().unwrap());
    };

    match args {
        Args::Help => unreachable!(),
        Args::Env { mut command, .. } => {
            if command.is_empty() {
                #[cfg(unix)]
                if let Ok(x) = std::env::var("SHELL") {
                    command.push(x);
                }
                #[cfg(target_os = "windows")]
                command.push("cmd.exe".to_string());
            }

            let mut c = Command::new(command.remove(0));
            let c = c
                .args(command)
                .stderr(Stdio::inherit())
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit());
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                panic!("execve() failed: {:#}", c.exec());
            }
            #[cfg(target_os = "windows")]
            {
                c.spawn().unwrap().wait().unwrap();
            }
        }
        Args::Build { build, env } => {
            fs::remove_dir_all(".build").ok();
            fs::remove_dir_all(".bin").ok();
            fs::create_dir_all(".bin").unwrap();

            let mut targets = Targets::default();
            let mut recipe = TargetList::default();

            let mut run = false;

            'b: for target in &build.targets {
                'a: {
                    match target.as_str() {
                        "all" => TARGET_NAMES
                            .iter()
                            .map(|x| Target::from_str(x).unwrap())
                            .filter(|x| !x.flags().deprecated)
                            .for_each(|target| recipe.set_build(target)),
                        "run" => run = true,
                        _ => break 'a,
                    }
                    continue 'b;
                }

                let target = match Target::from_str(target.as_str()) {
                    Ok(x) => x,
                    Err(_) => {
                        eprintln!("no target {target:?} defined");
                        exit(1);
                    }
                };
                recipe.set_build(target);
            }

            if run {
                recipe.set_build(Target::MProcs);
            }

            let mut params = InitParams::new(&build);

            targets.init_all(env, &mut recipe, &mut params);
            fs::write(
                "Cargo.toml",
                include_str!("../assets/Cargo.toml.in").replace("MEMBERS", &{
                    let mut s = "\"buildscript\"".to_string();
                    for x in &params.rust_workspace_members {
                        s += ", \"";
                        s += x.as_str();
                        s += "\"";
                    }
                    s
                }),
            )
            .unwrap();
            fs::write("settings.gradle", {
                let mut s = String::new();
                s += include_str!("../assets/settings.gradle.in");
                for x in &params.java_workspace_members {
                    // s += &format!("apply from: '{x}/coreplugin.gradle'\n");
                    // s += &format!("\ninclude ':{x}'");
                    s += &format!("\nincludeBuild '{x}'");
                }
                s
            })
            .unwrap();

            let mut params = BuildParams::new(params);

            targets.build_all(&mut params);

            if run {
                let mut params = RunParams::new(params, &build);

                if env != EnvTy::Isolate {
                    params.path.extend(
                        std::env::var("PATH")
                            .unwrap()
                            .split(if cfg!(unix) { ':' } else { ';' })
                            .map(PathBuf::from),
                    );
                }

                // if env == EnvTy::Isolate {
                //     if cfg!(unix) {
                //         params.path.push(PathBuf::from("/usr/bin"));
                //     } else if cfg!(target_os = "windows") {
                //         let sysroot = std::env::var("SYSTEMROOT").unwrap();
                //         params
                //             .path
                //             .push(PathBuf::from(format!("{sysroot}\\System32")));
                //     }
                // } else {
                //     params.path.extend(
                //         std::env::var("PATH")
                //             .unwrap()
                //             .split(if cfg!(unix) { ':' } else { ';' })
                //             .map(PathBuf::from),
                //     );
                // }

                if let Err(why) = fs::remove_dir_all(".run")
                    && why.kind() != io::ErrorKind::NotFound
                {
                    panic!("{why:#}");
                }
                fs::create_dir_all(".run").unwrap();

                targets.run_init_all(&mut params);
                targets.run_all(&mut params);

                if !targets.mprocs.as_mut().unwrap().wait() {
                    eprintln!("mprocs exited with a non-zero code");
                    exit(1);
                }
            }
        }
    }
}

// 't: for (tool, cmds, try_apply_local, install, apply_host) in t::<
//     &mut [(
//         &str,
//         &[&str],
//         &mut dyn FnMut() -> bool,
//         &mut dyn FnMut(),
//         &mut dyn FnMut(PathBuf),
//     )],
// >(&mut [
//     (
//         "java",
//         &["java", "javac"],
//         &mut || {
//             if is_executable(".cache/tools/java/bin/javac".as_ref())
//                 && is_executable(".cache/tools/java/bin/java".as_ref())
//             {
//                 unsafe { java_home.as_ptr().as_mut().unwrap_unchecked() }
//                     .replace(PathBuf::from(".cache/tools/java"));
//                 true
//             } else {
//                 false
//             }
//         },
//         &mut || {
//             #[cfg(unix)]
//             {
//                 eprintln!("Downloading JDK21");

//                 let url = "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.7%2B6/OpenJDK21U-jdk_x64_linux_hotspot_21.0.7_6.tar.gz";
//                 let archive = ".cache/tools/java/archive.tar.gz";

//                 fs::create_dir_all(t::<&Path>(archive.as_ref()).parent().unwrap()).unwrap();
//                 let mut resp = ureq::get(url).call().unwrap();
//                 let max_len: usize = if stderr().is_terminal() {
//                     resp.headers()
//                         .get("content-length")
//                         .map(|x| x.to_str().unwrap().parse().unwrap())
//                         .unwrap()
//                 } else {
//                     0
//                 };
//                 let mut buf = [0; 16384];
//                 let mut body = resp.body_mut().as_reader();
//                 let mut file = File::create(archive).unwrap();
//                 if stderr().is_terminal() {
//                     eprint!("[          ] 0% (0/{max_len})");
//                     stderr().flush().unwrap();
//                 }
//                 let mut total_len = 0usize;
//                 loop {
//                     let len = body.read(&mut buf).unwrap();
//                     if len == 0 {
//                         break;
//                     }
//                     file.write_all(&buf[0..len]).unwrap();
//                     if stderr().is_terminal() {
//                         total_len += len;
//                         let perc = total_len.mul(100) / max_len;
//                         eprint!(
//                             "\r\x1b[K[{}{}] {perc}% ({:.02}/{:.02}MiB)",
//                             "#".repeat(perc.div(10)),
//                             " ".repeat(10 - perc.div(10)),
//                             total_len as f32 / 1024.0 / 1024.0,
//                             max_len as f32 / 1024.0 / 1024.0,
//                         );
//                         stderr().flush().unwrap();
//                     }
//                 }
//                 file.flush().unwrap();
//                 drop(file);
//                 drop(body);
//                 drop(resp);

//                 eprintln!("\nUnpacking...");

//                 let file = BufReader::new(File::open(archive).unwrap());
//                 let file = flate2::bufread::GzDecoder::new(file);
//                 let mut file = tar::Archive::new(file);
//                 for x in file.entries().unwrap() {
//                     let mut x = x.unwrap();
//                     let path = x.path_bytes();
//                     let path = str::from_utf8(path.as_ref()).unwrap();
//                     if path.ends_with('/') {
//                         continue;
//                     }
//                     let i = path
//                         .char_indices()
//                         .find(|x| x.1 == '/')
//                         .map(|x| x.0)
//                         .unwrap();
//                     let path = &path[i + 1..];
//                     let path = Path::new(".cache/tools/java/").join(path);
//                     fs::create_dir_all(path.parent().unwrap()).unwrap();
//                     let mut file = File::create(&path).unwrap();
//                     loop {
//                         let len = x.read(&mut buf).unwrap();
//                         if len == 0 {
//                             break;
//                         }
//                         file.write_all(&buf[0..len]).unwrap();
//                     }
//                     file.flush().unwrap();
//                     let mut perms = fs::metadata(path).unwrap().permissions();
//                     if let Ok(x) = x.header().mode() {
//                         perms.set_mode(x);
//                     }
//                     file.set_permissions(perms).unwrap();
//                 }

//                 eprintln!("Installed Java in .cache/tools/java");

//                 fs::remove_file(archive).ok();
//                 unsafe { java_home.as_ptr().as_mut().unwrap_unchecked() }
//                     .replace(PathBuf::from(".cache/tools/java"));
//             }
//         },
//         &mut |path| {
//             drop(
//                 unsafe { java_home.as_ptr().as_mut().unwrap_unchecked() }
//                     .replace(path.parent().unwrap().parent().unwrap().to_path_buf()),
//             )
//         },
//     ),
//     (
//         "surrealdb",
//         &["surreal"],
//         &mut || {
//             if is_executable(".cache/tools/surrealdb/surreal".as_ref()) {
//                 unsafe { surrealdb.as_ptr().as_mut().unwrap_unchecked() }
//                     .replace(PathBuf::from(".cache/tools/surrealdb"));
//                 true
//             } else {
//                 false
//             }
//         },
//         &mut || {
//             #[cfg(unix)]
//             {
//                 eprintln!("Downloading SurrealDB");

//                 let mut resp = ureq::get("https://install.surrealdb.com/").call().unwrap();
//                 let mut reader = resp.body_mut().as_reader();
//                 let mut buf = [0; 16384];
//                 let mut process = Command::new("sh")
//                     .arg("-s")
//                     .arg(current_dir().unwrap().join(".cache/tools/surrealdb"))
//                     .stdin(Stdio::piped())
//                     .stdout(Stdio::inherit())
//                     .stderr(Stdio::inherit())
//                     .spawn()
//                     .unwrap();
//                 let mut stdin = process.stdin.take().unwrap();
//                 loop {
//                     let len = reader.read(&mut buf).unwrap();
//                     if len == 0 {
//                         break;
//                     }
//                     stdin.write(&buf[0..len]).unwrap();
//                 }
//                 stdin.flush().unwrap();
//                 drop(stdin);
//                 let code = process.wait().unwrap();
//                 if !code.success() {
//                     panic!("surrealdb installer exited with code {code}");
//                 }
//             }
//         },
//         &mut |path| *unsafe { surrealdb.as_ptr().as_mut().unwrap_unchecked() } = Some(path),
//     ),
//     (
//         "rabbitmq",
//         &["rabbitmq-server", "rabbitmqctl"],
//         &mut || {
//             if is_executable(".cache/tools/rabbitmq/sbin/rabbitmq-server".as_ref()) {
//                 unsafe { rabbitmq_home.as_ptr().as_mut().unwrap_unchecked() }
//                     .replace(PathBuf::from(".cache/tools/rabbitmq"));
//                 true
//             } else {
//                 false
//             }
//         },
//         &mut || {
//             #[cfg(unix)]
//             {
//                 eprintln!("Downloading RabbitMQ");

//                 let url = "https://github.com/rabbitmq/rabbitmq-server/releases/download/v4.1.2/rabbitmq-server-generic-unix-4.1.2.tar.xz";
//                 let archive = ".cache/tools/rabbitmq/archive.tar.xz";

//                 fs::create_dir_all(t::<&Path>(archive.as_ref()).parent().unwrap()).unwrap();
//                 let mut resp = ureq::get(url).call().unwrap();
//                 let max_len: usize = if stderr().is_terminal() {
//                     resp.headers()
//                         .get("content-length")
//                         .map(|x| x.to_str().unwrap().parse().unwrap())
//                         .unwrap()
//                 } else {
//                     0
//                 };
//                 let mut buf = [0; 16384];
//                 let mut body = resp.body_mut().as_reader();
//                 let mut file = File::create(archive).unwrap();
//                 if stderr().is_terminal() {
//                     eprint!("[          ] 0% (0/{max_len})");
//                     stderr().flush().unwrap();
//                 }
//                 let mut total_len = 0usize;
//                 loop {
//                     let len = body.read(&mut buf).unwrap();
//                     if len == 0 {
//                         break;
//                     }
//                     file.write_all(&buf[0..len]).unwrap();
//                     if stderr().is_terminal() {
//                         total_len += len;
//                         let perc = total_len.mul(100) / max_len;
//                         eprint!(
//                             "\r\x1b[K[{}{}] {perc}% ({:.02}/{:.02}MiB)",
//                             "#".repeat(perc.div(10)),
//                             " ".repeat(10 - perc.div(10)),
//                             total_len as f32 / 1024.0 / 1024.0,
//                             max_len as f32 / 1024.0 / 1024.0,
//                         );
//                         stderr().flush().unwrap();
//                     }
//                 }
//                 file.flush().unwrap();
//                 drop(file);
//                 drop(body);
//                 drop(resp);

//                 eprintln!("\nUnpacking...");

//                 let file = BufReader::new(File::open(archive).unwrap());
//                 let file = xz::bufread::XzDecoder::new(file);
//                 let mut file = tar::Archive::new(file);
//                 for x in file.entries().unwrap() {
//                     let mut x = x.unwrap();
//                     let path = x.path_bytes();
//                     let path = str::from_utf8(path.as_ref()).unwrap();
//                     if path.ends_with('/') {
//                         continue;
//                     }
//                     let i = path
//                         .char_indices()
//                         .find(|x| x.1 == '/')
//                         .map(|x| x.0)
//                         .unwrap();
//                     let path = &path[i + 1..];
//                     let path = Path::new(".cache/tools/rabbitmq/").join(path);
//                     fs::create_dir_all(path.parent().unwrap()).unwrap();
//                     let mut file = File::create(&path).unwrap();
//                     loop {
//                         let len = x.read(&mut buf).unwrap();
//                         if len == 0 {
//                             break;
//                         }
//                         file.write_all(&buf[0..len]).unwrap();
//                     }
//                     file.flush().unwrap();
//                     let mut perms = fs::metadata(path).unwrap().permissions();
//                     if let Ok(x) = x.header().mode() {
//                         perms.set_mode(x);
//                     }
//                     file.set_permissions(perms).unwrap();
//                 }

//                 eprintln!("Installed RabbitMQ in .cache/tools/rabbitmq");

//                 fs::remove_file(archive).ok();
//                 unsafe { rabbitmq_home.as_ptr().as_mut().unwrap_unchecked() }
//                     .replace(PathBuf::from(".cache/tools/rabbitmq"));
//             }
//         },
//         &mut |path| {
//             *unsafe { rabbitmq_home.as_ptr().as_mut().unwrap_unchecked() } =
//                 Some(path.parent().unwrap().parent().unwrap().to_path_buf())
//         },
//     ),
// ]) {
//     'c: for cmd in cmds.iter() {
//         if args.env_ty() != EnvTy::Isolate {
//             #[cfg(unix)]
//             let path = std::env::var("PATH").unwrap();
//             #[cfg(unix)]
//             let path = interject(path.split(':').map(|x| x.to_string()), |x, y| {
//                 if x.chars().rev().take_while(|x| x == &'\\').count() % 2 == 1 {
//                     let x = &x[0..x.len() - 1];
//                     (Some(format!("{x}:{y}")), None)
//                 } else {
//                     (Some(y), Some(x))
//                 }
//             });

//             for path in path.map(|x| PathBuf::from(x.as_str()).join(cmd)) {
//                 if is_executable(&path) {
//                     apply_host(path);
//                     continue 'c;
//                 }
//             }
//         }

//         if !try_apply_local() {
//             if match args.env_ty() {
//                 EnvTy::Host => {
//                     eprint!(
//                         "Could not find tool {tool}.\n\nInstall locally? (in .cache/tools/{tool})\n[yn] > "
//                     );
//                     stderr().flush().unwrap();
//                     stdin()
//                         .lines()
//                         .next()
//                         .is_some_and(|x| x.is_ok_and(|x| x.to_lowercase() == "y"))
//                 }
//                 EnvTy::Isolate | EnvTy::Autoinstall => true,
//             } {
//                 install();
//                 continue 't;
//             } else {
//                 eprintln!("Failed to create environment.");
//                 exit(1);
//             }
//         }
//     }
// }

// unsafe {
//     #[cfg(unix)]
//     {
//         let path = std::env::var("PATH").unwrap_or_default();
//         std::env::set_var(
//             "PATH",
//             format!(
//                 "{}:{}:{}:{path}",
//                 fs::canonicalize(java_home.get_mut().as_ref().unwrap().join("bin"))
//                     .unwrap()
//                     .to_str()
//                     .unwrap(),
//                 fs::canonicalize(surrealdb.get_mut().as_ref().unwrap())
//                     .unwrap()
//                     .to_str()
//                     .unwrap(),
//                 fs::canonicalize(rabbitmq_home.get_mut().as_ref().unwrap().join("sbin"))
//                     .unwrap()
//                     .to_str()
//                     .unwrap(),
//             ),
//         );
//     }
//     std::env::set_var("JAVA_HOME", java_home.get_mut().as_ref().unwrap());
// }

// match args {
//     Args::Help => unreachable!(),
//     Args::Build { build, .. } => {
//         if build.targets.iter().any(|x| x == "coreplugin") {
//             let dir = current_dir().unwrap().join("coreplugin");
//             if metadata("coreplugin").is_err() {
//                 if !Command::new("git")
//                     .arg("clone")
//                     .arg(build.git_backend.repo_url("Darkdustry-Coders/CorePlugin"))
//                     .arg(&dir)
//                     .spawn()
//                     .unwrap()
//                     .wait()
//                     .unwrap()
//                     .success()
//                 {
//                     panic!("git exited with a non-zero code");
//                 }
//             }

//             eprintln!(":coreplugin");
//             if !Command::new("./gradlew")
//                 .arg("build")
//                 .current_dir(&dir)
//                 .spawn()
//                 .unwrap()
//                 .wait()
//                 .unwrap()
//                 .success()
//             {
//                 exit(1);
//             }
//         }

//         if build.targets.iter().any(|x| x == "forts") {
//             let dir = current_dir().unwrap().join("forts");
//             if metadata("forts").is_err() {
//                 if !Command::new("git")
//                     .arg("clone")
//                     .arg(build.git_backend.repo_url("Darkdustry-Coders/Forts"))
//                     .arg(&dir)
//                     .spawn()
//                     .unwrap()
//                     .wait()
//                     .unwrap()
//                     .success()
//                 {
//                     panic!("git exited with a non-zero code");
//                 }
//             }

//             eprintln!(":forts");
//             if !Command::new("./gradlew")
//                 .arg("build")
//                 .current_dir(&dir)
//                 .spawn()
//                 .unwrap()
//                 .wait()
//                 .unwrap()
//                 .success()
//             {
//                 exit(1);
//             }
//         }
//     }
//     Args::Env { mut command, .. } => {
//         if command.is_empty() {
//             #[cfg(unix)]
//             if let Ok(x) = std::env::var("SHELL") {
//                 command.push(x);
//             }
//             #[cfg(target_os = "windows")]
//             command.push("cmd.exe".to_string());
//         }

//         let mut c = Command::new(command.remove(0));
//         let c = c
//             .args(command)
//             .stderr(Stdio::inherit())
//             .stdin(Stdio::inherit())
//             .stdout(Stdio::inherit());
//         #[cfg(unix)]
//         {
//             use std::os::unix::process::CommandExt;
//             panic!("execve() failed: {:#}", c.exec());
//         }
//         #[cfg(target_os = "windows")]
//         {
//             c.spawn().unwrap().wait().unwrap();
//         }
//     }
// }
