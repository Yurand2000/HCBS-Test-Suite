use hcbs_test_suite::prelude::*;
use rand::*;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// cgroup's name
    #[arg(short = 'c', long = "cgroup", default_value = "g0", value_name = "name")]
    pub cgroup: String,

    /// number of tests to run
    #[arg(short = 'n', long = "num-tests", value_name = "u64", default_value = "60")]
    pub num_tests: u64,

    /// max running time per test
    #[arg(short = 't', long = "max-time", value_name = "sec: u64", default_value = "60")]
    pub max_time_per_test: u64,

    /// RNG's seed
    #[arg(long = "seed", value_name = "u64", default_value = "42")]
    pub seed: u64,
}

#[derive(Debug)]
enum TestType {
    CgroupMakeDestroy,
    ChangeCgroupRuntime,
    ChangePinning,
    ChangePriority,
    Migrate,
    SwitchClass,
}

impl rand::distr::Distribution<TestType> for rand::distr::StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestType {
        match rng.random_range(0..=10) {
            0 => TestType::CgroupMakeDestroy,
            1..=2 => TestType::ChangeCgroupRuntime,
            3..=4 => TestType::ChangePinning,
            5..=6 => TestType::ChangePriority,
            7..=8 => TestType::Migrate,
            9..=10 => TestType::SwitchClass,
            _ => panic!("unexpected"),
        }
    }
}

pub fn main(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<()> {
    unsafe { set_batch_test(); }

    let ctrlc_flag = match ctrlc_flag {
        Some(exit) => exit,
        None => create_ctrlc_handler()?,
    };

    let mut rand = rand::rngs::StdRng::seed_from_u64(args.seed);
    for _ in 0..args.num_tests {
        if ctrlc_flag.is_exit() {
            break;
        }

        let test_type: TestType = rand.random();
        let period_ms = rand.random_range(10..=20) * 10;
        let runtime_max_ms = period_ms * 90 / 100;
        let runtime_min_ms = 20;
        let runtime_half_ms = (runtime_max_ms + runtime_min_ms) / 2;

        match test_type {
            TestType::CgroupMakeDestroy => {
                let _runtime_min_ms = rand.random_range(runtime_min_ms..=runtime_half_ms);
                let _runtime_max_ms = rand.random_range(runtime_half_ms..=runtime_max_ms);

                crate::cgroup_make_destroy::batch_runner(crate::cgroup_make_destroy::MyArgs {
                    cgroup: args.cgroup.clone(),
                    runtime_min_ms: _runtime_min_ms,
                    runtime_max_ms: _runtime_max_ms,
                    period_ms,
                    max_time: Some(args.max_time_per_test),
                },  Some(&mut rand),
                    Some(ctrlc_flag.clone()),
                )?
            },
            TestType::ChangePinning => {
                let runtime_ms = rand.random_range(runtime_min_ms..runtime_max_ms);
                let change_period = rand.random_range(0.5f32..=3f32);

                crate::change_pinning::batch_runner(crate::change_pinning::MyArgs {
                    cgroup: args.cgroup.clone(),
                    runtime_ms,
                    period_ms,
                    change_period,
                    cpu_set1: "0,2".parse()?,
                    cpu_set2: "1,3".parse()?,
                    max_time: Some(args.max_time_per_test),
                },  Some(ctrlc_flag.clone()),
                )?
            },
            TestType::ChangePriority => {
                let runtime_ms = rand.random_range(runtime_min_ms..runtime_max_ms);
                let change_period = rand.random_range(0.5f32..=3f32);

                crate::change_priority::batch_runner(crate::change_priority::MyArgs {
                    cgroup: args.cgroup.clone(),
                    runtime_ms,
                    period_ms,
                    change_period,
                    max_time: Some(args.max_time_per_test),
                },  Some(ctrlc_flag.clone()),
                )?
            },
            TestType::ChangeCgroupRuntime => {
                let runtime1_ms = rand.random_range(runtime_min_ms..=runtime_half_ms);
                let runtime2_ms = rand.random_range(runtime_half_ms..=runtime_max_ms);
                let change_period = rand.random_range(0.5f32..=3f32);

                crate::change_cgroup_runtime::batch_runner(crate::change_cgroup_runtime::MyArgs {
                    cgroup: args.cgroup.clone(),
                    runtime1_ms,
                    runtime2_ms,
                    period_ms,
                    change_period,
                    max_time: Some(args.max_time_per_test),
                },  Some(ctrlc_flag.clone()),
                )?

            },
            TestType::Migrate => {
                let runtime_ms = rand.random_range(runtime_min_ms..runtime_max_ms);
                let change_period = rand.random_range(0.5f32..=3f32);

                crate::migrate::batch_runner(crate::migrate::MyArgs {
                    cgroup: args.cgroup.clone(),
                    runtime_ms,
                    period_ms,
                    change_period,
                    max_time: Some(args.max_time_per_test),
                },  Some(ctrlc_flag.clone()),
                )?

            },
            TestType::SwitchClass => {
                let runtime_ms = rand.random_range(runtime_min_ms..runtime_max_ms);
                let change_period = rand.random_range(0.5f32..=3f32);

                crate::switch_class::batch_runner(crate::switch_class::MyArgs {
                    cgroup: args.cgroup.clone(),
                    runtime_ms,
                    period_ms,
                    change_period,
                    max_time: Some(args.max_time_per_test),
                },  Some(ctrlc_flag.clone()),
                )?

            },
        }
    }

    Ok(())
}