use hcbs_test_suite::prelude::*;
use std::thread;
use rand::Rng;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// cgroup's name
    #[arg(short = 'c', long = "cgroup", default_value = "g0", value_name = "name")]
    pub cgroup: String,

    /// cgroup's runtime minimum time
    #[arg(short = 'r', long = "runtime-min", value_name = "ms: u64")]
    pub runtime_min_ms: u64,

    /// cgroup's runtime maximum time
    #[arg(short = 'R', long = "runtime-max", value_name = "ms: u64")]
    pub runtime_max_ms: u64,

    /// cgroup's period
    #[arg(short = 'p', long = "period", value_name = "ms: u64")]
    pub period_ms: u64,

    /// max running time
    #[arg(short = 't', long = "max-time", value_name = "sec: u64")]
    pub max_time: Option<u64>,
}

pub fn batch_runner(args: MyArgs, rng: Option<&mut dyn rand::RngCore>, ctrlc_flag: Option<ExitFlag>) -> Result<(), Box<dyn std::error::Error>> {
    if is_batch_test() && args.max_time.is_none() {
        Err(format!("Batch testing requires a maximum running time"))?;
    }

    batch_test_header(&format!("cgroup_make_destroy c{} r{} R{} p{}", args.cgroup, args.runtime_min_ms, args.runtime_max_ms, args.period_ms), "stress");
    batch_test_result(main(args, rng, ctrlc_flag))?;

    Ok(())
}

pub fn main(args: MyArgs, rng: Option<&mut dyn rand::RngCore>, ctrlc_flag: Option<ExitFlag>) -> Result<(), Box<dyn std::error::Error>> {
    let mut thread_rng = rand::rng();
    let rng = rng.unwrap_or_else(|| &mut thread_rng);

    wait_loop_periodic_fn(0f32, args.max_time, ctrlc_flag,
        || {
            let runtime_ms = rng.random_range(args.runtime_min_ms ..= args.runtime_max_ms);
            let cgroup = MyCgroup::new(&args.cgroup, runtime_ms * 1000, args.period_ms * 1000, true)?;
            migrate_task_to_cgroup(&args.cgroup, std::process::id())?;
            set_scheduler(std::process::id(), SchedPolicy::RR(99))?;

            let num_procs = rng.random_range(1..=5);
            let procs = (0..num_procs)
                .map(|_| run_yes()).collect::<Result<Vec<_>, _>>()?;

            thread::sleep(std::time::Duration::from_secs_f32(rng.random_range(0.5f32..=2f32)));

            procs.into_iter()
                .try_for_each(|mut proc| proc.kill())?;

            set_scheduler(std::process::id(), SchedPolicy::other())?;
            migrate_task_to_cgroup(".", std::process::id())?;
            cgroup.destroy()?;

            Ok(())
        }
    )?;

    Ok(())
}