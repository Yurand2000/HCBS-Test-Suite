use crate::prelude::*;
use crate::tests::prelude::*;

pub mod prelude {
    pub use super::{
        main_run_taskset_array,
        main_run_taskset_single,
        main_read_results_array,
    };
}

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
    if run.config.num_cpus > args.max_num_cpus {
        println!("- Error on taskset {}, config {}", run.tasks.name, run.config.name);
        println!("  Attempted to run taskset with {0} CPUs on a maximum of {1} CPUs",
            run.config.num_cpus, args.max_num_cpus);
        panic!("unexpected");
    }

    let taskset_bw = run.config.runtime_ms as f32 / run.config.period_ms as f32;
    if taskset_bw > args.max_allocable_bw {
        println!("- Error on taskset {}, config {}", run.tasks.name, run.config.name);
        println!("  Attempted to allocate more bandwidth ({}) than the maximum allocable ({})",
            taskset_bw, args.max_allocable_bw);
        panic!("unexpected");
    }

    let tmp_output_file = "/tmp/out.txt";
    if std::fs::exists(tmp_output_file)
        .map_err(|err| format!("Error in checking existance of {tmp_output_file}: {err}"))?
    {
        std::fs::remove_file(tmp_output_file)
            .map_err(|err| format!("Error in removing {tmp_output_file}: {err}"))?
    }

    let cgroup = MyCgroup::new(
        &args.cgroup,
        run.config.runtime_ms * 1000,
        run.config.period_ms * 1000,
        true
    )?;

    migrate_task_to_cgroup(&args.cgroup, std::process::id())?;
    set_scheduler(std::process::id(), SchedPolicy::RR(99))?;
    set_cpuset_to_pid(std::process::id(), &CpuSet::any_subset(run.config.num_cpus)?)?;

    let pthread_data = PeriodicThreadData {
        start_priority: 98,
        cpu_speed: cycles,
        tasks: run.tasks.data.clone(),
        extra_args: String::new(),
        out_file: tmp_output_file.to_owned(),
        num_instances_per_job: args.num_instances_per_job,
    };

    let mut proc = run_periodic_thread(pthread_data)?;
    proc.wait()?;

    set_cpuset_to_pid(std::process::id(), &CpuSet::all()?)?;
    set_scheduler(std::process::id(), SchedPolicy::other())?;
    migrate_task_to_cgroup(".", std::process::id())?;

    cgroup.destroy()?;

    let result = TasksetRunResult {
        taskset: run.tasks,
        config: run.config,
        results: parse_taskset_results(tmp_output_file)?,
    };

    //assert result is compatible with program input
    for i in 0..result.taskset.data.len() {
        let ith_job_instances =
            result.results.iter()
            .filter(|res| res.task == (i as u64))
            .count() as u64;

        if ith_job_instances != args.num_instances_per_job {
            return Err(format!("Taskset {}, config {}, generated an incorrect output.", result.taskset.name, result.config.name).into());
        }
    }

    Ok(result)
}

fn compute_cpu_speed() -> Result<u64, Box<dyn std::error::Error>> {
    let out_file = format!("/tmp/calibration_data.txt");

    // run periodic thread to calibrate
    migrate_task_to_cgroup(".", std::process::id())?;
    set_scheduler(std::process::id(), SchedPolicy::RR(99))?;
    set_cpuset_to_pid(std::process::id(), &CpuSet::any_subset(1)?)?;

    let mut proc = run_periodic_thread(PeriodicThreadData {
        start_priority: 99,
        cpu_speed: None,
        tasks: vec![ PeriodicTaskData { runtime_ms: 10, period_ms: 100 }],
        num_instances_per_job: 1,
        extra_args: String::with_capacity(0),
        out_file: out_file.clone(),
    })?;
    proc.wait()?;

    set_cpuset_to_pid(std::process::id(), &CpuSet::all()?)?;
    set_scheduler(std::process::id(), SchedPolicy::other())?;

    // read calibration results
    let out_data = std::fs::read_to_string(out_file)?;
    out_data.lines().find(|line| line.starts_with("#Cycles:"))
        .ok_or(format!("Calibration error: Cycles measuring not found").into())
        .and_then(|line| {
            line.trim_ascii().split_ascii_whitespace().skip(1).next()
                .ok_or(format!("Calibration error: Cycles measuring not found [2]").into())
            .and_then(|cycles| cycles.parse::<u64>()
                .map_err(|err| format!("Calibration error: {err}").into())
            )
        })
}

fn parse_taskset_results(out_file: &str) -> Result<Vec<TasksetRunResultInstance>, Box<dyn std::error::Error>> {
    use nom::Parser;
    use nom::multi::*;
    use nom::character::complete::*;
    use nom::combinator::*;
    use nom::sequence::*;

    let data = std::fs::read_to_string(out_file)
        .map_err(|err| format!("Failed to read output file {}: {}", out_file, err))?;

    let u64_parser = map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());
    let f64_parser = map_res(recognize((
            opt(char('-')),
            digit1,
            char('.'),
            digit1
        )), |num: &str| num.parse::<f64>());
    let mut line_parser =
        map_res(
            (count(terminated(u64_parser, space1), 5), f64_parser),
            |(fields, offset)| {
                Ok::<_, ()>(TasksetRunResultInstance {
                    task: fields[0],
                    instance: fields[1],
                    abs_activation_time_us: fields[2],
                    rel_start_time_us: fields[3],
                    rel_finishing_time_us: fields[4],
                    deadline_offset: offset,
                })
            }
        );

    let data: Vec<_> = data.trim_ascii().lines()
        .filter_map(|line| {
            let line = line.trim_ascii();
            if line.starts_with("#") {
                None
            } else {
                Some(line_parser.parse(&line).map(|(_, res)| res))
            }
        })
        .try_collect()
        .map_err(|err| format!("Taskset result parser error: {err}"))?;

    Ok(data)
}