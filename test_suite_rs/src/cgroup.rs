use crate::prelude::*;

pub mod prelude {
    pub use super::{
        MyCgroup,
    };
}

pub struct MyCgroup {
    name: String,
    force_kill: bool,
}

impl MyCgroup {
    pub fn new(name: &str, runtime_us: u64, period_us: u64, force_kill: bool) -> anyhow::Result<MyCgroup> {
        if name == "." {
            anyhow::bail!("Cannot handle root cgroup");
        }

        if runtime_us > period_us {
            anyhow::bail!("Requested runtime {runtime_us} is greater than the period {period_us}");
        }

        create_cgroup(name)?;

        set_cgroup_runtime_us(name, 0)?;
        set_cgroup_period_us(name, period_us)?;
        set_cgroup_runtime_us(name, runtime_us)?;

        Ok(MyCgroup {
            name: name.to_owned(),
            force_kill
        })
    }

    pub fn update_runtime(&mut self, runtime_us: u64) -> anyhow::Result<()> {
        set_cgroup_runtime_us(&self.name, runtime_us)
    }

    pub fn destroy(mut self) -> anyhow::Result<()> {
        self.__destroy()
    }

    fn __destroy(&mut self) -> anyhow::Result<()> {
        if !cgroup_exists(&self.name) { return Ok(()); }

        if self.force_kill {
            if is_pid_in_cgroup(&self.name, std::process::id())? {
                assign_pid_to_cgroup(".", std::process::id())?;
            }

            cgroup_pids(&self.name)?.iter()
                .try_for_each(|pid| {
                    kill_pid(*pid)?;
                    assign_pid_to_cgroup(".", *pid)
                })?;
        }

        delete_cgroup(&self.name)
    }
}

impl Drop for MyCgroup {
    fn drop(&mut self) {
        let _ = self.__destroy();
    }
}