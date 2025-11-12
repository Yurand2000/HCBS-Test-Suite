use crate::prelude::*;
use crate::tests::prelude::*;

pub mod prelude {
    pub use super::{
        main_run_taskset_array,
        main_run_taskset_single,
        main_read_results_array,
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

#[inline(always)]
pub fn main_read_results_array(args: RunnerArgsAll) -> Result<Vec<TasksetRunResult>, Box<dyn std::error::Error>> {
    read_taskset_results(
        &args,
    )
}

fn run_taskset(run: TasksetRun, args: &RunnerArgsBase, cycles: Option<u64>)
    -> Result<TasksetRunResult, Box<dyn std::error::Error>>
{
    todo!()
}

fn compute_cpu_speed() -> Result<u64, Box<dyn std::error::Error>> {
    todo!()
}