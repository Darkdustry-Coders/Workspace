mod args;
mod fs2;
mod syncfs;
mod targets;
mod util;

use std::{
    borrow::Cow,
    fs,
    path::PathBuf,
    process::{Command, Stdio, exit},
    str::FromStr,
};

use args::{Args, EnvTy};
use targets::{BuildParams, InitParams, RunParams, TARGET_NAMES, Target, TargetList, Targets};
use util::CURRENT_DIR;

use crate::util::{current_dir, strip_extras, write_if_diff};

fn main() {
    unsafe {
        CURRENT_DIR = Some(std::env::current_dir().unwrap());
    }

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
        std::env::set_var("WORKSPACE", current_dir());
        std::env::set_var("MINDURKA_WORKSPACE", current_dir());
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
            write_if_diff(
                "buildscript/assets/shared.settings.gradle",
                fs::read_to_string("buildscript/assets/shared.settings.gradle.in")
                    .unwrap()
                    .replace(
                        "PKGS",
                        &strip_extras(
                            r#"
                            library("mindustry-core", "anuken.mindustry", "core").version("release")
                            library("mindustry-server", "anuken.mindustry", "server").version("release")
                            library("arc-core", "anuken.arc", "arc-core").version("1.0")
                            "#,
                            12,
                        ),
                    )
                    .replace(
                        "BUNDLES",
                        &strip_extras(
                            r#"
                            bundle("mindustry", ["mindustry-core", "mindustry-server", "arc-core"])
                            "#,
                            12,
                        ),
                    ),
            )
            .unwrap();
            write_if_diff(
                ".cache/tools/buildscript/shared.repos.gradle",
                // TODO: Windows.
                fs::read_to_string("buildscript/assets/shared.repos.gradle.in")
                    .unwrap()
                    .replace("WORKSPACE_PATH", current_dir().to_str().unwrap()),
            )
            .unwrap();
            write_if_diff(
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
            write_if_diff("settings.gradle", {
                let mut s = String::new();
                s += include_str!("../assets/settings.gradle.in");
                s += "def inWorkspace = System.env['MINDURKA_WORKSPACE'] != null";
                for x in &params.java_workspace_members {
                    s += &format!("\nincludeBuild '{x}'");
                }
                for x in &params.java_masked_members {
                    s += &format!("\nif (!inWorkspace) includeBuild '{x}'");
                }
                s
            })
            .unwrap();

            let mut params = BuildParams::new(params, &build);

            if env != EnvTy::Isolate {
                params.path.extend(
                    std::env::var("PATH")
                        .unwrap()
                        .split(if cfg!(unix) { ':' } else { ';' })
                        .map(PathBuf::from),
                );
            }

            targets.build_all(&mut params);

            if run {
                let mut params = RunParams::new(params, &build);

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

                params.run.restore(".run-save", "");
                targets.run_init_all(&mut params);

                if let Some(rabbitmq) = targets.rabbitmq.as_ref()
                    && let Some(surreal) = targets.surrealdb.as_ref()
                {
                    params.run.write(
                        "sharedConfig.toml",
                        format!(
                            "serverIp = {:?}\nrabbitMqUrl = {:?}\nsurrealDbUrl = {:?}",
                            if build.server_ip.is_empty() {
                                "127.0.0.1"
                            } else {
                                build.server_ip.as_str()
                            },
                            if build.rabbitmq_url.is_empty() {
                                Cow::Owned(rabbitmq.url())
                            } else {
                                Cow::Borrowed(build.rabbitmq_url.as_str())
                            },
                            if build.surrealdb_url.is_empty() {
                                Cow::Owned(surreal.url())
                            } else {
                                Cow::Borrowed(build.surrealdb_url.as_str())
                            },
                        ),
                    );
                }

                params.run.finalize().unwrap();
                params.run.clear();

                targets.run_all(&mut params);

                if !targets.mprocs.as_mut().unwrap().wait() {
                    eprintln!("mprocs exited with a non-zero code");
                    exit(1);
                }
            }
        }
    }
}
