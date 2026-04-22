use hcbs_test_suite::prelude::*;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// runtime
    #[arg(short = 'r', long = "runtime", value_name = "<us>")]
    runtime_us: u64,

    /// period
    #[arg(short = 'p', long = "period", value_name = "<us>")]
    period_us: u64,

    /// affect ext servers (fair servers otherwise)
    #[arg(long = "ext")]
    ext_servers: bool,
}

fn write_to_file(file: &str, data: &str) -> anyhow::Result<()> {
    std::fs::write(file, data)
        .map_err(|err| anyhow::format_err!("Error in writing {data} ns to {file}: {err}"))
}

fn setup_servers(runtime_us: u64, period_us: u64, ext_servers: bool) -> anyhow::Result<()> {
    let runtime_ns = runtime_us * 1000;
    let period_ns = period_us * 1000;

    let server_path =
        if ext_servers {
            "/sys/kernel/debug/sched/ext_server"
        } else {
            "/sys/kernel/debug/sched/fair_server"
        };

    for entry in std::fs::read_dir(server_path)? {
        let entry = entry?.path();
        if entry.is_dir() {
            let entry = entry.into_os_string().into_string().unwrap();

            write_to_file(&format!("{entry}/runtime"), "0")?;
            write_to_file(&format!("{entry}/period"), &format!("{period_ns}"))?;
            write_to_file(&format!("{entry}/runtime"), &format!("{runtime_ns}"))?;
        }
    }

    Ok(())
}

pub fn main(args: MyArgs) -> anyhow::Result<()> {
    mount_debug_fs()?;

    assign_pid_to_cgroup(".", std::process::id())?;
    set_sched_policy(std::process::id(), SchedPolicy::RR(99), SchedFlags::RESET_ON_FORK)?;

    setup_servers(
        args.runtime_us,
        args.period_us,
        args.ext_servers
    )?;

    Ok(())
}