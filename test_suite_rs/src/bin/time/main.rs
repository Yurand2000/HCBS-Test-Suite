mod unicpu;
mod multicpu;

#[derive(clap::Parser, Debug)]
#[command(about, long_about = None)]
pub enum Command {
    /// Run multiple yes tasks in a RT cgroup
    ///
    /// This command executes a user specified number of yes task in a RT cgroup
    /// with user specified parameters, and reports the cumulative total used
    /// bandwidth of the processes at the end of execution. The test is
    /// successful if the tasks consume no more bandwdith than the one allocated
    /// to the cgroup.
    ///
    /// Constraints: runtime <= period
    #[command(name = "uni", verbatim_doc_comment)]
    UniCpu(unicpu::MyArgs),

    /// Run multiple yes tasks in a multi RT cgroup
    ///
    /// Similar to the "uni" command, but allows to specify runtime and period
    /// for each server individually.
    ///
    /// Constraints: runtime <= period
    #[command(name = "multi", verbatim_doc_comment)]
    MultiCpu(multicpu::MyArgs),
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = <Command as clap::Parser>::parse();

    use Command::*;

    match args {
        UniCpu(args) => { unicpu::batch_runner(args, None)?; },
        MultiCpu(args) => { multicpu::batch_runner(args, None)?; },
    };

    Ok(())
}