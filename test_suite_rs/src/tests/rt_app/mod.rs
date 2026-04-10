use crate::prelude::*;
use crate::tests::prelude::*;

pub mod prelude {
    pub use super::{
        main_run_taskset_array,
        main_run_taskset_single,
        main_run_taskset_array_multi,
        main_run_taskset_single_multi,
    };
}

pub mod result_parser;
pub mod config_generator;

#[inline(always)]
pub fn main_run_taskset_array(args: RunnerArgsAll) -> anyhow::Result<Vec<TasksetRunResult>> {
    run_taskset_array(
        &args,
        compute_cpu_speed,
        |run, args, cycles| run_taskset(run, args, cycles, false),
    )
}

#[inline(always)]
pub fn main_run_taskset_single(args: RunnerArgsSingle) -> anyhow::Result<Option<TasksetRunResult>> {
    run_taskset_single(
        &args,
        compute_cpu_speed,
        |run, args, cycles| run_taskset(run, args, cycles, false),
    )
}

#[inline(always)]
pub fn main_run_taskset_array_multi(args: RunnerArgsAll) -> anyhow::Result<Vec<TasksetRunResult>> {
    run_taskset_array(
        &args,
        compute_cpu_speed,
        |run, args, cycles| run_taskset(run, args, cycles, true),
    )
}

#[inline(always)]
pub fn main_run_taskset_single_multi(args: RunnerArgsSingle) -> anyhow::Result<Option<TasksetRunResult>> {
    run_taskset_single(
        &args,
        compute_cpu_speed,
        |run, args, cycles| run_taskset(run, args, cycles, true),
    )
}

fn run_taskset(run: TasksetRun, args: &RunnerArgsBase, cycles: Option<u64>, multi_runtime: bool)
    -> anyhow::Result<TasksetRunResult>
{
    let log_dir = "/tmp/rt-app";
    let config_file = "/tmp/rt-app-config.json";
    let stdout_file = "/tmp/rt-app-stdout.txt";

    std::fs::create_dir_all(log_dir)?;

    config_generator::generate_taskset_config(
        &run.taskset,
        args,
        cycles,
        log_dir,
        config_file
    )?;

    let cpu_set = CpuSet::any_subset(run.config.cpus)?;
    let mut cgroup = HCBSCgroup::new(&args.cgroup)?
        .with_force_kill(true);
    cgroup.set_period_us(run.config.period.as_micros() as u64)?;
    cgroup.set_runtime_us(run.config.runtime.as_micros() as u64)?;

    cgroup.set_period_us(run.config.period.as_micros() as u64)?;
    if !multi_runtime  {
        cgroup.set_runtime_us(run.config.runtime.as_micros() as u64)?;
    } else {
        cgroup.set_runtime_us_multi([
            (run.config.runtime.as_micros() as u64,
            cpu_set.iter().map(|cpu| *cpu))
        ])?;
    }

    let self_proc = cgroup.assign_process(HCBSProcess::SelfProc).map_err(|(_, err)| err)?;
    self_proc.set_sched_policy(SchedPolicy::RR(99), SchedFlags::RESET_ON_FORK)?;
    if !multi_runtime  {
        self_proc.set_affinity(cpu_set)?;
    }

    let mut proc = run_rt_app(config_file, stdout_file)?;
    proc.wait()
        .map_err(|err| anyhow::format_err!("Error in waiting for rt-app: {err}"))?;

    std::mem::drop(cgroup);

    let result = result_parser::parse_taskset_results(&run.taskset, log_dir)?;
    let result = TasksetRunResult {
        taskset: run.taskset,
        config: run.config,
        results: result,
    };

    Ok(result)
}

fn compute_cpu_speed() -> anyhow::Result<u64> {
    let config_file = "/tmp/rt-app-config.json";
    let stdout_file = "/tmp/rt-app-calibration.txt";

    config_generator::generate_calibration_config(config_file)?;

    // run rt-app to calibrate
    assign_pid_to_cgroup(".", std::process::id())?;
    set_sched_policy(std::process::id(), SchedPolicy::RR(99), SchedFlags::RESET_ON_FORK)?;
    set_cpuset_to_pid(std::process::id(), &CpuSet::any_subset(1)?)?;

    let mut proc = run_rt_app(config_file, stdout_file)?;
    proc.wait()
        .map_err(|err| anyhow::format_err!("Error in waiting for rt-app: {err}"))?;

    set_cpuset_to_pid(std::process::id(), &CpuSet::all()?)?;
    set_sched_policy(std::process::id(), SchedPolicy::other(), SchedFlags::RESET_ON_FORK)?;

    // read calibration results
    let out_data = std::fs::read_to_string(&stdout_file)
        .map_err(|err| anyhow::format_err!("Couldn't read file: {}, reason: {}", &stdout_file, err))?;
    out_data.lines().find(|line| line.contains("pLoad ="))
        .ok_or(anyhow::format_err!("Calibration error: load measuring not found"))
        .and_then(|line| {
            line.trim_ascii().split_ascii_whitespace().skip(4).next()
                .ok_or(anyhow::format_err!("Calibration error: load measuring not found [2]"))
            .and_then(|cycles| {
                // remove the "ns" part from the token
                let cycles = &cycles[0 .. cycles.len() - 2];

                cycles.parse::<u64>()
                    .map_err(|err| anyhow::format_err!("Calibration error: {err}"))
            })
        })
}

fn run_rt_app(config_file: &str, stdout_file: &str) -> anyhow::Result<HCBSProcess> {
    use std::process::*;

    let cmd = local_executable_cmd("/bin", "rt-app")?;

    let stdout_file = std::fs::OpenOptions::new().write(true).create(true).open(stdout_file)
        .map_err(|err| anyhow::format_err!("Stdout file '{}' creation error: {}", stdout_file, err))?;

    let proc = Command::new(cmd)
        .args([config_file])
        .stdin(Stdio::null())
        .stdout(stdout_file.try_clone()?)
        .stderr(stdout_file)
        .spawn()
        .map_err(|err| anyhow::format_err!("Error in starting rt-app: {err}"))?;

    Ok(HCBSProcess::Child(proc))
}