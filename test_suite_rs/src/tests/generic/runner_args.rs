pub mod prelude {
    pub use super::{
        RunnerArgsBase,
        RunnerArgsAll,
        RunnerArgsSingle
    };
}

#[derive(clap::Parser, Debug)]
pub struct RunnerArgsAll {
    #[command(flatten)]
    pub args: RunnerArgsBase,

    /// directory of tasksets description
    #[arg(short = 'i', long = "tasksets_dir", value_name = "path")]
    pub tasksets_dir: String,

    /// results/output directory
    #[arg(short = 'o', long = "output_dir", value_name = "path")]
    pub output_dir: String,
}

#[derive(clap::Parser, Debug)]
pub struct RunnerArgsSingle {
    #[command(flatten)]
    pub args: RunnerArgsBase,

    /// taskset to run
    #[arg(short = 'T', long = "taskset", value_name = "path")]
    pub taskset: String,

    /// cpu config to use
    #[arg(short = 'C', long = "config", value_name = "path")]
    pub config: String,

    /// output file
    #[arg(short = 'O', long = "output", value_name = "path")]
    pub output: String,
}

#[derive(clap::Parser, Debug)]
pub struct RunnerArgsBase {
    /// cgroup's name
    #[arg(short = 'c', long = "cgroup", default_value = "g0", value_name = "name")]
    pub cgroup: String,

    /// number of cpus of the machine
    #[arg(short = 'n', long = "cpus", value_name = "u64")]
    pub max_num_cpus: u64,

    /// max allocable bandwidth for the cgroup. This is usually 0.90 as 5% of
    /// the bandwidth is reserved for SCHED_OTHER tasks and the other 5% is used
    /// for overheads (?).
    #[arg(short = 'b', long = "max-bw", value_name = "f32", default_value = "0.90")]
    pub max_allocable_bw: f64,

    /// number of instances per job
    #[arg(short = 'j', long = "job", value_name = "u64", default_value = "200")]
    pub num_instances_per_job: u64,
}