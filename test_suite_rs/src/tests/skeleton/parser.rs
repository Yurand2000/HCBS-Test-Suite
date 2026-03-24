use nom::Parser;
use nom::multi::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;

use crate::prelude::*;
use crate::tests::prelude::*;

pub mod prelude {
    pub use super::{
        parse_taskset,
        parse_config,
        parse_result,
        serialize_taskset,
        serialize_config,
        serialize_result,
    };
}

pub fn read_all_file(file: &str) -> anyhow::Result<String> {
    std::fs::read_to_string(file)
        .map_err(|err| anyhow::format_err!("Error on reading file {file}, reason {err}"))
}

pub fn parse_taskset(data: &str) -> anyhow::Result<NamedTaskset> {
    let u64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());
    let name_parser = || map(take_while1(|ch: char| !ch.is_whitespace()), |name: &str| name.to_owned());
    let line_parser = map(
        (space0, u64_parser(), space1, u64_parser(), space1, u64_parser(), space0),
        |(_, runtime_ms, _, deadline_ms, _, period_ms, _)|
            RTTask {
                wcet: Time::millis(runtime_ms as f64),
                deadline: Time::millis(deadline_ms as f64),
                period: Time::millis(period_ms as f64),
            }
    );

    let mut parser = map(
        (tag("Taskset"), space1, name_parser(), multispace1, separated_list1(newline, line_parser)),
        |(_, _, name, _, tasks)| NamedTaskset { name, tasks }
    );

    parser.parse(&data)
        .map(|(_, taskset)| taskset)
        .map_err(|err| anyhow::format_err!("Taskset parser error: {err}"))
}

pub fn serialize_taskset(taskset: &NamedTaskset) -> anyhow::Result<String> {
    if taskset.name.chars().any(|ch| ch.is_whitespace()) {
        anyhow::bail!("Taskset \'{}\' contains whitespaces in the name, cannot serialize.", taskset.name);
    }

    let mut out_string = format!("Taskset {}\n", taskset.name);
    taskset.tasks.iter()
        .for_each(|task| {
            out_string += &format!("{:.0} {:.0} {:.0}\n", task.wcet.as_millis(), task.deadline.as_millis(), task.period.as_millis());
        });

    Ok(out_string)
}

pub fn parse_config(data: &str) -> anyhow::Result<NamedConfig> {
    let u64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());
    let name_parser = || map(take_while1(|ch: char| !ch.is_whitespace()), |name: &str| name.to_owned());
    let mut parser = map(
        (tag("Config"), space1, name_parser(), space1, u64_parser(), space1, u64_parser(), space1, u64_parser()),
        |(_, _, name, _, cpus, _, runtime_ms, _, period_ms)|
            NamedConfig {
                name,
                cpus,
                runtime: Time::millis(runtime_ms as f64),
                period: Time::millis(period_ms as f64),
            }
    );

    parser.parse(&data)
        .map(|(_, config)| config)
        .map_err(|err| anyhow::format_err!("Taskset config parser error: {err}"))
}

pub fn serialize_config(config: &NamedConfig) -> anyhow::Result<String> {
    if config.name.chars().any(|ch| ch.is_whitespace()) {
        anyhow::bail!("Config \'{}\' contains whitespaces in the name, cannot serialize.", config.name)
    } else {
        Ok(format!("Config {} {} {:.0} {:.0}", config.name, config.cpus, config.runtime.as_millis(), config.period.as_millis()))
    }
}

pub fn parse_result(data: &str) -> anyhow::Result<Vec<TasksetRunResultInstance>> {
    let base_parser = (
        tag("Results"), space0, newline, space0,
        tag("Task"), space1, tag("Job"), space1, tag("AbsActivation_us"),
            space1, tag("RelStart_us"), space1, tag("RelFinish_us"), space1, tag("Slack_us"), space0);

    let u64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());

    let line_parser = || map(
        (space0, u64_parser(), space1, u64_parser(), space1, u64_parser(), space1, u64_parser(), space1, u64_parser(), space1, u64_parser(), space0),
        |(_, task, _, instance, _, abs_activation_time_us, _, rel_start_time_us, _, rel_finishing_time_us, _, _slack_us, _)|
            TasksetRunResultInstance {
                task,
                instance,
                abs_activation_time: Time::micros(abs_activation_time_us as f64),
                rel_start_time: Time::micros(rel_start_time_us as f64),
                rel_finishing_time: Time::micros(rel_finishing_time_us as f64),
            }
    );

    let mut parser = map(
        (base_parser, newline, separated_list1(newline, line_parser())),
        |(_, _, jobs)| jobs
    );

    parser.parse(&data)
        .map(|(_, jobs)| jobs)
        .map_err(|err| anyhow::format_err!("Taskset run result parser error: {err}").into())
}

pub fn serialize_result(taskset: &NamedTaskset, results: &Vec<TasksetRunResultInstance>) -> anyhow::Result<String> {
    let mut out_string = format!("Results\nTask Job AbsActivation_us RelStart_us RelFinish_us Slack_us\n");

    results.iter()
        .for_each(|data| {
            let task = &taskset.tasks[data.task as usize];
            let slack = data.slack_time(task);

            out_string += &format!("{} {} {:.0} {:.0} {:.0} {:.0}\n",
                data.task,
                data.instance,
                data.abs_activation_time.as_micros(),
                data.rel_start_time.as_micros(),
                data.rel_finishing_time.as_micros(),
                slack.as_micros(),
            );
        });

    Ok(out_string)
}
