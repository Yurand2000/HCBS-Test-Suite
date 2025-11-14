mod many_tasks;

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
    #[command(name = "many", verbatim_doc_comment)]
    ManyTasks(many_tasks::MyArgs),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = <Command as clap::Parser>::parse();

    use Command::*;

    match args {
        ManyTasks(args) => { many_tasks::batch_runner(args, None)?; },
    };

    Ok(())
}