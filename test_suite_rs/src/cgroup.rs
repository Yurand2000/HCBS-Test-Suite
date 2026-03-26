use crate::prelude::*;

pub mod prelude {
    pub use super::{
        MyCgroup,
        cgroup_setup,
        cgroup_setup_multi,
        is_multicpu_enabled,
        set_cgroup_runtime_us_multi_str,
        set_cgroup_period_us_multi_str,
    };
}

pub struct MyCgroup {
    name: String,
    force_kill: bool,
}

impl MyCgroup {
    pub fn new(name: &str, force_kill: bool) -> anyhow::Result<MyCgroup> {
        if name == "." {
            anyhow::bail!("Cannot handle root cgroup");
        }

        create_cgroup(name)?;

        Ok(MyCgroup {
            name: name.to_owned(),
            force_kill
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn destroy(mut self) -> anyhow::Result<()> {
        self.__destroy()
    }

    fn __destroy(&mut self) -> anyhow::Result<()> {
        if !cgroup_exists(&self.name) { return Ok(()); }

        if self.force_kill {
            if is_pid_in_cgroup(&self.name, std::process::id())? {
                assign_pid_to_cgroup(".", std::process::id())?;
            }

            cgroup_pids(&self.name)?.iter()
                .try_for_each(|pid| {
                    kill_pid(*pid)?;
                    assign_pid_to_cgroup(".", *pid)
                })?;
        }

        delete_cgroup(&self.name)
    }
}

pub fn cgroup_setup(name: &str, runtime_us: u64, period_us: u64) -> anyhow::Result<()> {
    if runtime_us > period_us {
        anyhow::bail!("Requested runtime {runtime_us} is greater than the period {period_us}");
    }

    set_cgroup_runtime_us(name, 0)?;
    set_cgroup_period_us(name, period_us)?;
    set_cgroup_runtime_us(name, runtime_us)?;

    Ok(())
}

pub fn cgroup_setup_multi(
    name: &str,
        runtimes_us: impl Iterator<Item = (u64, impl Iterator<Item = u32>)>,
        periods_us: impl Iterator<Item = (u64, impl Iterator<Item = u32>)>
    ) -> anyhow::Result<()>
{
    set_cgroup_runtime_us(name, 0)?;
    set_cgroup_period_us_multi(name, periods_us)?;
    set_cgroup_runtime_us_multi(name, runtimes_us)?;

    Ok(())
}

impl Drop for MyCgroup {
    fn drop(&mut self) {
        let _ = self.__destroy();
    }
}

fn parse_cgroup_time_str(times_us: &str) -> anyhow::Result<Vec<(u64, CpuSetUnchecked)>> {
    use std::str::FromStr as _;
    use nom::Parser;
    use nom::multi::*;
    use nom::character::complete::*;
    use nom::combinator::*;

    let uint = || {
        digit1::<&str, ()>
            .map_res(|num: &str| num.parse::<u64>())
    };

    let cpu_set = || {
        recognize(many1(one_of("0123456789-,")))
            .map_res(|str: &str| CpuSetUnchecked::from_str(str))
    };

    let time_set = || {
        map((uint(), space1, cpu_set()), |(time, _, set)| (time, set))
    };

    separated_list1(space1, time_set()).parse(times_us)
        .map(|(_, data)| data)
        .map_err(|err| {
            log::error!("Parse error for time string: {}", times_us);
            err.into()
        })
}

pub fn set_cgroup_runtime_us_multi_str(name: &str, runtimes_us: &str) -> anyhow::Result<()> {
    set_cgroup_runtime_us_multi(name, parse_cgroup_time_str(runtimes_us)?
        .into_iter().map(|(time, cpus)| (time, cpus.into_iter())) )
}

pub fn set_cgroup_period_us_multi_str(name: &str, periods_us: &str) -> anyhow::Result<()> {
    set_cgroup_period_us_multi(name, parse_cgroup_time_str(periods_us)?
        .into_iter().map(|(time, cpus)| (time, cpus.into_iter())) )
}

pub fn is_multicpu_enabled() -> anyhow::Result<bool> {
    mount_cgroup_cpu()?;

    let name = "multicpu_test_cgroup";
    if cgroup_exists(name) {
        return Ok(false);
    }

    let cgroup = MyCgroup::new(name, false)?;
    set_cgroup_runtime_us(name, 0)?;
    let res = set_cgroup_period_us_multi_str(name, "100000 1");
    cgroup.destroy()?;

    match res {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }

}