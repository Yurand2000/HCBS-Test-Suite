mod cgroup_setup;
mod cgroup_hierarchy;

#[derive(clap::Parser, Debug)]
#[command(about, long_about = None)]
pub enum Command {
    #[command(name = "cgroup-setup", verbatim_doc_comment)]
    CgroupSetup(()),

    #[command(name = "cgroup-hierarchy", verbatim_doc_comment)]
    CgroupHierarchy(cgroup_hierarchy::MyArgs),
}

fn main() -> anyhow::Result<()> {
    use Command::*;

    env_logger::init();

    let args = <Command as clap::Parser>::parse();

    hcbs_utils::prelude::mount_cgroup_cpu()?;
    match args {
        CgroupSetup(_) => cgroup_setup::main()?,
        CgroupHierarchy(args) => cgroup_hierarchy::main(args)?,
    };

    Ok(())
}