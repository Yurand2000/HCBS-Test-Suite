use hcbs_test_suite::prelude::*;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// bandwidth
    #[arg(short = 'b', long = "bandwidth", value_name = "<ms>")]
    bw_ms: u64,
}

pub fn set_fair_server_runtime_us(runtime_us: u64) -> anyhow::Result<()> {
    let runtime_ns = runtime_us * 1000;

    for entry in std::fs::read_dir("/sys/kernel/debug/sched/fair_server")? {
        let entry = entry?.path();
        if entry.is_dir() {
            let entry = entry.into_os_string().into_string().unwrap();
            std::fs::write(format!("{entry}/runtime"), format!("{runtime_ns}"))
                .map_err(|err| anyhow::format_err!("Error in writing runtime {runtime_ns} ns to {entry}/runtime: {err}"))?;
        }
    }

    Ok(())
}

pub fn main(args: MyArgs) -> anyhow::Result<()> {
    mount_debug_fs()?;

    assign_pid_to_cgroup(".", std::process::id())?;
    set_sched_policy(std::process::id(), SchedPolicy::RR(99), SchedFlags::RESET_ON_FORK)?;

    let target_runtime_us = args.bw_ms * 1000;
    let target_fair_server_us = 1000_000 - target_runtime_us;
    let curr_runtime_us = get_system_rt_runtime_us()?;

    if target_runtime_us > curr_runtime_us {
        set_fair_server_runtime_us(target_fair_server_us)?;
        set_system_rt_runtime_us(target_runtime_us)?;
    } else {
        set_system_rt_runtime_us(target_runtime_us)?;
        set_fair_server_runtime_us(target_fair_server_us)?;
    }

    Ok(())
}