#![cfg_attr(test, allow(clippy::await_holding_lock))]

use anyhow::Result;
use clap::CommandFactory;
use std::time::Instant;

use super::args::{
    Args, AuthCommand, Command, MemoryCommand, ModelCommand, ProviderCommand, SessionCommand,
};
use crate::{provider_catalog, server, session};

use super::{commands, debug, output, provider_init};
use provider_init::ProviderChoice;

#[cfg(target_os = "linux")]
fn is_file_controlled_debug_client() -> bool {
    std::env::var_os("JCODE_DEBUG_CMD_PATH").is_some()
}

#[cfg(target_os = "linux")]
fn is_orphan_adopter_name(name: &str) -> bool {
    matches!(name.trim(), "init" | "systemd")
}

#[cfg(target_os = "linux")]
fn parent_is_orphan_adopter(parent_pid: libc::pid_t) -> bool {
    if parent_pid <= 1 {
        return true;
    }
    std::fs::read_to_string(format!("/proc/{parent_pid}/comm"))
        .is_ok_and(|name| is_orphan_adopter_name(&name))
}

/// Tie file-controlled debug clients to the process that launched them.
///
/// These clients are automation helpers, not user-owned terminals. Without a
/// parent-death signal they are reparented to init when a verification script
/// or debug server exits, retaining a full TUI and session history indefinitely.
#[cfg(target_os = "linux")]
fn arm_debug_client_parent_death_signal() {
    if !is_file_controlled_debug_client() {
        return;
    }

    // Capture the parent first, then check it again after prctl. This closes the
    // race where the launcher exits immediately before the signal is armed.
    // Safety: getppid has no preconditions and does not dereference pointers.
    let parent_pid = unsafe { libc::getppid() };
    // Safety: PR_SET_PDEATHSIG accepts a signal number as its scalar argument.
    let armed = unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) } == 0;
    // Safety: getppid has no preconditions and does not dereference pointers.
    let current_parent_pid = unsafe { libc::getppid() };
    if armed
        && (parent_is_orphan_adopter(parent_pid)
            || current_parent_pid != parent_pid
            || parent_is_orphan_adopter(current_parent_pid))
    {
        std::process::exit(0);
    }
}

#[cfg(not(target_os = "linux"))]
fn arm_debug_client_parent_death_signal() {}

