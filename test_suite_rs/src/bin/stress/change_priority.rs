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

    /// priority change period
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

    let test_header = format!("change_prio c{} r{} p{} P{:.2}",
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
    let mut cgroup = HCBSCgroup::new(&args.cgroup)?
        .with_force_kill(true);
    cgroup.set_period_us(args.period_ms * 1000)?;
    cgroup.set_runtime_us(args.runtime_ms * 1000)?;

    cgroup.assign_process(HCBSProcess::SelfProc).map_err(|(_, err)| err)?
        .set_sched_policy(SchedPolicy::RR(99))?;

    cgroup.assign_process(run_yes()?).map_err(|(_, err)| err)?
        .set_sched_policy(SchedPolicy::RR(50))?;

    let mut state = 60;
    let proc = cgroup.assign_process(run_yes()?).map_err(|(_, err)| err)?;
    proc.set_sched_policy(SchedPolicy::RR(state))?;
    let update_fn = || {
        if state == 60 {
            state = 40;
        } else {
            state = 60;
        }

        proc.set_sched_policy(SchedPolicy::RR(state))?;
        Ok(())
    };

    wait_loop_periodic_fn(args.change_period, args.max_time, ctrlc_flag, update_fn)?;

    Ok(())
}