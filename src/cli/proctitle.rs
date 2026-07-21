//! Mapping from parsed CLI arguments to an initial process title.
//!
//! This logic depends on the clap `Args`/`Command` types defined in `cli`, so
//! it lives in the CLI layer. The low-level title-setting primitives it uses
//! (`compact_process_title`, `session_name`, `set_title`) live in the
//! `process_title` core module.

use crate::cli::args::{Args, Command};
use crate::process_title::set_title;

pub(crate) fn initial_title(args: &Args) -> String {
    match &args.command {
        Some(Command::Serve { .. }) => "jcode:server".to_string(),
        Some(Command::Run { .. }) => "jcode run".to_string(),
        Some(Command::Version { .. }) => "jcode version".to_string(),
        Some(Command::Usage { .. }) => "jcode usage".to_string(),
        Some(Command::Debug { .. }) => "jcode debug".to_string(),
        Some(Command::Auth(_)) => "jcode auth".to_string(),
        Some(Command::Provider(_)) => "jcode provider".to_string(),
        Some(Command::Memory(_)) => "jcode memory".to_string(),
        Some(Command::Session(_)) => "jcode session".to_string(),
        Some(Command::Model(_)) => "jcode model".to_string(),
        Some(Command::ProviderTestCoverage { .. }) => "jcode provider-test-coverage".to_string(),
        Some(Command::ProviderDoctor { .. }) => "jcode provider-doctor".to_string(),
        Some(Command::AuthTest { .. }) => "jcode auth-test".to_string(),
        None => "jcode".to_string(),
    }
}

pub(crate) fn set_initial_title(args: &Args) {
    set_title(initial_title(args));
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn initial_title_labels_server() {
        let args = Args::parse_from(["jcode", "serve"]);
        assert_eq!(initial_title(&args), "jcode:server");
    }

    #[test]
    fn initial_title_labels_run() {
        let args = Args::parse_from(["jcode", "run", "status"]);
        assert_eq!(initial_title(&args), "jcode run");
    }
}
