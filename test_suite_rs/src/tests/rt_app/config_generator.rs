use crate::prelude::*;
use crate::tests::prelude::*;

pub mod prelude {

}

pub fn generate_calibration_config(out_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let calibration_config =
r#"{
    "global" : {
		"duration" : -1,
		"calibration" : "CPU0",
		"default_policy" : "SCHED_FIFO",
		"pi_enabled" : false,
		"lock_pages" : false,
		"logdir" : "/tmp/rt-app",
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
        .map_err(|err| err.into())
}

pub fn generate_taskset_config(taskset: &Taskset, args: &RunnerArgsBase, out_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let duration =
        taskset.data.iter()
            .map(|task| task.period_ms)
            .max().unwrap() * args.num_instances_per_job;

    let logdir = "/tmp";

    let global_config =
format!(r#"
    "global" : {{
		"duration" : {},
		"calibration" : "CPU0",
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
    }}
"#, duration, logdir);

    let tasks_config = "";

    let config =
format!(r#"{{
{},
{}
}}"#, global_config, tasks_config);

    std::fs::write(out_file, config)
        .map_err(|err| err.into())
}