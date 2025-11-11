
use rand::Rng;
use eva_engine::prelude::*;
use eva_engine::analyses::multiprocessor_periodic_resource_model::MPRModel;

pub mod uunifast;

#[derive(Debug, Clone)]
pub struct TasksetGeneratorOptions {
    pub tasksets_per_utilization: u64,
    pub num_tasks: (u64, u64),
    pub task_period_ms: (Time, Time, Time),
    pub taskset_utilization: (f64, f64, f64),
}

#[derive(Debug, Clone)]
pub struct NamedTaskset {
    pub name: String,
    pub tasks: Vec<RTTask>,
}

pub fn generate_tasksets(
    options: &TasksetGeneratorOptions,
    rng_seed: u64,
) -> Vec<NamedTaskset> {
    let seed = std::sync::atomic::AtomicU64::new(rng_seed);
    let count = std::sync::atomic::AtomicU64::new(0);
    let (num_tasks_min, num_tasks_max) = options.num_tasks;
    let (util_min, util_max, util_step) = options.taskset_utilization;
    let (period_min, period_max, period_step) = options.task_period_ms;

    float_iter(util_min, util_max, util_step)
    .flat_map(|taskset_util| {
        std::iter::repeat_n(taskset_util, options.tasksets_per_utilization as usize)
        .map(|taskset_util| {
            let taskset_num = count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            let mut rng =
                <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(
                    seed.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                );

            let num_tasks = rng.random_range(num_tasks_min ..= num_tasks_max) as usize;

            let utils =
                loop {
                    if let Some(utils) = uunifast::uunifast_discard(
                        num_tasks,
                        taskset_util,
                        seed.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    ) {
                        break utils;
                    }
                };

            let period_diff = (period_max - period_min) / period_step;
            let mut tasks: Vec<_> =
                utils.into_iter().map(|util| {
                    let util = (util * 100.0).floor() / 100.0;
                    let period = (
                        rng.random_range(0.0 .. period_diff).floor()
                            * period_step + period_min
                        ).floor();

                    RTTask {
                        wcet: (util * period).floor(),
                        deadline: period,
                        period: period,
                    }
                }).collect();

            tasks.sort_by_key(|task| task.period);

            NamedTaskset {
                name: format!("taskset_U{:.1}_N{:02}_{:03}",
                                taskset_util, num_tasks, taskset_num),
                tasks: tasks,
            }
        })
    })
    .collect()
}

#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    pub cgroup_period: (Time, Time, Time),
    pub max_per_core_bandwidth: f64,
    pub max_cores: u64,
    pub precision: Time,
}

pub fn generate_config(
    taskset: &NamedTaskset,
    options: &AnalysisOptions,
) -> Vec<MPRModel> {
    let (period_min, period_max, period_step) = options.cgroup_period;

    time_iter(period_min, period_max, period_step)
        .flat_map(|period| {
            generate_interface_with_max_bw(
                &taskset.tasks,
                period,
                options.precision,
                options.max_cores,
                options.max_per_core_bandwidth,
            ).ok()
        })
        .collect()
}

fn float_iter(min: f64, max: f64, step: f64) -> impl Iterator<Item = f64>
{
    std::iter::repeat(min).enumerate()
        .map(move |(n, v)| v + (n as f64) * step)
        .take_while(move |&v| v <= max)
}

fn time_iter(min: Time, max: Time, step: Time) -> impl Iterator<Item = Time>
{
    std::iter::repeat(min).enumerate()
        .map(move |(n, v)| v + (n as f64) * step)
        .take_while(move |&v| v <= max)
}

// Custom Generator from Eva-Engine

fn generate_interface_with_max_bw(
    taskset: &[RTTask],
    period: Time,
    step_size: Time,
    max_cores: u64,
    max_per_core_bandwidth: f64,
) -> Result<MPRModel, Error> {
    use eva_engine::analyses::multiprocessor_periodic_resource_model::*;

    AnalysisUtils::assert_constrained_deadlines(taskset)?;
    AnalysisUtils::assert_integer_times(taskset)?;

    generic::generate_interface(
        taskset,
        period,
        generic::GenerationStrategy::MonotoneLinearSearch,
        num_processors_lower_bound,
        |taskset|
            u64::min(num_processors_upper_bound(taskset), max_cores),
        |taskset, model|
            generic::minimum_required_resource(
                taskset,
                model,
                step_size,
                |taskset, model|
                    Ok(generic::minimum_resource_for_taskset(taskset, model.period)),
                |taskset, model|
                    generic::minimum_required_resource_inv(
                        taskset,
                        model,
                        |taskset, k, task_k, model, _|
                            bcl_2009::demand_fp(taskset, k, task_k, model.concurrency),
                        |demand, interval, model|
                            resource_from_linear_sbf(demand, interval, model.period, model.concurrency),
                        |_, _, _, _| Ok(Time::zero()),
                        |_, _, _, _, _| true,
                    ),
                |taskset, model| {
                    let per_core_util =
                        model.resource / (model.concurrency as f64 * model.period);

                    if per_core_util > max_per_core_bandwidth {
                        Ok(false)
                    } else {
                        bcl_2009::is_schedulable_fp(taskset, model)
                    }
                }
            )
    )
}