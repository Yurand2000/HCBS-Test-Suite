use hcbs_test_suite::tests::prelude::*;

#[derive(clap::Parser, Debug)]
#[command(about, long_about = None)]
pub struct Args {
    #[arg(long="runner", default_value="periodic-thread")]
    runner: Runner,

    #[arg(long="multi-cpu", default_value="false")]
    multi: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
#[command(about, long_about = None)]
pub enum Command {
    /// Run all taskset tests
    ///
    /// Run all the taskset tests found in the given input folder.
    #[command(name = "all", verbatim_doc_comment)]
    All(RunnerArgsAll),

    /// Run single taskset
    #[command(name = "single", verbatim_doc_comment)]
    Single(RunnerArgsSingle),

    /// Read results from previously run tasksets
    #[command(name = "read-results", verbatim_doc_comment)]
    ReadResults(RunnerArgsAll),
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum Runner {
    #[value(name="periodic-thread")]
    PeriodicThread,
    #[value(name="rt-app")]
    RtApp,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = <Args as clap::Parser>::parse();

    let main_run_taskset_array =
        match (args.runner, args.multi) {
            (Runner::PeriodicThread, false) => periodic_thread::main_run_taskset_array,
            (Runner::PeriodicThread, true) => periodic_thread::main_run_taskset_array_multi,
            (Runner::RtApp, false) => rt_app::main_run_taskset_array,
            (Runner::RtApp, true) => rt_app::main_run_taskset_array_multi,
        };

    let main_run_taskset_single =
        match (args.runner, args.multi) {
            (Runner::PeriodicThread, false) => periodic_thread::main_run_taskset_single,
            (Runner::PeriodicThread, true) => periodic_thread::main_run_taskset_single_multi,
            (Runner::RtApp, false) => rt_app::main_run_taskset_single,
            (Runner::RtApp, true) => rt_app::main_run_taskset_single_multi,
        };

    match args.command {
        Command::All(args) => { main_run_taskset_array(args)?; },
        Command::Single(args) => { main_run_taskset_single(args)?; },
        Command::ReadResults(args) => { read_taskset_results(&args)?; },
    };

    Ok(())
}