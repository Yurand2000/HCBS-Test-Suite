use crate::{cgroup::{__cgroup_exists, __cgroup_path}, utils::__println_debug};

pub mod prelude {
    pub use super::policy::prelude::*;
    pub use super::{
        get_cgroup_of_pid,
        is_pid_in_cgroup,
        get_cgroup_pids,
        migrate_task_to_cgroup,
        kill,
        get_process_total_runtime_usage,
        get_process_total_cpu_usage,
    };
}

pub mod policy;

pub fn get_cgroup_of_pid(pid: u32) -> Result<String, Box<dyn std::error::Error>> {
    std::fs::read_to_string(format!("/proc/{}/cgroup", pid))
        .map_err(|err| err.into())
        .and_then(|str| {
            str.trim().strip_prefix("0::/")
                .ok_or("cgroup file should've started with 0::/".into())
                .map(|str| {
                    if str.is_empty() {
                        ".".to_string()
                    } else {
                        str.to_string()
                    }
                })
        })
}

pub fn is_pid_in_cgroup(name: &str, pid: u32) -> Result<bool, Box<dyn std::error::Error>> {
    if !__cgroup_exists(name) {
        return Err(format!("Cgroup {name} does not exist"))?;
    }

    let pid = format!("{pid}");
    let path = __cgroup_path(name);
    Ok(std::fs::read_to_string(format!("{path}/cgroup.procs"))?.lines()
        .find(|line| line == &pid).is_some())
}

pub fn get_cgroup_pids(name: &str) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    if !__cgroup_exists(name) {
        return Err(format!("Cgroup {name} does not exist"))?;
    }

    let path = __cgroup_path(name);
    std::fs::read_to_string(format!("{path}/cgroup.procs"))?.lines()
        .map(|line| line.parse::<u32>().map_err(|err| err.into()))
        .collect()
}

pub fn migrate_task_to_cgroup(name: &str, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    if !__cgroup_exists(name) {
        return Err(format!("Cgroup {name} does not exist"))?;
    }

    let path = __cgroup_path(name);
    std::fs::write(format!("{path}/cgroup.procs"), pid.to_string())
        .map_err(|err| format!("Error in migrating task {pid} to cgroup {name}: {err}"))?;

    __println_debug(|| format!("Migrated task {pid} to Cgroup {name}"));

    Ok(())
}

pub fn kill(pid: u32) {
    let system = sysinfo::System::new();
    let pid = sysinfo::Pid::from_u32(pid);

    system.process(pid).iter()
        .for_each(|proc| { proc.kill(); proc.wait(); });

    __println_debug(|| format!("Killed pid {pid}"));
}

pub fn get_process_total_runtime_usage(pid: u32) -> Result<f64, String> {
    let ticks_per_second = sysconf::sysconf(sysconf::SysconfVariable::ScClkTck)
        .map_err(|err| format!("{err:?}"))? as f64;

    let stats = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|err| format!("{err:?}"))?;
    let stats: Vec<_> = stats.split_whitespace().collect();

    let utime = stats.get(13).ok_or("Error in reading /proc/<pid>/stat".to_owned())?
        .parse::<isize>().map_err(|err| format!("{err:?}"))? as f64;
    let stime = stats.get(14).ok_or("Error in reading /proc/<pid>/stat".to_owned())?
        .parse::<isize>().map_err(|err| format!("{err:?}"))? as f64;

    Ok((utime + stime) / ticks_per_second)
}
pub fn get_process_total_cpu_usage(pid: u32) -> Result<f64, String> {
    let uptime: f64 =
        std::fs::read_to_string("/proc/uptime")
                .map_err(|err| format!("{err:?}"))?
            .split_whitespace().nth(0).ok_or("Error in reading /proc/uptime".to_owned())?
            .parse()
                .map_err(|err| format!("{err:?}"))?;

    let stats = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|err| format!("{err:?}"))?;
    let stats: Vec<_> = stats.split_whitespace().collect();

    let ticks_per_second = sysconf::sysconf(sysconf::SysconfVariable::ScClkTck)
        .map_err(|err| format!("{err:?}"))? as f64;

    let utime = stats.get(13).ok_or("Error in reading /proc/<pid>/stat".to_owned())?
        .parse::<isize>().map_err(|err| format!("{err:?}"))? as f64 / ticks_per_second;
    let stime = stats.get(14).ok_or("Error in reading /proc/<pid>/stat".to_owned())?
        .parse::<isize>().map_err(|err| format!("{err:?}"))? as f64 / ticks_per_second;
    let start_time = stats.get(21).ok_or("Error in reading /proc/<pid>/stat".to_owned())?
        .parse::<isize>().map_err(|err| format!("{err:?}"))? as f64 / ticks_per_second;

    let elapsed = uptime - start_time;
    Ok((utime + stime)/ elapsed)
}