use nom::Parser;
use nom::multi::*;
use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;

use eva_engine::prelude::*;
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
    std::fs::read_to_string(file)
        .map_err(|err| format!("Error on reading file {file}, reason {err}").into())
}

pub fn parse_taskset(data: &str) -> Result<NamedTaskset, Box<dyn std::error::Error>> {
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
        .map_err(|err| format!("Taskset parser error: {err}").into())
}

pub fn serialize_taskset(taskset: &NamedTaskset) -> Result<String, Box<dyn std::error::Error>> {
    if taskset.name.chars().any(|ch| ch.is_whitespace()) {
        return Err(format!("Taskset \'{}\' contains whitespaces in the name, cannot serialize.", taskset.name).into());
    }

    let mut out_string = format!("Taskset {}\n", taskset.name);
    taskset.tasks.iter()
        .for_each(|task| {
            out_string += &format!("{:.0} {:.0} {:.0}\n", task.wcet.as_millis(), task.deadline.as_millis(), task.period.as_millis());
        });

    Ok(out_string)
}

pub fn parse_config(data: &str) -> Result<NamedConfig, Box<dyn std::error::Error>> {
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
        .map_err(|err| format!("Taskset config parser error: {err}").into())
}

pub fn serialize_config(config: &NamedConfig) -> Result<String, Box<dyn std::error::Error>> {
    if config.name.chars().any(|ch| ch.is_whitespace()) {
        Err(format!("Config \'{}\' contains whitespaces in the name, cannot serialize.", config.name).into())
    } else {
        Ok(format!("Config {} {} {:.0} {:.0}", config.name, config.cpus, config.runtime.as_millis(), config.period.as_millis()))
    }
}

pub fn parse_result(data: &str) -> Result<Vec<TasksetRunResultInstance>, Box<dyn std::error::Error>> {
    let base_parser = (
        tag("Results"), space0, newline, space0,
        tag("Task"), space1, tag("Job"), space1, tag("AbsActivation_us"),
            space1, tag("RelStart_us"), space1, tag("RelFinish_us"), space1, tag("DlOffset"), space0);

    let u64_parser = || map_res(digit1::<&str, ()>, |num: &str| num.parse::<u64>());
    let f64_parser = || map_res(
        recognize((
            opt(tag("-")),
            alt((
                recognize((digit1::<&str, ()>, tag("."), digit1::<&str, ()>)),
                recognize(digit1::<&str, ()>),
                recognize((tag("."), digit1::<&str, ()>)),
            )),
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
