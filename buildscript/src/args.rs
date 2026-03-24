//! Command line argument parsing.
//!
//! This module handles parsing of command line arguments for the buildscript,
//! including build targets, environment settings, and various options.

use std::{collections::HashMap, path::PathBuf, process::exit, str::FromStr};

use crate::targets::TARGET_NAMES;

/// Supported Mindustry server versions.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum MindustryVersion {
    BleedingEdge,
    #[default]
    V156,
    V155,
    V154,
    V153,
    V150,
    V149,
    V146,
}
impl FromStr for MindustryVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "be" => Ok(MindustryVersion::BleedingEdge),
            "v156" => Ok(MindustryVersion::V156),
            "v155" => Ok(MindustryVersion::V155),
            "v154" => Ok(MindustryVersion::V154),
            "v153" => Ok(MindustryVersion::V153),
            "v150" => Ok(MindustryVersion::V150),
            "v149" => Ok(MindustryVersion::V149),
            "v146" => Ok(MindustryVersion::V146),
            _ => Err(()),
        }
    }
}

/// Command line parameters for build mode.
#[derive(Default)]
pub struct BuildArgs {
    /// Target Mindustry version for the build.
    pub mindustry_version: MindustryVersion,
    /// List of build targets to compile.
    pub targets: Vec<String>,

    /// Git backend to use for cloning repositories (SSH or HTTPS).
    pub git_backend: GitBackend,

    /// Starting port number for services.
    pub ports_start: u16,
    /// Server IP address for key authorization.
    pub server_ip: String,
    /// RabbitMQ connection URL (disables local RabbitMQ if set).
    pub rabbitmq_url: String,
    /// SurrealDB connection URL (disables local SurrealDB if set).
    pub surrealdb_url: String,

    /// Enable Java stacktrace output.
    pub java_stackstrace: bool,

