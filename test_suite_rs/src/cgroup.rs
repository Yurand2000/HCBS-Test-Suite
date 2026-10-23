use crate::prelude::*;
use crate::utils::*;

pub mod prelude {
    pub use super::hcbs::prelude::*;
    pub use super::{
        CGROUP_ROOT,
        CGROUP_ROOT_C,
        mount_cgroup_fs,
        mount_cgroup_cpu,
        get_system_rt_period_us,
        get_system_rt_runtime_us,
        set_system_rt_period_us,
        set_system_rt_runtime_us,
        cgroup_abs_path,
        create_cgroup,
        delete_cgroup,
        cgroup_exists,
        cgroup_num_procs,
        cgroup_pids,
        get_pid_cgroup,
        is_pid_in_cgroup,
        assign_pid_to_cgroup,
    };
}

pub mod hcbs;
pub mod cgroup_v2;

pub const CGROUP_ROOT: &'static str = "/sys/fs/cgroup";
pub const CGROUP_ROOT_C: &'static std::ffi::CStr = c"/sys/fs/cgroup";

/// Get absolute path of a given cgroup
pub fn cgroup_abs_path(name: &str) -> String {
    cgroup_v2::cgroup_abs_path(name)
}

/// Check whether the given cgroup exists
pub fn cgroup_exists(name: &str) -> bool {
    let path = cgroup_abs_path(name);
    let path = std::path::Path::new(&path);
    path.exists() && path.is_dir()
}

/// Get the number of processes assigned to the given cgroup
pub fn cgroup_num_procs(name: &str) -> anyhow::Result<usize> {
    let path = cgroup_abs_path(name);

    __read_file(format!("{path}/cgroup.procs"))
        .map(|procs| procs.lines().count())
}

/// Get the PIDs of the processes assigned to the given cgroup
pub fn cgroup_pids(name: &str) -> anyhow::Result<Vec<Pid>> {
    if !cgroup_exists(name) {
        anyhow::bail!("Cgroup {name} does not exist");
    }

    let path = cgroup_abs_path(name);
    __read_file(format!("{path}/cgroup.procs"))?.lines()
        .map(|line| line.parse::<Pid>().map_err(|err| err.into()))
        .collect()
}

/// Mount cgroup filesystem
pub fn mount_cgroup_fs() -> anyhow::Result<()> {
    cgroup_v2::__mount_cgroup_fs()?;

    Ok(())
}

/// Mount cgroup filesystem and the cpu controller
pub fn mount_cgroup_cpu() -> anyhow::Result<()> {
    mount_cgroup_fs()?;
    cgroup_v2::__mount_cpu_fs()?;

    Ok(())
}

/// Read /proc/sys/kernel/sched_rt_period_us
pub fn get_system_rt_period_us() -> anyhow::Result<u64> {
    __read_file_parse("/proc/sys/kernel/sched_rt_period_us", |s| s.trim().parse::<u64>())
}

/// Read /proc/sys/kernel/sched_rt_runtime_us
pub fn get_system_rt_runtime_us() -> anyhow::Result<u64> {
    __read_file_parse("/proc/sys/kernel/sched_rt_runtime_us", |s| s.trim().parse::<u64>())
}

/// Write to /proc/sys/kernel/sched_rt_period_us
pub fn set_system_rt_period_us(period_us: u64) -> anyhow::Result<()> {
    __write_file("/proc/sys/kernel/sched_rt_period_us", format!("{period_us}"))?;
    info!("Set period {period_us} us to /proc/sys/kernel/sched_rt_runtime_us");

    Ok(())
}

/// Write to /proc/sys/kernel/sched_rt_runtime_us
pub fn set_system_rt_runtime_us(runtime_us: u64) -> anyhow::Result<()> {
    __write_file("/proc/sys/kernel/sched_rt_runtime_us", format!("{runtime_us}"))?;
    info!("Set runtime {runtime_us} us to /proc/sys/kernel/sched_rt_runtime_us");

    Ok(())
}

fn __create_cgroup_common(name: &str) -> anyhow::Result<()> {
    mount_cgroup_fs()?;

    if name == "." { return Ok(()); }

    if cgroup_exists(name) {
        warn!("Cgroup {name} already exists");
        return Ok(());
    }

    let path = cgroup_abs_path(name);
    std::fs::create_dir_all(&path)
        .map_err(|err| anyhow::format_err!("Error in creating directory {path}: {err}"))?;

    Ok(())
}

/// Create new cgroup
///
/// Notes:: creates all the cgroup hierarchy recursively if necessary
pub fn create_cgroup(name: &str) -> anyhow::Result<()> {
    cgroup_v2::create_cgroup(name)?;

    Ok(())
}

/// Delete cgroup
///
/// Tries to delete the given cgroup. A cgroup can be deleted only if
/// there are no active processes inside. The function tries to wait to try and
/// give the kernel time to perform necessary cleanups on exit of processes.
pub fn delete_cgroup(name: &str) -> anyhow::Result<()> {
    if name == "." { return Ok(()); }

    if !cgroup_exists(name) {
        warn!("Cgroup {name} does not exist");
        return Ok(());
    }

    if cgroup_num_procs(name)? > 0 {
        let procs = cgroup_pids(name)?;
        error!("Cgroup {name} has active processes: {procs:?}");
        anyhow::bail!("Cgroup {name} has active processes");
    }

    // Release the cgroup resources before removing it for faster cleanups
    if let (HCBSTime::Max, _) = get_cgroup_us(name)? {
        set_cgroup_us(name, Time::zero(), Time::zero())?;
    }

    let path = cgroup_abs_path(name);

    std::fs::remove_dir(&path)
        .or_else(|_| {
            // Try to give the kernel some time to cleanup the system as this
            // will sometimes fail even if all the processes have been killed
            std::thread::sleep(std::time::Duration::from_millis(100));

            std::fs::remove_dir(&path)
        })
        .map_err(|err| anyhow::format_err!("Error in destroying directory {path}: {err}"))?;

    info!("Deleted Cgroup {name}");

    Ok(())
}

/// Get the cgroup of the given PID
pub fn get_pid_cgroup(pid: Pid) -> anyhow::Result<String> {
    __read_file_parse(format!("/proc/{}/cgroup", pid),
        |str| {
            str.trim().strip_prefix("0::/")
                .ok_or(anyhow::format_err!("cgroup file should've started with 0::/"))
                .map(|str| {
                    if str.is_empty() {
                        ".".to_string()
                    } else {
                        str.to_string()
                    }
                })
        })
}

/// Check if the given PID is assigned to the given cgroup
pub fn is_pid_in_cgroup(name: &str, pid: Pid) -> anyhow::Result<bool> {
    if !cgroup_exists(name) {
        anyhow::bail!("Cgroup {name} does not exist");
    }

    let pid = format!("{pid}");
    let path = cgroup_abs_path(name);
    Ok(__read_file(format!("{path}/cgroup.procs"))?.lines()
        .find(|line| line == &pid).is_some())
}

/// Assign PID to the given cgroup
pub fn assign_pid_to_cgroup(name: &str, pid: Pid) -> anyhow::Result<()> {
    if !cgroup_exists(name) {
        error!("Cannot migrate {pid} to cgroup {name}: cgroup does not exist");
        anyhow::bail!("Cgroup {name} does not exist");
    }

    let path = cgroup_abs_path(name);
    std::fs::write(format!("{path}/cgroup.procs"), pid.to_string())
        .map_err(|err| anyhow::format_err!("Error in migrating task {pid} to cgroup {name}: {err}"))?;

    info!("Migrated task {pid} to cgroup {name}");

    Ok(())
}
