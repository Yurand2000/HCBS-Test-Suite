use eva_engine::prelude::*;
use eva_engine::analyses::multiprocessor_periodic_resource_model::MPRModel;

mod args;
mod generator;

fn main() {
    let args = <args::Args as clap::Parser>::parse();

    let out_dir = std::path::Path::new(&args.output.out_directory);

    if out_dir.exists() {
        println!("Output folder {} already exists.", args.output.out_directory);
        std::process::exit(1);
    }

    let analysis_opts = args.analysis.into();

    let tasksets = generator::generate_tasksets(&args.taskset.into(), args.generator_seed);
    let tasksets_num = tasksets.len();

    tasksets
        .into_iter()
        .enumerate()
        .map(|(n, taskset)| {
            println!("Generating configs for taskset {}/{}", n + 1, tasksets_num);

            let configs = generator::generate_config(&taskset, &analysis_opts);
            (taskset, configs)
        })
        .for_each(|(taskset, configs)| {
            if configs.is_empty() {
                return;
            }

            let taskset_dir = format!("{}/{}", &args.output.out_directory, &taskset.name);

            std::fs::create_dir_all(&taskset_dir).unwrap();

            std::fs::write(
                &format!("{}/taskset.txt", taskset_dir),
                taskset_to_string(&taskset.tasks)
            ).unwrap();

            for (i, config) in configs.iter().enumerate() {
                std::fs::write(
                    format!("{}/{:03}.txt", taskset_dir, i),
                    config_to_string(&config),
                ).unwrap();
            }
        });
}

fn taskset_to_string(taskset: &[RTTask]) -> String {
    let mut output = String::new();
    for task in taskset {
        output += &format!("{:.0} {:.0} {:.0}\n", task.wcet.as_millis(), task.deadline.as_millis(), task.period.as_millis());
    }

    output
}

fn config_to_string(model: &MPRModel) -> String {
    format!("{} {:.0} {:.0}",
        model.concurrency,
        (model.resource / model.concurrency as f64).as_millis().ceil(),
        model.period.as_millis()
    )
}