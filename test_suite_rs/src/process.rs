use crate::prelude::*;

pub mod prelude {
    pub use super::{
        get_process_total_runtime_usage,
        get_process_total_cpu_usage,
        get_fair_server_avg_bw,
    };
}

pub fn get_process_total_runtime_usage(pid: Pid) -> anyhow::Result<f64> {
    let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if ticks_per_second < 0 {
        anyhow::bail!("Error reading sysconf _SC_CLK_TCK");
    }
    let ticks_per_second = ticks_per_second as f64;

    let stats = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|err| anyhow::format_err!("{err:?}"))?;
    let stats: Vec<_> = stats.split_whitespace().collect();

    let utime = stats.get(13)
        .ok_or(anyhow::format_err!("Error in reading /proc/<pid>/stat"))?
        .parse::<isize>()
        .map_err(|err| anyhow::format_err!("{err:?}"))? as f64;

    let stime = stats.get(14)
        .ok_or(anyhow::format_err!("Error in reading /proc/<pid>/stat"))?
        .parse::<isize>()
        .map_err(|err| anyhow::format_err!("{err:?}"))? as f64;

    Ok((utime + stime) / ticks_per_second)
}

pub fn get_process_total_cpu_usage(pid: Pid) -> anyhow::Result<f64> {
    let uptime: f64 =
        std::fs::read_to_string("/proc/uptime")
            .map_err(|err| anyhow::format_err!("{err:?}"))?
            .split_whitespace().nth(0)
            .ok_or(anyhow::format_err!("Error in reading /proc/uptime"))?
            .parse()
            .map_err(|err| anyhow::format_err!("{err:?}"))?;

    let stats = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|err| anyhow::format_err!("{err:?}"))?;
    let stats: Vec<_> = stats.split_whitespace().collect();

    let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if ticks_per_second < 0 {
        anyhow::bail!("Error reading sysconf _SC_CLK_TCK");
    }
    let ticks_per_second = ticks_per_second as f64;

    let utime = stats.get(13)
        .ok_or(anyhow::format_err!("Error in reading /proc/<pid>/stat"))?
        .parse::<isize>()
        .map_err(|err| anyhow::format_err!("{err:?}"))? as f64 / ticks_per_second;

    let stime = stats.get(14)
        .ok_or(anyhow::format_err!("Error in reading /proc/<pid>/stat"))?
        .parse::<isize>()
        .map_err(|err| anyhow::format_err!("{err:?}"))? as f64 / ticks_per_second;

    let start_time = stats.get(21)
        .ok_or(anyhow::format_err!("Error in reading /proc/<pid>/stat"))?
        .parse::<isize>()
        .map_err(|err| anyhow::format_err!("{err:?}"))? as f64 / ticks_per_second;

    let elapsed = uptime - start_time;
    Ok((utime + stime)/ elapsed)
}

pub fn get_fair_server_avg_bw() -> anyhow::Result<f64> {
    let mut avg_bw = 0f64;
    let mut num_cpus = 0f64;

    for entry in std::fs::read_dir("/sys/kernel/debug/sched/fair_server")? {
        let entry = entry?.path();
        if entry.is_dir() {
            let entry = entry.into_os_string().into_string().unwrap();

            let runtime: u64 =
                std::fs::read_to_string(format!("{entry}/runtime"))
                    .map_err(|err| anyhow::format_err!("Error in reading {entry}/runtime: {err}"))
                    .and_then(|value| value.trim().parse::<u64>()
                        .map_err(|err| anyhow::format_err!("Error in parsing {entry}/runtime: {err}"))
                    )?;
            let period: u64 =
                std::fs::read_to_string(format!("{entry}/period"))
                    .map_err(|err| anyhow::format_err!("Error in reading {entry}/period: {err}"))
                    .and_then(|value| value.trim().parse::<u64>()
                        .map_err(|err| anyhow::format_err!("Error in parsing {entry}/period: {err}"))
                    )?;

            avg_bw += runtime as f64 / period as f64;
            num_cpus += 1f64;
        }
    }

    Ok(avg_bw / num_cpus)
}
