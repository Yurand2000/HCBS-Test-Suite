use crate::utils::*;

pub mod prelude {
    pub use super::{
        set_batch_test,
        is_batch_test,
        batch_test_success,
        batch_test_success_details,
        batch_test_failure,
        batch_test_skipped,
        batch_test_header,
        batch_test_result,
        batch_test_result_details,
        batch_test_result_skippable,
        batch_test_result_skippable_details,
    };
}

fn __is_env_var_set(var: &str) -> bool {
    match std::env::var(var) {
        Ok(v) if v != "" => true,
        _ => false,
    }
}

pub fn set_batch_test() {
    unsafe { std::env::set_var("BATCH_TEST", "1") };
}

pub fn is_batch_test() -> bool {
    __is_env_var_set("BATCH_TEST")
}

pub fn batch_test_success() {
    if __is_env_var_set("TERM_COLORS") {
        println!("\x1b[32mSuccess ✔\x1b[0m");
    } else {
        println!("Success ✔");
    }
}

pub fn batch_test_success_details<T: std::fmt::Display>(msg: T) {
    batch_test_success();

    if __is_env_var_set("TERM_COLORS") {
        println!("\x1b[32m    Details: \x1b[0m{msg}");
    } else {
        println!("    Details: {msg}");
    }
}

pub fn batch_test_failure<E: std::fmt::Display>(err: E) {
    if __is_env_var_set("TERM_COLORS") {
        println!("\x1b[31mFailure ✖\n    Reason: \x1b[0m{err}")
    } else {
        println!("Failure ✖\n    Reason: {err}")
    }
}

pub fn batch_test_skipped<T: std::fmt::Display>(msg: T) {
    if __is_env_var_set("TERM_COLORS") {
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

pub fn batch_test_result<T>(result: anyhow::Result<T>) -> anyhow::Result<()> {
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

pub fn batch_test_result_skippable<T>(result: anyhow::Result<Skippable<T>>) -> anyhow::Result<()> {
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

pub fn batch_test_result_details<T: std::fmt::Display>(result: anyhow::Result<T>) -> anyhow::Result<()> {
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

pub fn batch_test_result_skippable_details<T: std::fmt::Display>(result: anyhow::Result<Skippable<T>>) -> anyhow::Result<()> {
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