pub(crate) async fn run_main(mut args: Args) -> Result<()> {
    arm_debug_client_parent_death_signal();
    resolve_resume_arg(&mut args)?;

    if let Some(profile_name) = args
        .provider_profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        provider_catalog::apply_named_provider_profile_env(profile_name)?;
        crate::env::set_var("JCODE_PROVIDER_PROFILE_NAME", profile_name);
        crate::env::set_var("JCODE_PROVIDER_PROFILE_ACTIVE", "1");
        args.provider = ProviderChoice::OpenaiCompatible;
    }

    if let Some(tool_profile) = args.tool_profile.as_deref() {
        crate::env::set_var("JCODE_TOOL_PROFILE", tool_profile);
    }
    if let Some(tools) = args.tools.as_deref() {
        crate::env::set_var("JCODE_TOOLS", tools);
    }
    if let Some(disabled_tools) = args.disabled_tools.as_deref() {
        crate::env::set_var("JCODE_DISABLED_TOOLS", disabled_tools);
    }
    if args.disable_base_tools {
        crate::env::set_var("JCODE_DISABLE_BASE_TOOLS", "1");
    }
    if args.tool_profile.is_some()
        || args.tools.is_some()
        || args.disabled_tools.is_some()
        || args.disable_base_tools
    {
        crate::config::invalidate_config_cache();
    }

    match args.command {
        Some(Command::Serve {
            temporary_server,
            owner_pid,
            temp_idle_timeout_secs,
            server_name,
        }) => {
            let serve_start = Instant::now();
            crate::env::set_var("JCODE_NON_INTERACTIVE", "1");
            if temporary_server {
                server::configure_temporary_server(owner_pid, temp_idle_timeout_secs);
            }
            let provider_start = Instant::now();
            let provider =
                provider_init::init_provider(&args.provider, args.model.as_deref()).await?;
            let provider_ms = provider_start.elapsed().as_millis();
            let server_new_start = Instant::now();
            let server = server::Server::new_with_name(provider, server_name);
            let server_new_ms = server_new_start.elapsed().as_millis();
            crate::logging::info(&format!(
                "[TIMING] serve bootstrap: provider_init={}ms, server_new={}ms, before_run={}ms",
                provider_ms,
                server_new_ms,
                serve_start.elapsed().as_millis()
            ));
            server.run().await?;
        }
        Some(Command::Run {
            message,
            json,
            ndjson,
        }) => {
            commands::run_single_message_command(
                &args.provider,
                args.model.as_deref(),
                args.resume.as_deref(),
                &message,
                json,
                ndjson,
            )
            .await?;
        }
        Some(Command::Version { json }) => {
            commands::run_version_command(json)?;
        }
        Some(Command::Usage { json }) => {
            commands::run_usage_command(json).await?;
        }
        Some(Command::Debug {
            command,
            arg,
            session,
            socket,
            wait,
        }) => {
            debug::run_debug_command(&command, &arg, session, socket, wait).await?;
        }
        Some(Command::Auth(subcmd)) => match subcmd {
            AuthCommand::Import {
                provider,
                stdin,
                json,
            } => commands::run_auth_import_command(&provider, stdin, json)?,
            AuthCommand::Status { json } => commands::run_auth_status_command(json)?,
            AuthCommand::Doctor {
                provider,
                validate,
                json,
            } => {
                let provider_arg = auth_doctor_provider_arg(provider.as_deref(), &args.provider);
                commands::run_auth_doctor_command(provider_arg, validate, json).await?
            }
        },
        Some(Command::Provider(subcmd)) => match subcmd {
            ProviderCommand::List { json } => {
                commands::run_provider_list_command(json)?;
            }
            ProviderCommand::Current { json } => {
                commands::run_provider_current_command(&args.provider, args.model.as_deref(), json)
                    .await?;
            }
            ProviderCommand::Add {
                name,
                base_url,
                model,
                context_window,
                api_key_env,
                api_key,
                api_key_stdin,
                no_api_key,
                auth,
                auth_header,
                env_file,
                set_default,
                overwrite,
                provider_routing,
                model_catalog,
                json,
            } => {
                commands::run_provider_add_command(commands::ProviderAddOptions {
                    name,
                    base_url,
                    model,
                    context_window,
                    api_key_env,
                    api_key,
                    api_key_stdin,
                    no_api_key,
                    auth,
                    auth_header,
                    env_file,
                    set_default,
                    overwrite,
                    provider_routing,
                    model_catalog,
                    json,
                })?;
            }
        },
        Some(Command::Memory(subcmd)) => {
            commands::run_memory_command(map_memory_subcommand(subcmd))?;
        }
        Some(Command::Session(subcmd)) => match subcmd {
            SessionCommand::Rename {
                session,
                name,
                clear,
                json,
            } => commands::run_session_rename_command(&session, name.as_deref(), clear, json)?,
        },
        Some(Command::Model(subcmd)) => match subcmd {
            ModelCommand::List { json, verbose } => {
                commands::run_model_command(&args.provider, args.model.as_deref(), json, verbose)
                    .await?;
            }
        },
        None => run_default_command(args).await?,
    }

    Ok(())
}

fn auth_doctor_provider_arg<'a>(
    positional_provider: Option<&'a str>,
    global_provider: &'a ProviderChoice,
) -> Option<&'a str> {
    positional_provider.or_else(|| {
        if *global_provider == ProviderChoice::Auto {
            None
        } else {
            Some(global_provider.as_arg_value())
        }
    })
}

fn resolve_resume_arg(args: &mut Args) -> Result<()> {
    if let Some(ref resume_id) = args.resume {
        let resume_id = resume_id.clone();
        match resolve_resume_id(&resume_id) {
            Ok(full_id) => {
                args.resume = Some(full_id);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                if !output::quiet_enabled() {
                    eprintln!("\nUse `jcode --resume` to list available sessions.");
                }
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn resolve_resume_id(resume_id: &str) -> Result<String> {
    match session::find_session_by_name_or_id(resume_id) {
        Ok(full_id) => Ok(full_id),
        Err(native_err) => match crate::import::import_external_resume_id(resume_id)? {
            Some(imported_id) => Ok(imported_id),
            None => Err(native_err),
        },
    }
}

fn map_memory_subcommand(subcmd: MemoryCommand) -> commands::MemorySubcommand {
    match subcmd {
        MemoryCommand::List { scope, tag } => commands::MemorySubcommand::List { scope, tag },
        MemoryCommand::Search { query, semantic } => {
            commands::MemorySubcommand::Search { query, semantic }
        }
        MemoryCommand::Export { output, scope } => {
            commands::MemorySubcommand::Export { output, scope }
        }
        MemoryCommand::Import {
            input,
            scope,
            overwrite,
        } => commands::MemorySubcommand::Import {
            input,
            scope,
            overwrite,
        },
        MemoryCommand::Stats => commands::MemorySubcommand::Stats,
        MemoryCommand::ClearTest => commands::MemorySubcommand::ClearTest,
    }
}

async fn run_default_command(args: Args) -> Result<()> {
    let _ = args;
    let mut command = Args::command();
    command.print_help()?;
    println!();
    anyhow::bail!("a command is required in headless mode")
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod dispatch_tests;
