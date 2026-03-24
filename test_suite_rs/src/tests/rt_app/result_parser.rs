use crate::prelude::*;
use crate::tests::prelude::*;

pub mod prelude {
    pub use super::{
        parse_taskset_results,
    };
}

pub fn parse_taskset_results(taskset: &NamedTaskset, log_dir: &str) -> anyhow::Result<Vec<TasksetRunResultInstance>> {
    Ok(
        taskset.tasks.iter().enumerate()
        .map(|(i, _)| {
            let log_name = format!("{}/rt-app-task{:02}-{}.log", log_dir, i, i);

            if !std::path::Path::new(&log_name).exists() {
                anyhow::bail!("Log file {log_name} does not exist");
            }

            parse_task_result(&log_name)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect()
    )
}

fn parse_task_result(run_file: &str) -> anyhow::Result<Vec<TasksetRunResultInstance>> {
    use nom::Parser;
    use nom::multi::*;
    use nom::bytes::complete::*;
    use nom::character::complete::*;
    use nom::combinator::*;

    let data = std::fs::read_to_string(run_file)?;

    let base_parser =
        ((space0, tag("#idx"), space1, tag("perf"), space1, tag("run"), space1, tag("period")),
         (space1, tag("start"), space1, tag("end"), space1, tag("rel_st"), space1, tag("slack")),
         (space1, tag("c_duration"), space1, tag("c_period"), space1, tag("wu_lat")));

    let u64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());
    let i64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<i64>());
    let line_parser = || map(
        ((space0, u64_parser(), space1, u64_parser(), space1, u64_parser(), space1, u64_parser()),
         (space1, u64_parser(), space1, u64_parser(), space1, u64_parser(), space1, i64_parser()),
         (space1, u64_parser(), space1, u64_parser(), space1, u64_parser())),

        | ((_, idx, _, _perf, _, run, _, _period),
           (_, start, _, _end, _, _rel_st, _, slack),
           (_, _c_duration, _, c_period, _, _wu_lat)) | {
            TasksetRunResultInstance {
                task: idx,
                instance: 0,
                abs_activation_time: Time::micros(start as f64),
                rel_start_time: Time::micros((c_period as i64 - slack - run as i64) as f64),
                rel_finishing_time: Time::micros((c_period as i64 - slack) as f64),
            }
        }
    );

    let mut parser = map(
        (base_parser, newline, separated_list1(multispace1, line_parser())),
        |(_, _, jobs)| jobs
    );

    parser.parse(&data)
        .map(|(_, mut jobs)| {
            jobs.iter_mut().enumerate()
                .for_each(|(i, task)| task.instance = i as u64);
            jobs
        })
        .map_err(|err| anyhow::format_err!("Taskset run result parser error: {err}"))
}