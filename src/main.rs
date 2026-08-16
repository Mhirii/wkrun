use std::process::ExitCode;

use wkrun::run;

fn main() -> ExitCode {
    run(std::env::args_os()).into_exit_code()
}
