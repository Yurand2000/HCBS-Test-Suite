#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// enable HRTICK_DL
    #[arg(short = 'e', value_name = "<bool>", default_value="true")]
    enable: bool,
}

pub fn main(args: MyArgs) -> anyhow::Result<()> {
    let feature_str =
        if args.enable {
            "HRTICK_DL"
        } else {
            "NO_HRTICK_DL"
        };

    std::fs::write("/sys/kernel/debug/sched/features", feature_str)?;

    Ok(())
}