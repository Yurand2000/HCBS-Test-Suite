use crate::prelude::*;

pub mod prelude {
    pub use super::runner_args::prelude::*;
    pub use super::{
        Taskset,
        TasksetConfig,
        TasksetRun,
        TasksetRunInsights,
        TasksetRunResultInstance,
        TasksetRunResult,
        TasksetRunResultInsights,
        compute_insights,
        compute_result_insights,
        can_run_taskset,
        check_root_cgroup,
    };
}

pub mod runner_args;
use runner_args::*;

#[derive(Debug, Clone)]
pub struct Taskset {
    pub name: String,
    pub data: Vec<PeriodicTaskData>,
}

#[derive(Debug, Clone)]
pub struct TasksetConfig {
    pub name: String,
    pub num_cpus: u64,
    pub runtime_ms: u64,
    pub period_ms: u64,
}

#[derive(Debug, Clone)]
pub struct TasksetRun {
    pub tasks: Taskset,
    pub config: TasksetConfig,
    pub results_file: String,
}

#[derive(Debug, Clone)]
pub struct TasksetRunInsights {
    pub expected_runtime_us: u64
}

#[derive(Debug, Clone)]
pub struct TasksetRunResultInstance {
    pub task: u64,
    pub instance: u64,
    pub abs_activation_time_us: u64,
    pub rel_start_time_us: u64,
    pub rel_finishing_time_us: u64,
    pub deadline_offset: f64,
}

#[derive(Debug, Clone)]
pub struct TasksetRunResult {
    pub taskset: Taskset,
    pub config: TasksetConfig,
    pub results: Vec<TasksetRunResultInstance>,
}

#[derive(Debug, Clone)]
pub struct TasksetRunResultInsights {
    pub num_overruns: u64,
    pub overruns_ratio: f64,
    pub worst_overrun: f64,
}

/* -------------------------------------------------------------------------- */

pub fn compute_insights(run: &TasksetRun, args: &RunnerArgsBase) -> TasksetRunInsights {
    let expected_runtime_us =
        run.tasks.data.iter()
        .map(|task| task.period_ms * args.num_instances_per_job * 1000)
        .max().unwrap();

    TasksetRunInsights {
        expected_runtime_us,
    }
}

pub fn compute_result_insights(run: &TasksetRunResult) -> TasksetRunResultInsights {
    let (num_overruns, worst_overrun) =
        run.results.iter()
        .fold((0u64, f64::NEG_INFINITY), |(mut num_overruns, worst_overrun), job_instance| {
            if job_instance.deadline_offset > 0f64 { num_overruns+= 1; }
            (num_overruns, worst_overrun.max(job_instance.deadline_offset))
        });

    let overruns_ratio = num_overruns as f64 / run.results.len() as f64;

    TasksetRunResultInsights {
        num_overruns,
        overruns_ratio,
        worst_overrun,
    }
}

pub fn can_run_taskset(run: &TasksetRun, args: &RunnerArgsBase) -> bool {
    if run.config.num_cpus > args.max_num_cpus {
        return false;
    }

    let taskset_bw = run.config.runtime_ms as f32 / run.config.period_ms as f32;
    if taskset_bw > args.max_allocable_bw {
        return false;
    }

    let min_task_period_ms = run.tasks.data.iter()
        .map(|task| task.period_ms).min().unwrap();

    if min_task_period_ms < (run.config.period_ms - run.config.runtime_ms) {
        return false;
    }

    true
}

pub fn check_root_cgroup(args: &RunnerArgsBase) -> Result<(), Box<dyn std::error::Error>> {
    mount_cgroup_fs()?;
    let cgroup_period = crate::cgroup::get_cgroup_period_us(".")?;
    let cgroup_runtime = crate::cgroup::get_cgroup_runtime_us(".")?;
    let cgroup_bw = cgroup_runtime as f32 / cgroup_period as f32;
    if cgroup_bw < args.max_allocable_bw {
        return Err(format!("Cannot run tasksets as the maximum allocable bandwidth is {cgroup_bw}, \
                            while you are requesting {} max bw", args.max_allocable_bw).into());
    }

    Ok(())
}

/* -------------------------------------------------------------------------- */

pub fn __os_str_to_str(string: &std::ffi::OsStr) -> Result<String, Box<dyn std::error::Error>> {
    Ok(
        string.to_os_string().into_string()
            .map_err(|err| format!("Conversion error: {err:?}"))?
    )
}

pub fn __path_to_str(path: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    __os_str_to_str(path.to_path_buf().as_os_str())
}