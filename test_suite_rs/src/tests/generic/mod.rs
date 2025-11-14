use crate::prelude::*;
use eva_engine::prelude::*;

pub mod prelude {
    pub use super::runner_args::prelude::*;
    pub use super::{
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
pub struct TasksetRun {
    pub taskset: NamedTaskset,
    pub config: NamedConfig,
    pub results_file: String,
}

#[derive(Debug, Clone)]
pub struct TasksetRunInsights {
    pub expected_runtime: Time
}

#[derive(Debug, Clone)]
pub struct TasksetRunResultInstance {
    pub task: u64,
    pub instance: u64,
    pub abs_activation_time: Time,
    pub rel_start_time: Time,
    pub rel_finishing_time: Time,
}

#[derive(Debug, Clone)]
pub struct TasksetRunResult {
    pub taskset: NamedTaskset,
    pub config: NamedConfig,
    pub results: Vec<TasksetRunResultInstance>,
}

#[derive(Debug, Clone)]
pub struct TasksetRunResultInsights {
    pub num_overruns: u64,
    pub overruns_ratio: f64,
    pub worst_overrun: Time,
}

/* -------------------------------------------------------------------------- */

impl TasksetRunResultInstance {
    pub fn slack_time(&self, task: &RTTask) -> Time {
        task.deadline - self.rel_finishing_time
    }
}

pub fn compute_insights(run: &TasksetRun, args: &RunnerArgsBase) -> TasksetRunInsights {
    let expected_runtime =
        run.taskset.tasks.iter()
        .map(|task| task.period)
        .max().unwrap() * args.num_instances_per_job as f64;

    TasksetRunInsights {
        expected_runtime,
    }
}

pub fn compute_result_insights(run: &TasksetRunResult) -> TasksetRunResultInsights {
    let (num_overruns, worst_overrun) =
        run.results.iter()
        .fold((0u64, Time::zero()), |(mut num_overruns, worst_overrun), job_instance| {
            let task = &run.taskset.tasks[job_instance.task as usize];
            let slack = job_instance.slack_time(task);

            if slack < Time::zero() { num_overruns += 1; }

            (num_overruns, worst_overrun.min(slack))
        });

    let overruns_ratio = num_overruns as f64 / run.results.len() as f64;

    TasksetRunResultInsights {
        num_overruns,
        overruns_ratio,
        worst_overrun,
    }
}

pub fn can_run_taskset(run: &TasksetRun, args: &RunnerArgsBase) -> bool {
    if run.config.cpus > args.max_num_cpus {
        return false;
    }

    let taskset_bw = run.config.runtime / run.config.period;
    if taskset_bw > args.max_allocable_bw {
        return false;
    }

    let min_task_period = run.taskset.tasks.iter()
        .map(|task| task.period).min().unwrap();

    if min_task_period < (run.config.period - run.config.runtime) {
        return false;
    }

    true
}

pub fn check_root_cgroup(args: &RunnerArgsBase) -> Result<(), Box<dyn std::error::Error>> {
    mount_cgroup_fs()?;
    let cgroup_period = crate::cgroup::get_cgroup_period_us(".")?;
    let cgroup_runtime = crate::cgroup::get_cgroup_runtime_us(".")?;
    let cgroup_bw = cgroup_runtime as f64 / cgroup_period as f64;
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