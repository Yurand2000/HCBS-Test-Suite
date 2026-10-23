use crate::prelude::*;
use crate::utils::*;

pub mod prelude {
    pub use super::{
        HCBSTime,
        get_cgroup_us,
        set_cgroup_max,
        set_cgroup_us,
        set_cgroup,
    };
}

#[derive(Debug, Clone, Copy)]
pub enum HCBSTime {
    Max,
    Time(Time),
}

impl std::fmt::Display for HCBSTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HCBSTime::Max => write!(f, "max"),
            HCBSTime::Time(t) => t.fmt(f),
        }
    }
}

/// \[HCBS specific\] Get the cgroup server's bandwidth
pub fn get_cgroup_us(name: &str) -> anyhow::Result<(HCBSTime, HCBSTime)> {
    __read_file_parse(
        format!("{}/cpu.rt.max", cgroup_abs_path(name)),
        |s| {
            let mut iter = s.trim().split_ascii_whitespace();
            let Some((runtime, period)) =
                iter.next()
                    .and_then(|r| iter.next().map(|p| (r, p)))
                    .and_then(|v| if iter.next().is_none() { Some(v) } else { None } )
                else { anyhow::bail!("Error reading cpu.rt.max file: {s:?}") };

            let runtime =
                if runtime == "max" {
                    HCBSTime::Max
                } else {
                    HCBSTime::Time(Time::micros(runtime.parse::<u64>()? as f64))
                };
            let period = HCBSTime::Time(Time::micros(period.parse::<u64>()? as f64));

            Ok(( runtime, period ))
        }
    )
}

/// \[HCBS specific\] Set the cgroup server to max
pub fn set_cgroup_max(name: &str) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);

    __write_file(
        format!("{path}/cpu.rt.max"),
        format!("max 0")
    )
        .map_err(|_| anyhow::format_err!("Error in writing max/0 to {path}/cpu.rt.max"))?;

    info!("Set runtime/period max/0 us to {path}/cpu.rt.max");

    Ok(())

}

/// \[HCBS specific\] Set the cgroup server's bandwidth
pub fn set_cgroup_us(name: &str, runtime: Time, period: Time) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);

    let runtime_us = runtime.as_micros() as u64;
    let period_us = period.as_micros() as u64;

    __write_file(
        format!("{path}/cpu.rt.max"),
        format!("{runtime_us} {period_us}")
    )
        .map_err(|_| anyhow::format_err!("Error in writing {runtime_us}/{period_us} to {path}/cpu.rt.max"))?;

    info!("Set runtime/period {runtime_us}/{period_us} us to {path}/cpu.rt.max");

    Ok(())
}

pub fn set_cgroup(name: &str, runtime: HCBSTime, period: HCBSTime) -> anyhow::Result<()> {
    let path = cgroup_abs_path(name);

    let (runtime_us, period_us);
    match (runtime, period) {
        (HCBSTime::Time(runtime), HCBSTime::Time(period)) =>
            { runtime_us = format!("{}", runtime.as_micros() as u64); period_us = format!("{}", period.as_micros() as u64); },
        (HCBSTime::Max, _) =>
            { runtime_us = "max".to_owned(); period_us = "0".to_owned() },
        _ => panic!()
    }

    __write_file(
        format!("{path}/cpu.rt.max"),
        format!("{runtime_us} {period_us}")
    )
        .map_err(|_| anyhow::format_err!("Error in writing {runtime_us}/{period_us} to {path}/cpu.rt.max"))?;

    info!("Set runtime/period {runtime_us}/{period_us} us to {path}/cpu.rt.max");

    Ok(())
}
