use hcbs_test_suite::*;
use hcbs_test_suite::prelude::*;
use libtux::pid::cpuset::CpuSet;

fn cgroup_setup_fail(cgroup_name: &str, runtime_us: u64, period_us: u64) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?;

    let failure: Result<(), _> = cgroup.set_cgroup_us(
        Time::micros(runtime_us as f64),
        Time::micros(period_us as f64)
    );

    if failure.is_ok() {
        anyhow::bail!("Cgroup \'{cgroup_name}\' creation with {runtime_us}/{period_us} did not fail")
    } else {
        Ok(())
    }
}

fn cgroup_setup_fail_multi(cgroup_name: &str, mask: pid::CpuSet, overrides: &[(u32, u64, u64)]) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new_w_cpuset(cgroup_name)?;

    let failure: anyhow::Result<()> =
        cgroup.set_cgroup_cpumask(&mask)
        .and_then(|_| cgroup.set_cgroup_us(Time::zero(), Time::zero()) )
        .and_then(|_| {
            for (cpu, runtime_us, period_us) in overrides.iter() {
                cgroup.set_cgroup_override(
                    &CpuSet::single(CpuID(*cpu)),
                    Time::micros(*runtime_us as f64),
                    Time::micros(*period_us as f64)
                )?;
            }

            Ok(())
        });

    if failure.is_ok() {
        anyhow::bail!("Cgroup \'{cgroup_name}\' creation with cpumask {mask}, overrides {overrides:?} did not fail")
    } else {
        Ok(())
    }
}

fn add_task_to_runtime_zero(cgroup_name: &str) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?;
    cgroup.set_cgroup_us(Time::zero(), Time::micros(100_000f64))?;

    let mut yes = run_yes()?;

    let failure: anyhow::Result<()> =
        yes.set_sched_policy(pid::SchedPolicy::RR(50), pid::SchedFlags::empty()).map_err(|err| err.into())
            .and_then(|_| cgroup.assign_process(yes).map(|_| ()).map_err(|(_, err)| err));

    if failure.is_ok() {
        anyhow::bail!("Cgroup with 0 runtime must not allow to run tasks")
    } else {
        Ok(())
    }
}

fn set_runtime_zero_to_active(cgroup_name: &str) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?;
    cgroup.set_cgroup_us(Time::micros(10_000f64), Time::micros(100_000f64))?;

    let mut yes = run_yes()?;

    yes.set_sched_policy(pid::SchedPolicy::RR(50), pid::SchedFlags::empty()).map_err(|err| err.into())
        .and_then(|_| cgroup.assign_process(yes).map(|_| ()).map_err(|(_, err)| err))?;

    let failure = cgroup.set_cgroup_us(Time::zero(), Time::micros(100_000f64));

    if failure.is_ok() {
        anyhow::bail!("Cannot set runtime zero to cgroup with active tasks")
    } else {
        Ok(())
    }
}

fn set_runtime_zero_to_active_multi(cgroup_name: &str) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?
        .with_force_kill(true);

    cgroup.set_cgroup_cpumask(&CpuSet::single(CpuID(0)))?;
    cgroup.set_cgroup_us(Time::zero(), Time::micros(100_000f64))?;
    cgroup.set_cgroup_override(&CpuSet::single(CpuID(0)), Time::micros(10_000f64), Time::micros(100_000f64))?;

    let mut yes = run_yes()?;
    yes.set_sched_policy(pid::SchedPolicy::RR(50), pid::SchedFlags::empty()).map_err(|err| err.into())
        .and_then(|_| cgroup.assign_process(yes).map(|_| ()).map_err(|(_, err)| err))?;

    let failure: anyhow::Result<()> =
        cgroup.reset_cgroup_override(&CpuSet::single(CpuID(0)));

    if failure.is_ok() {
        anyhow::bail!("Cannot set runtime zero to cgroup with active tasks")
    } else {
        Ok(())
    }
}

fn hotunplug_test(cgroup_name: &str, mask: pid::CpuSet, offline_cpu: CpuID, must_fail: bool) -> anyhow::Result<()> {
    let mut cgroup = HCBSCgroup::new(cgroup_name)?
        .with_force_kill(true);

    cgroup.set_cgroup_cpumask(&mask)?;
    cgroup.set_cgroup_us(Time::micros(10_000f64), Time::micros(100_000f64))?;

    let mut yes = run_yes()?;

    yes.set_sched_policy(pid::SchedPolicy::RR(50), pid::SchedFlags::empty()).map_err(|err| err.into())
        .and_then(|_| cgroup.assign_process(yes).map(|_| ()).map_err(|(_, err)| err))?;

    let hotplug_res = cpu_hotplug(offline_cpu, false);

    match (hotplug_res, must_fail) {
        (Ok(_), true) => anyhow::bail!("Expected hotunplug failure!"),
        (Err(e), false) => anyhow::bail!("Expected hotunplug to succeed: {e}"),
        _ => (),
    }

    cpu_hotplug(offline_cpu, true)?;

    Ok(())
}

pub fn main() -> anyhow::Result<()> {
    assign_pid_to_cgroup(".", Pid::current())?;
    pid::set_sched_policy(Pid::current(), pid::SchedPolicy::RR(99), pid::SchedFlags::RESET_ON_FORK)?;

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
    batch_test_result(cgroup_setup_fail_multi("g0", "0-1".parse()?, &[(0, 1, 100000), (1, 50000, 100000)]))?;

    batch_test_header("runtime_too_small_multi_1", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0", "0-1".parse()?, &[(0, 50000, 100000), (1, 1, 100000)]))?;

    // cannot set runtime greater than period
    batch_test_header("runtime_gt_period_multi_0", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0", "0-1".parse()?, &[(0, 110000, 100000), (1, 50000, 100000)]))?;

    batch_test_header("runtime_gt_period_multi_1", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0", "0-1".parse()?, &[(0, 50000, 100000), (1, 110000, 100000)]))?;

    // period cannot be greater than ~2^53us (i.e. >=2^63ns, which is a negative integer in signed 64-bit)
    batch_test_header("period_too_big_multi_0", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0", "0-1".parse()?, &[(0, (1u64 << 63) / 1000 + 1, 100000), (1, 50000, 100000)]))?;

    batch_test_header("period_too_big_multi_1", test_category);
    batch_test_result(cgroup_setup_fail_multi("g0", "0-1".parse()?, &[(0, 50000, 100000), (1, (1u64 << 63) / 1000 + 1, 100000)]))?;

    // set runtime to zero of running cgroup
    batch_test_header("runtime_0_while_running_multi", test_category);
    batch_test_result(set_runtime_zero_to_active_multi("g0"))?;

    // hotplug tests
    batch_test_header("hotunplug_cpu_3_not_used", test_category);
    batch_test_result(hotunplug_test("g0", "0-2".parse()?, CpuID(3), false))?;

    batch_test_header("hotunplug_fail_cpu_3_used", test_category);
    batch_test_result(hotunplug_test("g0", "0-3".parse()?, CpuID(3), true))?;

    Ok(())
}
