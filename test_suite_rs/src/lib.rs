use crate::prelude::*;

pub mod process;
pub mod utils;
pub mod tests;

pub mod prelude {
    pub use hcbs_utils::prelude::*;
    pub use eva_rt_common::prelude::RTTask;

    pub use super::process::prelude::*;
    pub use super::utils::prelude::*;

    pub use super::{
        NamedTaskset,
        NamedConfig,
        run_yes,
        cpu_hog,
        local_executable_cmd,
        is_multicpu_enabled,
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

pub fn cpu_hog() -> anyhow::Result<HCBSProcess> {
    use std::process::*;

    let cmd = local_executable_cmd("/root/test_suite", "tools")?;

    let proc = Command::new(cmd)
        .arg("hog")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(HCBSProcess::Child(proc))
}

pub fn run_yes() -> Result<HCBSProcess, std::io::Error> {
    use std::process::*;

    let proc = Command::new("yes")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(HCBSProcess::Child(proc))
}

pub fn local_executable_cmd(def_dir: &str, name: &str) -> anyhow::Result<String> {
    let cmd = std::env::var("TESTBINDIR").unwrap_or_else(|_| def_dir.to_owned()) + "/" + name;

    if !std::fs::exists(&cmd)
        .map_err(|err| anyhow::format_err!("Error in checking existance of {cmd}: {err}"))?
    {
        anyhow::bail!("Cannot find {name} executable at {cmd}");
    }

    Ok(cmd)
}

pub fn is_multicpu_enabled() -> anyhow::Result<bool> {
    mount_cgroup_cpu()?;

    let name = "multicpu_test_cgroup";
    if cgroup_exists(name) {
        return Ok(false);
    }

    let mut cgroup = HCBSCgroup::new(name)?
        .with_force_kill(false);

    match cgroup.set_period_us_multi_str("100000 1") {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
