//! Something that must happen after a panic, done in `catch_unwind`'s `Err` arm: a report a
//! supervisor would read, or a cleanup. Built with this package's `panic = "abort"`, the process
//! dies at the panic and the arm never runs; built with `--config profile.dev.panic='"unwind"'`,
//! it prints `recovered` and exits 0.

use std::panic;
use std::process::ExitCode;

fn main() -> ExitCode {
    // The default hook prints the panic; silence it so stdout shows only what the arm did.
    panic::set_hook(Box::new(|_| {}));
    match panic::catch_unwind(|| -> u8 { panic!("boom") }) {
        Ok(_) => ExitCode::from(3),
        Err(_) => {
            println!("recovered: panic was caught, cleanup ran");
            ExitCode::SUCCESS
        }
    }
}
