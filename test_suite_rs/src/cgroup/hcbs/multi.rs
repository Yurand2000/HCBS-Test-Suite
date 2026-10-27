use crate::prelude::*;

pub mod prelude {
    pub use super::{
        CgroupConfig,
        CgroupTimeConfig,
        get_cgroup,
        set_cgroup_cpumask,
        set_cgroup_override,
        reset_cgroup_override,
    };
}

#[derive(Debug, Clone)]
pub struct CgroupConfig {
    pub cpuset: Option<pid::CpuSet>,
    pub config: CgroupTimeConfig,
}

#[derive(Debug, Clone)]
pub enum CgroupTimeConfig {
    Max,
    Time {
        default: (Time, Time),
        overrides: std::collections::HashMap<CpuID, (Time, Time)>
    }
}

fn parse_first_line(line: &str) -> anyhow::Result<CgroupTimeConfig> {
    let mut iter = line.trim().split_ascii_whitespace();
    let Some((runtime, period)) =
        iter.next()
            .and_then(|r| iter.next().map(|p| (r, p)))
            .and_then(|v| if iter.next().is_none() { Some(v) } else { None } )
        else { anyhow::bail!("Error reading cpu.rt.max first line: {line:?}") };

    let runtime =
        if runtime == "max" {
            return Ok(CgroupTimeConfig::Max);
        } else {
            Time::micros(runtime.parse::<u64>()? as f64)
        };
    let period = Time::micros(period.parse::<u64>()? as f64);

    Ok(CgroupTimeConfig::Time { default: (runtime, period), overrides: std::collections::HashMap::new() })
}

fn get_cgroup_time(path: &str) -> anyhow::Result<CgroupTimeConfig> {
    __read_file_parse(
        format!("{}/cpu.rt.max", path),
        |s| {
            let mut lines = s.lines();

            let Some(first_line) = lines.next()
                else { anyhow::bail!("Error reading cpu.rt.max: first line missing") };

            let mut time_cfg = parse_first_line(first_line)?;

            let CgroupTimeConfig::Time { default: _, ref mut overrides } = time_cfg
                else { return Ok(time_cfg); };

            for line in lines {
                let mut iter = line.trim().split_ascii_whitespace();
                let Some((cpu, runtime, period)) =
                    iter.next()
                        .and_then(|r| iter.next().map(|c| (c, r)))
                        .and_then(|(c, r)| iter.next().map(|p| (c, r, p)))
                        .and_then(|v| if iter.next().is_none() { Some(v) } else { None } )
                    else { anyhow::bail!("Error reading cpu.rt.max line: {line:?}") };

                let (cpu, runtime, period) = (
                    CpuID(cpu.parse::<u32>()?),
                    Time::micros(runtime.parse::<u64>()? as f64),
                    Time::micros(period.parse::<u64>()? as f64)
                );

                overrides.insert(cpu, (runtime, period));
            }

            return Ok(time_cfg);
        }
    )
}

fn get_cgroup_cpuset(path: &str) -> anyhow::Result<Option<pid::CpuSet>> {
    __read_file_parse_if_exists(
        format!("{}/cpuset.cpus.effective", path),
        |s| {
            <pid::CpuSet as std::str::FromStr>::from_str(&s)
        }
    )
}

pub fn get_cgroup(name: &str) -> anyhow::Result<CgroupConfig> {
    let path = cgroup_abs_path(name);

    Ok(CgroupConfig {
        cpuset: get_cgroup_cpuset(&path)?,
        config: get_cgroup_time(&path)?
    })
}

fn cpuset_string(cpuset: &pid::CpuSet) -> String {
    let mut str = String::new();
    for cpu in cpuset.iter() {
        str += &format!("{cpu},");
    }

    str.pop();
    str
}

pub fn set_cgroup_cpumask(name: &str, mask: &pid::CpuSet) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);
    let cpuset = cpuset_string(mask);

    __write_file(
        format!("{path}/cpuset.cpus"),
        format!("{cpuset}")
    )
        .map_err(|_| anyhow::format_err!("Error in writing '{cpuset}' to {path}/cpuset.cpus"))?;

    info!("Set cpumask '{cpuset}' to {path}/cpuset.cpus");

    Ok(())

}

pub fn set_cgroup_override(name: &str, mask: &pid::CpuSet, runtime: Time, period: Time) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);
    let cpuset = cpuset_string(mask);

    let runtime = runtime.as_micros() as u64;
    let period = period.as_micros() as u64;

    __write_file(
        format!("{path}/cpu.rt.max"),
        format!("{cpuset} runtime={runtime} period={period}")
    )
        .map_err(|_| anyhow::format_err!("Error in writing '{cpuset} {runtime} {period}' to {path}/cpu.rt.max"))?;

    info!("Set runtime/period {runtime}/{period} us (set: {cpuset}) to {path}/cpu.rt.max");

    Ok(())
}

pub fn reset_cgroup_override(name: &str, mask: &pid::CpuSet) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);
    let cpuset = cpuset_string(mask);

    __write_file(
        format!("{path}/cpu.rt.max"),
        format!("{cpuset} default")
    )
        .map_err(|_| anyhow::format_err!("Error in writing '{cpuset} default' to {path}/cpu.rt.max"))?;

    info!("Set 'default' (set: {cpuset}) to {path}/cpu.rt.max");

    Ok(())
}
