use crate::{prelude::*, utils::{batch_test_failure, batch_test_skipped, batch_test_success}};

#[derive(clap::Parser, Debug)]
pub struct MyArgsAll {
    #[command(flatten)]
    args: MyArgs,

    /// directory of tasksets description
    #[arg(short = 'i', long = "tasksets_dir", value_name = "path")]
    pub tasksets_dir: String,

    /// results/output directory
    #[arg(short = 'o', long = "output_dir", value_name = "path")]
    pub output_dir: String,
}

#[derive(clap::Parser, Debug)]
pub struct MyArgsSpecific {
    #[command(flatten)]
    args: MyArgs,

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
pub struct MyArgs {
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
    pub max_allocable_bw: f32,

    /// number of instances per job
    #[arg(short = 'j', long = "job", value_name = "u64", default_value = "200")]
    pub num_instances_per_job: u64,
}

pub struct MyResult {
    results: Vec<TasksetRunResult>,
}

#[derive(Debug)]
#[derive(Clone)]
struct Taskset {
    name: String,
    data: Vec<PeriodicTaskData>,
}

#[derive(Debug)]
#[derive(Clone)]
struct TasksetConfig {
    name: String,
    num_cpus: u64,
    runtime_ms: u64,
    period_ms: u64,
}

#[derive(Debug)]
#[derive(Clone)]
struct TasksetRun {
    tasks: Taskset,
    config: TasksetConfig,
    output_file: String,
}

#[derive(Debug)]
#[derive(Clone)]
struct TasksetRunInsights {
    expected_runtime_us: u64
}

#[derive(Debug)]
#[derive(Clone)]
struct TasksetRunResultInstance {
    task: u64,
    instance: u64,
    abs_activation_time_us: u64,
    rel_start_time_us: u64,
    rel_finishing_time_us: u64,
    deadline_offset: f64,
}

#[derive(Debug)]
#[derive(Clone)]
pub struct TasksetRunResult {
    taskset: Taskset,
    config: TasksetConfig,
    results: Vec<TasksetRunResultInstance>,
}

#[derive(Clone)]
struct TasksetRunResultInsights {
    num_overruns: u64,
    overruns_ratio: f64,
    worst_overrun: f64,
}

mod parser;
use parser::*;

fn __os_str_to_str(string: &std::ffi::OsStr) -> Result<String, Box<dyn std::error::Error>> {
    Ok(
        string.to_os_string().into_string()
            .map_err(|err| format!("Conversion error: {err:?}"))?
    )
}

fn __path_to_str(path: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    __os_str_to_str(path.to_path_buf().as_os_str())
}

fn run_taskset(run: TasksetRun, args: &MyArgs, cycles: Option<u64>)
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

    let dirs = std::path::Path::new(&run.output_file).parent()
        .ok_or_else(|| format!("Unknown parent"))?;

    std::fs::create_dir_all(dirs)
        .map_err(|err| format!("Error in creating directory(ies) {dirs:?}: {err}"))?;
    std::fs::copy(tmp_output_file, &run.output_file)
        .map_err(|err| format!("Output file {} copy error: {}", &run.output_file, err))?;

