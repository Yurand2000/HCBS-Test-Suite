use hcbs_test_suite::*;
use hcbs_test_suite::prelude::*;

fn cgroup_setup_fail(cgroup_name: &str, runtime_us: u64, period_us: u64) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?;

    let failure: Result<(), _> =
        cgroup.set_period_us(period_us)
            .and_then(|_| cgroup.set_runtime_us(runtime_us));

    if failure.is_ok() {
        anyhow::bail!("Cgroup \'{cgroup_name}\' creation with {runtime_us}/{period_us} did not fail")
    } else {
        Ok(())
    }
}

fn cgroup_setup_fail_multi(cgroup_name: &str, runtimes_us: &str, periods_us: &str) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?;

    let failure: Result<(), _> =
        cgroup.set_period_us_multi_str(periods_us)
            .and_then(|_| cgroup.set_runtime_us_multi_str(runtimes_us));

    if failure.is_ok() {
        anyhow::bail!("Cgroup \'{cgroup_name}\' creation with {runtimes_us:?}/{periods_us:?} did not fail")
    } else {
        Ok(())
    }
}

fn add_task_to_runtime_zero(cgroup_name: &str) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?;
    cgroup.set_period_us(100_000)?;
    cgroup.set_runtime_us(0)?;

    let mut yes = run_yes()?;

    let failure: anyhow::Result<()> =
        yes.set_sched_policy(SchedPolicy::RR(50), SchedFlags::empty()).map_err(|err| err.into())
            .and_then(|_| cgroup.assign_process(yes).map(|_| ()).map_err(|(_, err)| err));

    if failure.is_ok() {
        anyhow::bail!("Cgroup with 0 runtime must not allow to run tasks")
    } else {
        Ok(())
    }
}

fn set_runtime_zero_to_active(cgroup_name: &str) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?;
    cgroup.set_period_us(100_000)?;
    cgroup.set_runtime_us(10_000)?;

    let mut yes = run_yes()?;

    yes.set_sched_policy(SchedPolicy::RR(50), SchedFlags::empty()).map_err(|err| err.into())
        .and_then(|_| cgroup.assign_process(yes).map(|_| ()).map_err(|(_, err)| err))?;

    let failure = cgroup.set_runtime_us(0);

    if failure.is_ok() {
        anyhow::bail!("Cannot set runtime zero to cgroup with active tasks")
    } else {
        Ok(())
    }
}

fn set_runtime_zero_to_active_multi(cgroup_name: &str) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?;
    cgroup.set_period_us(100_000)?;
    cgroup.set_runtime_us_multi_str("10000 0")?;

    let mut yes = run_yes()?;
    yes.set_sched_policy(SchedPolicy::RR(50), SchedFlags::empty()).map_err(|err| err.into())
        .and_then(|_| cgroup.assign_process(yes).map(|_| ()).map_err(|(_, err)| err))?;

    let failure = cgroup.set_runtime_us_multi_str("0 0");

    if failure.is_ok() {
        anyhow::bail!("Cannot set runtime zero to cgroup with active tasks")
    } else {
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    mount_cgroup_cpu()?;

    assign_pid_to_cgroup(".", std::process::id())?;
    set_sched_policy(std::process::id(), SchedPolicy::RR(99), SchedFlags::RESET_ON_FORK)?;

    // batch test utils
    let test_category = "constraints";

    // given DL_SCALE = 10, runtime must be at least 1024ns, i.e. > 1us
    batch_test_header("runtime_too_small", test_category);
    batch_test_result(cgroup_setup_fail("g0", 1, 100_000))?;

    // cannot set runtime greater than period
    batch_test_header("runtime_gt_period", test_category);
    batch_test_result(cgroup_setup_fail("g0", 110_000, 100_000))?;

    // period cannot be greater than ~2^53us (i.e. >=2^63ns, which is a negative integer in signed 64-bit)
    batch_test_header("period_too_big", test_category);
    batch_test_result(cgroup_setup_fail("g0", 110_000, (1u64 << 63) / 1000 + 1))?;

    // adding task to cgroup with runtime zero
    batch_test_header("runtime_0_add_task", test_category);
    batch_test_result(add_task_to_runtime_zero("g0"))?;

    // set runtime to zero of running cgroup
    batch_test_header("runtime_0_while_running", test_category);
    batch_test_result(set_runtime_zero_to_active("g0"))?;

    // multicpu tests
    if !is_multicpu_enabled()? {
        return Ok(());
    }

    // given DL_SCALE = 10, runtime must be at least 1024ns, i.e. > 1us
    batch_test_header("runtime_too_small_multi_0", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0", "1 0 50000 1", "100000 0-1"))?;

    batch_test_header("runtime_too_small_multi_1", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0", "50000 0 1 1", "100000 0-1"))?;

    // cannot set runtime greater than period
    batch_test_header("runtime_gt_period_multi_0", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0", "110000 0 50000 1", "100000 0-1"))?;

    batch_test_header("runtime_gt_period_multi_1", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0", "110000 1 50000 0", "100000 0-1"))?;

    // period cannot be greater than ~2^53us (i.e. >=2^63ns, which is a negative integer in signed 64-bit)
    batch_test_header("period_too_big_multi_0", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0",
        "50000 0-1",
        &format!("{} 0 100000 1", (1u64 << 63) / 1000 + 1)
    ))?;

    batch_test_header("period_too_big_multi_1", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0",
        "50000 0-1",
        &format!("{} 1 100000 0", (1u64 << 63) / 1000 + 1)
    ))?;

    // set runtime to zero of running cgroup
    batch_test_header("runtime_0_while_running_multi", test_category);
    batch_test_result(set_runtime_zero_to_active_multi("g0"))?;

    Ok(())
}