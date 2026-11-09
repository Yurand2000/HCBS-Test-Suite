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
        compute_calibration_value,
        |run, args, cycles| run_taskset(run, args, cycles, false),
    )
}

#[inline(always)]
pub fn main_run_taskset_single(args: RunnerArgsSingle) -> anyhow::Result<Option<TasksetRunResult>> {
    run_taskset_single(
        &args,
        compute_calibration_value,
        |run, args, cycles| run_taskset(run, args, cycles, false),
    )
}

#[inline(always)]
pub fn main_run_taskset_array_multi(args: RunnerArgsAll) -> anyhow::Result<Vec<TasksetRunResult>> {
    run_taskset_array(
        &args,
        compute_calibration_value,
        |run, args, cycles| run_taskset(run, args, cycles, true),
    )
}

#[inline(always)]
pub fn main_run_taskset_single_multi(args: RunnerArgsSingle) -> anyhow::Result<Option<TasksetRunResult>> {
    run_taskset_single(
        &args,
        compute_calibration_value,
        |run, args, calibration| run_taskset(run, args, calibration, true),
    )
}

fn run_taskset(run: TasksetRun, args: &RunnerArgsBase, calibration: Option<f64>, multi_runtime: bool)
    -> anyhow::Result<TasksetRunResult>
{
    let tmp_output_file = "/tmp/out.txt";
    if std::fs::exists(tmp_output_file)
        .map_err(|err| anyhow::format_err!("Error in checking existance of {tmp_output_file}: {err}"))?
    {
        std::fs::remove_file(tmp_output_file)
            .map_err(|err| anyhow::format_err!("Error in removing {tmp_output_file}: {err}"))?
    }

    let cpu_set = pid::CpuSet::online()?.any_subset(run.config.cpus)?;
    let mut cgroup = HCBSCgroup::new(&args.cgroup)?
        .with_force_kill(true);

    if multi_runtime {
        cgroup.set_cgroup_cpumask(&cpu_set)?;
    }

    cgroup.set_cgroup_us(run.config.runtime, run.config.period)?;

    let self_proc = cgroup.assign_process(HCBSProcess::SelfProc).map_err(|(_, err)| err)?;
    self_proc.set_sched_policy(pid::SchedPolicy::RR(99), pid::SchedFlags::RESET_ON_FORK)?;
    if !multi_runtime  {
        self_proc.set_affinity(cpu_set)?;
    }

    let max_time =
        run.taskset.tasks.iter().map(|t| t.period).max().unwrap()
            *
        args.num_instances_per_job as f64;

    let rt_synth_data = RtSynthData {
        calibration_value: calibration,
        tasks: run.taskset.tasks.clone(),
        max_time: Some(max_time),
        out_file: tmp_output_file.to_owned(),
    };

    let mut proc = run_rt_synth(rt_synth_data)?;
    proc.wait()
        .map_err(|err| anyhow::format_err!("Error in waiting for rt_synth: {err}"))?;

    cgroup.force_destroy();

    let result = TasksetRunResult {
        taskset: run.taskset,
        config: run.config,
        results: parse_taskset_results(tmp_output_file)?,
    };

    Ok(result)
}

fn compute_calibration_value() -> anyhow::Result<f64> {
    use std::process::*;

    let self_pid = Pid::current();

    // run periodic thread to calibrate
    assign_pid_to_cgroup(".", self_pid)?;
    pid::set_sched_policy(self_pid, pid::SchedPolicy::RR(99), pid::SchedFlags::RESET_ON_FORK)?;
    pid::set_cpuset_to_pid(self_pid, &pid::CpuSet::online()?.any_subset(1)?)?;

    let cmd = local_executable_cmd("/bin", "rt-synth-calibrate")?;

    let out_file_name = format!("/tmp/calibration_data.txt");
    let out_file = std::fs::OpenOptions::new().write(true).create(true).open(&out_file_name)
        .map_err(|err| anyhow::format_err!("OutFile creation error {}: {err}", &out_file_name))?;

    let mut proc = Command::new(cmd)
        .stdin(Stdio::null())
        .stdout(out_file)
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| anyhow::format_err!("Error in starting rt-synth: {err}"))?;

    proc.wait()
        .map_err(|err| anyhow::format_err!("Error in waiting for rt-synth: {err}"))?;

    pid::set_cpuset_to_pid(self_pid, &pid::CpuSet::online()?)?;
    pid::set_sched_policy(self_pid, pid::SchedPolicy::other(), pid::SchedFlags::RESET_ON_FORK)?;

    // read calibration results
    let out_data = std::fs::read_to_string(&out_file_name)
        .map_err(|err| anyhow::format_err!("Couldn't read file: {}, reason: {}", out_file_name, err))?;
    Ok(
        out_data.trim_ascii().strip_prefix("Calibration value: ")
        .ok_or_else(|| anyhow::format_err!("Invalid calibration format"))?
        .parse().map_err(|err| anyhow::format_err!("Invalid calibration format: {err}"))?
    )
}

