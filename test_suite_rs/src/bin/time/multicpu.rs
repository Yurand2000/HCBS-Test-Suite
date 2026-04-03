use std::{collections::{HashMap, HashSet}, str::FromStr};

use hcbs_test_suite::prelude::*;

#[derive(clap::Parser, Debug)]
pub struct MyArgs {
    /// cgroup's name
    #[arg(short = 'c', long = "cgroup", default_value = "g0", value_name = "name")]
    pub cgroup: String,

    /// cgroup's configuration
    #[arg(short = 'C', long = "config", value_name = "config-tuple", num_args = 1.., value_parser= CgroupConfigSet::parse_config)]
    pub config: Vec<CgroupConfigSet>,

    /// number of processes to spawn
    #[arg(short = 'n', long = "num-tasks", default_value= "1", value_name = "#num")]
    pub num_tasks: u64,

    /// max running time
    #[arg(short = 't', long = "max-time", value_name = "sec: u64")]
    pub max_time: Option<u64>,
}

pub fn batch_runner(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<()> {
    if is_batch_test() && args.max_time.is_none() {
        anyhow::bail!("Batch testing requires a maximum running time");
    }

    let mut single_cpu_bws = HashMap::new();
    for CgroupConfigSet { runtime_ms, period_ms, cpu_set } in args.config.iter() {
        for cpu in cpu_set.iter() {
            let bw = *runtime_ms as f64 / *period_ms as f64;
            single_cpu_bws.insert(cpu, bw);
        }
    }

    let total_cgroup_bw = single_cpu_bws.into_iter().map(|(_, bw)| bw).sum();
    let max_expected_bw = f64::min(total_cgroup_bw, args.num_tasks as f64);
    let max_error = 0.01;

    let mut test_header = format!("time multi c{} n{}", args.cgroup, args.num_tasks);
    for CgroupConfigSet { runtime_ms, period_ms, cpu_set } in args.config.iter() {
        test_header += &format!(" C{}/{}/{:?}", runtime_ms, period_ms, cpu_set);
    }
    let test_header =
        if is_batch_test() {
            test_header
        } else {
            test_header + " (Ctrl+C to stop)"
        };

    batch_test_header(&test_header, "time");

    let result = main(args, ctrlc_flag)
        .and_then(|used_bw| {
            match used_bw {
                Skippable::Result(used_bw) =>
                    if f64::abs(used_bw - max_expected_bw) < max_error {
                        Ok(Skippable::Result(format!("Processes used an average of {used_bw:.5} units of CPU bandwidth.")))
                    } else {
                        Err(anyhow::format_err!("Expected cgroup's task to use {:.2} units of runtime, but used {:.2}", max_expected_bw, used_bw))
                    },
                Skippable::Skipped(err) => Ok(Skippable::Skipped(err)),
            }
        });

    if is_batch_test() {
        batch_test_result_skippable(result)
    } else {
        batch_test_result_skippable_details(result)
    }
}

pub fn main(args: MyArgs, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<Skippable<f64>> {
    assert!(args.config.len() >= 1);

    // run the tasks
    let mut cgroup_runtimes_us: HashMap<u64, HashSet<CpuID>> = HashMap::new();
    let mut cgroup_periods_us: HashMap<u64, HashSet<CpuID>> = HashMap::new();

    for CgroupConfigSet { runtime_ms, period_ms, cpu_set } in args.config.into_iter() {
        let runtime_us = runtime_ms * 1000;
        let period_us = period_ms * 1000;
        let cpu_set: HashSet<_> = cpu_set.into_iter().collect();

        for &cpu in cpu_set.iter() {
            if let Err(err) = CpuSet::single(cpu) {
                return Ok(Skippable::Skipped(err.into()));
            }
        }

        if let Some(cpus) = cgroup_runtimes_us.get_mut(&runtime_us) {
            *cpus = cpus.union(&cpu_set).map(|t| *t).collect();
        } else {
            cgroup_runtimes_us.insert(runtime_us, cpu_set.clone());
        }

        if let Some(cpus) = cgroup_periods_us.get_mut(&period_us) {
            *cpus = cpus.union(&cpu_set).map(|t| *t).collect();
        } else {
            cgroup_periods_us.insert(period_us, cpu_set.clone());
        }
    }

    let mut cgroup = HCBSCgroup::new(&args.cgroup)?
        .with_force_kill(true);
    cgroup.set_period_us_multi(cgroup_periods_us)?;
    cgroup.set_runtime_us_multi(cgroup_runtimes_us)?;

    cgroup.assign_process(HCBSProcess::SelfProc).map_err(|(_, err)| err)?
        .set_sched_policy(SchedPolicy::RR(99))?;

    let procs =
        (0..args.num_tasks)
        .map(|_| -> anyhow::Result<Pid> {
            let proc = cgroup.assign_process(run_yes()?).map_err(|(_, err)| err)?;
            proc.set_sched_policy(SchedPolicy::RR(50))?;

            Ok(proc.id())
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    wait_loop(args.max_time, ctrlc_flag)?;

    let total_usage =
        procs.into_iter()
            .try_fold(0f64, |sum, proc| Ok::<f64, anyhow::Error>(sum + get_process_total_cpu_usage(proc)?))?;

    Ok(Skippable::Result(total_usage))
}

#[derive(Debug, Clone)]
pub struct CgroupConfigSet {
    runtime_ms: u64,
    period_ms: u64,
    cpu_set: CpuSetUnchecked,
}

impl CgroupConfigSet {
    pub fn parse_config(set: &str) -> anyhow::Result<Self> {
        use nom::Parser;
        use nom::bytes::complete::*;
        use nom::multi::*;
        use nom::character::complete::*;
        use nom::combinator::*;

        let uint = || {
            digit1::<&str, ()>
                .map_res(|num: &str| num.parse::<u64>())
        };

        let cpu_set = || {
            recognize(many1(one_of("0123456789-,")))
                .map_res(|str: &str| CpuSetUnchecked::from_str(str))
        };

        map(
            ( uint(), tag("/"), uint(), tag("/"), cpu_set() ),
            |(runtime_ms, _, period_ms, _, cpu_set)|
                CgroupConfigSet { runtime_ms, period_ms, cpu_set }
        ).parse(set)
            .map(|(_, set)| set)
            .map_err(|err| err.into())
    }
}