    let result = TasksetRunResult {
        taskset: run.tasks,
        config: run.config,
        results: parse_taskset_results(&run.output_file)?,
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

fn compute_insights(run: &TasksetRun, args: &MyArgs) -> TasksetRunInsights {
    let expected_runtime_us =
        run.tasks.data.iter()
        .map(|task| task.period_ms * args.num_instances_per_job * 1000)
        .max().unwrap();

    TasksetRunInsights { expected_runtime_us }
}

fn compute_result_insights(run: &TasksetRunResult) -> TasksetRunResultInsights {
    let (num_overruns, worst_overrun) =
        run.results.iter()
        .fold((0u64, f64::NEG_INFINITY), |(mut num_overruns, worst_overrun), job_instance| {
            if job_instance.deadline_offset > 0f64 { num_overruns+= 1; }
            (num_overruns, worst_overrun.max(job_instance.deadline_offset))
        });

    TasksetRunResultInsights {
        num_overruns,
        overruns_ratio: num_overruns as f64 / run.results.len() as f64,
        worst_overrun,
    }
}

fn can_run_taskset(run: &TasksetRun, args: &MyArgs) -> bool {
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

fn get_taskset_run(taskset: &str, config: &str, output_file: &str) -> Result<TasksetRun, Box<dyn std::error::Error>> {
    let taskset = parse_taskset_file(taskset)?;
    let config = parse_config_file(config)?;

    Ok(TasksetRun {
        tasks: taskset,
        config,
        output_file: output_file.to_owned(),
    })
}

fn get_tasksets_runs(args: &MyArgsAll) -> Result<Vec<TasksetRun>, Box<dyn std::error::Error>> {
    let tasksets_dir = &args.tasksets_dir;

    let mut taskset_runs = Vec::new();
    for taskset_dir in std::fs::read_dir(&tasksets_dir)
        .map_err(|err| format!("Tasksets directory {} error: {}", &tasksets_dir, err))?
    {
        let taskset_dir = taskset_dir?.path();
        if !taskset_dir.is_dir() {
            continue;
        }

        let files: Vec<String> = std::fs::read_dir(&taskset_dir)
            .map_err(|err| format!("Taskset data directory {:?} error: {}", &taskset_dir, err))?
            .map(|entry| entry.map(|entry| entry.path()))
            .filter(|entry| entry.as_ref().is_ok_and(|entry| entry.is_file()))
            .map(|file| file
                .map_err(|err| Into::<Box<dyn std::error::Error>>::into(err))
                .and_then(|file| file.file_name()
                    .ok_or_else(|| Into::<Box<dyn std::error::Error>>::into(
                        format!("File name not found"))
                    )
                    .and_then(|file| __os_str_to_str(file))
                )
            )
            .try_collect()?;

        let taskset_dir = __path_to_str(taskset_dir.as_path())?;
        if files.iter().find(|file| *file == "taskset.txt").is_none() {
            Err(format!("taskset.txt file not found for taskset {}", taskset_dir))?;
        }

        if files.len() <= 1 {
            continue;
        }

        let taskset = parse_taskset_file(&format!("{taskset_dir}/taskset.txt"))?;
        let mut runs: Vec<_> = files.iter().filter(|f| *f != "taskset.txt")
            .map(|config| {
                parse_config_file(&format!("{taskset_dir}/{config}"))
                    .map(|config| {
                        let output_file = format!("{}/{}/output-{}",
                            &args.output_dir, &taskset.name, &config.name);

                        TasksetRun {
                            tasks: taskset.clone(),
                            config,
                            output_file,
                        }
                    })
            })
            .try_collect()?;

        taskset_runs.append(&mut runs);
    }

    taskset_runs.sort_unstable_by(|l, r| {
        match l.tasks.name.cmp(&r.tasks.name) {
            std::cmp::Ordering::Equal =>
                l.config.name.cmp(&r.config.name),
            other =>
                other,
        }
    });

    Ok(taskset_runs)
}

fn check_root_cgroups(args: &MyArgs) -> Result<(), Box<dyn std::error::Error>> {
    mount_cgroup_fs()?;
    let cgroup_period = crate::cgroup::get_cgroup_period_us(".")?;
    let cgroup_runtime = crate::cgroup::get_cgroup_runtime_us(".")?;
    let cgroup_bw = cgroup_runtime as f32 / cgroup_period as f32;
    if cgroup_bw < args.max_allocable_bw {
        return Err(format!("Cannot run tasksets as the maximum allocable bandwidth is {cgroup_bw}, while you are requesting {} bw", args.max_allocable_bw).into());
    }

    Ok(())
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

pub fn main_run_taskset_array(args: MyArgsAll) -> Result<MyResult, Box<dyn std::error::Error>> {
    check_root_cgroups(&args.args)?;

    // run tasksets
    let taskset_runs = get_tasksets_runs(&args)?;

    // taskset first insights
    let total_expected_runtime_us: u64 = taskset_runs.iter()
        .filter(|run| can_run_taskset(run, &args.args))
        .filter(|run| !std::path::Path::new(&run.output_file).exists())
        .map(|run| compute_insights(run, &args.args).expected_runtime_us)
        .sum();

    let total_runs = taskset_runs.len() as u64;
    let todo_runs = taskset_runs.iter()
        .filter(|run| can_run_taskset(run, &args.args))
        .filter(|run| !std::path::Path::new(&run.output_file).exists())
        .count() as u64;
    let any_skips = taskset_runs.iter()
        .filter(|run| can_run_taskset(run, &args.args))
        .any(|run| std::path::Path::new(&run.output_file).exists());

    println!("[taskset] Taskset Tests ");
    println!("          Running {}/{} tasksets", todo_runs, total_runs);
    println!("          Expected runtime: {:.2} secs", total_expected_runtime_us as f64 / 1000_000f64);
    if any_skips {
        println!("          Delete the folder '{}' to rerun all tests", args.output_dir);
    }

    // pre-compute the number of cycles per second the CPUs can do.
    let cycles = compute_cpu_speed()?;
    println!("  [debug] Calibration results: {} cycles", cycles);

    // run experiments
    let mut failures = 0u64;
    let mut results = Vec::with_capacity(taskset_runs.len());
    for run in taskset_runs.into_iter() {
        match run_taskset_single(run, Some(cycles), &args.args)? {
            Some(result) => {
                if compute_result_insights(&result).num_overruns > 0 {
                    failures += 1;
                }

                results.push(result);
            },
            None => continue,
        }
    }

    println!("[taskset] Taskset Tests ");
    println!("          Outcome: {}/{} failures/tests, {:.2} failure ratio",
        failures, total_runs, failures as f64 / total_runs as f64);

    Ok(MyResult { results })
}

pub fn main_run_taskset_single(args: MyArgsSpecific) -> Result<Option<TasksetRunResult>, Box<dyn std::error::Error>> {
    check_root_cgroups(&args.args)?;

    println!("[taskset] Taskset Single Test ");

    let cycles = compute_cpu_speed()?;
    println!("  [debug] Calibration results: {} cycles", cycles);

    let run = get_taskset_run(&args.taskset, &args.config, &args.output)?;
    Ok(run_taskset_single(run, Some(cycles), &args.args)?)
}

fn run_taskset_single(run: TasksetRun, cycles: Option<u64>, args: &MyArgs) -> Result<Option<TasksetRunResult>, Box<dyn std::error::Error>> {
    let already_run = std::path::Path::new(&run.output_file).exists();

    let insights = compute_insights(&run, &args);
    let taskset_header = format!("{} on {}", run.tasks.name, run.config.name);
    let taskset_header =
        if already_run {
            taskset_header + " (already run)"
        } else {
            taskset_header + &format!(" (~{:.2} secs)", insights.expected_runtime_us as f64 / 1000_000f64)
        };
    batch_test_header(&taskset_header, "taskset");

    if !can_run_taskset(&run, &args) {
        batch_test_skipped("cannot run on current config");
        return Ok(None);
    }

    let result =
        if already_run {
            Ok(TasksetRunResult {
                taskset: run.tasks,
                config: run.config,
                results: parse_taskset_results(&run.output_file)?,
            })
        } else {
            run_taskset(run, &args, cycles)
        }?;

    let insights = compute_result_insights(&result);

    if insights.num_overruns > 0 {
        batch_test_failure(format!("Deadline overrun: {:.2} % error rate, {} worst overrun",
            insights.overruns_ratio * 100f64, insights.worst_overrun));
    } else {
        batch_test_success();
    }

    Ok(Some(result))
}

pub fn main_read_results_array(args: MyArgsAll) -> Result<MyResult, Box<dyn std::error::Error>> {
    let taskset_runs = get_tasksets_runs(&args)?;

    // taskset first insights
    let total_runs = taskset_runs.len() as u64;
    let todo_runs = taskset_runs.iter()
        .filter(|run| can_run_taskset(run, &args.args))
        .filter(|run| std::path::Path::new(&run.output_file).exists())
        .count() as u64;

    println!("Run {}/{} tasksets.", todo_runs, total_runs);

    // read results
    let mut failures = 0u64;
    let mut results = Vec::with_capacity(taskset_runs.len());
    for run in taskset_runs.into_iter() {
        if !can_run_taskset(&run, &args.args) {
            continue;
        }

        let taskset_name = run.tasks.name.clone();
        let config_name = run.config.name.clone();

        let result =
            if std::path::Path::new(&run.output_file).exists() {
                TasksetRunResult {
                    taskset: run.tasks,
                    config: run.config,
                    results: parse_taskset_results(&run.output_file)?,
                }
            } else {
                println!("* Taskset {}, config {}: no output", run.tasks.name, run.config.name);
                continue;
            };

        let insights = compute_result_insights(&result);

        if insights.num_overruns > 0 {
            println!("- Taskset {}, config {} failed: {:.2} % error rate, {} worst overrun",
            taskset_name, config_name, insights.overruns_ratio * 100f64, insights.worst_overrun);

            failures += 1;
        }

        results.push(result);
    }

    println!("Outcome: {}/{} failures/tests, {:.2} failure ratio",
        failures, total_runs, failures as f64 / total_runs as f64);

    Ok(MyResult { results })
}