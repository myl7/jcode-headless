#![cfg_attr(test, allow(clippy::await_holding_lock))]

use anyhow::Result;
use clap::CommandFactory;
use std::io::IsTerminal;
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Instant;

use super::args::{
    AmbientCommand, Args, AuthCommand, CloudCommand, CloudSessionsCommand, Command, MemoryCommand,
    ModelCommand, ProviderCommand, ServerCommand, SessionCommand,
};
use crate::{provider_catalog, server, session, startup_profile};

use super::{acp, commands, debug, hot_exec, output, provider_init};
use provider_init::ProviderChoice;

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
        Some(Command::Acp) => {
            acp::run_acp_command(
                args.provider,
                args.model.clone(),
                args.provider_profile.clone(),
                args.tool_profile.is_some(),
            )
            .await?;
        }
        Some(Command::Server { action }) => match action {
            ServerCommand::Start { json } => {
                spawn_server(
                    &args.provider,
                    args.model.as_deref(),
                    args.provider_profile.as_deref(),
                )
                .await?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "running",
                        })
                    );
                } else {
                    println!("Jcode server is running.");
                }
            }
            ServerCommand::Keepalive => {
                run_server_keepalive(
                    &args.provider,
                    args.model.as_deref(),
                    args.provider_profile.as_deref(),
                )
                .await?;
            }
            ServerCommand::Reload { force, json } => {
                commands::run_server_reload_command(force, json).await?;
            }
            ServerCommand::Stop { force, json } => {
                commands::run_server_stop_command(force, json).await?;
            }
        },
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
        Some(Command::Update) => {
            hot_exec::run_update()?;
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
        Some(Command::Ambient(subcmd)) => {
            commands::run_ambient_command(map_ambient_subcommand(subcmd)).await?;
        }
        Some(Command::Cloud(subcmd)) => {
            commands::run_cloud_command(map_cloud_subcommand(subcmd))?;
        }
        Some(Command::Model(subcmd)) => match subcmd {
            ModelCommand::List { json, verbose } => {
                commands::run_model_command(&args.provider, args.model.as_deref(), json, verbose)
                    .await?;
            }
        },
        Some(Command::ProviderTestCoverage {
            provider_query,
            model_query,
            coverage_file,
            coverage_limit,
        }) => {
            let coverage_path = coverage_file.as_deref().map(std::path::Path::new);
            let colorize = std::io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").is_none()
                && std::env::var_os("JCODE_NO_COLOR").is_none();
            if let Some(provider) = provider_query {
                let model = model_query
                    .or_else(|| args.model.clone())
                    .unwrap_or_else(|| "*".to_string());
                let report = crate::live_tests::format_provider_test_coverage_report(
                    &provider,
                    &model,
                    coverage_path,
                );
                print_provider_test_coverage_report(&report, colorize);
            } else {
                let (coverage, path) = crate::live_tests::load_coverage(coverage_path)?;
                let summary = crate::live_tests::strict_live_provider_model_coverage_summary(
                    &coverage,
                    path.display().to_string(),
                );
                let report = crate::live_tests::format_strict_live_provider_model_coverage_summary(
                    &summary,
                    coverage_limit,
                );
                print_provider_test_coverage_report(&report, colorize);
            }
        }
        Some(Command::ProviderDoctor {
            provider,
            tier,
            json,
        }) => {
            crate::cli::provider_doctor::run_provider_doctor_command(
                &provider,
                args.model.as_deref(),
                &tier,
                json,
            )
            .await?;
        }
        Some(Command::AuthTest {
            all_configured,
            no_smoke,
            no_tool_smoke,
            prompt,
            json,
            output,
            coverage,
            context_audit,
            coverage_file,
            coverage_limit,
        }) => {
            if coverage {
                commands::run_auth_test_coverage_command(
                    json,
                    output.as_deref(),
                    coverage_file.as_deref(),
                    coverage_limit,
                )?;
            } else if context_audit {
                commands::run_auth_test_context_audit_command(
                    &args.provider,
                    all_configured,
                    json,
                    output.as_deref(),
                )
                .await?;
            } else {
                commands::run_auth_test_command(
                    &args.provider,
                    args.model.as_deref(),
                    all_configured,
                    no_smoke,
                    no_tool_smoke,
                    prompt.as_deref(),
                    json,
                    output.as_deref(),
                )
                .await?;
            }
        }
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
                match resume_resolution_failure_action(&resume_id, |key| std::env::var_os(key)) {
                    // During a reload/update/restart handoff the client re-execs
                    // itself with `--resume <id>` and `JCODE_RESUMING=1`. In the
                    // client/server architecture the shared server is the authority
                    // for session lifecycle, so an id that is not in the local store
                    // can still be valid server-side. Hard-exiting here dumped the
                    // user back to a shell with "No session found matching ...",
                    // making jcode unusable after an auto-update (issue #328).
                    // Instead, keep the raw id and let the remote connection resolve
                    // it; if the server cannot find it either, the TUI surfaces a
                    // recoverable message and falls back to a fresh session rather
                    // than killing the process.
                    ResumeResolutionFailureAction::DeferToServer => {
                        crate::logging::warn(&format!(
                            "Resume id '{}' not found locally during reload handoff ({}); deferring resolution to the server instead of exiting",
                            resume_id, e
                        ));
                        // Leave args.resume as the raw id for the server to resolve.
                    }
                    ResumeResolutionFailureAction::Exit => {
                        eprintln!("Error: {}", e);
                        if !output::quiet_enabled() {
                            eprintln!("\nUse `jcode --resume` to list available sessions.");
                        }
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    Ok(())
}

/// What to do when a `--resume <id>` cannot be resolved from the local session
/// store. Extracted as a pure function so the reload-handoff recovery path can
/// be unit-tested without invoking `std::process::exit` (issue #328).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResumeResolutionFailureAction {
    /// Keep the raw id and let the shared server resolve it (reload handoff).
    DeferToServer,
    /// No live handoff in progress; the id is genuinely bad, so exit.
    Exit,
}

fn resume_resolution_failure_action<F, V>(
    _resume_id: &str,
    var_os: F,
) -> ResumeResolutionFailureAction
where
    F: Fn(&str) -> Option<V>,
{
    if var_os("JCODE_RESUMING").is_some() {
        ResumeResolutionFailureAction::DeferToServer
    } else {
        ResumeResolutionFailureAction::Exit
    }
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

fn map_ambient_subcommand(subcmd: AmbientCommand) -> commands::AmbientSubcommand {
    match subcmd {
        AmbientCommand::Status => commands::AmbientSubcommand::Status,
        AmbientCommand::Log => commands::AmbientSubcommand::Log,
        AmbientCommand::Trigger => commands::AmbientSubcommand::Trigger,
        AmbientCommand::Stop => commands::AmbientSubcommand::Stop,
    }
}

fn map_cloud_subcommand(subcmd: CloudCommand) -> commands::CloudSubcommand {
    match subcmd {
        CloudCommand::Sessions { action } => {
            commands::CloudSubcommand::Sessions(map_cloud_sessions_subcommand(action))
        }
    }
}

fn map_cloud_sessions_subcommand(
    action: CloudSessionsCommand,
) -> commands::CloudSessionsSubcommand {
    match action {
        CloudSessionsCommand::Configure {
            api_base,
            api_token,
            api_token_env,
            api_token_id,
            user_id,
            helper,
            clear,
        } => commands::CloudSessionsSubcommand::Configure {
            api_base,
            api_token,
            api_token_env,
            api_token_id,
            user_id,
            helper,
            clear,
        },
        CloudSessionsCommand::Status { json } => commands::CloudSessionsSubcommand::Status { json },
        CloudSessionsCommand::Upload {
            session_file,
            raw,
            jade,
        } => commands::CloudSessionsSubcommand::Upload {
            session_file,
            raw,
            user_id: jade.user_id,
            profile: jade.profile,
            region: jade.region,
            helper: jade.helper,
        },
        CloudSessionsCommand::UploadLatest {
            sessions_dir,
            raw,
            jade,
        } => commands::CloudSessionsSubcommand::UploadLatest {
            sessions_dir,
            raw,
            user_id: jade.user_id,
            profile: jade.profile,
            region: jade.region,
            helper: jade.helper,
        },
        CloudSessionsCommand::Sync {
            sessions_dir,
            since_days,
            all,
            max,
            min_interval_mins,
            raw,
            dry_run,
            force,
            json,
            jade,
        } => commands::CloudSessionsSubcommand::Sync {
            sessions_dir,
            since_days,
            all,
            max,
            min_interval_mins,
            raw,
            dry_run,
            force,
            json,
            user_id: jade.user_id,
            profile: jade.profile,
            region: jade.region,
            helper: jade.helper,
        },
        CloudSessionsCommand::List { limit, json, jade } => {
            commands::CloudSessionsSubcommand::List {
                limit,
                json,
                user_id: jade.user_id,
                profile: jade.profile,
                region: jade.region,
                helper: jade.helper,
            }
        }
        CloudSessionsCommand::Verify { session_id, jade } => {
            commands::CloudSessionsSubcommand::Verify {
                session_id,
                user_id: jade.user_id,
                profile: jade.profile,
                region: jade.region,
                helper: jade.helper,
            }
        }
    }
}

async fn run_default_command(args: Args) -> Result<()> {
    let _ = args;
    let mut command = Args::command();
    command.print_help()?;
    println!();
    anyhow::bail!("a command is required in headless mode")
}

fn print_provider_test_coverage_report(report: &str, colorize: bool) {
    if colorize {
        print!(
            "{}",
            crate::live_tests::colorize_provider_test_coverage_output(report)
        );
    } else {
        print!("{}", report);
    }
}

pub(crate) async fn server_is_running() -> bool {
    server_is_running_at(&server::socket_path()).await
}

async fn wait_for_existing_reload_server(context: &str) -> bool {
    if let Some(state) = server::recent_reload_state(std::time::Duration::from_secs(30)) {
        match state.phase {
            server::ReloadPhase::Starting => {
                crate::logging::info(&format!(
                    "Reload state=starting during {}; waiting for existing server to return",
                    context
                ));
                return wait_for_reloading_server().await;
            }
            server::ReloadPhase::Failed => {
                crate::logging::warn(&format!(
                    "Reload state=failed during {} on {}: {}; recent_state={}",
                    context,
                    server::socket_path().display(),
                    state
                        .detail
                        .unwrap_or_else(|| "unknown reload failure".to_string()),
                    server::reload_state_summary(std::time::Duration::from_secs(60))
                ));
            }
            server::ReloadPhase::SocketReady => {}
        }
    }

    false
}

pub(crate) async fn wait_for_resuming_server(context: &str, timeout: std::time::Duration) -> bool {
    let socket_path = server::socket_path();
    let start = std::time::Instant::now();
    let mut announced = false;

    while start.elapsed() < timeout {
        if server_is_running_at(&socket_path).await {
            crate::logging::info(&format!(
                "Server became available during resume wait for {} after {}ms",
                context,
                start.elapsed().as_millis()
            ));
            return true;
        }

        if !announced {
            crate::logging::info(&format!(
                "Server not ready during {}; waiting up to {}ms for a resumed/reloading server before spawning a replacement",
                context,
                timeout.as_millis()
            ));
            announced = true;
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    false
}

pub(crate) async fn wait_for_reloading_server() -> bool {
    match server::await_reload_handoff(&server::socket_path(), std::time::Duration::from_secs(30))
        .await
    {
        server::ReloadWaitStatus::Ready => true,
        server::ReloadWaitStatus::Failed(detail) => {
            crate::logging::warn(&format!(
                "Reload handoff failed while waiting for server on {}: {}; recent_state={}",
                server::socket_path().display(),
                detail.unwrap_or_else(|| "unknown reload failure".to_string()),
                server::reload_state_summary(std::time::Duration::from_secs(60))
            ));
            false
        }
        server::ReloadWaitStatus::Idle => false,
        server::ReloadWaitStatus::Waiting { .. } => false,
    }
}

async fn server_is_running_at(path: &std::path::Path) -> bool {
    // Check liveness before performing a protocol handshake. On Windows the
    // named pipe may be busy while another client is connecting; that already
    // proves a daemon exists, while a handshake connect can otherwise wait in
    // the transport's ERROR_PIPE_BUSY retry loop and block server startup.
    server::has_live_listener(path).await || server::is_server_ready(path).await
}

#[cfg(unix)]
fn spawn_lock_path(socket_path: &std::path::Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.spawning", socket_path.display()))
}

#[cfg(unix)]
struct SpawnLockGuard {
    _file: std::fs::File,
    path: std::path::PathBuf,
}

#[cfg(unix)]
impl Drop for SpawnLockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(unix)]
fn try_acquire_spawn_lock(path: &std::path::Path) -> Result<Option<SpawnLockGuard>> {
    use std::fs::OpenOptions;
    use std::os::fd::AsRawFd;

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)?;
    let fd = file.as_raw_fd();
    let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if ret == 0 {
        Ok(Some(SpawnLockGuard {
            _file: file,
            path: path.to_path_buf(),
        }))
    } else {
        Ok(None)
    }
}

#[cfg(unix)]
async fn acquire_spawn_lock_or_wait(
    socket_path: &std::path::Path,
) -> Result<Option<SpawnLockGuard>> {
    let lock_path = spawn_lock_path(socket_path);
    let wait_start = std::time::Instant::now();
    let wait_timeout = std::time::Duration::from_secs(10);
    let mut announced_wait = false;

    loop {
        if let Some(lock) = try_acquire_spawn_lock(&lock_path)? {
            return Ok(Some(lock));
        }

        if server_is_running_at(socket_path).await {
            return Ok(None);
        }

        if !announced_wait {
            output::stderr_info("Another client is starting the server, waiting...");
            announced_wait = true;
        }

        if wait_start.elapsed() >= wait_timeout {
            anyhow::bail!(
                "Timed out waiting for another client to start server at {}",
                socket_path.display()
            );
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

pub(crate) async fn spawn_server(
    provider_choice: &ProviderChoice,
    model: Option<&str>,
    provider_profile: Option<&str>,
) -> Result<()> {
    let socket_path = server::socket_path();
    if server_is_running_at(&socket_path).await {
        startup_profile::mark("server_ready");
        return Ok(());
    }

    if wait_for_existing_reload_server("server spawn").await {
        startup_profile::mark("server_ready");
        return Ok(());
    }

    #[cfg(unix)]
    let _spawn_lock = acquire_spawn_lock_or_wait(&socket_path).await?;

    if server_is_running_at(&socket_path).await {
        startup_profile::mark("server_ready");
        return Ok(());
    }

    if wait_for_existing_reload_server("server spawn after lock").await {
        startup_profile::mark("server_ready");
        return Ok(());
    }

    startup_profile::mark("server_spawn_start");
    output::stderr_info("Starting server...");
    let exe = std::env::current_exe()
        .ok()
        .ok_or_else(|| anyhow::anyhow!("Could not determine executable path for server spawn"))?;
    let mut cmd = ProcessCommand::new(&exe);
    cmd.arg("--provider").arg(provider_choice.as_arg_value());
    if let Some(provider_profile) = provider_profile {
        cmd.arg("--provider-profile").arg(provider_profile);
    }
    if let Some(model) = model {
        cmd.arg("--model").arg(model);
    }
    cmd.arg("serve")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        let _child = server::spawn_server_notify(&mut cmd).await?;
        startup_profile::mark("server_ready");
    }
    #[cfg(not(unix))]
    {
        use std::io::Read;

        let mut child = cmd.spawn()?;
        let start = std::time::Instant::now();
        // Windows server bootstrap can legitimately take tens of seconds on
        // slow hosts (auth preflights + provider init were observed at 15-60s
        // on a Windows Server VPS, issue #503). The child's liveness is
        // checked every poll, so a generous budget only delays the error for
        // a genuinely hung server, while a crashed server still fails fast
        // with its stderr.
        let timeout = std::time::Duration::from_secs(120);
        while start.elapsed() < timeout {
            if server::has_live_listener(&socket_path).await {
                startup_profile::mark("server_ready");
                return Ok(());
            }

            if let Some(status) = child.try_wait()? {
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                let detail = stderr.trim();
                if detail.is_empty() {
                    anyhow::bail!("Server exited before becoming ready (status: {})", status);
                }
                anyhow::bail!(
                    "Server exited before becoming ready (status: {}). {}",
                    status,
                    detail
                );
            }

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        anyhow::bail!(
            "Timed out waiting for server to become ready at {} after {}ms",
            server::socket_path().display(),
            timeout.as_millis()
        );
    }

    #[cfg(unix)]
    Ok(())
}

async fn run_server_keepalive(
    provider_choice: &ProviderChoice,
    model: Option<&str>,
    provider_profile: Option<&str>,
) -> Result<()> {
    let mut owner_closed = tokio::task::spawn_blocking(|| {
        let mut stdin = std::io::stdin();
        let mut buffer = [0u8; 256];
        loop {
            match std::io::Read::read(&mut stdin, &mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
        }
    });
    let mut client: Option<server::Client> = None;
    let mut first_attempt = true;

    loop {
        let delay = if first_attempt {
            first_attempt = false;
            std::time::Duration::ZERO
        } else if client.is_some() {
            std::time::Duration::from_secs(30)
        } else {
            std::time::Duration::from_secs(1)
        };
        tokio::select! {
            _ = &mut owner_closed => return Ok(()),
            _ = tokio::time::sleep(delay) => {
                if client.is_some() {
                    // A Ping is a one-shot control request, so sending it over
                    // the held connection would make the server close that
                    // connection after replying. Probe through a short-lived
                    // client instead and leave the counted keepalive connected.
                    let healthy = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        async {
                            let mut probe = server::Client::connect().await?;
                            probe.ping().await
                        },
                    )
                    .await
                    .is_ok_and(|result| result.unwrap_or(false));
                    if healthy {
                        continue;
                    }
                    client = None;
                }
                if spawn_server(provider_choice, model, provider_profile).await.is_ok()
                    && let Ok(connected) = server::Client::connect().await
                {
                    client = Some(connected);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod dispatch_tests;
