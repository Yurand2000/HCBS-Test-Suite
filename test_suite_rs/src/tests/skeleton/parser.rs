use nom::Parser;
use nom::multi::*;
use nom::branch::*;
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

pub fn read_all_file(file: &str) -> Result<String, Box<dyn std::error::Error>> {
    std::fs::read_to_string(file).map_err(|err| err.into())
}

pub fn parse_taskset(data: &str) -> Result<Taskset, Box<dyn std::error::Error>> {
    let u64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());
    let name_parser = || map(take_while1(|ch: char| !ch.is_whitespace()), |name: &str| name.to_owned());
    let line_parser = map_res(
        (space0, u64_parser(), space1, u64_parser(), space1, u64_parser(), space0),
        |(_, runtime_ms, _, deadline_ms, _, period_ms, _)| {
            if deadline_ms == period_ms {
                Ok(PeriodicTaskData { runtime_ms, period_ms })
            } else {
                Err(format!("Expected deadline to be equal to period"))
            }
        }
    );

    let mut parser = map(
        (tag("Taskset"), space1, name_parser(), multispace1, separated_list1(newline, line_parser)),
        |(_, _, name, _, data)| Taskset { name, data }
    );

    parser.parse(&data)
        .map(|(_, taskset)| taskset)
        .map_err(|err| format!("Taskset parser error: {err}").into())
}

pub fn serialize_taskset(taskset: &Taskset) -> Result<String, Box<dyn std::error::Error>> {
    if taskset.name.chars().any(|ch| ch.is_whitespace()) {
        return Err(format!("Taskset \'{}\' contains whitespaces in the name, cannot serialize.", taskset.name).into());
    }

    let mut out_string = format!("Taskset {}\n", taskset.name);
    taskset.data.iter()
        .for_each(|task| {
            out_string += &format!("{} {} {}\n", task.runtime_ms, task.period_ms, task.period_ms);
        });

    Ok(out_string)
}

pub fn parse_config(data: &str) -> Result<TasksetConfig, Box<dyn std::error::Error>> {
    let u64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());
    let name_parser = || map(take_while1(|ch: char| !ch.is_whitespace()), |name: &str| name.to_owned());
    let mut parser = map(
        (tag("Config"), space1, name_parser(), space1, u64_parser(), space1, u64_parser(), space1, u64_parser()),
        |(_, _, name, _, num_cpus, _, runtime_ms, _, period_ms)|
            TasksetConfig {
                name,
                num_cpus,
                runtime_ms,
                period_ms,
            }
    );

    parser.parse(&data)
        .map(|(_, config)| config)
        .map_err(|err| format!("Taskset config parser error: {err}").into())
}

pub fn serialize_config(config: &TasksetConfig) -> Result<String, Box<dyn std::error::Error>> {
    if config.name.chars().any(|ch| ch.is_whitespace()) {
        Err(format!("Config \'{}\' contains whitespaces in the name, cannot serialize.", config.name).into())
    } else {
        Ok(format!("Config {} {} {} {}", config.name, config.num_cpus, config.runtime_ms, config.period_ms))
    }
}

pub fn parse_result(data: &str) -> Result<Vec<TasksetRunResultInstance>, Box<dyn std::error::Error>> {
    let base_parser = (
        tag("Results"), multispace1,
        tag("Task"), space1, tag("Job"), space1, tag("AbsActivation_us"),
            space1, tag("RelStart_us"), space1, tag("RelFinish_us"), space1, tag("DlOffset"), space1);

    let u64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());
    let f64_parser = || map_res(
        alt((
            recognize((digit1::<&str, ()>, tag("."), digit1::<&str, ()>)),
            recognize(digit1::<&str, ()>),
            recognize((tag("."), digit1::<&str, ()>)),
        )),
        |num: &str| num.parse::<f64>()
    );

    let line_parser = || map(
        (space0, u64_parser(), space1, u64_parser(), space1, u64_parser(), space1, u64_parser(), space1, u64_parser(), space1, f64_parser(), space0),
        |(_, task, _, instance, _, abs_activation_time_us, _, rel_start_time_us, _, rel_finishing_time_us, _, deadline_offset, _)|
            TasksetRunResultInstance {
                task,
                instance,
                abs_activation_time_us,
                rel_start_time_us,
                rel_finishing_time_us,
                deadline_offset,
            }
    );

    let mut parser = map(
        (base_parser, newline, separated_list1(newline, line_parser())),
        |(_, _, jobs)| jobs
    );

    parser.parse(&data)
        .map(|(_, jobs)| jobs)
        .map_err(|err| format!("Taskset run result parser error: {err}").into())
}

pub fn serialize_result(results: &Vec<TasksetRunResultInstance>) -> Result<String, Box<dyn std::error::Error>> {
    let mut out_string = format!("Results\nTask Job AbsActivation_us RelStart_us RelFinish_us DlOffset\n");

    results.iter()
        .for_each(|data| {
            out_string += &format!("{} {} {} {} {} {}\n",
                data.task,
                data.instance,
                data.abs_activation_time_us,
                data.rel_start_time_us,
                data.rel_finishing_time_us,
                data.deadline_offset
            );
        });

    Ok(out_string)
}
