mod cgroup_make_destroy;
mod change_cgroup_runtime;
mod change_pinning;
mod change_priority;
mod migrate;
mod run_all;
mod switch_class;

#[derive(clap::Parser, Debug)]
#[command(about, long_about = None)]
pub enum Command {
    /// Run all tests
    ///
    /// This command runs all the listed tests. It generates pseudo-random
    /// parameters for each of these tests and chooses which to run at random,
    /// totalling to a user specified amount of tests.
    #[command(name = "all", verbatim_doc_comment)]
    All(run_all::MyArgs),

    /// Stress test on cgroup creation and destruction
    ///
    /// This test creates and destroys a single cgroup at high rate, with the
    /// specified period and a random runtime chosen between min/max values. For
    /// each cgroup it creates a random number of yes processes and runs for a
    /// random time (<= 2 secs)
    ///
    /// Constraints: runtime-max <= period; runtime-min <= runtime-max
    #[command(name = "cgroup-setup", verbatim_doc_comment)]
    CgroupMakeDestroy(cgroup_make_destroy::MyArgs),

    /// Stress test on cgroup runtime change
    ///
    /// This test continuously changes an active cgroup's runtime between two
    /// given values.
    ///
    /// Constraints: runtime-1 <= period; runtime-2 <= period
    #[command(name = "cgroup-runtime", verbatim_doc_comment)]
    ChangeCgroupRuntime(change_cgroup_runtime::MyArgs),

    /// Stress test on pinning change
    ///
    /// This test continuously changes a task's pinning between two given cpu
    /// sets.
    ///
    /// Constraints: runtime <= period
    #[command(name = "task-pinning", verbatim_doc_comment)]
    ChangePinning(change_pinning::MyArgs),

    /// Stress test on priority change
    ///
    /// This test continuosly changes a task's priority while another task runs
    /// at fixed priority. The changing task should be preempted when set to the
    /// lower priority, and should be running when set to the higher priority
    /// while the other task is preempted.
    ///
    /// Constraints: runtime <= period
    #[command(name = "task-priority", verbatim_doc_comment)]
    ChangePriority(change_priority::MyArgs),

    /// Stress test on task migration
    ///
    /// This test continuously migrates a task between a RT cgroup and the root
    /// control group.
    ///
    /// Constraints: runtime <= period
    #[command(name = "task-migration", verbatim_doc_comment)]
    Migrate(migrate::MyArgs),

    /// Stress test on scheduling class switch
    ///
    /// This test continuously changes the scheduling class of a task, migrated
    /// into a cgroup, to SCHED_RR and SCHED_OTHER.
    ///
    /// Constraints: runtime <= period
    #[command(name = "task-sched-class", verbatim_doc_comment)]
    SwitchClass(switch_class::MyArgs),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = <Command as clap::Parser>::parse();

    use Command::*;

    match args {
        All(args) => run_all::main(args, None),
        CgroupMakeDestroy(args) => cgroup_make_destroy::batch_runner(args, None, None),
        ChangeCgroupRuntime(args) => change_cgroup_runtime::batch_runner(args, None),
        ChangePinning(args) => change_pinning::batch_runner(args, None),
        ChangePriority(args) => change_priority::batch_runner(args, None),
        Migrate(args) => migrate::batch_runner(args, None),
        SwitchClass(args) => switch_class::batch_runner(args, None),
    }
}

