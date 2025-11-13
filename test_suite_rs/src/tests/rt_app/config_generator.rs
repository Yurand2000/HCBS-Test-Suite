use crate::prelude::*;
use crate::tests::prelude::*;

pub mod prelude {
	pub use super::{
		generate_calibration_config,
		generate_taskset_config,
	};
}

pub fn generate_calibration_config(out_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let calibration_config =
r#"{
    "global" : {
		"duration" : 1,
		"calibration" : "CPU0",
		"default_policy" : "SCHED_FIFO",
		"pi_enabled" : false,
		"lock_pages" : false,
		"logdir" : "/tmp",
		"log_size" : "file",
		"log_basename" : "rt-app",
		"ftrace" : "none",
		"gnuplot" : false,
		"io_device" : "/dev/null",
		"mem_buffer_size" : 4194304,
		"cumulative_slack" : false
	},
    "tasks" : {
        "thread00" : {
			"run": 10000
        }
    }
}"#;

    std::fs::write(&out_file, calibration_config)
        .map_err(|err| format!("Error in writing file {out_file}, reason {err}").into())
}

pub fn generate_taskset_config(
	taskset: &NamedTaskset,
	args: &RunnerArgsBase,
	calibration: Option<u64>,
	log_dir: &str,
	out_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let duration =
        taskset.tasks.iter()
            .map(|task| task.period)
            .max().unwrap().as_millis() / 1000.0 * (args.num_instances_per_job as f64 + 1.0);

	let calibration = calibration
		.map(|c| format!("{}", c))
		.unwrap_or_else(|| format!("CPU0"));

    let global_config =
format!(r#"
    "global" : {{
		"duration" : {:.0},
		"calibration" : {},
		"default_policy" : "SCHED_OTHER",
		"pi_enabled" : false,
		"lock_pages" : false,
		"logdir" : "{}",
		"log_size" : "file",
		"log_basename" : "rt-app",
		"ftrace" : "none",
		"gnuplot" : false,
		"io_device" : "/dev/null",
		"mem_buffer_size" : 4194304,
		"cumulative_slack" : false
    }}"#, duration.ceil(), calibration, log_dir);

	let mut tasks_config = String::with_capacity(0);
	let mut iter = taskset.tasks.iter().enumerate().peekable();
	loop {
		match iter.next() {
			Some((i, task)) => {
				let prio = 99 - i;

				tasks_config +=
&format!(r#"
		"task{:02}": {{
            "policy": "SCHED_FIFO",
            "priority": {},
            "run": {:.0},
			"timer": {{
				"ref": "unique",
				"period": {:.0},
				"mode": "absolute"
			}}
		}}"#, i, prio, task.wcet.as_micros(), task.period.as_micros());
			},
			None => break,
		}

		if iter.peek().is_some() {
			tasks_config += ",\n";
		}
	}

    let tasks_config =
format!(r#"
	"tasks": {{
		{}
	}}"#, tasks_config);

    let config =
format!(r#"{{
{},
{}
}}"#, global_config, tasks_config);

    std::fs::write(out_file, config)
        .map_err(|err| format!("Error in writing file {out_file}, reason {err}").into())
}