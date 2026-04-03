mod realtime_bw_change;
mod move_rt_to_root_cgroup;
mod cgroup_setup;
mod hrtick;
mod chrt;

#[derive(clap::Parser, Debug)]
#[command(about, long_about = None)]
pub enum Command {
    /// CPU hog
    #[command(name = "hog", verbatim_doc_comment)]
    Hog,

    /// Mount CGroup filesystem
    #[command(name = "mount-cgroup-fs", verbatim_doc_comment)]
    MountCgroupFS,

    /// Mount CGroup filesystem and CPU controller
    #[command(name = "mount-cgroup-cpu", verbatim_doc_comment)]
    MountCgroupCPU,

    /// Mount DebugFS
    #[command(name = "mount-debug-fs", verbatim_doc_comment)]
    MountDebugFS,

    /// Change the badwidth allocated to real-time tasks (both FIFO/RR and DEADLINE)
    #[command(name = "rt-bw-change", verbatim_doc_comment)]
    RealtimeBwChange(realtime_bw_change::MyArgs),

    /// Move all real-time tasks to the root control group
    #[command(name = "move-to-root", verbatim_doc_comment)]
    MoveRtTasksToRootCgroup,

    /// Change the runtime and period to the given control group
    #[command(name = "cgroup-setup", verbatim_doc_comment)]
    CgroupBwChange(cgroup_setup::MyArgs),

    /// Enable/Disable the HRTICK_DL scheduler feature
    #[command(name = "hrtick", verbatim_doc_comment)]
    HRTick(hrtick::MyArgs),

    /// CHRT process to SCHED_DEADLINE
    #[command(name = "chrt-deadline", verbatim_doc_comment)]
    ChrtDeadline(chrt::MyArgs),

    /// CHRT process to SCHED_DEADLINE
    #[command(name = "check-multicpu", verbatim_doc_comment)]
    MultiCPUEnabled(()),
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = <Command as clap::Parser>::parse();

    use Command::*;

    match args {
        Hog => {
            let mut i = 0;
            loop { unsafe {
                let i_val = core::ptr::read_volatile(&i);
                core::ptr::write_volatile(&mut i, i_val + 1);
            } }
        }
        MountCgroupFS => hcbs_utils::cgroup::mount_cgroup_fs()?,
        MountCgroupCPU => hcbs_utils::cgroup::mount_cgroup_cpu()?,
        MountDebugFS => hcbs_utils::debugfs::mount_debug_fs()?,
        RealtimeBwChange(args) => realtime_bw_change::main(args)?,
        MoveRtTasksToRootCgroup => move_rt_to_root_cgroup::main()?,
        CgroupBwChange(args) => cgroup_setup::main(args)?,
        HRTick(args) => hrtick::main(args)?,
        ChrtDeadline(args) => chrt::main(args)?,
        MultiCPUEnabled(_) => {
            if hcbs_test_suite::prelude::is_multicpu_enabled()? {
                println!("Multi CPU available");
                std::process::exit(0);
            } else {
                println!("Multi CPU NOT available");
                std::process::exit(1);
            }
        }
    };

    Ok(())
}