use hcbs_test_suite::prelude::*;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    // process pid
    #[arg(short = 'p', value_name = "<PID>")]
    pid: u32,

    /// runtime
    #[arg(short = 'R', long = "runtime", value_name = "ms: u64")]
    pub runtime_ms: u64,

    /// deadline
    #[arg(short = 'D', long = "deadline", value_name = "ms: u64")]
    pub deadline_ms: u64,

    /// period
    #[arg(short = 'P', long = "period", value_name = "ms: u64")]
    pub period_ms: u64,
}

pub fn main(args: MyArgs) -> anyhow::Result<()> {
    set_sched_policy(args.pid, SchedPolicy::DEADLINE {
        runtime_ms: args.runtime_ms,
        deadline_ms: args.deadline_ms,
        period_ms: args.period_ms,
    })?;

    Ok(())
}