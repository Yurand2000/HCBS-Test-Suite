#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// cgroup's name
    #[arg(short = 'c', long = "cgroup", default_value = ".", value_name = "name")]
    cgroup: String,

    #[command(flatten)]
    bw: Bandwidth,
}

#[derive(clap::Parser, Debug)]
#[group(required = true, multiple = true)]
pub struct Bandwidth {
    /// cgroup's runtime
    #[arg(short = 'r', long = "runtime", value_name = "ms: u64")]
    pub runtime_ms: Option<u64>,

    /// cgroup's period
    #[arg(short = 'p', long = "period", value_name = "ms: u64")]
    pub period_ms: Option<u64>,
}

pub fn main(args: MyArgs) -> anyhow::Result<()> {
    use hcbs_utils::prelude::*;

    mount_cgroup_cpu()?;

    let runtime_us = match args.bw.runtime_ms {
        Some(ms) => ms * 1000,
        None => get_cgroup_runtime_us(&args.cgroup)?,
    };

    let period_us = match args.bw.period_ms {
        Some(ms) => ms * 1000,
        None => get_cgroup_period_us(&args.cgroup)?,
    };

    create_cgroup(&args.cgroup)?;
    set_cgroup_runtime_us(&args.cgroup, 0)?;
    set_cgroup_period_us(&args.cgroup, period_us)?;
    set_cgroup_runtime_us(&args.cgroup, runtime_us)?;

    Ok(())
}