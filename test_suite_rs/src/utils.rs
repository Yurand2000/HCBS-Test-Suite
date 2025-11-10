use std::{io::Write, time::Duration};

pub mod prelude {
    pub use super::{
        __shell,
        __println_debug,
        set_batch_test,
        is_batch_test,
        wait_loop,
        wait_loop_periodic_fn,
        create_ctrlc_handler,
        ExitFlag,
        mount_debug_fs,
        batch_test_header,
        batch_test_result,
        batch_test_result_details,
        batch_test_result_skippable,
        batch_test_result_skippable_details,
        batch_test_success,
        batch_test_success_details,
        batch_test_skipped,
        batch_test_failure,
        get_fair_server_avg_bw,
        Skippable,
    };
}

pub fn __println_debug<'a, F: FnOnce() -> String>(fun: F) {
    match std::env::var("DEBUG") {
        Ok(v) if v != "" => {
            let str = fun();
            println!("{str}");
        },
        _ => (),
    };
}

pub unsafe fn set_batch_test() {
    unsafe { std::env::set_var("BATCH_TEST", "1") };
}

pub fn is_env_var_set(var: &str) -> bool {
    match std::env::var(var) {
        Ok(v) if v != "" => true,
        _ => false,
    }
}

pub fn is_batch_test() -> bool {
    is_env_var_set("BATCH_TEST")
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

pub fn create_ctrlc_handler() -> Result<ExitFlag, Box<dyn std::error::Error>> {
    let (send, recv) = crossbeam::channel::bounded(1);

    ctrlc::set_handler(move || { send.send(()).unwrap(); })?;
    Ok(ExitFlag { ch: recv })
}

pub fn wait_loop(max_time: Option<u64>, ctrlc_flag: Option<ExitFlag>) -> Result<(), Box<dyn std::error::Error>> {
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

pub fn wait_loop_periodic_fn<F>(period_secs: f32, max_time: Option<u64>, ctrlc_flag: Option<ExitFlag>, mut fun: F) -> Result<(), Box<dyn std::error::Error>>
    where F: FnMut() -> Result<(), Box<dyn std::error::Error>>
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

pub fn __shell(cmd: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    use std::process::Command;

    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|err| format!("Error in executing \"sh -c {cmd}\": {err}").into())
}

// mount -t debugfs none /sys/kernel/debug
pub fn mount_debug_fs() -> Result<(), Box<dyn std::error::Error>> {
    if __shell(&format!("mount | grep debugfs"))?.stdout.len() > 0 {
        __println_debug(|| format!("DebugFS already mounted"));
        return Ok(());
    }

    if !__shell(&format!("mount -t debugfs none /sys/kernel/debug"))?.status.success() {
        __println_debug(|| format!("Error in mounting DebugFS"));
        return Err(format!("Error in mounting DebugFS"))?;
    }

    __println_debug(|| format!("Mounted DebugFS"));

    Ok(())
}

pub fn batch_test_success() {
    if is_env_var_set("TERM_COLORS") {
        println!("\x1b[32mSuccess ✔\x1b[0m");
    } else {
        println!("Success ✔");
    }
}

pub fn batch_test_success_details<T: std::fmt::Display>(msg: T) {
    batch_test_success();

    if is_env_var_set("TERM_COLORS") {
        println!("\x1b[32m    Details: \x1b[0m{msg}");
    } else {
        println!("    Details: {msg}");
    }
}

pub fn batch_test_failure<E: std::fmt::Display>(err: E) {
    if is_env_var_set("TERM_COLORS") {
        println!("\x1b[31mFailure ✖\n    Reason: \x1b[0m{err}")
    } else {
        println!("Failure ✖\n    Reason: {err}")
    }
}

pub fn batch_test_skipped<T: std::fmt::Display>(msg: T) {
    if is_env_var_set("TERM_COLORS") {
        println!("\x1b[33mSkipped ⛒\n    Reason: \x1b[0m{msg}");
    } else {
        println!("Skipped ⛒\n    Reason: {msg}");
    }
}

pub fn batch_test_header(test_name: &str, test_category: &str) {
    match std::env::var("BATCH_TEST_CUSTOM_NAME") {
        Ok(custom) if custom != "" => print!("[{}] {}: ", test_category, custom),
        _ => print!("[{}] {}: ", test_category, test_name),
    };

    std::io::stdout().flush().unwrap();
}

pub fn batch_test_result<T>(result: Result<T, Box<dyn std::error::Error>>) -> Result<(), Box<dyn std::error::Error>> {
    match &result {
        Ok(_) => batch_test_success(),
        Err(err) => batch_test_failure(err),
    };

    if is_batch_test() {
        Ok(())
    } else {
        result.map(|_| ())
    }
}

pub fn batch_test_result_skippable<T>(result: Result<Skippable<T, Box<dyn std::error::Error>>, Box<dyn std::error::Error>>) -> Result<(), Box<dyn std::error::Error>> {
    match &result {
        Ok(Skippable::Result(_)) => batch_test_success(),
        Ok(Skippable::Skipped(err)) => batch_test_skipped(err),
        Err(err) => batch_test_failure(err),
    };

    if is_batch_test() {
        Ok(())
    } else {
        result.map(|_| ())
    }
}

pub fn batch_test_result_details<T: std::fmt::Display>(result: Result<T, Box<dyn std::error::Error>>) -> Result<(), Box<dyn std::error::Error>> {
    match &result {
        Ok(msg) => batch_test_success_details(msg),
        Err(err) => batch_test_failure(err),
    };

    if is_batch_test() {
        Ok(())
    } else {
        result.map(|_| ())
    }
}

pub fn batch_test_result_skippable_details<T: std::fmt::Display>(result: Result<Skippable<T, Box<dyn std::error::Error>>, Box<dyn std::error::Error>>) -> Result<(), Box<dyn std::error::Error>> {
    match &result {
        Ok(Skippable::Result(msg)) => batch_test_success_details(msg),
        Ok(Skippable::Skipped(err)) => batch_test_skipped(err),
        Err(err) => batch_test_failure(err),
    };

    if is_batch_test() {
        Ok(())
    } else {
        result.map(|_| ())
    }
}

pub fn get_fair_server_avg_bw() -> Result<f64, Box<dyn std::error::Error>> {
    let mut avg_bw = 0f64;
    let mut num_cpus = 0f64;

    for entry in std::fs::read_dir("/sys/kernel/debug/sched/fair_server")? {
        let entry = entry?.path();
        if entry.is_dir() {
            let entry = entry.into_os_string().into_string().unwrap();

            let runtime: u64 =
                std::fs::read_to_string(format!("{entry}/runtime"))
                    .map_err(|err| format!("Error in reading {entry}/runtime: {err}"))
                    .and_then(|value| value.trim().parse::<u64>()
                        .map_err(|err| format!("Error in parsing {entry}/runtime: {err}"))
                    )?;
            let period: u64 =
                std::fs::read_to_string(format!("{entry}/period"))
                    .map_err(|err| format!("Error in reading {entry}/period: {err}"))
                    .and_then(|value| value.trim().parse::<u64>()
                        .map_err(|err| format!("Error in parsing {entry}/period: {err}"))
                    )?;

            avg_bw += runtime as f64 / period as f64;
            num_cpus += 1f64;
        }
    }

    Ok(avg_bw / num_cpus)
}

pub enum Skippable<T, E> {
    Result(T),
    Skipped(E)
}

impl<T, E> Into<Result<T, E>> for Skippable<T, E> {
    fn into(self) -> Result<T, E> {
        match self {
            Skippable::Result(ok) => Ok(ok),
            Skippable::Skipped(err) => Err(err),
        }
    }
}