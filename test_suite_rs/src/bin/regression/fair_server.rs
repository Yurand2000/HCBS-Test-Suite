use hcbs_test_suite::prelude::*;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// max running time
    #[arg(short = 't', long = "max-time", value_name = "sec: u64")]
    pub max_time: Option<u64>
}

pub fn batch_runner(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<()> {
    if is_batch_test() && args.max_time.is_none() {
        anyhow::bail!("Batch testing requires a maximum running time");
    }

    let fair_server_bw = get_fair_server_avg_bw()?;
    let error = 0.01f64; // 1% error

    let test_header =
        if is_batch_test() {
            "fair_server"
        } else {
            "fair_server (Ctrl+C to stop)"
        };

    batch_test_header(test_header, "regression");

    let result = main(args, ctrlc_flag)
        .and_then(|used_bw| {
            if f64::abs(used_bw - fair_server_bw) < error {
                Ok(format!("SCHED_OTHER processes got {:.2} % of total runtime.", used_bw * 100f64))
            } else {
                Err(anyhow::format_err!("Expected SCHED_OTHER tasks to use {:.2} % of total runtime, but used {:.2} %", used_bw * 100.0, fair_server_bw * 100.0))
            }
        });

    if is_batch_test() {
        batch_test_result(result)
    } else {
        batch_test_result_details(result)
    }
}

pub fn main(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<f64> {
    let cpus = CpuSet::all()?.num_cpus();

    assign_pid_to_cgroup(".", std::process::id())?;
    let fifo_processes = (0..cpus).map(|_| cpu_hog()).collect::<Result<Vec<_>, _>>()?;
    let non_fifo_processes = (0..cpus).map(|_| cpu_hog()).collect::<Result<Vec<_>, _>>()?;

    set_sched_policy(std::process::id(), SchedPolicy::RR(99), SchedFlags::RESET_ON_FORK)?;
    non_fifo_processes.iter()
        .enumerate()
        .try_for_each(|(i, proc)| {
            set_cpuset_to_pid(proc.id(), &CpuSet::single(i as u32)?)
        })?;

    fifo_processes.iter()
        .enumerate()
        .try_for_each::<_, anyhow::Result<_>>(|(i, proc)| {
            set_cpuset_to_pid(proc.id(), &CpuSet::single(i as u32)?)?;
            set_sched_policy(proc.id(), SchedPolicy::RR(50), SchedFlags::empty())?;

            Ok(())
        })?;

    wait_loop(args.max_time, ctrlc_flag)?;

    let fifo_total_usage =
        fifo_processes.iter()
        .map(|proc| get_process_total_runtime_usage(proc.id()))
        .sum::<Result<f64, _>>()?;

    let non_fifo_total_usage =
        non_fifo_processes.iter()
        .map(|proc| get_process_total_runtime_usage(proc.id()))
        .sum::<Result<f64, _>>()?;

    let non_fifo_ratio =
        non_fifo_total_usage / (non_fifo_total_usage + fifo_total_usage);

    Ok(non_fifo_ratio)
}