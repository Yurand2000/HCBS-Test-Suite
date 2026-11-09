use hcbs_test_suite::prelude::*;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// enable/disable HRTICK_DL, if unspecified print the current state.
    #[arg(short = 'e', value_name = "<bool>")]
    set_state: Option<bool>,
}

pub fn main(args: MyArgs) -> anyhow::Result<()> {
    if let Some(enable) = args.set_state {

        let feature_str =
            if enable {
                "HRTICK_DL"
            } else {
                "NO_HRTICK_DL"
            };

        std::fs::write("/sys/kernel/debug/sched/features", feature_str)?;
    } else {
        let features = __read_file("/sys/kernel/debug/sched/features")?;

        if features.contains("NO_HRTICK_DL") {
            println!("HRTICK_DL disabled");
        } else if features.contains("HRTICK_DL") {
            println!("HRTICK_DL enabled");
        } else {
            println!("HRTICK_DL not supported");
        }
    }

    Ok(())
}
