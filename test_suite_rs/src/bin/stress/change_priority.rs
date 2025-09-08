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

pub fn batch_runner(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> Result<(), Box<dyn std::error::Error>> {
    if is_batch_test() && args.max_time.is_none() {
        Err(format!("Batch testing requires a maximum running time"))?;
    }

    batch_test_header(&format!("change_prio c{} r{} p{} P{:.2}", args.cgroup, args.runtime_ms, args.period_ms, args.change_period), "stress");
    batch_test_result(main(args, ctrlc_flag))?;

    Ok(())
}

pub fn main(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> Result<(), Box<dyn std::error::Error>> { 
    let cgroup = MyCgroup::new(&args.cgroup, args.runtime_ms * 1000, args.period_ms * 1000, true)?;
    migrate_task_to_cgroup(&args.cgroup, std::process::id())?;
    set_scheduler(std::process::id(), SchedPolicy::RR(99))?;

    let (mut proc1, mut proc2) = (run_yes()?, run_yes()?);
    let mut state = 60;
    set_scheduler(proc1.id(), SchedPolicy::RR(state))?;
    set_scheduler(proc2.id(), SchedPolicy::RR(50))?;

    let update_fn = || {
        if state == 60 {
            state = 40;
        } else {
            state = 60;
        }

        set_scheduler(proc1.id(), SchedPolicy::RR(state))?;
        Ok(())
    };

    if !is_batch_test() {
        println!("Started Yes processes\nPress Ctrl+C to stop");
    }

    wait_loop_periodic_fn(args.change_period, args.max_time, ctrlc_flag, update_fn)?;

    proc1.kill()?;
    proc2.kill()?;
    set_scheduler(std::process::id(), SchedPolicy::other())?;
    migrate_task_to_cgroup(".", std::process::id())?;
    cgroup.destroy()?;

    Ok(())
}