    pub templates: HashMap<String, PathBuf>,
    pub keep_states: Vec<PathBuf>,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
/// Environment type for tool management.
pub enum EnvTy {
    /// Install all tools locally in workspace cache.
    Isolate,
    /// Automatically install missing tools locally.
    Autoinstall,
    /// Use host system tools.
    #[default]
    Host,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
/// Git backend protocol for cloning repositories.
pub enum GitBackend {
    /// SSH protocol (git@github.com:).
    Ssh,
    /// HTTPS protocol (https://github.com/) (default).
    #[default]
    Https,
}
impl GitBackend {
    /// Generates the full repository URL.
    ///
    /// # Arguments
    /// * `repo` - Repository path (e.g., "user/repo")
    ///
    /// # Returns
    /// Full repository URL
    pub fn repo_url(&self, repo: &str) -> String {
        match self {
            Self::Ssh => format!("git@github.com:{repo}"),
            Self::Https => format!("https://github.com/{repo}"),
        }
    }
}

/// Parsed command line arguments.
pub enum Args {
    /// Build command with build parameters and environment type.
    Build { build: BuildArgs, env: EnvTy },
    /// Run command in environment.
    #[allow(dead_code)]
    Env { command: Vec<String>, env: EnvTy },
    /// Show help message.
    Help,
}
impl Args {
    /// Returns the environment type for this command.
    #[allow(dead_code)]
    pub fn env_ty(&self) -> EnvTy {
        match self {
            Self::Help => EnvTy::Host,
            Self::Env { env, .. } => *env,
            Self::Build { env, .. } => *env,
        }
    }
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum TaskTy {
    #[default]
    Build,
    Env,
    Help,
}

/// Prints the help message with usage information.
pub fn print_help() {
    eprintln!("buildscript");
    eprintln!();
    eprintln!("Usage: [WPARAMS] [PARAMS] [TARGETS..]");
    eprintln!();
    eprintln!("Wrapper params:");
    eprintln!("\t--recompile-build-script");
    eprintln!("\t--isolate          - force install local tools");
    eprintln!("\t--autoinstall      - install missing tools locally automatically");
    eprintln!();
    eprintln!("Params:");
    eprintln!("\t--env [CMD..]      - run command within local environment");
    eprintln!("\t--help             - print this message");
    eprintln!("\t--ssh              - use ssh instead of https when pulling repos");
    eprintln!();
    eprintln!("Extra params for build:");
    eprintln!("\t--mindustry [VER]  - set mindustry version (v154 by default)");
    eprintln!("\t--run              - run targets");
    eprintln!("\t--stacktrace       - pass '--stacktrace' to gradle");
    eprintln!("\t--server-ip [IP]   - set ip used for key authorization");
    eprintln!("\t--rabbbitmq [URL]  - set rabbitmq url");
    eprintln!("\t                     Also disables installing and running RabbitMQ");
    eprintln!("\t--surrealdb [URL]  - set surrealdb url");
    eprintln!("\t                     Also disables installing and running SurrealDB");
    eprintln!("\t--keep      [PATH] - keep path intact (relative to `.run`)");
    eprintln!();
    eprintln!("Available targets:");
    for x in TARGET_NAMES {
        eprintln!("\t{x}");
    }
    eprintln!("note: targets are always compiled in this exact order.");
    eprintln!();
    eprintln!("Special targets:");
    eprintln!("\tall                - enable all targets (except for deprecated ones)");
    eprintln!("\trun                - run all targets");
    eprintln!();
    eprintln!(
        "By default, calling build script with or without args creates editor configurations."
    );
}

pub fn args() -> Args {
    let mut env = EnvTy::Host;
    let mut task_ty = TaskTy::Build;
    let mut template: HashMap<String, PathBuf> = HashMap::new();
    let mut argv = std::env::args().peekable();
    argv.next();

    // Core args
    while let Some(x) = argv.peek() {
        match x.as_str() {
            "--isolate" => env = EnvTy::Isolate,
            "--autoinstall" => {
                if env != EnvTy::Isolate {
                    env = EnvTy::Autoinstall
                }
            }
            "--env" => {
                if task_ty != TaskTy::Help {
                    task_ty = TaskTy::Env
                }
            }
            str if str.starts_with("--template=") => template.extend(
                str.strip_prefix("--template=")
                    .unwrap()
                    .split(":")
                    .map(|it| it.split("="))
                    .map(|mut it| {
                        (
                            it.next().unwrap().into(),
                            it.next().unwrap().parse().unwrap(),
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => break,
        }
        argv.next();
    }

    while let Some(x) = argv.peek() {
        match x.as_str() {
            "--isolate" | "--autoinstall" | "--env" => {
                exit(1);
            }
            str if str.starts_with("--template") => {
                exit(1);
            }
            "--help" => {
                task_ty = TaskTy::Help;
            }
            _ => break,
        }
        argv.next();
    }

    match task_ty {
        TaskTy::Help => Args::Help,
        TaskTy::Build => {
            let mut build = BuildArgs::default();
            build.templates = template;
            let mut errors = vec![];

            build.ports_start = 4100;

            while let Some(x) = argv.next() {
                if let Some(x) = x.strip_prefix("--") {
                    match x {
                        "ssh" => {
                            build.git_backend = GitBackend::Ssh;
                        }
                        "stacktrace" => {
                            build.java_stackstrace = true;
                        }
                        "mindustry" => {
                            if let Some(x) = argv.next() {
                                if let Ok(x) = MindustryVersion::from_str(&x) {
                                    build.mindustry_version = x;
                                } else {
                                    errors.push(format!("--mindustry: invalid argument {x:?}"));
                                }
                            } else {
                                errors.push("--mindustry: no value specified".to_string());
                            }
                        }
                        "server-ip" => {
                            if let Some(x) = argv.next() {
                                build.server_ip = x;
                            } else {
                                errors.push("--server-ip: no value specified".to_string());
                            }
                        }
                        "rabbitmq" => {
                            if let Some(x) = argv.next() {
                                build.rabbitmq_url = x;
                            } else {
                                errors.push("--rabbitmq: no value specified".to_string());
                            }
                        }
                        "keep" => {
                            if let Some(x) = argv.next() {
                                build.keep_states.push(x.into());
                            } else {
                                errors.push("--rabbitmq: no value specified".to_string());
                            }
                        }
                        x => errors.push(format!("unknown option {:?}", format!("--{x}"))),
                    }
                } else if let Some(x) = x.strip_prefix("-") {
                    _ = x;
                } else {
                    build.targets.push(x.to_string());
                }
            }

            if !errors.is_empty() {
                for x in errors {
                    eprintln!("{x}");
                }
                exit(1);
            }

            Args::Build { build, env }
        }
        TaskTy::Env => {
            let command: Vec<String> = argv.collect();
            Args::Env { command, env }
        }
    }
}
