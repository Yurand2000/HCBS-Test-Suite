use hcbs_test_suite::utils::is_batch_test;

pub fn main() -> anyhow::Result<()> {
    use hcbs_utils::prelude::*;

    // check if cgroup filesystem is mounted
    if !cgroup_exists(".") {
        return Ok(());
    }

    let system = sysinfo::System::new_all();

    for (pid, _) in system.processes() {
        use hcbs_test_suite::prelude::SchedPolicy::*;

        match get_sched_policy(pid.as_u32()).map(|(policy, _)| policy) {
            Ok(OTHER {..}) | Ok(BATCH {..}) | Ok(IDLE) => { continue; },
            Ok(_) => (),
            Err(err) => {
                println!("Error getting policy for pid {pid}: {err}");
                continue;
            }
        };

        let cgroup = get_pid_cgroup(pid.as_u32())?;
        if cgroup == "." { continue; };

        assign_pid_to_cgroup(".", pid.as_u32())?;
        if !is_batch_test() {
            println!("Migrated task {} to root cgroup", pid.as_u32());
        }
    }

    Ok(())
}