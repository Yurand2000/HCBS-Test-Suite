#![feature(iterator_try_collect)]

use std::{ops::{Deref, DerefMut}, process::{Command, Stdio}};

pub mod cgroup;
pub mod process;
pub mod utils;
pub mod cpuset;
pub mod tests;

pub mod prelude {
    pub use super::cgroup::prelude::*;
    pub use super::process::prelude::*;
    pub use super::utils::prelude::*;
    pub use super::cpuset::prelude::*;

    pub use super::{
        MyProcess,
        run_yes,
        cpu_hog,
        PeriodicTaskData,
        PeriodicThreadData,
        run_periodic_thread,
    };
}

pub struct MyProcess {
    process: std::process::Child,
}

impl Drop for MyProcess {
    fn drop(&mut self) {
        self.kill().unwrap()
    }
}

impl Deref for MyProcess {
    type Target = std::process::Child;

    fn deref(&self) -> &Self::Target {
        &self.process
    }
}

impl DerefMut for MyProcess {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.process
    }
}

pub fn cpu_hog() -> Result<MyProcess, Box<dyn std::error::Error>> {
    use std::process::*;

    let cmd = local_executable_cmd("/root/test_suite", "tools")?;

    let proc = Command::new(cmd)
        .arg("hog")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(MyProcess { process: proc })
}

pub fn run_yes() -> Result<MyProcess, std::io::Error> {
    use std::process::*;

    let proc = Command::new("yes")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(MyProcess { process: proc })
}

#[derive(Debug)]
#[derive(Clone)]
pub struct PeriodicTaskData {
    pub runtime_ms: u64,
    pub period_ms: u64,
}

#[derive(Debug)]
#[derive(Clone)]
pub struct PeriodicThreadData {
    pub start_priority: u64,
    pub cpu_speed: Option<u64>,
    pub tasks: Vec<PeriodicTaskData>,
    pub num_instances_per_job: u64,
    pub extra_args: String,
    pub out_file: String,
}

fn local_executable_cmd(def_dir: &str, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cmd = std::env::var("TESTBINDIR").unwrap_or_else(|_| def_dir.to_owned()) + "/" + name;

    if !std::fs::exists(&cmd)
        .map_err(|err| format!("Error in checking existance of {cmd}: {err}"))?
    {
        Err(format!("Cannot find {name} executable at {cmd}"))?;
    }

    Ok(cmd)
}

pub fn run_periodic_thread(args: PeriodicThreadData) -> Result<MyProcess, Box<dyn std::error::Error>> {
    let cmd = local_executable_cmd("/bin", "periodic_thread")?;

    if args.tasks.len() == 0 {
        Err(format!("Attempted executing periodic_thread with no tasks"))?;
    }

    // assert tasks are ordered by period (ascending)
    if args.tasks.iter()
        .fold(Some(0), |last_period, task| {
            let last_period = last_period?;
            if task.period_ms >= last_period {
                Some(task.period_ms)
            } else {
                None
            }
        }).is_none()
    {
        return Err(format!("Taskset tasks are not sorted by period.").into());
    }


    let mut num_tasks = 0;
    let mut cmd_str = String::new();
    for (prio, task) in (1..=args.start_priority).rev().zip(args.tasks.iter()) {
        cmd_str += &format!(" -C {0} -p {1} -P {2}", task.runtime_ms * 1000, task.period_ms * 1000, prio);
        num_tasks += 1;
    }

    if args.cpu_speed.is_some() {
        cmd_str += &format!(" -R {0}", args.cpu_speed.unwrap());
    }

    cmd_str += &format!(" {0} -N {1} -n {2}", args.extra_args, args.num_instances_per_job, num_tasks);
    let cmd_str: Vec<_> = cmd_str.trim_ascii().split_ascii_whitespace().collect();

    let out_file = std::fs::OpenOptions::new().write(true).create(true).open(&args.out_file)
        .map_err(|err| format!("OutFile creation error {}: {err}", &args.out_file))?;

    let proc = Command::new(cmd)
        .args(cmd_str)
        .stdin(Stdio::null())
        .stdout(out_file)
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("Error in starting periodic thread: {err}"))?;

    Ok(MyProcess { process: proc })
}