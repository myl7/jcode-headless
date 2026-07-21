use anyhow::Result;
use clap::Parser;

use crate::{logging, server, startup_profile, storage};

use super::{args::Args, dispatch, output, terminal};

pub async fn run() -> Result<()> {
    // This distribution has no interactive UI or login surface. Provider
    // initialization must never prompt and only consumes injected credentials.
    crate::env::set_var("JCODE_NON_INTERACTIVE", "1");

    startup_profile::init();

    terminal::install_panic_hook();
    startup_profile::mark("panic_hook");

    logging::init();
    startup_profile::mark("logging_init");
    // Old log pruning now runs on a background thread inside logging::init(),
    // so it no longer blocks startup. Memory-event logs have a separate,
    // longer (14-day) retention, so prune them on their own background thread.
    std::thread::Builder::new()
        .name("jcode-memlog-cleanup".to_string())
        .spawn(crate::memory_log::cleanup_old_memory_logs)
        .ok();
    // Prune stale per-session `.bak` recovery copies (never the transcripts
    // themselves) so the sessions directory does not grow without bound.
    std::thread::Builder::new()
        .name("jcode-session-bak-prune".to_string())
        .spawn(crate::session::prune_old_session_backups)
        .ok();
    logging::info("jcode starting");

    // Wire config-reload reactions without making config depend on auth/bus:
    // when the config cache reloads, invalidate the auth-status cache and
    // broadcast a models-updated event.
    crate::config::on_config_reloaded(crate::auth::AuthStatus::invalidate_cache);
    crate::config::on_config_reloaded(|| crate::bus::Bus::global().publish_models_updated());

    // Invert the legacy provider_catalog -> auth dependency: provider_catalog
    // consults registered fallback resolvers, and auth (the higher layer)
    // registers its external-CLI credential scan here.
    crate::provider_catalog::register_api_key_fallback_resolver(
        crate::auth::external::load_api_key_for_env,
    );

    // Register externally-implemented provider runtimes with the base
    // provider registry. These crates sit downstream of jcode-base (so
    // provider edits do not rebuild the app spine), which means base cannot
    // name their concrete types; this composition root wires them up instead.
    register_external_provider_runtimes();

    // Invert the legacy memory -> skill dependency: memory collects synthetic
    // entries from registered providers, and skill (the higher layer that
    // depends on MemoryEntry) registers its registry->memory adapter here.
    // The shared snapshot holds global skills only; memory retrieval is
    // process-scoped, so compose the project overlay from the process cwd
    // (issue #457 keeps session overlays out of the shared registry).
    crate::memory::register_synthetic_entry_provider(|| {
        let global = crate::skill::SkillRegistry::shared_snapshot();
        crate::skill::SkillRegistry::effective_for_working_dir(&global, None)
            .list()
            .into_iter()
            .map(|skill| skill.as_memory_entry())
            .collect()
    });

    crate::platform::raise_nofile_limit_best_effort(8_192);
    startup_profile::mark("nofile_limit");

    storage::harden_user_config_permissions();
    startup_profile::mark("perm_harden");

    let args = parse_and_prepare_args()?;

    if let Err(e) = dispatch::run_main(args).await {
        report_main_error(&e);
        return Err(e);
    }

    Ok(())
}

