use std::ops::{Deref, DerefMut};
use eva_engine::prelude::*;

pub mod cgroup;
pub mod process;
pub mod utils;
pub mod cpuset;
pub mod tests;

pub mod prelude {
    pub use super::cgroup::prelude::*;
    pub use super::process::prelude::*;
    pub use super::utils::prelude::*;
    pub use super::cpuset::prelude::*;

    pub use super::{
        NamedTaskset,
        NamedConfig,
        MyProcess,
        run_yes,
        cpu_hog,
        local_executable_cmd,
    };
}

#[derive(Debug, Clone)]
pub struct NamedTaskset {
    pub name: String,
    pub tasks: Vec<RTTask>,
}

#[derive(Debug, Clone)]
pub struct NamedConfig {
    pub name: String,
    pub cpus: u64,
    pub runtime: Time,
    pub period: Time,
}

pub struct MyProcess {
    process: std::process::Child,
}

impl Drop for MyProcess {
    fn drop(&mut self) {
        self.kill().unwrap()
    }
}

impl Deref for MyProcess {
    type Target = std::process::Child;

    fn deref(&self) -> &Self::Target {
        &self.process
    }
}

impl DerefMut for MyProcess {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.process
    }
}

pub fn cpu_hog() -> Result<MyProcess, Box<dyn std::error::Error>> {
    use std::process::*;

    let cmd = local_executable_cmd("/root/test_suite", "tools")?;

    let proc = Command::new(cmd)
        .arg("hog")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(MyProcess { process: proc })
}

pub fn run_yes() -> Result<MyProcess, std::io::Error> {
    use std::process::*;

    let proc = Command::new("yes")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(MyProcess { process: proc })
}

pub fn local_executable_cmd(def_dir: &str, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cmd = std::env::var("TESTBINDIR").unwrap_or_else(|_| def_dir.to_owned()) + "/" + name;

    if !std::fs::exists(&cmd)
        .map_err(|err| format!("Error in checking existance of {cmd}: {err}"))?
    {
        Err(format!("Cannot find {name} executable at {cmd}"))?;
    }

    Ok(cmd)
}

