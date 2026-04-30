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
    let tmp_output_file = "/tmp/out.txt";
    if std::fs::exists(tmp_output_file)
        .map_err(|err| anyhow::format_err!("Error in checking existance of {tmp_output_file}: {err}"))?
    {
        std::fs::remove_file(tmp_output_file)
            .map_err(|err| anyhow::format_err!("Error in removing {tmp_output_file}: {err}"))?
    }

    let cpu_set = CpuSet::any_subset(run.config.cpus)?;
    let mut cgroup = HCBSCgroup::new(&args.cgroup)?
        .with_force_kill(true);
    cgroup.set_period_us(run.config.period.as_micros() as u64)?;
    if !multi_runtime  {
        cgroup.set_runtime_us(run.config.runtime.as_micros() as u64)?;
    } else {
        cgroup.set_runtime_us_multi([(
            run.config.runtime.as_micros() as u64,
            cpu_set.iter().map(|cpu| *cpu).collect::<Vec<_>>()
        )])?;
    }

    let self_proc = cgroup.assign_process(HCBSProcess::SelfProc).map_err(|(_, err)| err)?;
    self_proc.set_sched_policy(SchedPolicy::RR(99), SchedFlags::RESET_ON_FORK)?;
    if !multi_runtime  {
        self_proc.set_affinity(cpu_set)?;
    }

    let pthread_data = PeriodicThreadData {
        start_priority: 98,
        cpu_speed: cycles,
        tasks: run.taskset.tasks.clone(),
        extra_args: String::new(),
        out_file: tmp_output_file.to_owned(),
        num_instances_per_job: args.num_instances_per_job,
    };

    let mut proc = run_periodic_thread(pthread_data)?;
    proc.wait()
        .map_err(|err| anyhow::format_err!("Error in waiting for periodic_thread: {err}"))?;

    cgroup.force_destroy();

    let result = TasksetRunResult {
        taskset: run.taskset,
        config: run.config,
        results: parse_taskset_results(tmp_output_file)?,
    };

    Ok(result)
}

fn compute_cpu_speed() -> anyhow::Result<u64> {
    let out_file = format!("/tmp/calibration_data.txt");

    // run periodic thread to calibrate
    assign_pid_to_cgroup(".", std::process::id())?;
    set_sched_policy(std::process::id(), SchedPolicy::RR(99), SchedFlags::RESET_ON_FORK)?;
    set_cpuset_to_pid(std::process::id(), &CpuSet::any_subset(1)?)?;

    let mut proc = run_periodic_thread(PeriodicThreadData {
        start_priority: 99,
        cpu_speed: None,
        tasks: vec![ RTTask {
            wcet: Time::millis(10.0),
            deadline: Time::millis(100.0),
             period: Time::millis(100.0),
        } ],
        num_instances_per_job: 1,
        extra_args: String::with_capacity(0),
        out_file: out_file.clone(),
    })?;
    proc.wait()
        .map_err(|err| anyhow::format_err!("Error in waiting for periodic_thread: {err}"))?;

    set_cpuset_to_pid(std::process::id(), &CpuSet::all()?)?;
    set_sched_policy(std::process::id(), SchedPolicy::other(), SchedFlags::RESET_ON_FORK)?;

    // read calibration results
    let out_data = std::fs::read_to_string(&out_file)
        .map_err(|err| anyhow::format_err!("Couldn't read file: {}, reason: {}", out_file, err))?;
    out_data.lines().find(|line| line.starts_with("#Cycles:"))
        .ok_or(anyhow::format_err!("Calibration error: Cycles measuring not found"))
        .and_then(|line| {
            line.trim_ascii().split_ascii_whitespace().skip(1).next()
                .ok_or(anyhow::format_err!("Calibration error: Cycles measuring not found [2]"))
            .and_then(|cycles| cycles.parse::<u64>()
                .map_err(|err| anyhow::format_err!("Calibration error: {err}"))
            )
        })
}

fn parse_taskset_results(out_file: &str) -> anyhow::Result<Vec<TasksetRunResultInstance>> {
    use nom::Parser;
    use nom::multi::*;
    use nom::character::complete::*;
    use nom::combinator::*;
    use nom::sequence::*;

    let data = std::fs::read_to_string(out_file)
        .map_err(|err| anyhow::format_err!("Failed to read output file {}: {}", out_file, err))?;

    let u64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());
    let f64_parser = map_res(recognize((
            opt(char('-')),
            digit1,
            char('.'),
            digit1
        )), |num: &str| num.parse::<f64>());
    let mut line_parser =
        map_res(
            (count(terminated(u64_parser(), space1), 5), f64_parser),
            |(fields, _dl_offset)| {
                let task_num = fields[0];
                let instance_num = fields[1];
                let abs_finish_us = fields[2] as f64;
                let rel_finish_us = fields[3] as f64;
                let runtime_us = fields[4] as f64;

                Ok::<_, ()>(TasksetRunResultInstance {
                    task: task_num,
                    instance: instance_num,
                    abs_activation_time: Time::micros(abs_finish_us - rel_finish_us),
                    rel_start_time: Time::micros(rel_finish_us - runtime_us),
                    rel_finishing_time: Time::micros(rel_finish_us),
                })
            }
        );

    data.trim_ascii().lines()
        .filter_map(|line| {
            let line = line.trim_ascii();
            if line.starts_with("#") {
                None
            } else {
                Some(
                    line_parser.parse(&line)
                        .map(|(_, res)| res)
                )
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| anyhow::format_err!("Taskset result parser error: {err}"))
}

#[derive(Debug)]
#[derive(Clone)]
pub struct PeriodicThreadData {
    pub start_priority: u64,
    pub cpu_speed: Option<u64>,
    pub tasks: Vec<RTTask>,
    pub num_instances_per_job: u64,
    pub extra_args: String,
    pub out_file: String,
}

pub fn run_periodic_thread(args: PeriodicThreadData) -> anyhow::Result<HCBSProcess> {
    use std::process::*;

    let cmd = local_executable_cmd("/bin", "periodic_thread")?;

    if args.tasks.len() == 0 {
        anyhow::bail!("Attempted executing periodic_thread with no tasks");
    }

    // assert tasks are ordered by period (ascending)
    if !eva_rt_common::utils::RTUtils::is_taskset_sorted_by_period(&args.tasks) {
        anyhow::bail!("Taskset for periodic_thread must be sorted by period.");
    }

    let mut num_tasks = 0;
    let mut cmd_str = String::new();
    for (prio, task) in (1..=args.start_priority).rev().zip(args.tasks.iter()) {
        cmd_str += &format!(" -C {0:.0} -p {1:.0} -P {2}", task.wcet.as_micros(), task.period.as_micros(), prio);
        num_tasks += 1;
    }

    if let Some(cpu_speed) = args.cpu_speed {
        cmd_str += &format!(" -R {0}", cpu_speed);
    }

    cmd_str += &format!(" {0} -N {1} -n {2}", args.extra_args, args.num_instances_per_job, num_tasks);
    let cmd_str: Vec<_> = cmd_str.trim_ascii().split_ascii_whitespace().collect();

    let out_file = std::fs::OpenOptions::new().write(true).create(true).open(&args.out_file)
        .map_err(|err| anyhow::format_err!("OutFile creation error {}: {err}", &args.out_file))?;

    let proc = Command::new(cmd)
        .args(cmd_str)
        .stdin(Stdio::null())
        .stdout(out_file)
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| anyhow::format_err!("Error in starting periodic thread: {err}"))?;

    Ok(HCBSProcess::Child(proc))
}