fn parse_taskset_results(out_file: &str) -> anyhow::Result<Vec<TasksetRunResultInstance>> {
    let data = std::fs::read_to_string(out_file)
        .map_err(|err| anyhow::format_err!("Failed to read output file {}: {}", out_file, err))?;

    data.lines()
        .skip(1)
        .map(|line| -> anyhow::Result<_> {
            let mut iter = line.split(',');

            let task = iter.next().ok_or_else(|| anyhow::format_err!("Expected task number!"))?.parse()?;
            let instance = iter.next().ok_or_else(|| anyhow::format_err!("Expected job number!"))?.parse()?;
            let _runtime: u64 = iter.next().ok_or_else(|| anyhow::format_err!("Expected task runtime!"))?.parse()?;
            let _deadline: u64 = iter.next().ok_or_else(|| anyhow::format_err!("Expected task deadline!"))?.parse()?;
            let _period: u64 = iter.next().ok_or_else(|| anyhow::format_err!("Expected task period!"))?.parse()?;
            let job_activation: u64 = iter.next().ok_or_else(|| anyhow::format_err!("Expected job activation!"))?.parse()?;
            let job_start: u64 = iter.next().ok_or_else(|| anyhow::format_err!("Expected job start!"))?.parse()?;
            let _job_deadline: u64 = iter.next().ok_or_else(|| anyhow::format_err!("Expected job deadline!"))?.parse()?;
            let job_end: u64 = iter.next().ok_or_else(|| anyhow::format_err!("Expected job end!"))?.parse()?;

            Ok(TasksetRunResultInstance {
                task,
                instance,
                abs_activation_time: Time::nanos(job_activation as f64),
                rel_start_time: Time::nanos((job_start - job_activation) as f64),
                rel_finishing_time: Time::nanos((job_end - job_activation) as f64),
            })
        })
        .collect()
}

#[derive(Debug)]
#[derive(Clone)]
pub struct RtSynthData {
    pub calibration_value: Option<f64>,
    pub tasks: Vec<PriorityTask<LLTask>>,
    pub max_time: Option<Time>,
    pub out_file: String,
}

pub fn run_rt_synth(mut args: RtSynthData) -> anyhow::Result<HCBSProcess> {
    use std::process::*;

    let cmd = local_executable_cmd("/bin", "rt-synth")?;

    if args.tasks.len() == 0 {
        anyhow::bail!("Attempted executing rt-synth with no tasks");
    }

    let mut cmd_str = String::new();
    if let Some(calibration_value) = args.calibration_value {
        cmd_str += &format!("-c {:.3}", calibration_value);
    }

    if let Some(max_time) = args.max_time {
        cmd_str += &format!(" -t {:.2}", max_time.as_secs());
    }

    args.tasks.sort_by_key(|t| std::cmp::Reverse(t.priority));
    for task in args.tasks.iter() {
        cmd_str += &format!(" [{:.0},{:.0},{:.0}]", task.wcet.as_micros(), task.deadline.as_micros(), task.period.as_micros());
    }

    let cmd_str: Vec<_> = cmd_str.trim_ascii().split_ascii_whitespace().collect();

    let out_file = std::fs::OpenOptions::new().write(true).create(true).open(&args.out_file)
        .map_err(|err| anyhow::format_err!("OutFile creation error {}: {err}", &args.out_file))?;

    let proc = Command::new(cmd)
        .args(cmd_str)
        .stdin(Stdio::null())
        .stdout(out_file)
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| anyhow::format_err!("Error in starting rt-synth: {err}"))?;

    Ok(HCBSProcess::Child(proc))
}
