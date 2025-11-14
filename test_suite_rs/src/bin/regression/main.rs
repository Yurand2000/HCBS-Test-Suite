mod fair_server;
mod sched_deadline;
mod sched_fifo;

#[derive(clap::Parser, Debug)]
#[command(about, long_about = None)]
pub enum Command {
    /// Stress test the fair deadline servers
    ///
    /// This test creates a set of SCHED_FIFO and SCHED_OTHER cpu-bound tasks,
    /// and asserts that the SCHED_OTHER process get at least the minimum amount
    /// of bandwidth that is reserved to non-real-time tasks.
    #[command(name = "fair-server", verbatim_doc_comment)]
    FairServer(fair_server::MyArgs),

    /// Stress test on cgroups vs SCHED_DEADLINE
    ///
    /// This test creates a number of SCHED_DEADLINE tasks to run on the global
    /// runqueue and another batch of FIFO tasks that is run inside a cgroup.
    /// The test expects that the cgroup's tasks consume at least the amount of
    /// requested bandwidth.
    ///
    /// Constraints: runtime <= 0.45 * period
    #[command(name = "deadline", verbatim_doc_comment)]
    SchedDeadline(sched_deadline::MyArgs),

    /// Stress test on cgroups vs SCHED_FIFO
    ///
    /// This test creates a number of SCHED_FIFO tasks to run on the global
    /// runqueue and another batch of FIFO tasks that is run inside a cgroup.
    /// The test expects that the cgroup's tasks consume at least the amount of
    /// requested bandwidth.
    ///
    /// Constraints: runtime <= period
    #[command(name = "fifo", verbatim_doc_comment)]
    SchedFifo(sched_fifo::MyArgs),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = <Command as clap::Parser>::parse();
    hcbs_test_suite::cgroup::mount_cgroup_fs()?;

    use Command::*;

    match args {
        FairServer(args) => fair_server::batch_runner(args, None).map(|_| ()),
        SchedDeadline(args) => sched_deadline::batch_runner(args, None).map(|_| ()),
        SchedFifo(args) => sched_fifo::batch_runner(args, None).map(|_| ()),
    }
}