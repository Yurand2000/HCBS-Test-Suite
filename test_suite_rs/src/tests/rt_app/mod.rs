use crate::prelude::*;
use crate::tests::prelude::*;

pub mod prelude {
    pub use super::{
        main_run_taskset_array,
        main_run_taskset_single,
    };
}

pub mod result_parser;
pub mod config_generator;

#[inline(always)]
pub fn main_run_taskset_array(args: RunnerArgsAll) -> Result<Vec<TasksetRunResult>, Box<dyn std::error::Error>> {
    run_taskset_array(
        &args,
        compute_cpu_speed,
        run_taskset,
    )
}

#[inline(always)]
pub fn main_run_taskset_single(args: RunnerArgsSingle) -> Result<Option<TasksetRunResult>, Box<dyn std::error::Error>> {
    run_taskset_single(
        &args,
        compute_cpu_speed,
        run_taskset,
    )
}

fn run_taskset(run: TasksetRun, args: &RunnerArgsBase, cycles: Option<u64>)
    -> Result<TasksetRunResult, Box<dyn std::error::Error>>
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

    let cgroup = MyCgroup::new(
        &args.cgroup,
        run.config.runtime.as_micros() as u64,
        run.config.period.as_micros() as u64,
        true
    )?;

    migrate_task_to_cgroup(&args.cgroup, std::process::id())?;
    set_scheduler(std::process::id(), SchedPolicy::RR(99))?;
    set_cpuset_to_pid(std::process::id(), &CpuSet::any_subset(run.config.cpus)?)?;

    let mut proc = run_rt_app(config_file, stdout_file)?;
    proc.wait()
        .map_err(|err| format!("Error in waiting for rt-app: {err}"))?;

    set_cpuset_to_pid(std::process::id(), &CpuSet::all()?)?;
    set_scheduler(std::process::id(), SchedPolicy::other())?;
    migrate_task_to_cgroup(".", std::process::id())?;

    cgroup.destroy()?;

    let result = result_parser::parse_taskset_results(&run.taskset, log_dir)?;
    let result = TasksetRunResult {
        taskset: run.taskset,
        config: run.config,
        results: result,
    };

    Ok(result)
}

fn compute_cpu_speed() -> Result<u64, Box<dyn std::error::Error>> {
    let config_file = "/tmp/rt-app-config.json";
    let stdout_file = "/tmp/rt-app-calibration.txt";

    config_generator::generate_calibration_config(config_file)?;

    // run rt-app to calibrate
    migrate_task_to_cgroup(".", std::process::id())?;
    set_scheduler(std::process::id(), SchedPolicy::RR(99))?;
    set_cpuset_to_pid(std::process::id(), &CpuSet::any_subset(1)?)?;

    let mut proc = run_rt_app(config_file, stdout_file)?;
    proc.wait()
        .map_err(|err| format!("Error in waiting for rt-app: {err}"))?;

    set_cpuset_to_pid(std::process::id(), &CpuSet::all()?)?;
    set_scheduler(std::process::id(), SchedPolicy::other())?;

    // read calibration results
    let out_data = std::fs::read_to_string(&stdout_file)
        .map_err(|err| format!("Couldn't read file: {}, reason: {}", &stdout_file, err))?;
    out_data.lines().find(|line| line.contains("pLoad ="))
        .ok_or(format!("Calibration error: load measuring not found").into())
        .and_then(|line| {
            line.trim_ascii().split_ascii_whitespace().skip(4).next()
                .ok_or(format!("Calibration error: load measuring not found [2]").into())
            .and_then(|cycles| {
                // remove the "ns" part from the token
                let cycles = &cycles[0 .. cycles.len() - 2];

                cycles.parse::<u64>()
                    .map_err(|err| format!("Calibration error: {err}").into())
            })
        })
}

fn run_rt_app(config_file: &str, stdout_file: &str) -> Result<MyProcess, Box<dyn std::error::Error>> {
    use std::process::*;

    let cmd = local_executable_cmd("/bin", "rt-app")?;

    let stdout_file = std::fs::OpenOptions::new().write(true).create(true).open(stdout_file)
        .map_err(|err| format!("Stdout file '{}' creation error: {}", stdout_file, err))?;

    let proc = Command::new(cmd)
        .args([config_file])
        .stdin(Stdio::null())
        .stdout(stdout_file.try_clone()?)
        .stderr(stdout_file)
        .spawn()
        .map_err(|err| format!("Error in starting rt-app: {err}"))?;

    Ok(MyProcess { process: proc })
}