/// Register provider runtimes that live downstream of `jcode-base` with the
/// base crate's external provider registry. Keep every downstream runtime
/// registration in this one function so the composition-root wiring stays
/// discoverable as more providers move out of the base crate.
pub fn register_external_provider_runtimes() {
    crate::provider::external::register_external_provider(
        crate::provider::external::GEMINI_RUNTIME,
        || std::sync::Arc::new(jcode_provider_gemini_runtime::GeminiProvider::new()),
    );
    crate::provider::external::register_external_provider(
        crate::provider::external::CURSOR_RUNTIME,
        || std::sync::Arc::new(jcode_provider_cursor_runtime::CursorCliProvider::new()),
    );
    crate::provider::external::register_external_provider(
        crate::provider::external::ANTIGRAVITY_RUNTIME,
        || std::sync::Arc::new(jcode_provider_antigravity_runtime::AntigravityProvider::new()),
    );
    crate::provider::external::register_external_provider(
        crate::provider::external::ANTHROPIC_RUNTIME,
        || std::sync::Arc::new(jcode_provider_anthropic_runtime::AnthropicProvider::new()),
    );
    // OpenRouter serves several identities (aggregator, pinned API-key
    // runtime, direct OpenAI-compatible profiles, named config profiles)
    // through one concrete type, so it registers a parameterized factory.
    crate::provider::external::register_openrouter_factory(|spec| {
        use crate::provider::external::OpenRouterRuntimeSpec;
        use jcode_provider_openrouter_runtime::OpenRouterProvider;
        let provider: std::sync::Arc<dyn crate::provider::Provider> = match spec {
            OpenRouterRuntimeSpec::Default => std::sync::Arc::new(OpenRouterProvider::new()?),
            OpenRouterRuntimeSpec::OpenRouterApiKey => {
                std::sync::Arc::new(OpenRouterProvider::new_openrouter_api_key_runtime()?)
            }
            OpenRouterRuntimeSpec::CompatibleProfile(profile) => std::sync::Arc::new(
                OpenRouterProvider::new_openai_compatible_profile_runtime(profile)?,
            ),
            OpenRouterRuntimeSpec::NamedProfile { name, config } => std::sync::Arc::new(
                OpenRouterProvider::new_named_openai_compatible(&name, &config)?,
            ),
        };
        Ok(provider)
    });
    crate::provider::external::register_profile_catalog_refresh(
        jcode_provider_openrouter_runtime::maybe_schedule_openai_compatible_profile_catalog_refresh,
    );
    crate::provider::external::register_standard_openrouter_catalog_refresh(
        jcode_provider_openrouter_runtime::maybe_schedule_standard_openrouter_catalog_refresh,
    );
    // OpenAI routes require a mounted Codex subscription credential or an API key.
    crate::provider::external::register_external_provider_fallible(
        crate::provider::external::OPENAI_RUNTIME,
        || {
            let credentials = crate::auth::codex::load_credentials().ok()?;
            let provider = jcode_provider_openai_runtime::OpenAIProvider::new(credentials);
            Some(std::sync::Arc::new(provider) as std::sync::Arc<dyn crate::provider::Provider>)
        },
    );
    // Copilot's constructor is fallible (needs a GitHub token) and the runtime
    // wants tier detection scheduled right after construction, eagerly for
    // interactive sessions and deferred for non-interactive ones. That policy
    // lives here in the composition root so base stays provider-agnostic.
    crate::provider::external::register_external_provider_fallible(
        crate::provider::external::COPILOT_RUNTIME,
        || {
            let provider = std::sync::Arc::new(
                jcode_provider_copilot_runtime::CopilotApiProvider::new().ok()?,
            );
            let eager_tier_detection = std::env::var("JCODE_NON_INTERACTIVE").is_err();
            if eager_tier_detection && tokio::runtime::Handle::try_current().is_ok() {
                let p_clone = std::sync::Arc::clone(&provider);
                tokio::spawn(async move {
                    p_clone.detect_tier_and_set_default().await;
                });
            } else {
                provider.complete_init_without_tier_detection();
            }
            Some(provider as std::sync::Arc<dyn crate::provider::Provider>)
        },
    );
}

fn parse_and_prepare_args() -> Result<Args> {
    let args = Args::parse();
    startup_profile::mark("args_parse");

    output::set_quiet_enabled(args.quiet);

    if let Some(cwd) = &args.cwd {
        std::env::set_current_dir(cwd)?;
        logging::info(&format!("Changed working directory to: {}", cwd));
    }

    if args.trace {
        crate::env::set_var("JCODE_TRACE", "1");
    }

    if let Some(ref socket) = args.socket {
        server::set_socket_path(socket);
    }

    crate::cli::proctitle::set_initial_title(&args);

    Ok(args)
}

fn report_main_error(error: &anyhow::Error) {
    let error_str = format!("{:?}", error);
    logging::error(&error_str);

    if let Some(session_id) = terminal::get_current_session() {
        output::stderr_blank_line();
        output::stderr_info("\x1b[33mTo restore this session, run:\x1b[0m");
        output::stderr_info(format!("  jcode --resume {}", session_id));
        output::stderr_blank_line();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_provider_runtimes_register_and_instantiate() {
        register_external_provider_runtimes();
        for (key, expected_name) in [
            (crate::provider::external::GEMINI_RUNTIME, "gemini"),
            (crate::provider::external::CURSOR_RUNTIME, "cursor"),
            (
                crate::provider::external::ANTIGRAVITY_RUNTIME,
                "antigravity",
            ),
        ] {
            assert!(
                crate::provider::external::external_provider_registered(key),
                "{key} runtime should be registered"
            );
            let provider = crate::provider::external::instantiate_external_provider(key)
                .unwrap_or_else(|| panic!("{key} runtime factory should instantiate"));
            assert_eq!(provider.name(), expected_name);
            assert!(!provider.model().is_empty());
        }

        // Copilot's factory is fallible (requires a GitHub token), so only
        // assert registration; instantiation legitimately returns None when no
        // Copilot credentials exist on the machine running the tests.
        assert!(crate::provider::external::external_provider_registered(
            crate::provider::external::COPILOT_RUNTIME
        ));
    }
}
