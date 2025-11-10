pub fn uunifast(num_tasks: usize, utilization: f64, rng_seed: u64) -> Vec<f64>
{
    let mut out_vec = Vec::with_capacity(num_tasks);
    let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(rng_seed);

    let mut sum_u = utilization;
    for i in 0 .. (num_tasks - 1) {
        let next_sum_u = sum_u * f64::powf(
                rand::Rng::random_range(&mut rng, 0.0 ..= 1.0),
                1.0 / (num_tasks - i) as f64
            );
        out_vec.push(sum_u - next_sum_u);
        sum_u = next_sum_u;
    }

    out_vec.push(sum_u);
    out_vec
}

pub fn uunifast_discard(num_tasks: usize, utilization: f64, rng_seed: u64) -> Option<Vec<f64>>
{
    let vec = uunifast(num_tasks, utilization, rng_seed);

    if vec.iter().any(|&util| util > 1.0) {
        None
    } else {
        Some(vec)
    }
}