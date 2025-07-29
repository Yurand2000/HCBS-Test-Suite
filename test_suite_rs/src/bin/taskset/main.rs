use hcbs_test_suite::tests::taskset::*;

#[derive(clap::Parser, Debug)]
#[command(about, long_about = None)]
pub enum Command {
    /// Run all taskset tests
    /// 
    /// Run all the taskset tests found in the given input folder.
    #[command(name = "all", verbatim_doc_comment)]
    All(MyArgsAll),

    /// Run single taskset
    #[command(name = "single", verbatim_doc_comment)]
    Single(MyArgsSpecific),

    /// Read results from previously run tasksets
    #[command(name = "read-results", verbatim_doc_comment)]
    ReadResults(MyArgsAll),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = <Command as clap::Parser>::parse();
    
    match args {
        Command::All(args) => { run_taskset_array(args)?; },
        Command::Single(args) => { run_taskset_single(args)?; },
        Command::ReadResults(args) => { read_results_array(args)?; },
    };

    Ok(())
}