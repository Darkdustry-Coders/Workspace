use std::{
    any::Any,
    collections::HashMap,
    ffi::{OsStr, OsString},
    io::{Write, stderr, stdin},
    ops::{Deref, DerefMut},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, exit},
    str::FromStr,
};

use crate::{
    args::{BuildArgs, EnvTy, GitBackend, MindustryVersion},
    util::current_dir,
};

/// List of target flags.
///
/// ## Version stability.
/// Flags may be added at any time.
///
/// To ensure stability, make sure to always expand defaults when
/// setting flags.
///
/// ```rust
/// TargetFlags {
///     always_local: true,
///     deprecated: false,
///     ..Default::default(),
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct TargetFlags {
    /// A target must always be installed in workspace.
    ///
    /// Intended to be used for plugins.
    pub always_local: bool,
    /// A target is to be removed in the future.
    pub deprecated: bool,
}
impl TargetFlags {
    #[allow(unused)]
    pub const fn new() -> TargetFlags {
        TargetFlags {
            always_local: false,
            deprecated: false,
        }
    }

    #[allow(unused)]
    pub const fn always_local(mut self) -> TargetFlags {
        self.always_local = true;
        self
    }

    #[allow(unused)]
    pub const fn deprecated(mut self) -> TargetFlags {
        self.deprecated = true;
        self
    }
}
impl Default for TargetFlags {
    fn default() -> Self {
        Self {
            always_local: true,
            deprecated: false,
        }
    }
}

/// Build state of a target.
#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum TargetEnabled {
    /// Target is disabled for this build.
    #[default]
    No,

    /// Target is a dependency of another target.
    ///
    /// This should download the latest binary release.
    ///
    /// If the target has already been built from source, reuse the latest artifact.
    Depend,

    /// Target is set to build.
    ///
    /// This should download the repo and build it from source.
    ///
    /// For binary targets has the same effect as [TargetEnabled::Depend].
    Build,
}
impl TargetEnabled {
    /// Upgrades the enabled state to a higher priority.
    ///
    /// Priority order: No < Depend < Build
    /// # Arguments
    /// * `enabled` - New state to upgrade to
    pub fn upgrade(&mut self, enabled: TargetEnabled) {
        *self = match (*self, enabled) {
            (Self::No, x) => x,
            (Self::Depend, Self::Build) => Self::Build,
            (x, _) => x,
        }
    }
}

/// Base trait for targets.
pub trait TargetImpl: Any {
    /// Build target.
    fn build(&mut self, deps: Targets<'_>, params: &mut BuildParams);

    /// Initialize run target.
    fn run_init(&mut self, deps: Targets<'_>, params: &mut RunParams) {
        _ = (deps, params)
    }

    /// Run target.
    fn run(&mut self, deps: Targets<'_>, params: &mut RunParams) {
        _ = (deps, params)
    }
}
pub trait TargetImplStatic: TargetImpl
where
    Self: 'static + Sized,
{
    /// Flags for this target.
    ///
    /// Tools should always override this method.
    fn flags() -> TargetFlags {
        TargetFlags::default()
    }
    /// Enable dependencies for target.
    fn depends(list: &mut TargetList) {
        _ = list
    }

    /// Initialize target using host tools.
    fn initialize_host(
        enabled: TargetEnabled,
        deps: Targets<'_>,
        params: &mut InitParams,
    ) -> Option<Self>;
    /// Try to initialize target locally.
    ///
    /// This will only attempt to use already available data.
    fn initialize_cached(
        enabled: TargetEnabled,
        deps: Targets<'_>,
        params: &mut InitParams,
    ) -> Option<Self>;
    /// Initialize target locally.
    fn initialize_local(enabled: TargetEnabled, deps: Targets<'_>, params: &mut InitParams)
    -> Self;
}
/// Extension trait for downcasting target implementations.
#[allow(dead_code)]
pub trait TargetImplExt: TargetImpl {
    /// Attempts to cast to a specific target type.
    fn try_as<T: TargetImpl>(&self) -> Option<&T>;
    /// Attempts to cast to a mutable specific target type.
    fn try_as_mut<T: TargetImpl>(&mut self) -> Option<&mut T>;
}
impl<I: TargetImpl> TargetImplExt for I {
    fn try_as<T: TargetImpl>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref()
    }

    fn try_as_mut<T: TargetImpl>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut()
    }
}

/// Smart pointer that can hold either a borrowed or owned value.
pub enum BorrowedMut<'a, T> {
    Borrowed(&'a mut T),
    Owned(T),
}
impl<'a, T> BorrowedMut<'a, T> {
    /// Creates a new owned BorrowedMut.
    pub fn new_owned(t: T) -> Self {
        Self::Owned(t)
    }

