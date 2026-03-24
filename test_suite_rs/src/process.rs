pub mod prelude {
    pub use super::{
        get_process_total_runtime_usage,
        get_process_total_cpu_usage,
    };
}

pub fn get_process_total_runtime_usage(pid: u32) -> anyhow::Result<f64> {
    let ticks_per_second = sysconf::sysconf(sysconf::SysconfVariable::ScClkTck)
        .map_err(|err| anyhow::format_err!("{err:?}"))? as f64;

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
pub fn get_process_total_cpu_usage(pid: u32) -> anyhow::Result<f64> {
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

    let ticks_per_second = sysconf::sysconf(sysconf::SysconfVariable::ScClkTck)
        .map_err(|err| anyhow::format_err!("{err:?}"))? as f64;

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