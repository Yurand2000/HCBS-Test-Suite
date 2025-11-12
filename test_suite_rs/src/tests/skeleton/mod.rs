use crate::prelude::*;
use crate::tests::prelude::*;
use crate::tests::generic::{
    __os_str_to_str,
    __path_to_str,
};

pub mod prelude {
    pub use super::parser::prelude::*;
    pub use super::{
        run_taskset_array,
        run_taskset_single,
        read_taskset_results,
    };
}

pub mod parser;
use parser::read_all_file;

pub fn run_taskset_array<FnSpeed, FnRun>(
    args: &RunnerArgsAll,
    fn_compute_cpu_speed: FnSpeed,
    fn_run_taskset: FnRun,
) -> Result<Vec<TasksetRunResult>, Box<dyn std::error::Error>>
    where
        FnSpeed:        Fn()
                          -> Result<u64, Box<dyn std::error::Error>>,
        FnRun:          Fn(TasksetRun, &RunnerArgsBase, Option<u64>)
                            -> Result<TasksetRunResult, Box<dyn std::error::Error>>,
{
    check_root_cgroup(&args.args)?;

    // get taskset runs (i.e. taskset + config combinations)
    let taskset_runs: Vec<TasksetRun> = get_taskset_runs(&args)?;

    // compute taskset insights
    let total_expected_runtime_us: u64 = taskset_runs.iter()
        .filter(|run| can_run_filter(run, &args.args))
        .map(|run| compute_insights(run, &args.args).expected_runtime_us)
        .sum();

    let total_runs = taskset_runs.len();
    let todo_runs = taskset_runs.iter()
        .filter(|run| can_run_filter(run, &args.args))
        .count();

    println!("[taskset] Taskset Tests ");
    println!("          Running {}/{} tasksets", todo_runs, total_runs);
    println!("          Expected runtime: {:.2} secs", total_expected_runtime_us as f64 / 1000_000f64);
    if total_runs - todo_runs > 0 {
        println!("          Delete the folder '{}' to rerun all tests", args.output_dir);
    }

    // pre-compute the number of cycles per second the CPUs can do.
    let cycles = fn_compute_cpu_speed()?;
    println!("  [debug] Calibration results: {} cycles", cycles);

    // run experiments
    let mut failures = 0u64;
    let mut results = Vec::with_capacity(taskset_runs.len());
    for run in taskset_runs.into_iter() {
        match run_taskset_one(run, &args.args, Some(cycles), &fn_run_taskset)? {
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

    Ok(results)
}

pub fn run_taskset_single<FnSpeed, FnRun>(
    args: &RunnerArgsSingle,
    compute_cpu_speed: FnSpeed,
    fn_run_taskset: FnRun,
) -> Result<Option<TasksetRunResult>, Box<dyn std::error::Error>>
    where
        FnSpeed:        Fn()
                            -> Result<u64, Box<dyn std::error::Error>>,
        FnRun:          Fn(TasksetRun, &RunnerArgsBase, Option<u64>)
                            -> Result<TasksetRunResult, Box<dyn std::error::Error>>,
{
    check_root_cgroup(&args.args)?;

    println!("[taskset] Taskset Single Test ");

    let cycles = compute_cpu_speed()?;
    println!("  [debug] Calibration results: {} cycles", cycles);

    let run = get_taskset_run(&args.taskset, &args.config, &args.output)?;
    Ok(run_taskset_one(run, &args.args, Some(cycles), &fn_run_taskset)?)
}

pub fn read_taskset_results(
    args: &RunnerArgsAll
) -> Result<Vec<TasksetRunResult>, Box<dyn std::error::Error>> {
    let taskset_runs = get_taskset_runs(&args)?;

    // taskset first insights
    let total_runs = taskset_runs.len() as u64;
    let todo_runs = taskset_runs.iter()
        .filter(|run| can_run_filter(run, &args.args))
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
            if std::path::Path::new(&run.results_file).exists() {
                TasksetRunResult {
                    taskset: run.tasks,
                    config: run.config,
                    results: parse_result(&read_all_file(&run.results_file)?)?,
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

    Ok(results)
}

fn get_taskset_run(taskset: &str, config: &str, output_file: &str) -> Result<TasksetRun, Box<dyn std::error::Error>> {
    let taskset = parse_taskset(&read_all_file(taskset)?)?;
    let config = parse_config(&read_all_file(config)?)?;

    Ok(TasksetRun {
        tasks: taskset,
        config,
        results_file: output_file.to_owned(),
    })
}

fn get_taskset_runs(args: &RunnerArgsAll) -> Result<Vec<TasksetRun>, Box<dyn std::error::Error>> {
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

        let taskset = parse_taskset(&read_all_file(&format!("{taskset_dir}/taskset.txt"))?)?;
        let mut runs: Vec<_> = files.iter().filter(|f| *f != "taskset.txt")
            .map(|config| {
                parse_config(&read_all_file(&format!("{taskset_dir}/{config}"))?)
                    .map(|config| {
                        let output_file = format!("{}/{}/output-{}",
                            &args.output_dir, &taskset.name, &config.name);

                        TasksetRun {
                            tasks: taskset.clone(),
                            config,
                            results_file: output_file,
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

fn can_run_filter(
    run: &TasksetRun,
    args: &RunnerArgsBase,
) -> bool {
    // filter out tasksets that cannot be run
    can_run_taskset(run, args) &&
    // filter out tasksets that are already run
    !std::path::Path::new(&run.results_file).exists()
}

fn run_taskset_one<FnRun>(
    run: TasksetRun,
    args: &RunnerArgsBase,
    cycles: Option<u64>,
    run_taskset: &FnRun,
) -> Result<Option<TasksetRunResult>, Box<dyn std::error::Error>>
    where
        FnRun:          Fn(TasksetRun, &RunnerArgsBase, Option<u64>)
                            -> Result<TasksetRunResult, Box<dyn std::error::Error>>,
{
    let already_run = std::path::Path::new(&run.results_file).exists();

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
            TasksetRunResult {
                taskset: run.tasks,
                config: run.config,
                results: parse_result(&read_all_file(&run.results_file)?)?,
            }
        } else {
            let results_file = std::path::Path::new(&run.results_file).to_owned();
            let result = run_taskset(run, &args, cycles)?;

            // create results file
            let dirs = results_file.parent()
                .ok_or_else(|| format!("Unknown parent"))?;

            std::fs::create_dir_all(dirs)
                .map_err(|err| format!("Error in creating directory(ies) {dirs:?}: {err}"))?;

            std::fs::write(results_file, serialize_result(&result.results)?)?;

            result
        };

    let insights = compute_result_insights(&result);

    if insights.num_overruns > 0 {
        batch_test_failure(format!("Deadline overrun: {:.2} % error rate, {} worst overrun",
            insights.overruns_ratio * 100f64, insights.worst_overrun));
    } else {
        batch_test_success();
    }

    Ok(Some(result))
}