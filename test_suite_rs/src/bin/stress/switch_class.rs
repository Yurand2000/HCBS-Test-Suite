use hcbs_test_suite::prelude::*;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// cgroup's name
    #[arg(short = 'c', long = "cgroup", default_value = "g0", value_name = "name")]
    pub cgroup: String,

    /// cgroup's runtime
    #[arg(short = 'r', long = "runtime", value_name = "ms: u64")]
    pub runtime_ms: u64,

    /// cgroup's period
    #[arg(short = 'p', long = "period", value_name = "ms: u64")]
    pub period_ms: u64,

    /// pinning change period
    #[arg(short = 'P', long = "change-period", value_name = "secs: f32")]
    pub change_period: f32,

    /// max running time
    #[arg(short = 't', long = "max-time", value_name = "sec: u64")]
    pub max_time: Option<u64>,
}

pub fn batch_runner(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<()> {
    if is_batch_test() && args.max_time.is_none() {
        anyhow::bail!("Batch testing requires a maximum running time");
    }

    let test_header = format!("switch_class c{} r{} p{} P{:.2}",
        args.cgroup, args.runtime_ms, args.period_ms, args.change_period);
    let test_header =
        if is_batch_test() {
            test_header
        } else {
            test_header + "(Ctrl+C to stop)"
        };

    batch_test_header(&test_header, "stress");
    batch_test_result(main(args, ctrlc_flag))?;

    Ok(())
}

pub fn main(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<()> {
    let cgroup = MyCgroup::new(&args.cgroup, args.runtime_ms * 1000, args.period_ms * 1000, true)?;
    assign_pid_to_cgroup(&args.cgroup, std::process::id())?;
    set_sched_policy(std::process::id(), SchedPolicy::RR(99))?;

    let mut proc = run_yes()?;
    let mut state = SchedPolicy::RR(50);

    let update_fn = || {
        if state == SchedPolicy::RR(50) {
            state = SchedPolicy::other();
        } else {
            state = SchedPolicy::RR(50);
        }

        set_sched_policy(proc.id(), state)?;
        Ok(())
    };

    wait_loop_periodic_fn(args.change_period, args.max_time, ctrlc_flag, update_fn)?;

    proc.kill()?;
    set_sched_policy(std::process::id(), SchedPolicy::other())?;
    assign_pid_to_cgroup(".", std::process::id())?;
    cgroup.destroy()?;

    Ok(())
}