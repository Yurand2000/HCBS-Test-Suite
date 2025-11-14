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

    /// number of processes to spawn
    #[arg(short = 'n', long = "num-tasks", default_value= "1", value_name = "#num")]
    pub num_tasks: u64,

    /// task's allowed cpus
    #[arg(long = "cpu-set", value_parser = <CpuSetUnchecked as std::str::FromStr>::from_str)]
    pub cpu_set: Option<CpuSetUnchecked>,

    /// max running time
    #[arg(short = 't', long = "max-time", value_name = "sec: u64")]
    pub max_time: Option<u64>,
}

pub fn batch_runner(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> Result<(), Box<dyn std::error::Error>> {
    if is_batch_test() && args.max_time.is_none() {
        Err(format!("Batch testing requires a maximum running time"))?;
    }

    let single_bw = args.runtime_ms as f64 / args.period_ms as f64;
    let num_cpus = args.cpu_set.as_ref()
        .map_or(CpuSet::all()?.num_cpus(), |cpu_set| cpu_set.num_cpus());

    let total_cgroup_bw = single_bw * num_cpus as f64;
    let max_expected_bw = f64::min(total_cgroup_bw, args.num_tasks as f64);
    let max_error = 0.01;

    let test_header = format!("time c{} n{} r{} p{} set{:?}",
        args.cgroup, args.num_tasks, args.runtime_ms, args.period_ms, args.cpu_set);
    let test_header =
        if is_batch_test() {
            test_header
        } else {
            test_header + " (Ctrl+C to stop)"
        };

    batch_test_header(&test_header, "time");

    let result = main(args, ctrlc_flag)
        .and_then(|used_bw| {
            match used_bw {
                Skippable::Result(used_bw) =>
                    if f64::abs(used_bw - max_expected_bw) < max_error {
                        Ok(Skippable::Result(format!("Processes used an average of {used_bw:.5} units of CPU bandwidth.")))
                    } else {
                        Err(format!("Expected cgroup's task to use {:.2} units of runtime, but used {:.2}", max_expected_bw, used_bw).into())
                    },
                Skippable::Skipped(err) => Ok(Skippable::Skipped(err)),
            }
        });

    if is_batch_test() {
        batch_test_result_skippable(result)
    } else {
        batch_test_result_skippable_details(result)
    }
}

pub fn main(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> Result<Skippable<f64, Box<dyn std::error::Error>>, Box<dyn std::error::Error>> {
    // check if the cpu_set is valid
    let cpu_set = args.cpu_set
        .map(|cpu_set| Into::<Result<CpuSet, CpuSetBuildError>>::into(cpu_set))
        .transpose();

    let cpu_set =
        match cpu_set {
            Err(err @ CpuSetBuildError::UnavailableCPU(_)) =>
                { return Ok(Skippable::Skipped(err.into())); },
            Ok(cpu_set) => cpu_set,
            Err(err) =>
                { return Err(err.into()); },
        };

    // run the tasks
    let cgroup = MyCgroup::new(&args.cgroup, args.runtime_ms * 1000, args.period_ms * 1000, true)?;

    migrate_task_to_cgroup(&args.cgroup, std::process::id())?;

    let procs = (0..args.num_tasks)
        .map(|_| run_yes()).collect::<Result<Vec<_>, _>>()?;

    set_scheduler(std::process::id(), SchedPolicy::RR(99))?;
    procs.iter()
        .try_for_each(|proc| {
            migrate_task_to_cgroup(&args.cgroup, proc.id())?;
            set_scheduler(proc.id(), SchedPolicy::RR(50))?;
            if cpu_set.is_some() {
                set_cpuset_to_pid(proc.id(), cpu_set.as_ref().unwrap())?;
            }

            Ok::<_, Box<dyn std::error::Error>>(())
        })?;

    wait_loop(args.max_time, ctrlc_flag)?;

    let total_usage =
        procs.iter()
            .try_fold(0f64, |sum, proc| Ok::<f64, String>(sum + get_process_total_cpu_usage(proc.id())?))?;

    procs.into_iter()
        .try_for_each(|mut proc| proc.kill())?;

    set_scheduler(std::process::id(), SchedPolicy::other())?;
    migrate_task_to_cgroup(".", std::process::id())?;
    cgroup.destroy()?;

    Ok(Skippable::Result(total_usage))
}