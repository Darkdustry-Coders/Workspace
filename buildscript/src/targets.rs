use std::{
    any::Any,
    collections::HashMap,
    ffi::{OsStr, OsString},
    io::{Write, stderr, stdin},
    ops::{Deref, DerefMut},
    path::PathBuf,
    process::{Command, exit},
    str::FromStr,
};

use crate::{
    args::{BuildArgs, EnvTy, GitBackend},
    syncfs::SyncFs,
    util::{self, current_dir},
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

    /// Check the environment and add the appropriate init parametets.
    ///
    /// This function is called after all targets have been initialized and the final parameters
    /// can be assembled. It's also called on inactive targets so even if a target is not
    /// explicitly enabled, it is kepts as a subproject in workspaces.
    fn postinit(
        #[allow(unused)] enabled: TargetEnabled,
        #[allow(unused)] deps: Targets<'_>,
        #[allow(unused)] params: &mut InitParams,
    ) {
    }
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
    /// Whether native builds are used.
    pub native_image: bool,
    /// List of Rust workspace members to add.
    pub rust_workspace_members: Vec<String>,
    /// List of Java workspace members to add.
    pub java_workspace_members: Vec<String>,
    /// List of Java workspace members to add (only when not running ./b).
    pub java_masked_members: Vec<String>,
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
            native_image: args.native_image,
            rust_workspace_members: Default::default(),
            java_workspace_members: Default::default(),
            java_masked_members: Default::default(),
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
    /// Whether native builds are used.
    pub native_image: bool,
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
            native_image: params.native_image,
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
    #[must_use]
    pub fn cmd(&self, cmd: impl AsRef<OsStr>) -> Command {
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
    #[must_use]
    pub fn gradle(&self) -> Command {
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

    #[must_use]
    pub fn cargo(&self) -> Command {
        let cargo = util::find_executable_on_path("cargo", &self.path)
            .expect("Could not find cargo executable");
        self.cmd(cargo)
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
    /// Whether native builds are used.
    pub native_image: bool,

    pub templates: HashMap<String, PathBuf>,
    pub run: SyncFs,
}
impl RunParams {
    /// Creates new run parameters from build parameters and arguments.
    pub fn new(params: BuildParams, args: &BuildArgs) -> Self {
        let mut run = SyncFs::new(".run".into());
        args.keep_states
            .iter()
            .for_each(|x| run.keep_path(x.clone()));

        Self {
            env: params.env,
            path: params.path,
            port: args.ports_start,
            root: params.root,
            host_rabbitmq: !args.rabbitmq_url.is_empty(),
            host_surrealdb: !args.surrealdb_url.is_empty(),
            templates: args.templates.clone(),
            native_image: args.native_image,
            run,
        }
    }

    /// Returns the next available port and increments the counter.
    #[must_use]
    pub fn next_port(&mut self) -> u16 {
        let port = self.port;
        self.port += 1;
        port
    }

    #[must_use]
    pub fn cmd(&self, cmd: impl AsRef<OsStr>) -> Command {
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

    #[must_use]
    pub fn cargo(&self) -> Command {
        let cargo = util::find_executable_on_path("cargo", &self.path)
            .expect("Could not find cargo executable");
        self.cmd(cargo)
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
                $(
                    $name::Impl::postinit(
                        recipe.$name,
                        self.target_deps(Target::$enumname).1,
                        params,
                    );
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

macro_rules! simple_server_target {
    (
        jar = $jar:expr,
        dir = $dir:expr,
        target = $target:expr,
        prefix = $prefix:expr,
        server = $server:expr,
        startcommand = $startcommand:expr,
        repo = $repo:expr,

        fn setup_server($setup_server_params:ident) $setup_server_body:expr,
    ) => {
        pub struct Impl {
            /// Path to the plugin repository.
            #[allow(unused)]
            repo: ::std::path::PathBuf,
            /// Path to the built JAR file.
            #[allow(unused)]
            path: ::std::path::PathBuf,
            /// Command to run the server.
            command: ::std::option::Option<::std::process::Command>,
        }
        impl Impl {
            fn new(path: ::std::path::PathBuf) -> Self {
                Self {
                    repo: path,
                    path: crate::current_dir().join(concat!(".bin/", $jar, ".jar")),
                    command: None,
                }
            }
        }
        impl super::TargetImpl for Impl {
            fn build(&mut self, deps: super::Targets<'_>, params: &mut super::BuildParams) {
                // On Forts side it should copy resulting jar into `.bin/$jar.jar`.
                if !params
                    .gradle()
                    .arg(concat!(":", $target, ":build"))
                    .status()
                    .unwrap()
                    .success()
                {
                    panic!(concat!("building ", $jar, " failed"));
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
                                        match ::std::io::Write::write(
                                            &mut output,
                                            &mut buffer[..pos],
                                        ) {
                                            Ok(0) => {
                                                panic!(
                                                    "Could not write into {name:?}: unexpected EOF"
                                                )
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
                        (concat!(".bin/", $jar, ".jar"), concat!($prefix, "/")),
                    ] {
                        let mut input = ::zip::ZipArchive::new(match ::std::fs::File::open(name) {
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
                                        ::std::borrow::Cow::Owned(
                                            String::from(prefix) + name.as_ref(),
                                        )
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
                                        match ::std::io::Write::write(
                                            &mut output,
                                            &mut buffer[..pos],
                                        ) {
                                            Ok(0) => {
                                                panic!(
                                                    "Could not write into {name:?}: unexpected EOF"
                                                )
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
                    ::std::io::Write::write_all(
                        &mut output,
                        concat!("coreplugin\n", $prefix).as_ref(),
                    )
                    .unwrap();

                    output.finish().expect("failed to finish 'tmp.jar'");

                    ::std::fs::remove_dir_all(".cache/tools/buildscript/genenv").ok();
                    crate::fs2::create_dir_all(".cache/tools/buildscript/genenv/config").unwrap();

                    crate::fs2::write(
                        ".cache/tools/buildscript/genenv/config/corePlugin.toml",
                        format!(
                            "serverName = {:?}\ngamemode = {:?}\nsharedConfigPath = {:?}",
                            $server, $server, "config/sharedConfig.toml"
                        ),
                    )
                    .unwrap();
                    crate::fs2::write(
                        ".cache/tools/buildscript/genenv/config/sharedConfig.toml",
                        "serverIp = \"127.0.0.1\"\nrabbitMqUrl=\"\"\nsurrealDbUrl=\"\"",
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
                                        match ::std::io::Write::write(
                                            &mut writer,
                                            &mut buffer[..pos],
                                        ) {
                                            Ok(0) => {
                                                panic!(
                                                    "Could not write into {name:?}: unexpected EOF"
                                                )
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

                        if let Ok(x) = ::std::fs::read_to_string(crate::current_dir().join(".cache/tools/buildscript/genenv/reachability-metadata.json")) {
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
                        if ::std::fs::File::open(crate::current_dir().join(".cache/tools/buildscript/genenv/reachability-metadata.json")).is_err() {
                            cmd
                                .arg(format!(
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
                            println!("\x1b[33m[WARN] 'reachability-metadata.json' is buggy af and thus not supported. Please use GraalVM 17\x1b[0m");
                        }
                        cmd.arg("--trace-class-initialization=kotlin.DeprecationLevel")
                            .arg("-H:CStandard=C11")
                            .arg("--initialize-at-build-time=kotlin.DeprecationLevel")
                            .arg("--no-fallback")
                            .arg("--enable-url-protocols=http")
                            .arg("-o")
                            .arg(crate::exe_path!(concat!(".bin/", $jar)))
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

            fn run_init(&mut self, deps: super::Targets<'_>, mut params: &mut super::RunParams) {
                let root = ::std::path::Path::new(concat!(".run/", $dir));

                if !params.native_image {
                    params.run.link_global(
                        params.root.join(".bin/CorePlugin.jar"),
                        concat!($dir, "/config/mods/CorePlugin.jar"),
                    );
                    params.run.link_global(
                        params.root.join(concat!(".bin/", $jar, ".jar")),
                        concat!($dir, "/config/mods/", $jar, ".jar"),
                    );
                }
                params.run.write(
                    concat!($dir, "/config/corePlugin.toml"),
                    format!(
                        "serverName = {:?}\ngamemode = {:?}\nsharedConfigPath = {:?}",
                        $server,
                        $server,
                        params.root.join(".run/sharedConfig.toml")
                    ),
                );
                {
                    let $setup_server_params = &mut params;
                    $setup_server_body;
                }

                let port = params.next_port();

                {
                    let mut contents = vec![];
                    contents.extend_from_slice(&3i32.to_be_bytes());

                    let option = "servername";
                    contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
                    contents.extend_from_slice(option.as_bytes());

                    let name = concat!("[scarlet]Workspace [accent]| [white]", $jar);
                    contents.push(4);
                    contents.extend_from_slice(&(name.len() as u16).to_be_bytes());
                    contents.extend_from_slice(name.as_bytes());

                    let option = "port";
                    contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
                    contents.extend_from_slice(option.as_bytes());

                    contents.push(1);
                    contents.extend_from_slice(&(port as i32).to_be_bytes());

                    let option = "startCommands";
                    contents.extend_from_slice(&(option.len() as u16).to_be_bytes());
                    contents.extend_from_slice(option.as_bytes());

                    let commands = $startcommand;
                    contents.push(4);
                    contents.extend_from_slice(&(commands.len() as u16).to_be_bytes());
                    contents.extend_from_slice(commands.as_bytes());

                    params
                        .run
                        .write(concat!($dir, "/config/settings.bin"), contents);
                }

                if params.native_image {
                    let mut cmd = params.cmd(
                        std::fs::canonicalize(crate::exe_path!(concat!(".bin/", $jar))).unwrap(),
                    );
                    cmd.current_dir(root);
                    self.command = Some(cmd);
                } else {
                    let java = deps.java.as_ref().unwrap().home().join("bin/java");
                    let mindustry = deps.mindustry.as_ref().unwrap().path();

                    let mut cmd = params.cmd(java);
                    cmd.arg("-jar").arg(mindustry).current_dir(root);
                    self.command = Some(cmd);
                }
            }

            fn run(&mut self, deps: super::Targets<'_>, params: &mut super::RunParams) {
                deps.mprocs.as_ref().unwrap().spawn_task(
                    params,
                    &mut self.command.take().unwrap(),
                    $server,
                );
            }
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
                if ::std::fs::read_dir($dir).is_err() {
                    return None;
                }

                Some(Self::new(::std::fs::canonicalize($dir).unwrap()))
            }

            fn initialize_local(
                _: super::TargetEnabled,
                _: super::Targets<'_>,
                params: &mut super::InitParams,
            ) -> Self {
                if !::std::process::Command::new("git")
                    .arg("clone")
                    .arg(params.git_backend.repo_url($repo))
                    .arg(params.root.join($dir))
                    .status()
                    .unwrap()
                    .success()
                {
                    panic!("failed to fetch repo");
                }

                Self::new(::std::fs::canonicalize($dir).unwrap())
            }

            fn postinit(
                _: super::TargetEnabled,
                _: super::Targets<'_>,
                params: &mut super::InitParams,
            ) {
                if ::std::fs::read_dir($dir).is_ok() {
                    params.java_workspace_members.push($dir.into());
                }
            }
        }
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
