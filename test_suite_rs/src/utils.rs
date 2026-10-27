use std::{io::Write, time::Duration};

pub mod batch_test;

pub mod prelude {
    pub use super::batch_test::*;

    pub use super::{
        ExitFlag,
        create_ctrlc_handler,
        wait_loop,
        wait_loop_periodic_fn,
        __shell,
        __println_debug,
        __read_file,
        __read_file_parse,
        __read_file_parse_if_exists,
        __write_file,
        Skippable,
        mount_debug_fs,
    };
}

#[derive(Clone)]
pub struct ExitFlag {
    ch: crossbeam::channel::Receiver<()>,
}

impl ExitFlag {
    pub fn is_exit(&self) -> bool {
        use crossbeam::channel::TryRecvError::*;

        match self.ch.try_recv() {
            Ok(()) => true,
            Err(Empty) => false,
            _ => panic!("unexpected"),
        }
    }
}

pub fn create_ctrlc_handler() -> anyhow::Result<ExitFlag> {
    let (send, recv) = crossbeam::channel::bounded(1);

    ctrlc::set_handler(move || { send.send(()).unwrap(); })?;
    Ok(ExitFlag { ch: recv })
}

pub fn wait_loop(max_time: Option<u64>, ctrlc_flag: Option<ExitFlag>) -> anyhow::Result<()> {
    let exit = match ctrlc_flag {
        Some(exit) => exit,
        None => create_ctrlc_handler()?,
    };

    let max_time_ch =
        match max_time {
            Some(max_time) => crossbeam::channel::after(Duration::from_secs(max_time)),
            None => crossbeam::channel::never(),
        };

    crossbeam::channel::select! {
        recv(exit.ch) -> _ => (),
        recv(max_time_ch) -> _ => (),
    };

    Ok(())
}

pub fn wait_loop_periodic_fn<F>(period_secs: f32, max_time: Option<u64>, ctrlc_flag: Option<ExitFlag>, mut fun: F) -> anyhow::Result<()>
    where F: FnMut() -> anyhow::Result<()>
{
    let exit = match ctrlc_flag {
        Some(exit) => exit,
        None => create_ctrlc_handler()?,
    };

    let max_time_ch =
        match max_time {
            Some(max_time) => crossbeam::channel::after(Duration::from_secs(max_time)),
            None => crossbeam::channel::never(),
        };

    let periodic_ch = crossbeam::channel::tick(Duration::from_secs_f32(period_secs));

    loop {
        crossbeam::channel::select! {
            recv(exit.ch) -> _ => { break; },
            recv(periodic_ch) -> _ => { fun()?; },
            recv(max_time_ch) -> _ => { break; },
        }
    }

    Ok(())
}

pub fn __println_debug<'a, F: FnOnce() -> String>(fun: F) {
    match std::env::var("DEBUG") {
        Ok(v) if v != "" => {
            let str = fun();
            println!("{str}");
            std::io::stdout().flush().unwrap();
        },
        _ => (),
    };
}

pub fn __shell(cmd: &str) -> anyhow::Result<std::process::Output> {
    use std::process::Command;

    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|err| anyhow::format_err!("Error in executing \"sh -c {cmd}\": {err}"))
}

pub fn __read_file<P>(file: P) -> anyhow::Result<String>
    where P: AsRef<std::path::Path> + std::fmt::Display
{
    std::fs::read_to_string(file.as_ref())
        .map_err(|err| anyhow::format_err!("Error in reading file {file}: {err}"))
}

pub fn __read_file_parse<P, F, U, E>(file: P, mut parse: F) -> anyhow::Result<U>
    where
        P: AsRef<std::path::Path> + std::fmt::Display,
        F: FnMut(String) -> Result<U, E>,
        E: std::fmt::Display
{
    std::fs::read_to_string(file.as_ref())
        .map_err(|err| anyhow::format_err!("Error in reading file {file}: {err}"))
        .and_then(|data| parse(data).map_err(|err| anyhow::format_err!("Error in parsing file {file}: {err}")))
}

pub fn __read_file_parse_if_exists<P, F, U, E>(file: P, mut parse: F) -> anyhow::Result<Option<U>>
    where
        P: AsRef<std::path::Path> + std::fmt::Display,
        F: FnMut(String) -> Result<U, E>,
        E: std::fmt::Display
{
    if !file.as_ref().is_file() {
        return Ok(None);
    }

    std::fs::read_to_string(file.as_ref())
        .map_err(|err| anyhow::format_err!("Error in reading file {file}: {err}"))
        .and_then(|data| parse(data).map(|v| Some(v)).map_err(|err| anyhow::format_err!("Error in parsing file {file}: {err}")))
}

pub fn __write_file<P, C>(file: P, data: C) -> anyhow::Result<()>
    where P: AsRef<std::path::Path> + std::fmt::Display, C: AsRef<[u8]>
{
    std::fs::write(file.as_ref(), data)
        .map_err(|err| anyhow::format_err!("Error in writing file {file}: {err}"))
}

pub fn try_op_timeout<F, T, E>(mut fun: F, timeout: std::time::Duration) -> Result<T, E>
    where F: FnMut() -> Result<T, E>
{
    let start = std::time::Instant::now();

    while (std::time::Instant::now() - start) < timeout {
        match fun() {
            res @ Ok(_) => { return res; },
            Err(_) => (),
        }
    }

    fun()
}

pub enum Skippable<T, E = anyhow::Error> {
    Result(T),
    Skipped(E),
}

impl<T, E> Into<Result<T, E>> for Skippable<T, E> {
    fn into(self) -> Result<T, E> {
        match self {
            Skippable::Result(ok) => Ok(ok),
            Skippable::Skipped(err) => Err(err),
        }
    }
}

/// Try mounting the debug filesystem
pub fn mount_debug_fs() -> anyhow::Result<()> {
    use log::*;
    use crate::utils::__shell;

    // Runs: mount -t debugfs none /sys/kernel/debug
    if __shell(&format!("mount | grep debugfs"))?.stdout.len() > 0 {
        info!("DebugFS already mounted");
        return Ok(());
    }

    if !__shell(&format!("mount -t debugfs none /sys/kernel/debug"))?.status.success() {
        error!("Error in mounting DebugFS");
        anyhow::bail!("Error in mounting DebugFS");
    }

    info!("Mounted DebugFS");

    Ok(())
}
