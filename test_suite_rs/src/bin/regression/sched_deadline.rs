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

    /// max running time
    #[arg(short = 't', long = "max-time", value_name = "sec: u64")]
    pub max_time: Option<u64>
}

pub fn batch_runner(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<()> {
    if is_batch_test() && args.max_time.is_none() {
        anyhow::bail!("Batch testing requires a maximum running time");
    }

    let cpus = CpuSet::all()?.num_cpus();
    let cgroup_expected_bw = cpus as f64 * args.runtime_ms as f64 / args.period_ms as f64;
    let deadline_expected_bw = cpus as f64 * 4.0 / 10.0;
    let cgroup_error = cgroup_expected_bw * 0.025; // 2.5% error
    let deadline_error = deadline_expected_bw * 0.025; // 2.5% error

    let test_header =
        if is_batch_test() {
            "sched_deadline"
        } else {
            "sched_deadline (Ctrl+C to stop)"
        };

    batch_test_header(test_header, "regression");

    let result = main(args, ctrlc_flag)
        .and_then(|(deadline_bw, cgroup_bw)| {
            if f64::abs(cgroup_bw - cgroup_expected_bw) >= cgroup_error {
                anyhow::bail!("Expected cgroup tasks to use {:.2} units of total runtime, but used {:.2} units", cgroup_expected_bw, cgroup_bw);
            }

            if f64::abs(deadline_bw - deadline_expected_bw) >= deadline_error {
                anyhow::bail!("Expected SCHED_DEADLINE tasks to use {:.2} units of total runtime, but used {:.2} units", deadline_expected_bw, deadline_bw);
            }

            Ok(format!("Cgroup processes got {:.2} units of total runtime, while SCHED_DEADLINE processes got {:.2} units of total runtime ", cgroup_bw, deadline_bw))
        });

    if is_batch_test() {
        batch_test_result(result)
    } else {
        batch_test_result_details(result)
    }
}

pub fn main(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<(f64, f64)> {
    let rt_cgroup_runtime_orig = reduce_cgroups_runtime()?;

    let cpus = CpuSet::all()?.num_cpus();
    let mut cgroup = HCBSCgroup::new(&args.cgroup)?
        .with_force_kill(false);
    cgroup.set_period_us(args.period_ms * 1000)?;
    cgroup.set_runtime_us(args.runtime_ms * 1000)?;
    let dl_runtime_ms = args.period_ms * 4 / 10;

    assign_pid_to_cgroup(".", std::process::id())?;
    let dl_processes = (0..cpus).map(|_| run_yes()).collect::<Result<Vec<_>, _>>()?;
    let cgroup_processes = (0..cpus).map(|_| run_yes()).collect::<Result<Vec<_>, _>>()?;

    set_sched_policy(std::process::id(), SchedPolicy::RR(99), SchedFlags::RESET_ON_FORK)?;
    cgroup_processes.iter().enumerate()
        .try_for_each(|(cpu, proc)| -> anyhow::Result<()> {
            assign_pid_to_cgroup(&args.cgroup, proc.id())?;
            set_cpuset_to_pid(proc.id(), &CpuSet::single(cpu as u32)?)?;
            set_sched_policy(proc.id(), SchedPolicy::RR(50), SchedFlags::empty())?;

            Ok(())
        })?;

    dl_processes.iter()
        .try_for_each(|proc| -> anyhow::Result<()> {
            set_sched_policy(proc.id(), SchedPolicy::DEADLINE {
                runtime_ms: dl_runtime_ms,
                deadline_ms: args.period_ms,
                period_ms: args.period_ms,
            }, SchedFlags::RESET_ON_FORK)?;

            Ok(())
        })?;

    wait_loop(args.max_time, ctrlc_flag)?;

    let mut cgroup_total_usage = 0f64;
    for proc in cgroup_processes.iter() {
        cgroup_total_usage += get_process_total_cpu_usage(proc.id())?;
    }

    let mut deadline_total_usage = 0f64;
    for proc in dl_processes.iter() {
        deadline_total_usage += get_process_total_cpu_usage(proc.id())?;
    }

    restore_cgroups_runtime(rt_cgroup_runtime_orig)?;

    Ok((deadline_total_usage, cgroup_total_usage))
}

fn reduce_cgroups_runtime() -> anyhow::Result<u64> {
    let rt_runtime = get_cgroup_runtime_us(".")?;
    let rt_period = get_cgroup_period_us(".")?;
    set_cgroup_runtime_us(".", rt_period * 5 / 10)?;
    Ok(rt_runtime)
}

fn restore_cgroups_runtime(rt_runtime_us: u64) -> anyhow::Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(100));

    set_cgroup_runtime_us(".", rt_runtime_us)
}