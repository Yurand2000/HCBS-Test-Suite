use crate::prelude::*;
use crate::utils::*;

/// Get absolute path of a given cgroup
pub fn cgroup_abs_path(name: &str) -> String {
    format!("{CGROUP_ROOT}/{name}")
}

pub fn create_cgroup(name: &str) -> anyhow::Result<()> {
    super::__create_cgroup_common(name)?;
    __enable_controller_v2_recursive(name, "cpu")?;
    info!("Created Cgroup {name}");

    Ok(())
}

pub fn create_cgroup_cpuset(name: &str) -> anyhow::Result<()> {
    super::__create_cgroup_common(name)?;
    __enable_controller_v2_recursive(name, "cpu")?;
    __enable_controller_v2_recursive(name, "cpuset")?;
    info!("Created Cgroup (+ CPUSet) {name}");

    Ok(())
}

pub fn __mount_cgroup_fs() -> anyhow::Result<()> {
    if __shell(&format!("mount | grep cgroup2"))?.stdout.len() > 0 {
        info!("Cgroup v2 FS already mounted");
        return Ok(());
    }

    let res;
    unsafe {
        res = libc::mount(
            c"none".as_ptr(),
            CGROUP_ROOT_C.as_ptr(),
            c"cgroup2".as_ptr(),
            0,
            std::ptr::null(),
        );
    }

    if res != 0 {
        error!("Error in mounting Cgroup v2 FS: {}", std::io::Error::last_os_error());
        anyhow::bail!("Error in mounting Cgroup v2 FS: {}", std::io::Error::last_os_error());
    }

    info!("Mounted Cgroup v2 FS");

    Ok(())
}

pub fn __mount_cpu_fs() -> anyhow::Result<()> {
    __enable_controller_v2(".", "cpu")
}

pub fn __mount_cpuset_fs() -> anyhow::Result<()> {
    __enable_controller_v2(".", "cpuset")
}

pub fn __is_controller_v2_enabled(name: &str, controller: &str) -> anyhow::Result<bool> {
    if !cgroup_exists(name) {
        anyhow::bail!("Cgroup {name} does not exist");
    }

    let controllers_path = format!("{CGROUP_ROOT}/{name}/cgroup.subtree_control");
    let controllers_path = std::path::Path::new(&controllers_path);
    if !controllers_path.exists() || !controllers_path.is_file() {
        anyhow::bail!("Unexpected! Controllers file for cgroup {name} does not exist");
    }

    Ok(
        std::fs::read_to_string(controllers_path)
        .map_err(|err| anyhow::format_err!("Error in reading controllers for cgroup {name}: {err}") )?
        .contains(controller)
    )
}

pub fn __enable_controller_v2(name: &str, controller: &str) -> anyhow::Result<()> {
    if __is_controller_v2_enabled(name, controller)? { return Ok(()); }

    let controllers_path = format!("{CGROUP_ROOT}/{name}/cgroup.subtree_control");
    std::fs::write(controllers_path, &format!("+{controller}"))
        .map_err(|err| anyhow::format_err!("Error in enabling CPU controller for cgroup {name}: {err}") )?;

    info!("Enabled CPU controller for cgroup {name}");

    Ok(())
}

pub fn __enable_controller_v2_recursive(name: &str, controller: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new(name);
    let ancestors: Vec<_> = path.ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .collect();

    ancestors.into_iter().rev()
        .try_for_each(|ancestror| __enable_controller_v2(ancestror.to_str().unwrap(), controller))?;

    Ok(())
}