    /// Creates a new borrowed BorrowedMut.
    pub fn new_borrowed(t: &'a mut T) -> Self {
        Self::Borrowed(t)
    }
}
impl<'a, T> Deref for BorrowedMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(x) => x,
            Self::Owned(x) => x,
        }
    }
}
impl<'a, T> DerefMut for BorrowedMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Borrowed(x) => x,
            Self::Owned(x) => x,
        }
    }
}
impl<'a, T> AsRef<T> for BorrowedMut<'a, T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Borrowed(x) => x,
            Self::Owned(x) => x,
        }
    }
}
impl<'a, T> AsMut<T> for BorrowedMut<'a, T> {
    fn as_mut(&mut self) -> &mut T {
        match self {
            Self::Borrowed(x) => x,
            Self::Owned(x) => x,
        }
    }
}

/// Parameters for target initialization.
pub struct InitParams {
    /// Git backend for cloning repositories.
    pub git_backend: GitBackend,
    /// Target Mindustry version.
    pub mindustry_version: MindustryVersion,
    /// List of Rust workspace members to add.
    pub rust_workspace_members: Vec<String>,
    /// List of Java workspace members to add.
    pub java_workspace_members: Vec<String>,
    /// Root path of the workspace.
    pub root: PathBuf,
    /// Whether RabbitMQ is hosted externally.
    pub host_rabbitmq: bool,
    /// Whether SurrealDB is hosted externally.
    pub host_surrealdb: bool,
}
impl InitParams {
    /// Creates new initialization parameters from build arguments.
    pub fn new(args: &BuildArgs) -> Self {
        Self {
            git_backend: args.git_backend,
            mindustry_version: args.mindustry_version,
            rust_workspace_members: Default::default(),
            java_workspace_members: Default::default(),
            root: current_dir().to_path_buf(),
            host_rabbitmq: !args.rabbitmq_url.is_empty(),
            host_surrealdb: !args.surrealdb_url.is_empty(),
        }
    }
}

/// Parameters for the build phase.
#[allow(dead_code)]
pub struct BuildParams {
    /// Git backend for cloning repositories.
    pub git_backend: GitBackend,
    /// Target Mindustry version.
    pub mindustry_version: MindustryVersion,
    /// Environment variables to set during build.
    pub env: HashMap<OsString, OsString>,
    /// PATH directories for build commands.
    pub path: Vec<PathBuf>,
    /// Root path of the workspace.
    pub root: PathBuf,
    /// Enable Java stacktrace output.
    pub java_stacktrace: bool,
    /// Whether RabbitMQ is hosted externally.
    pub host_rabbitmq: bool,
    /// Whether SurrealDB is hosted externally.
    pub host_surrealdb: bool,
}
impl BuildParams {
    /// Creates new build parameters from initialization parameters and arguments.
    pub fn new(params: InitParams, args: &BuildArgs) -> Self {
        Self {
            git_backend: params.git_backend,
            mindustry_version: params.mindustry_version,
            env: Default::default(),
            path: Default::default(),
            root: params.root,
            java_stacktrace: args.java_stackstrace,
            host_rabbitmq: !args.rabbitmq_url.is_empty(),
            host_surrealdb: !args.surrealdb_url.is_empty(),
        }
    }

    /// Create a [Command] with correctly set up environment.
    ///
    /// This does not set anything except for environment
    /// variables.
    ///
    /// It's recommended to always pass absolute paths to `cmd`
    /// as the requested application may not be available on `PATH`.
    pub fn cmd(&mut self, cmd: impl AsRef<OsStr>) -> Command {
        let mut cmd = Command::new(cmd);
        let mut path = OsString::with_capacity(
            self.path
                .iter()
                .map(|x| x.as_os_str().len() + 1)
                .sum::<usize>()
                .max(1)
                - 1,
        );
        for (i, x) in self.path.iter().enumerate() {
            if i != 0 {
                path.push(if cfg!(unix) { ":" } else { ";" });
            }
            path.push(x);
        }
        cmd.envs(&self.env);
        cmd.env("PATH", path);
        cmd
    }

    /// Creates a gradle command with appropriate wrapper.
    pub fn gradle(&mut self) -> Command {
        let gradle = {
            #[cfg(unix)]
            {
                current_dir().join("gradlew")
            }
            #[cfg(target_os = "windows")]
            {
                current_dir().join("gradlew.bat")
            }
        };

        let mut cmd = self.cmd(gradle);
        if self.java_stacktrace {
            cmd.arg("--stacktrace");
        }
        cmd
    }

