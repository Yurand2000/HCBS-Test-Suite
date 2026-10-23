use hcbs_test_suite::utils::batch_test::*;
use hcbs_test_suite::prelude::*;

pub fn main() -> anyhow::Result<()> {
    // check if cgroup filesystem is mounted
    if !cgroup_exists(".") {
        return Ok(());
    }

    let pids = __shell("pgrep .")?;
    let pids = String::from_utf8(pids.stdout)?;
    let pids = pids.split_ascii_whitespace()
        .map(|pid| Pid(pid.parse::<u32>().unwrap()) );

    for pid in pids {
        use libtux::pid::sched::SchedPolicy::*;

        match pid::get_sched_policy(pid).map(|(policy, _)| policy) {
            Ok(OTHER {..}) | Ok(BATCH {..}) | Ok(IDLE) => { continue; },
            Ok(_) => (),
            Err(err) => {
                println!("Error getting policy for pid {pid}: {err}");
                continue;
            }
        };

        let cgroup = get_pid_cgroup(pid)?;
        if cgroup == "." { continue; };

        assign_pid_to_cgroup(".", pid)?;
        if !is_batch_test() {
            println!("Migrated task {} to root cgroup", pid);
        }
    }

    Ok(())
}
