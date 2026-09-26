//! The `rr` binary. Everything lives in the library (`rr_cli`) so the commands can be tested
//! without spawning a process; this file only hands over the arguments and the exit status.
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    rr_cli::main(std::env::args_os())
}