    pub fn cargo(&mut self) -> Command {
        let cargo = self.path.iter().find_map(|path| {
            if path.is_dir()
                && let Ok(mut read_dir) = path.read_dir()
            {
                read_dir.find_map(|member| {
                    if let Ok(member) = member
                        && let Ok(member_file_type) = member.file_type()
                        && member_file_type.is_file()
                        && member.file_name() == "cargo"
                    {
                        #[cfg(target_os = "linux")]
                        {
                            Some(member.path())
                        }
                        #[cfg(target_os = "windows")]
                        {
                            Some(member.path())
                        }
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        });

        match cargo {
            Some(cargo) => self.cmd(cargo),
            None => {
                eprintln!("Can't find cargo on you system. Do you have correctly installed rust?");
                exit(1);
            }
        }
    }
}

/// Parameters for the run phase.
pub struct RunParams {
    /// Environment variables for running processes.
    pub env: HashMap<OsString, OsString>,
    /// PATH directories for running commands.
    pub path: Vec<PathBuf>,
    /// Next available port number.
    pub port: u16,
    /// Root path of the workspace.
    pub root: PathBuf,
    /// Whether RabbitMQ is hosted externally.
    pub host_rabbitmq: bool,
    /// Whether SurrealDB is hosted externally.
    pub host_surrealdb: bool,

    pub templates: HashMap<String, PathBuf>,
}
impl RunParams {
    /// Creates new run parameters from build parameters and arguments.
    pub fn new(params: BuildParams, args: &BuildArgs) -> Self {
        Self {
            env: params.env,
            path: params.path,
            port: args.ports_start,
            root: params.root,
            host_rabbitmq: !args.rabbitmq_url.is_empty(),
            host_surrealdb: !args.surrealdb_url.is_empty(),
            templates: args.templates.clone(),
        }
    }

    /// Returns the next available port and increments the counter.
    pub fn next_port(&mut self) -> u16 {
        let port = self.port;
        self.port += 1;
        port
    }

    pub fn cmd(&mut self, cmd: impl AsRef<OsStr>) -> Command {
        let mut cmd = Command::new(cmd);
        let mut path = OsString::with_capacity(
            self.path
                .iter()
                .map(|x| x.as_os_str().len() + 1)
                .sum::<usize>()
                .max(1)
                - 1,
        );
        for (i, x) in self.path.iter().enumerate() {
            if i != 0 {
                path.push(if cfg!(unix) { ":" } else { ";" });
            }
            path.push(x);
        }
        cmd.envs(&self.env);
        cmd.env("PATH", path);
        cmd
    }
    pub fn cargo(&mut self) -> Command {
        let cargo = self.path.iter().find_map(|path| {
            if path.is_dir()
                && let Ok(mut read_dir) = path.read_dir()
            {
                read_dir.find_map(|member| {
                    if let Ok(member) = member
                        && let Ok(member_file_type) = member.file_type()
                        && member_file_type.is_file()
                        && member.file_name() == "cargo"
                    {
                        #[cfg(target_os = "linux")]
                        {
                            Some(member.path())
                        }
                        #[cfg(target_os = "windows")]
                        {
                            Some(member.path())
                        }
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        });

        match cargo {
            Some(cargo) => self.cmd(cargo),
            None => {
                eprintln!("Can't find cargo on you system. Do you have correctly installed rust?");
                exit(1);
            }
        }
    }
}

macro_rules! targets {
    ($($(#[$doc:meta])* $name:ident: $enumname: ident);* $(;)?) => {
        $(
            $(#[$doc])*
            pub mod $name;
        )*

        /// List of targets.
        #[derive(Default, PartialEq, Eq, Clone, Copy)]
        pub struct TargetList {$(
            $(#[$doc])*
            $name: TargetEnabled,
        )*}
        impl TargetList {
            pub fn set_build(&mut self, target: Target) {
                match target {$(
                    Target::$enumname => {
                        self.$name = TargetEnabled::Build;
                        $name::Impl::depends(self);
                    }
                )*}
            }
            pub fn set_depend(&mut self, target: Target) {
                match target {$(
                    Target::$enumname => {
                        self.$name.upgrade(TargetEnabled::Depend);
                        $name::Impl::depends(self);
                    }
                )*}
            }
        }

        /// Named target.
        #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
        pub enum Target {$(
            $(#[$doc])*
            $enumname,
        )*}
        impl Target {
            pub fn flags(&self) -> TargetFlags {
                match self {
                    $(Self::$enumname => $name::Impl::flags(),)*
                }
            }
        }
        impl FromStr for Target {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, ()> {
                match s {
                    $(stringify!($name) => Ok(Target::$enumname),)*
                    _ => Err(()),
                }
            }
        }

        #[derive(Default)]
        pub struct Targets<'a> {
            $(pub $name: Option<BorrowedMut<'a, $name::Impl>>,)*
        }
        impl<'a> Targets<'a> {
            #[allow(unused)]
            pub fn target(&self, target: Target) -> Option<&dyn TargetImpl> {
                match target {$(
                    Target::$enumname => self.$name.as_ref().map(|x| x.as_ref() as &dyn TargetImpl),
                )*}
            }
            pub fn target_mut(&mut self, target: Target) -> Option<&mut dyn TargetImpl> {
                match target {$(
                    Target::$enumname => self.$name.as_mut().map(|x| x.as_mut() as &mut dyn TargetImpl),
                )*}
            }

            pub fn target_deps<'b>(&mut self, target: Target) -> (Option<&'b mut dyn TargetImpl>, Targets<'b>)
            where 'a: 'b
            { unsafe {
                let Some(build_target) = (self as *mut Self).as_mut().unwrap_unchecked().target_mut(target)
                else {
                    return (None, Targets {
                        $($name: match &mut (self as *mut Self).as_mut().unwrap_unchecked().$name {
                            None => None,
                            Some(x) => Some(BorrowedMut::new_borrowed(x)),
                        },)*
                    });
                };

                (
                    Some(build_target),
                    Targets {
                        $($name: if target != Target::$enumname {
                            match &mut (self as *mut Self).as_mut().unwrap_unchecked().$name {
                                None => None,
                                Some(x) => Some(BorrowedMut::new_borrowed(x)),
                            }
                        } else { None },)*
                    },
                )
            } }

            pub fn init_all(&mut self, env: EnvTy, recipe: &mut TargetList, params: &mut InitParams) {
                $(
                    self.$name = 'a: {
                        if recipe.$name == TargetEnabled::No {
                            break 'a None;
                        }

                        if env == EnvTy::Isolate || $name::Impl::flags().always_local ||
                            match $name::Impl::initialize_host(
                                recipe.$name,
                                self.target_deps(Target::$enumname).1,
                                params,
                            ) {
                                Some(x) => break 'a Some(BorrowedMut::new_owned(x)),
                                None => true,
                            } {
                            if let Some(x) = $name::Impl::initialize_cached(
                                recipe.$name,
                                self.target_deps(Target::$enumname).1,
                                params,
                            ) {
                                break 'a Some(BorrowedMut::new_owned(x));
                            }

                            if env == EnvTy::Host && !$name::Impl::flags().always_local {
                                eprintln!();
                                eprintln!("Could not find tool {:?}", stringify!($name));
                                eprintln!();
                                eprintln!("Install in $WORKSPACE/.cache?");
                                eprint!("[yn] >");
                                stderr().flush().ok();
                                if stdin().lines().next()
                                    .is_some_and(|x|
                                        x.is_ok_and(|x| !x.to_lowercase().starts_with("y"))) {
                                    exit(1);
                                }
                            }

                            Some(BorrowedMut::new_owned($name::Impl::initialize_local(
                                recipe.$name,
                                self.target_deps(Target::$enumname).1,
                                params,
                            )))
                        } else {
                            unreachable!();
                        }
                    };
                )*
            }

            pub fn build_all(&mut self, params: &mut BuildParams) {
                $(
                    if let (Some(x), targets) = self.target_deps(Target::$enumname) {
                        x.build(targets, params);
                    }
                )*
            }

            pub fn run_init_all(&mut self, params: &mut RunParams) {
                $(
                    if let (Some(x), targets) = self.target_deps(Target::$enumname) {
                        x.run_init(targets, params);
                    }
                )*
            }

            pub fn run_all(&mut self, params: &mut RunParams) {
                $(
                    if let (Some(x), targets) = self.target_deps(Target::$enumname) {
                        x.run(targets, params);
                    }
                )*
            }
        }

        /// Names of targets.
        pub const TARGET_NAMES: &[&str] = &[$(stringify!($name)),*];
    };
}

targets! {
    /// Mprocs task runner.
    mprocs: MProcs;

    /// OS-specific coreutils.
    coreutils: CoreUtils;
    /// RabbitMQ message queue.
    rabbitmq: RabbitMq;
    /// SurrealDB database.
    surrealdb: SurrealDb;
    /// Mindustry server.
    mindustry: Mindustry;

    /// Java programming language.
    java: Java;

    /// Mindurka's core plugin.
    coreplugin: CorePlugin;
    /// Forts plugin.
    forts: Forts;
    /// Hub plugin.
    hub: Hub;

    hexed: Hexed;
    newtd: Newtd;
    mindurkabot: MindurkaBot;
    mindurkansfwdetector: MindurkaNsfwDetector;
}
