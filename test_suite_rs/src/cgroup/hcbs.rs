use std::collections::HashMap;

use crate::cgroup::cgroup_v2::create_cgroup_cpuset;
use crate::prelude::*;
use crate::utils::try_op_timeout;

mod uni;
mod multi;

pub mod prelude {
    pub use super::uni::prelude::*;
    pub use super::multi::prelude::*;
    pub use super::{
        HCBSCgroup,
        HCBSProcess,
    };
}

pub struct HCBSCgroup {
    name: String,
    force_kill: bool,
    processes: HashMap<Pid, HCBSProcess>,
}

impl HCBSCgroup {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        if name == "." {
            anyhow::bail!("Cannot handle root cgroup");
        }

        create_cgroup(name)?;

        Ok(Self {
            name: name.to_owned(),
            force_kill: false,
            processes: HashMap::new(),
        })
    }

    pub fn new_w_cpuset(name: &str) -> anyhow::Result<Self> {
        if name == "." {
            anyhow::bail!("Cannot handle root cgroup");
        }

        create_cgroup_cpuset(name)?;

        Ok(Self {
            name: name.to_owned(),
            force_kill: false,
            processes: HashMap::new(),
        })
    }

    pub fn with_force_kill(mut self, force_kill: bool) -> Self {
        self.force_kill = force_kill;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn assign_process(&mut self, process: HCBSProcess) -> Result<&mut HCBSProcess, (HCBSProcess, anyhow::Error)> {
        let pid = process.id();

        match assign_pid_to_cgroup(&self.name, pid) {
            Ok(_) => {
                self.processes.insert(pid, process);
                Ok(self.processes.get_mut(&pid).unwrap())
            },
            Err(err) => {
                Err((process, err))
            },
        }
    }

    pub fn get_process(&self, pid: Pid) -> Option<&HCBSProcess> {
        self.processes.get(&pid)
    }

    pub fn get_process_mut(&mut self, pid: Pid) -> Option<&mut HCBSProcess> {
        self.processes.get_mut(&pid)
    }

    pub fn take_process(&mut self, pid: Pid) -> anyhow::Result<HCBSProcess> {
        if !self.processes.contains_key(&pid) {
            anyhow::bail!("No such process with PID {}", pid);
        }

        assign_pid_to_cgroup(&self.name, pid)
            .map(|_| self.processes.remove(&pid).unwrap())
    }

    pub fn get_cgroup(&mut self) -> anyhow::Result<multi::CgroupConfig> {
        multi::get_cgroup(&self.name)
    }

    pub fn set_cgroup_us(&mut self, runtime_us: Time, period_us: Time) -> anyhow::Result<()> {
        try_op_timeout(
            || set_cgroup_us(&self.name, runtime_us, period_us),
            std::time::Duration::from_millis(1000)
        )
    }

    pub fn set_cpu_max(&mut self) -> anyhow::Result<()> {
        try_op_timeout(
            || set_cgroup_max(&self.name),
            std::time::Duration::from_millis(1000)
        )
    }

    pub fn set_cgroup_cpumask(&mut self, mask: &pid::CpuSet) -> anyhow::Result<()> {
        try_op_timeout(
            || multi::set_cgroup_cpumask(&self.name, mask),
            std::time::Duration::from_millis(1000)
        )
    }

    pub fn set_cgroup_override(&mut self, mask: &pid::CpuSet, runtime: Time, period: Time) -> anyhow::Result<()> {
        try_op_timeout(
            || multi::set_cgroup_override(&self.name, mask, runtime, period),
            std::time::Duration::from_millis(1000)
        )
    }

    pub fn reset_cgroup_override(&mut self, mask: &pid::CpuSet) -> anyhow::Result<()> {
        try_op_timeout(
            || multi::reset_cgroup_override(&self.name, mask),
            std::time::Duration::from_millis(1000)
        )
    }

    pub fn force_destroy(self) { }

    fn __force_destroy(&mut self) {
        if !cgroup_exists(&self.name) { return; }

        self.processes.clear();

        let self_id = Pid(std::process::id());
        if is_pid_in_cgroup(&self.name, self_id).unwrap_or(false) {
            let _ = assign_pid_to_cgroup(".", self_id);
        }

        if self.force_kill {
            if let Ok(pids) = cgroup_pids(&self.name) {
                pids.iter().for_each(|pid| {
                    let _ = pid::signal(*pid, pid::Signal::SIGKILL);
                    let _ = assign_pid_to_cgroup(".", *pid);
                });
            }
        }

        let _ = delete_cgroup(&self.name);
    }
}

impl Drop for HCBSCgroup {
    fn drop(&mut self) {
        self.__force_destroy();
    }
}

pub enum HCBSProcess {
    Child(std::process::Child),
    SelfProc,
}

impl From<std::process::Child> for HCBSProcess {
    fn from(proc: std::process::Child) -> Self {
        Self::Child(proc)
    }
}

impl HCBSProcess {
    pub fn id(&self) -> Pid {
        match self {
            HCBSProcess::Child(child) => Pid(child.id()),
            HCBSProcess::SelfProc => Pid(std::process::id()),
        }
    }

    pub fn wait(&mut self) -> anyhow::Result<()> {
        match self {
            HCBSProcess::Child(_) => wait_pid(self.id()),
            HCBSProcess::SelfProc => anyhow::bail!("Cannot wait Self Process"),
        }
    }

    pub fn kill(&mut self) -> anyhow::Result<()> {
        match self {
            HCBSProcess::Child(_) => pid::signal(self.id(), pid::Signal::SIGKILL).map_err(|err| err.into()),
            HCBSProcess::SelfProc => anyhow::bail!("Cannot kill Self Process"),
        }
    }

    pub fn set_sched_policy(&mut self, policy: pid::SchedPolicy, flags: pid::SchedFlags) -> anyhow::Result<()> {
        pid::set_sched_policy(self.id(), policy, flags)
            .map_err(|err| err.into())
    }

    pub fn get_sched_policy(&self) -> anyhow::Result<(pid::SchedPolicy, pid::SchedFlags)> {
        pid::get_sched_policy(self.id())
            .map_err(|err| err.into())
    }

    pub fn set_affinity(&mut self, affinity: pid::CpuSet) -> anyhow::Result<()> {
        pid::set_cpuset_to_pid(self.id(), &affinity)
            .map_err(|err| err.into())
    }

    pub fn get_affinity(&self) -> anyhow::Result<pid::CpuSet> {
        pid::get_cpuset_to_pid(self.id())
            .map_err(|err| err.into())
    }
}

impl Drop for HCBSProcess {
    fn drop(&mut self) {
        let _ = self.set_sched_policy(pid::SchedPolicy::other(), pid::SchedFlags::empty());
        let _ = self.kill();
    }
}

pub fn wait_pid(pid: Pid) -> anyhow::Result<()> {
    use libc::*;

    unsafe {
        let res =
            waitpid(
                *pid as pid_t,
                std::ptr::null_mut(),
                0,
            );

        if res < 0 {
            Err(anyhow::format_err!("{}", std::io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }
}
