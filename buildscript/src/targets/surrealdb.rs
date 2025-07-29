use std::{
    env::current_dir,
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::util::{find_executable, is_executable};

use super::{RunParams, TargetEnabled, TargetFlags, TargetImpl, TargetImplStatic, Targets};

pub struct Impl {
    surreal: PathBuf,
    port: u16,
}
impl Impl {}
impl TargetImpl for Impl {
    fn build(&mut self, _: Targets<'_>, _: &mut super::BuildParams) {
        // STUB: This target is not compiled from source.
    }

    fn run_init(&mut self, _: Targets<'_>, params: &mut RunParams) {
        self.port = params.next_port();
        params.env.insert("SURREAL_USER".into(), "admin".into());
        params.env.insert("SURREAL_PASS".into(), "password".into());
        params.env.insert(
            "SURREAL_BIND".into(),
            format!("127.0.0.1:{}", self.port).into(),
        );
    }

    fn run(&mut self, mut deps: Targets<'_>, params: &mut RunParams) {
        deps.mprocs.as_mut().unwrap().spawn_task(
            params,
            Command::new(self.surreal.join("surreal"))
                .arg("start")
                .arg(format!(
                    "surrealkv://{}",
                    params.root.join(".run/surrealdb").to_str().unwrap()
                )),
            "surreal",
        );
    }
}
impl TargetImplStatic for Impl {
    fn flags() -> TargetFlags {
        TargetFlags {
            always_local: true,
            ..Default::default()
        }
    }

    fn initialize_host(
        _: TargetEnabled,
        _: Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        let surreal = find_executable("surreal").map(|x| x.parent().unwrap().to_path_buf());
        surreal.map(|surreal| Impl { surreal, port: 0 })
    }
    fn initialize_cached(
        _: TargetEnabled,
        _: Targets<'_>,
        _: &mut super::InitParams,
    ) -> Option<Self> {
        if is_executable(".cache/tools/surrealdb/surreal") {
            Some(Self {
                surreal: fs::canonicalize(".cache/tools/surrealdb").unwrap(),
                port: 0,
            })
        } else {
            None
        }
    }
    #[allow(unreachable_code)]
    fn initialize_local(_: TargetEnabled, _: Targets<'_>, _: &mut super::InitParams) -> Self {
        #[cfg(unix)]
        {
            eprintln!("Downloading SurrealDB");

            let mut resp = ureq::get("https://install.surrealdb.com/").call().unwrap();
            let mut reader = resp.body_mut().as_reader();
            let mut buf = [0; 16384];
            let mut process = Command::new("sh")
                .arg("-s")
                .arg(current_dir().unwrap().join(".cache/tools/surrealdb"))
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let mut stdin = process.stdin.take().unwrap();
            loop {
                let len = reader.read(&mut buf).unwrap();
                if len == 0 {
                    break;
                }
                stdin.write(&buf[0..len]).unwrap();
            }
            stdin.flush().unwrap();
            drop(stdin);
            let code = process.wait().unwrap();
            if !code.success() {
                panic!("surrealdb installer exited with code {code}");
            }

            return Self {
                surreal: fs::canonicalize(".cache/tools/surrealdb").unwrap(),
                port: 0,
            };
        }

        // TODO: Implement on Windows.

        todo!();
    }
}
