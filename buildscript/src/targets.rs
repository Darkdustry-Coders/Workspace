use std::{
    any::Any,
    collections::HashMap,
    env::current_dir,
    ffi::{OsStr, OsString},
    io::{Write, stderr, stdin},
    ops::{Deref, DerefMut},
    path::PathBuf,
    process::{Command, exit},
    str::FromStr,
};

use dotenvy::var;

use crate::args::{BuildArgs, EnvTy, GitBackend, MindustryVersion};

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
impl Default for TargetFlags {
    fn default() -> Self {
        Self {
            always_local: true,
            deprecated: false,
        }
    }
}

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
pub trait TargetImplExt: TargetImpl {
    fn try_as<T: TargetImpl>(&self) -> Option<&T>;
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

pub enum BorrowedMut<'a, T> {
    Borrowed(&'a mut T),
    Owned(T),
}
impl<'a, T> BorrowedMut<'a, T> {
    pub fn new_owned(t: T) -> Self {
        Self::Owned(t)
    }

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

pub struct InitParams {
    pub git_backend: GitBackend,
    pub mindustry_version: MindustryVersion,
    pub rust_workspace_members: Vec<String>,
    pub java_workspace_members: Vec<String>,
    pub root: PathBuf,
}
impl InitParams {
    pub fn new(args: &BuildArgs) -> Self {
        Self {
            git_backend: args.git_backend,
            mindustry_version: args.mindustry_version,
            rust_workspace_members: Default::default(),
            java_workspace_members: Default::default(),
            root: current_dir().unwrap(),
        }
    }
}

pub struct BuildParams {
    pub git_backend: GitBackend,
    pub mindustry_version: MindustryVersion,
    pub env: HashMap<OsString, OsString>,
    pub path: Vec<PathBuf>,
    pub root: PathBuf,
}
impl BuildParams {
    pub fn new(params: InitParams) -> Self {
        Self {
            git_backend: params.git_backend,
            mindustry_version: params.mindustry_version,
            env: Default::default(),
            path: Default::default(),
            root: params.root,
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
}

pub struct RunParams {
    pub env: HashMap<OsString, OsString>,
    pub path: Vec<PathBuf>,
    pub port: u16,
    pub root: PathBuf,
}
impl RunParams {
    pub fn new(params: BuildParams, args: &BuildArgs) -> Self {
        Self {
            env: params.env,
            path: params.path,
            port: args.ports_start,
            root: params.root,
        }
    }

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

                            if env == EnvTy::Host {
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
}
