use std::process::exit;

use crate::targets::TARGET_NAMES;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum MindustryVersion {
    BleedingEdge,
    #[default]
    V154,
    V153,
    V150,
    V149,
    V146,
}

/// Command line parameters for build mode.
#[derive(Default)]
pub struct BuildArgs {
    pub mindustry_version: MindustryVersion,
    pub targets: Vec<String>,

    pub git_backend: GitBackend,

    pub ports_start: u16,

    pub java_stackstrace: bool,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum EnvTy {
    /// Install all tools locally.
    Isolate,
    /// Automatically install tools if not present.
    Autoinstall,
    /// Use host system tools.
    #[default]
    Host,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum GitBackend {
    Ssh,
    #[default]
    Https,
}
impl GitBackend {
    pub fn repo_url(&self, repo: &str) -> String {
        match self {
            Self::Ssh => format!("git@github.com:{repo}"),
            Self::Https => format!("https://github.com/{repo}"),
        }
    }
}

pub enum Args {
    Build { build: BuildArgs, env: EnvTy },
    Env { command: Vec<String>, env: EnvTy },
    Help,
}
impl Args {
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
    eprintln!("\t--mindustry [VER]  - set mindustry version (v7 by default)");
    eprintln!("\t--run              - run targets");
    eprintln!("\t--clean            - clean temporary files");
    eprintln!("\t--pack             - build a package");
    eprintln!("\t--stacktrace       - pass '--stacktrace' to gradle");
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
            _ => break,
        }
        argv.next();
    }

    while let Some(x) = argv.peek() {
        match x.as_str() {
            "--isolate" | "--autoinstall" | "--env" => {
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
                        "mindustry" => {}
                        x => errors.push(format!("unknown option {:?}", format!("--{x}"))),
                    }
                } else if let Some(x) = x.strip_prefix("-") {
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
