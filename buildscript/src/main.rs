mod args;
mod fs2;
mod syncfs;
mod targets;
mod util;

use std::{
    borrow::Cow,
    env::current_dir,
    fs,
    path::PathBuf,
    process::{Command, Stdio, exit},
    str::FromStr,
};

use args::{Args, EnvTy};
use targets::{BuildParams, InitParams, RunParams, TARGET_NAMES, Target, TargetList, Targets};
use util::CURRENT_DIR;

use crate::util::write_if_diff;

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
            write_if_diff(
                "buildscript/assets/shared.settings.gradle",
                fs::read_to_string("buildscript/assets/shared.settings.gradle.in")
                    .unwrap()
                    .replace("MINDUSTRY_PKGS", match build.mindustry_version {
                        args::MindustryVersion::V146 => r#"
                                                        library("mindustry-arc", "com.github.5GameMaker.ArcV7", "arc-core").version("v146")
                                                        library("mindustry-core", "com.github.5GameMaker.MindustryV7", "core").version("v146")
                                                        library("mindustry-server", "com.github.5GameMaker.MindustryV7", "server").version("v146")
                                                        "#,
                        args::MindustryVersion::V149 => r#"
                                                        library("mindustry-arc", "com.github.Anuken.Arc", "arc-core").version("v149")
                                                        library("mindustry-core", "com.github.Anuken.Mindustry", "core").version("v149")
                                                        library("mindustry-server", "com.github.Anuken.Mindustry", "server").version("v149")
                                                        "#,
                        args::MindustryVersion::V150 => r#"
                                                        library("mindustry-arc", "com.github.Anuken.Arc", "arc-core").version("v150")
                                                        library("mindustry-core", "com.github.Anuken.Mindustry", "core").version("v150")
                                                        library("mindustry-server", "com.github.Anuken.Mindustry", "server").version("v150")
                                                        "#,
                        args::MindustryVersion::V153 => r#"
                                                        library("mindustry-arc", "com.github.Anuken.Arc", "arc-core").version("v153")
                                                        library("mindustry-core", "com.github.Anuken.Mindustry", "core").version("v153")
                                                        library("mindustry-server", "com.github.Anuken.Mindustry", "server").version("v153")
                                                        "#,
                        args::MindustryVersion::V154 => r#"
                                                        library("mindustry-arc", "com.github.Anuken.Arc", "arc-core").version("v154")
                                                        library("mindustry-core", "com.github.Anuken.Mindustry", "core").version("v154")
                                                        library("mindustry-server", "com.github.Anuken.Mindustry", "server").version("v154")
                                                        "#,
                        args::MindustryVersion::V155 => r#"
                                                        library("mindustry-arc", "com.github.Anuken.Arc", "arc-core").version("v155")
                                                        library("mindustry-core", "com.github.Anuken.Mindustry", "core").version("v155")
                                                        library("mindustry-server", "com.github.Anuken.Mindustry", "server").version("v155")
                                                        "#,
                        args::MindustryVersion::V156 => r#"
                                                        library("mindustry", "Anuken", "Mindustry").version("v156")
                                                        "#,
                        args::MindustryVersion::V157 => r#"
                                                        library("mindustry", "Anuken", "Mindustry").version("v157")
                                                        "#,
                        args::MindustryVersion::BleedingEdge => r#"
                                                                library("mindustry", "Anuken", "MindustryBuilds").version("latest")
                                                                "#,
                    }).replace("MINDUSTRY_BUNDLE", match build.mindustry_version {
                             args::MindustryVersion::V146 |
                             args::MindustryVersion::V149 |
                             args::MindustryVersion::V150 |
                             args::MindustryVersion::V153 |
                             args::MindustryVersion::V154 |
                             args::MindustryVersion::V155 => r#"
                                                             bundle("mindustry", ["mindustry-arc", "mindustry-core", "mindustry-server"])
                                                             "#,
                             args::MindustryVersion::V156 |
                             args::MindustryVersion::V157 |
                             args::MindustryVersion::BleedingEdge => r#"
                                                                     bundle("mindustry", ["mindustry"])
                                                                     "#,
                    }),
            ).unwrap();
            write_if_diff(
                ".cache/tools/buildscript/shared.repos.gradle",
                fs::read_to_string("buildscript/assets/shared.repos.gradle.in").unwrap().replace("MINDUSTRY_REPO", match build.mindustry_version {
                             args::MindustryVersion::V146 |
                             args::MindustryVersion::V149 |
                             args::MindustryVersion::V150 |
                             args::MindustryVersion::V153 |
                             args::MindustryVersion::V154 |
                             args::MindustryVersion::V155 => r#"
                                                             maven { url 'https://jitpack.io' }
                                                             maven { url 'https://raw.githubusercontent.com/Zelaux/MindustryRepo/master/repository' }
                                                             "#,
                             args::MindustryVersion::V156 | args::MindustryVersion::V157 => r#"
                                                             ivy { url 'https://github.com/'
                                                                   patternLayout { artifact '/[organisation]/[module]/releases/download/[revision]/server-release.jar' }
                                                                   metadataSources { artifact() } }
                                                             "#,
                             args::MindustryVersion::BleedingEdge => r#"
                                                                     ivy { url 'https://github.com/'
                                                                           patternLayout { artifact '/[organisation]/[module]/releases/download/master/[revision].jar' }
                                                                           metadataSources { artifact() } }
                                                                     "#,
                    }),
            )
            .unwrap();
            // write_if_diff(
            //     "buildscript/assets/shared.settings.gradle",
            //     include_str!("../assets/shared.settings.gradle.in")
            //         .replace(
            //             "MINDUSTRY_VERSION",
            //             match &build.mindustry_version {
            //                 args::MindustryVersion::V146 => "v146.8",
            //                 args::MindustryVersion::V149 => "v149",
            //                 args::MindustryVersion::V150 => "v150",
            //                 args::MindustryVersion::V153 => "v153",
            //                 args::MindustryVersion::V154 => "v154",
            //                 args::MindustryVersion::V155 => "v155",
            //                 args::MindustryVersion::V156 => "v156",
            //                 args::MindustryVersion::BleedingEdge => "v155",
            //             },
            //         )
            //         .replace(
            //             "MINDUSTRY_REPO",
            //             match &build.mindustry_version {
            //                 args::MindustryVersion::V146 => "v146.8",
            //                 args::MindustryVersion::V149 => "v149",
            //                 args::MindustryVersion::V150 => "v150",
            //                 args::MindustryVersion::V153 => "v153",
            //                 args::MindustryVersion::V154 => "v154",
            //                 args::MindustryVersion::V155 => "v155",
            //                 args::MindustryVersion::BleedingEdge => "v155",
            //             },
            //         )
            //         .replace(
            //             "MINDUSTRY_PKG_ARC",
            //             match build.mindustry_version {
            //                 args::MindustryVersion::V146 => "com.github.5GameMaker.ArcV7",
            //                 args::MindustryVersion::V149
            //                 | args::MindustryVersion::V150
            //                 | args::MindustryVersion::V153
            //                 | args::MindustryVersion::V154 => "com.github.Anuken.Arc",
            //                 args::MindustryVersion::BleedingEdge =>
            //             },
            //         )
            //         .replace(
            //             "MINDUSTRY_PKG_MINDUSTRY",
            //             if build.mindustry_version == args::MindustryVersion::V146 {
            //                 "com.github.5GameMaker.MindustryV7"
            //             } else {
            //                 "com.github.Anuken.Mindustry"
            //             },
            //         ),
            // )
            // .unwrap();
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
                for x in &params.java_workspace_members {
                    s += &format!("\nincludeBuild '{x}'");
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
