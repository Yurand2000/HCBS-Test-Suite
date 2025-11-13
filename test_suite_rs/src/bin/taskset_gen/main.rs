use hcbs_test_suite::tests::prelude::{
    serialize_taskset,
    serialize_config,
};

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
            print!("Generating configs for taskset {}/{}\r", n + 1, tasksets_num);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            let configs = generator::generate_config("config", &taskset, &analysis_opts);
            (taskset, configs)
        })
        .for_each(|(taskset, configs)| {
            let taskset_dir = format!("{}/{}", &args.output.out_directory, &taskset.name);

            std::fs::create_dir_all(&taskset_dir).unwrap();

            std::fs::write(
                &format!("{}/taskset.txt", taskset_dir),
                serialize_taskset(&taskset).unwrap()
            ).unwrap();

            for (i, config) in configs.iter().enumerate() {
                std::fs::write(
                    format!("{}/config_{:03}.txt", taskset_dir, i),
                    serialize_config(&config).unwrap(),
                ).unwrap();
            }
        });
}