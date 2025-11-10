use eva_engine::prelude::*;
use crate::generator::{
    AnalysisOptions,
    TasksetGeneratorOptions
};

#[derive(Debug, Clone)]
#[derive(clap::Parser)]
pub struct Args {
    /// RNG seed
    #[arg(short='R', default_value="42", value_name="SEED")]
    pub generator_seed: u64,

    #[command(flatten, next_help_heading="Taskset Generation Options")]
    pub taskset: TasksetGeneratorArgs,

    #[command(flatten, next_help_heading="Schedulability Analysis Options")]
    pub analysis: AnalysisArgs,

    #[command(flatten, next_help_heading="Output Options")]
    pub output: OutputArgs,
}

#[derive(Debug, Clone)]
#[derive(clap::Args)]
pub struct TasksetGeneratorArgs {
    /// Number of tasksets to generate for the same utilization value
    #[arg(long="tasksets-per-utilization", default_value="3", value_name="TASKSETS")]
    pub tasksets_per_utilization: u64,

    /// Minimum number of tasks in a taskset
    #[arg(short='n', default_value="6", value_name="TASKS")]
    pub min_num_tasks: u64,

    /// Maximum number of tasks in a taskset
    #[arg(short='N', default_value="16", value_name="TASKS")]
    pub max_num_tasks: u64,

    /// Minimum period of a task
    #[arg(short='p', default_value="100", value_name="PERIOD ms")]
    pub min_task_period_ms: u64,

    /// Maximum period of a task
    #[arg(short='P', default_value="500", value_name="PERIOD ms")]
    pub max_task_period_ms: u64,

    /// Period granularity of a task
    #[arg(long="p-gran", default_value="200", value_name="PERIOD ms")]
    pub step_task_period_ms: u64,

    /// Minimum taskset total utilization
    #[arg(short='u', default_value="0.5", value_name="UTILIZATION")]
    pub min_taskset_utilization: f64,

    /// Maximum taskset total utilization
    #[arg(short='U', default_value="2.5", value_name="UTILIZATION")]
    pub max_taskset_utilization: f64,

    /// Taskset total utilization granularity
    #[arg(long="u-gran", default_value="0.2", value_name="UTILIZATION")]
    pub step_taskset_utilization: f64,
}

#[derive(Debug, Clone)]
#[derive(clap::Args)]
pub struct AnalysisArgs {
    /// Minimum cgroup period
    #[arg(short='c', default_value="20", value_name="PERIOD ms")]
    pub min_cgroup_period_ms: u64,

    /// Maximum cgroup period
    #[arg(short='C', default_value="100", value_name="PERIOD ms")]
    pub max_cgroup_period_ms: u64,

    /// Cgroup period granularity
    #[arg(long="c-gran", default_value="40", value_name="PERIOD ms")]
    pub step_cgroup_period_ms: u64,

    /// Max bandwidth per core in cgroup
    #[arg(long="max-core-bw", default_value="0.9", value_name="BANDWIDTH")]
    pub max_per_core_bandwidth: f64,
}

#[derive(Debug, Clone)]
#[derive(clap::Args)]
pub struct OutputArgs {
    /// Output directory for generated tasksets
    #[arg(short='O', value_name="OUTPUT DIR")]
    pub out_directory: String,
}

impl Into<TasksetGeneratorOptions> for TasksetGeneratorArgs {
    fn into(self) -> TasksetGeneratorOptions {
        TasksetGeneratorOptions {
            tasksets_per_utilization:
                self.tasksets_per_utilization,
            num_tasks: (
                self.min_num_tasks,
                self.max_num_tasks,
            ),
            task_period_ms: (
                Time::millis(self.min_task_period_ms as f64),
                Time::millis(self.max_task_period_ms as f64),
                Time::millis(self.step_task_period_ms as f64),
            ),
            taskset_utilization: (
                self.min_taskset_utilization,
                self.max_taskset_utilization,
                self.step_taskset_utilization,
            ),
        }
    }
}

impl Into<AnalysisOptions> for AnalysisArgs {
    fn into(self) -> AnalysisOptions {
        AnalysisOptions {
            cgroup_period: (
                Time::millis(self.min_cgroup_period_ms as f64),
                Time::millis(self.max_cgroup_period_ms as f64),
                Time::millis(self.step_cgroup_period_ms as f64),
            ),
            max_per_core_bandwidth:
                self.max_per_core_bandwidth,
        }
    }
}