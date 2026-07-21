use anyhow::Result;
use std::sync::Arc;

use crate::auth;
use crate::provider;
use crate::provider::Provider;
use crate::provider_catalog::{
    LoginProviderDescriptor, LoginProviderTarget, OpenAiCompatibleProfile,
    apply_openai_compatible_profile_env, force_apply_openai_compatible_profile_env,
    resolve_openai_compatible_profile,
};
use crate::tool;

use super::output;
use crate::external_auth::external_auth_blocked_message;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ProviderChoice {
    Jcode,
    Claude,
    #[value(alias = "claude-api", alias = "anthropic-key", alias = "claude-key")]
    AnthropicApi,
    Openai,
    #[value(
        alias = "openai-key",
        alias = "openai-apikey",
        alias = "openai-platform"
    )]
    OpenaiApi,
    Openrouter,
    #[value(alias = "aws-bedrock", alias = "aws_bedrock")]
    Bedrock,
    #[value(alias = "azure-openai", alias = "aoai")]
    Azure,
    #[value(alias = "opencode-zen", alias = "zen")]
    Opencode,
    #[value(alias = "opencodego")]
    OpencodeGo,
    #[value(alias = "z.ai", alias = "z-ai", alias = "zai-coding")]
    Zai,
    #[value(
        alias = "kimi-code",
        alias = "kimi-coding",
        alias = "kimi-coding-plan",
        alias = "kimi-for-coding",
        alias = "moonshot-coding"
    )]
    Kimi,
    #[value(alias = "302.ai")]
    Ai302,
    Baseten,
    Cortecs,
    #[value(alias = "cgc", alias = "comtegra-gpu-cloud")]
    Comtegra,
    Deepseek,
    #[value(alias = "fpt-ai", alias = "fptcloud", alias = "fpt-cloud")]
    Fpt,
    Firmware,
    #[value(alias = "hugging-face", alias = "hf")]
    HuggingFace,
    #[value(alias = "moonshot")]
    MoonshotAi,
    Nebius,
    Scaleway,
    Stackit,
    Groq,
    #[value(alias = "mistralai")]
    Mistral,
    #[value(alias = "pplx")]
    Perplexity,
    #[value(alias = "together", alias = "together-ai")]
    TogetherAi,
    #[value(alias = "deep-infra")]
    Deepinfra,
    #[value(alias = "fireworks-ai", alias = "fireworks.ai")]
    Fireworks,
    #[value(alias = "minimax-ai", alias = "minimaxi")]
    Minimax,
    #[value(alias = "x.ai", alias = "x-ai", alias = "grok")]
    Xai,
    #[value(alias = "nvidia", alias = "nim")]
    NvidiaNim,
    #[value(alias = "xiaomi", alias = "mimo", alias = "xiaomi-mimo-api")]
    XiaomiMimo,
    #[value(alias = "lm-studio")]
    Lmstudio,
    Ollama,
    Chutes,
    #[value(alias = "cerebrascode", alias = "cerberascode")]
    Cerebras,
    #[value(
        alias = "bailian",
        alias = "aliyun-bailian",
        alias = "coding-plan",
        alias = "alibaba-coding"
    )]
    AlibabaCodingPlan,
    #[value(alias = "compat", alias = "custom")]
    OpenaiCompatible,
    Cursor,
    Copilot,
    Gemini,
    #[value(
        alias = "gemini-key",
        alias = "gemini-apikey",
        alias = "google-ai-studio",
        alias = "ai-studio"
    )]
    GeminiApi,
    Antigravity,
    Google,
    Auto,
}

impl ProviderChoice {
    #[allow(deprecated)]
    pub fn as_arg_value(&self) -> &'static str {
        match self {
            Self::Jcode => "jcode",
            Self::Claude => "claude",
            Self::AnthropicApi => "anthropic-api",
            Self::Openai => "openai",
            Self::OpenaiApi => "openai-api",
            Self::Openrouter => "openrouter",
            Self::Bedrock => "bedrock",
            Self::Azure => "azure",
            Self::Opencode => "opencode",
            Self::OpencodeGo => "opencode-go",
            Self::Zai => "zai",
            Self::Kimi => "kimi",
            Self::Ai302 => "302ai",
            Self::Baseten => "baseten",
            Self::Cortecs => "cortecs",
            Self::Comtegra => "comtegra",
            Self::Deepseek => "deepseek",
            Self::Fpt => "fpt",
            Self::Firmware => "firmware",
            Self::HuggingFace => "huggingface",
            Self::MoonshotAi => "moonshotai",
            Self::Nebius => "nebius",
            Self::Scaleway => "scaleway",
            Self::Stackit => "stackit",
            Self::Groq => "groq",
            Self::Mistral => "mistral",
            Self::Perplexity => "perplexity",
            Self::TogetherAi => "togetherai",
            Self::Deepinfra => "deepinfra",
            Self::Fireworks => "fireworks",
            Self::Minimax => "minimax",
            Self::Xai => "xai",
            Self::NvidiaNim => "nvidia-nim",
            Self::XiaomiMimo => "xiaomi-mimo",
            Self::Lmstudio => "lmstudio",
            Self::Ollama => "ollama",
            Self::Chutes => "chutes",
            Self::Cerebras => "cerebras",
            Self::AlibabaCodingPlan => "alibaba-coding-plan",
            Self::OpenaiCompatible => "openai-compatible",
            Self::Cursor => "cursor",
            Self::Copilot => "copilot",
            Self::Gemini => "gemini",
            Self::GeminiApi => "gemini-api",
            Self::Antigravity => "antigravity",
            Self::Google => "google",
            Self::Auto => "auto",
        }
    }
}

#[allow(deprecated)]
const PROVIDER_CHOICE_LOGIN_PROVIDERS: &[(ProviderChoice, LoginProviderDescriptor)] = &[
    (
        ProviderChoice::Jcode,
        crate::provider_catalog::JCODE_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Claude,
        crate::provider_catalog::CLAUDE_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::AnthropicApi,
        crate::provider_catalog::ANTHROPIC_API_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Openai,
        crate::provider_catalog::OPENAI_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::OpenaiApi,
        crate::provider_catalog::OPENAI_API_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Openrouter,
        crate::provider_catalog::OPENROUTER_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Bedrock,
        crate::provider_catalog::BEDROCK_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Azure,
        crate::provider_catalog::AZURE_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Opencode,
        crate::provider_catalog::OPENCODE_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::OpencodeGo,
        crate::provider_catalog::OPENCODE_GO_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Zai,
        crate::provider_catalog::ZAI_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Kimi,
        crate::provider_catalog::KIMI_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Ai302,
        crate::provider_catalog::AI302_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Baseten,
        crate::provider_catalog::BASETEN_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Cortecs,
        crate::provider_catalog::CORTECS_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Comtegra,
        crate::provider_catalog::COMTEGRA_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Deepseek,
        crate::provider_catalog::DEEPSEEK_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Fpt,
        crate::provider_catalog::FPT_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Firmware,
        crate::provider_catalog::FIRMWARE_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::HuggingFace,
        crate::provider_catalog::HUGGING_FACE_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::MoonshotAi,
        crate::provider_catalog::MOONSHOT_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Nebius,
        crate::provider_catalog::NEBIUS_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Scaleway,
        crate::provider_catalog::SCALEWAY_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Stackit,
        crate::provider_catalog::STACKIT_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Groq,
        crate::provider_catalog::GROQ_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Mistral,
        crate::provider_catalog::MISTRAL_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Perplexity,
        crate::provider_catalog::PERPLEXITY_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::TogetherAi,
        crate::provider_catalog::TOGETHER_AI_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Deepinfra,
        crate::provider_catalog::DEEPINFRA_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Fireworks,
        crate::provider_catalog::FIREWORKS_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Minimax,
        crate::provider_catalog::MINIMAX_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Xai,
        crate::provider_catalog::XAI_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::NvidiaNim,
        crate::provider_catalog::NVIDIA_NIM_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::XiaomiMimo,
        crate::provider_catalog::XIAOMI_MIMO_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Lmstudio,
        crate::provider_catalog::LMSTUDIO_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Ollama,
        crate::provider_catalog::OLLAMA_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Chutes,
        crate::provider_catalog::CHUTES_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Cerebras,
        crate::provider_catalog::CEREBRAS_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::AlibabaCodingPlan,
        crate::provider_catalog::ALIBABA_CODING_PLAN_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::OpenaiCompatible,
        crate::provider_catalog::OPENAI_COMPAT_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Cursor,
        crate::provider_catalog::CURSOR_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Copilot,
        crate::provider_catalog::COPILOT_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Gemini,
        crate::provider_catalog::GEMINI_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::GeminiApi,
        crate::provider_catalog::GEMINI_API_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Antigravity,
        crate::provider_catalog::ANTIGRAVITY_LOGIN_PROVIDER,
    ),
    (
        ProviderChoice::Google,
        crate::provider_catalog::GOOGLE_LOGIN_PROVIDER,
    ),
];

pub fn login_provider_choice_mappings() -> &'static [(ProviderChoice, LoginProviderDescriptor)] {
    PROVIDER_CHOICE_LOGIN_PROVIDERS
}

pub fn profile_for_choice(choice: &ProviderChoice) -> Option<OpenAiCompatibleProfile> {
    match login_provider_for_choice(choice)?.target {
        LoginProviderTarget::OpenAiCompatible(profile) => Some(profile),
        _ => None,
    }
}

pub fn login_provider_for_choice(choice: &ProviderChoice) -> Option<LoginProviderDescriptor> {
    PROVIDER_CHOICE_LOGIN_PROVIDERS
        .iter()
        .find(|(candidate, _)| candidate == choice)
        .map(|(_, provider)| *provider)
}

pub fn choice_for_login_provider(provider: LoginProviderDescriptor) -> Option<ProviderChoice> {
    PROVIDER_CHOICE_LOGIN_PROVIDERS
        .iter()
        .find(|(_, candidate)| candidate.id == provider.id)
        .map(|(choice, _)| *choice)
}

struct AutoProviderAvailability {
    auth_status: auth::AuthStatus,
    has_claude: bool,
    has_openai: bool,
    has_copilot: bool,
    has_antigravity: bool,
    has_gemini: bool,
    has_cursor: bool,
    has_openrouter: bool,
}

impl AutoProviderAvailability {
    fn has_any_provider(&self) -> bool {
        self.has_claude
            || self.has_openai
            || self.has_copilot
            || self.has_antigravity
            || self.has_gemini
            || self.has_cursor
            || self.has_openrouter
    }
}

fn maybe_enable_config_default_provider_for_auto() -> Result<bool> {
    let cfg = crate::config::config();
    let Some(default_provider) = cfg
        .provider
        .default_provider
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };

    if let Some(profile) =
        crate::provider_catalog::resolve_openai_compatible_profile_selection(default_provider)
    {
        apply_openai_compatible_profile_env(Some(profile));
        return Ok(provider::openrouter::has_credentials());
    }

    if cfg.providers.contains_key(default_provider) {
        crate::provider_catalog::apply_named_provider_profile_env_from_config(
            default_provider,
            cfg,
        )?;
        return Ok(provider::openrouter::has_credentials());
    }

    Ok(false)
}

async fn detect_auto_provider_flags() -> AutoProviderAvailability {
    let auth_status = auth::AuthStatus::check_fast();
    AutoProviderAvailability {
        has_claude: auth_status.anthropic.has_oauth || auth_status.anthropic.has_api_key,
        has_openai: auth_status.openai_has_oauth || auth_status.openai_has_api_key,
        has_copilot: auth_status.copilot_has_api_token,
        has_antigravity: auth::antigravity::load_tokens().is_ok(),
        has_gemini: auth_status.gemini == auth::AuthState::Available,
        has_cursor: auth_status.cursor == auth::AuthState::Available,
        has_openrouter: auth_status.openrouter == auth::AuthState::Available,
        auth_status,
    }
}

fn provider_label_for_api_key_env(env_key: &str) -> String {
    if env_key == "OPENROUTER_API_KEY" {
        return "OpenRouter".to_string();
    }

    crate::provider_catalog::openai_compatible_profiles()
        .iter()
        .find_map(|profile| {
            let resolved = resolve_openai_compatible_profile(*profile);
            (resolved.api_key_env == env_key).then_some(resolved.display_name)
        })
        .unwrap_or_else(|| env_key.to_string())
}

fn ensure_external_api_key_auth_allowed_for_explicit_choice(env_key: &str) -> Result<()> {
    if direct_api_key_configured_for_env(env_key) {
        return Ok(());
    }
    let Some(source) = auth::external::preferred_unconsented_api_key_source_for_env(env_key) else {
        return Ok(());
    };
    let path = source.path()?;
    let provider_name = provider_label_for_api_key_env(env_key);
    anyhow::bail!(external_auth_blocked_message(
        &provider_name,
        source.display_name(),
        &path,
    ))
}

fn direct_api_key_configured_for_env(env_key: &str) -> bool {
    let env_key = env_key.trim();
    if env_key.is_empty() {
        return false;
    }
    if std::env::var(env_key)
        .ok()
        .map(|key| !key.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    crate::provider_catalog::openai_compatible_profiles()
        .iter()
        .filter_map(|profile| {
            let resolved = resolve_openai_compatible_profile(*profile);
            (resolved.api_key_env == env_key).then_some(resolved.env_file)
        })
        .any(|env_file| direct_env_file_contains_key(env_key, &env_file))
}

fn direct_env_file_contains_key(env_key: &str, env_file: &str) -> bool {
    if !crate::provider_catalog::is_safe_env_file_name(env_file) {
        return false;
    }
    let Some(config_dir) = crate::storage::app_config_dir().ok() else {
        return false;
    };
    let path = config_dir.join(env_file);
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let prefix = format!("{}=", env_key);
    content.lines().any(|line| {
        line.strip_prefix(&prefix)
            .map(|key| !key.trim().trim_matches('"').trim_matches('\'').is_empty())
            .unwrap_or(false)
    })
}

fn handle_untrusted_external_auth_source(
    provider_name: &str,
    source: Option<auth::external::ExternalAuthSource>,
    auto: bool,
) -> Result<bool> {
    let Some(source) = source else {
        return Ok(false);
    };
    let path = source.path()?;
    let message = external_auth_blocked_message(provider_name, source.display_name(), &path);
    if auto {
        crate::logging::warn(&message);
        Ok(false)
    } else {
        anyhow::bail!(message)
    }
}

fn ensure_openai_auth_allowed_for_explicit_choice() -> Result<()> {
    if auth::codex::load_credentials().is_ok() {
        return Ok(());
    }

    if handle_untrusted_external_auth_source(
        "OpenAI/Codex",
        auth::external::preferred_unconsented_openai_oauth_source(),
        false,
    )? {
        return Ok(());
    }

    if !auth::codex::has_unconsented_legacy_credentials() {
        return Ok(());
    }

    let path = auth::codex::legacy_auth_file_path()?;

    anyhow::bail!(external_auth_blocked_message(
        "OpenAI/Codex",
        "Codex",
        &path,
    ))
}

fn ensure_claude_auth_allowed_for_explicit_choice() -> Result<()> {
    if auth::claude::load_credentials().is_ok() {
        return Ok(());
    }

    if handle_untrusted_external_auth_source(
        "Claude",
        auth::external::preferred_unconsented_anthropic_oauth_source(),
        false,
    )? {
        return Ok(());
    }

    let Some(source) = auth::claude::has_unconsented_external_auth() else {
        return Ok(());
    };
    let path = source.path()?;
    anyhow::bail!(external_auth_blocked_message(
        "Claude",
        source.display_name(),
        &path,
    ))
}

fn ensure_gemini_auth_allowed_for_explicit_choice() -> Result<()> {
    // An official Gemini Developer API key (GEMINI_API_KEY) authenticates
    // directly against generativelanguage.googleapis.com and needs no OAuth
    // consent flow, so allow it without further prompting.
    if auth::gemini::has_api_key() {
        return Ok(());
    }
    if auth::gemini::load_tokens().is_ok() {
        return Ok(());
    }

    if handle_untrusted_external_auth_source(
        "Gemini",
        auth::external::preferred_unconsented_gemini_oauth_source(),
        false,
    )? {
        return Ok(());
    }

    if !auth::gemini::has_unconsented_cli_auth() {
        return Ok(());
    }
    let path = auth::gemini::gemini_cli_oauth_path()?;
    anyhow::bail!(external_auth_blocked_message("Gemini", "Gemini CLI", &path,))
}

fn ensure_antigravity_auth_allowed_for_explicit_choice() -> Result<()> {
    if auth::antigravity::load_tokens().is_ok() {
        return Ok(());
    }

    if handle_untrusted_external_auth_source(
        "Antigravity",
        auth::external::preferred_unconsented_antigravity_oauth_source(),
        false,
    )? {
        return Ok(());
    }

    Ok(())
}

fn ensure_copilot_auth_allowed_for_explicit_choice() -> Result<()> {
    if auth::copilot::load_github_token().is_ok() {
        return Ok(());
    }
    let Some(source) = auth::copilot::has_unconsented_external_auth() else {
        return Ok(());
    };
    let path = source.path();
    anyhow::bail!(external_auth_blocked_message(
        "GitHub Copilot",
        source.display_name(),
        &path,
    ))
}

fn ensure_cursor_auth_allowed_for_explicit_choice() -> Result<()> {
    if auth::cursor::has_cursor_native_auth() || auth::cursor::has_cursor_api_key() {
        return Ok(());
    }
    let Some(source) = auth::cursor::has_unconsented_external_auth() else {
        return Ok(());
    };
    let path = source.path()?;
    anyhow::bail!(external_auth_blocked_message(
        "Cursor",
        source.display_name(),
        &path,
    ))
}

pub fn lock_model_provider(provider_key: &str) {
    crate::provider::activation::lock_runtime_provider_key(provider_key);
}

pub fn unlock_model_provider() {
    crate::provider::activation::unlock_runtime_provider();
}

/// A CLI provider choice for a dual-auth backend is also a credential choice.
/// Pin it through the provider's credential-mode API
/// so `--provider anthropic-api` cannot remain in Auto mode and prefer a stored
/// Claude OAuth credential over `ANTHROPIC_API_KEY` (and likewise for OpenAI).
fn explicit_credential_mode(choice: &ProviderChoice) -> Option<provider::CredentialMode> {
    match choice {
        ProviderChoice::AnthropicApi | ProviderChoice::OpenaiApi => {
            Some(provider::CredentialMode::ApiKey)
        }
        _ => None,
    }
}

fn disable_subscription_runtime_mode() {
    crate::subscription_catalog::clear_runtime_env();
}

fn disable_subscription_runtime_mode_preserving_active_provider_profile() {
    if std::env::var_os("JCODE_PROVIDER_PROFILE_ACTIVE").is_some()
        || std::env::var_os("JCODE_NAMED_PROVIDER_PROFILE").is_some()
    {
        crate::env::remove_var(crate::subscription_catalog::JCODE_SUBSCRIPTION_ACTIVE_ENV);
    } else {
        disable_subscription_runtime_mode();
    }
}

pub fn apply_login_provider_profile_env(provider: LoginProviderDescriptor) {
    match provider.target {
        LoginProviderTarget::OpenAiCompatible(profile) => {
            force_apply_openai_compatible_profile_env(Some(profile));
            // Bootstrap login still spawns the daemon with `--provider auto`. Mark the
            // just-selected compatible provider as active so the child process does
            // not clear these inherited runtime vars before credential detection.
            crate::env::set_var("JCODE_PROVIDER_PROFILE_ACTIVE", "1");
        }
        LoginProviderTarget::AutoImport | LoginProviderTarget::Google => {}
        _ => {
            // A later non-compatible login selection must not inherit a stale
            // compatible-provider profile from an earlier bootstrap/configuration path.
            force_apply_openai_compatible_profile_env(None);
        }
    }
}

fn resolved_profile_default_model(profile: OpenAiCompatibleProfile) -> Option<String> {
    resolve_openai_compatible_profile(profile).default_model
}

pub async fn init_provider(
    choice: &ProviderChoice,
    model: Option<&str>,
) -> Result<Arc<dyn provider::Provider>> {
    init_provider_with_options(choice, model, true, true).await
}

pub async fn init_provider_quiet(
    choice: &ProviderChoice,
    model: Option<&str>,
) -> Result<Arc<dyn provider::Provider>> {
    init_provider_with_options(choice, model, false, true).await
}

pub async fn init_provider_for_validation(
    choice: &ProviderChoice,
    model: Option<&str>,
) -> Result<Arc<dyn provider::Provider>> {
    init_provider_with_options(choice, model, false, false).await
}

#[allow(deprecated)]
async fn init_provider_with_options(
    choice: &ProviderChoice,
    model: Option<&str>,
    show_init_messages: bool,
    _allow_login_bootstrap: bool,
) -> Result<Arc<dyn provider::Provider>> {
    // Provider construction resolves concrete runtimes through the base
    // crate's external-runtime registry (composition-root pattern). The
    // binary's normal path registers them in `startup::run()`, but this
    // function is also entered directly by validation/configuration/test flows that
    // never run startup. Registration is idempotent, so do it here too;
    // otherwise Auto-init silently loses registry-backed runtimes (e.g. the
    // OpenRouter/OpenAI-compatible factory) and their model-picker routes.
    super::startup::register_external_provider_runtimes();

    if let Ok(profile_name) = std::env::var("JCODE_PROVIDER_PROFILE_NAME")
        && !profile_name.trim().is_empty()
    {
        crate::provider_catalog::apply_named_provider_profile_env(profile_name.trim())?;
        crate::env::set_var("JCODE_PROVIDER_PROFILE_ACTIVE", "1");
    }

    if std::env::var_os("JCODE_PROVIDER_PROFILE_ACTIVE").is_none()
        && std::env::var_os("JCODE_NAMED_PROVIDER_PROFILE").is_none()
    {
        if let Some(profile) = profile_for_choice(choice) {
            apply_openai_compatible_profile_env(Some(profile));
        } else {
            apply_openai_compatible_profile_env(None);
        }
    }

    let init_notice = |message: &str| {
        if show_init_messages {
            output::stderr_info(message);
        }
    };

    let provider: Arc<dyn provider::Provider> = match choice {
        ProviderChoice::Jcode => {
            init_notice("Using Jcode subscription provider (provider locked)");
            Arc::new(provider::jcode::JcodeProvider::new())
        }
        ProviderChoice::Claude => {
            disable_subscription_runtime_mode();
            ensure_claude_auth_allowed_for_explicit_choice()?;
            init_notice("Using Claude (provider locked)");
            lock_model_provider("claude");
            Arc::new(provider::MultiProvider::with_preference_fast(false))
        }
        ProviderChoice::AnthropicApi => {
            disable_subscription_runtime_mode();
            ensure_external_api_key_auth_allowed_for_explicit_choice("ANTHROPIC_API_KEY")?;
            init_notice("Using Anthropic API key provider (provider locked)");
            lock_model_provider("claude");
            Arc::new(provider::MultiProvider::with_preference_fast(false))
        }
        ProviderChoice::Openai => {
            disable_subscription_runtime_mode();
            ensure_openai_auth_allowed_for_explicit_choice()?;
            init_notice("Using OpenAI (provider locked)");
            lock_model_provider("openai");
            Arc::new(provider::MultiProvider::with_preference_fast(true))
        }
        ProviderChoice::OpenaiApi => {
            disable_subscription_runtime_mode();
            ensure_external_api_key_auth_allowed_for_explicit_choice("OPENAI_API_KEY")?;
            init_notice("Using OpenAI API key provider (provider locked)");
            lock_model_provider("openai");
            Arc::new(provider::MultiProvider::with_preference_fast(true))
        }
        ProviderChoice::Cursor => {
            disable_subscription_runtime_mode();
            ensure_cursor_auth_allowed_for_explicit_choice()?;
            init_notice("Using Cursor native HTTPS provider (experimental)");
            unlock_model_provider();
            crate::env::set_var("JCODE_ACTIVE_PROVIDER", "cursor");
            Arc::new(jcode_provider_cursor_runtime::CursorCliProvider::new())
        }
        ProviderChoice::Copilot => {
            disable_subscription_runtime_mode();
            ensure_copilot_auth_allowed_for_explicit_choice()?;
            init_notice("Using GitHub Copilot API provider (provider locked)");
            lock_model_provider("copilot");
            Arc::new(provider::MultiProvider::new_fast())
        }
        ProviderChoice::Gemini => {
            disable_subscription_runtime_mode();
            ensure_gemini_auth_allowed_for_explicit_choice()?;
            if auth::gemini::has_api_key() {
                init_notice(
                    "Using Gemini provider (official Gemini Developer API key, generativelanguage.googleapis.com)",
                );
            } else {
                init_notice("Using Gemini provider (native Google Code Assist OAuth)");
            }
            unlock_model_provider();
            crate::env::set_var("JCODE_ACTIVE_PROVIDER", "gemini");
            Arc::new(jcode_provider_gemini_runtime::GeminiProvider::new())
        }
        ProviderChoice::Openrouter => {
            disable_subscription_runtime_mode();
            ensure_external_api_key_auth_allowed_for_explicit_choice("OPENROUTER_API_KEY")?;
            init_notice("Using OpenRouter provider (provider locked)");
            lock_model_provider("openrouter");
            Arc::new(provider::MultiProvider::new_fast())
        }
        ProviderChoice::Bedrock => {
            disable_subscription_runtime_mode();
            init_notice("Using AWS Bedrock provider (provider locked)");
            lock_model_provider("bedrock");
            Arc::new(provider::MultiProvider::new_fast())
        }
        ProviderChoice::Azure => {
            disable_subscription_runtime_mode();
            let model = crate::provider::activation::apply_azure_openai_runtime()?;
            init_notice("Using Azure OpenAI provider (provider locked)");
            let multi = provider::MultiProvider::new_fast();
            if let Some(model) = model {
                let _ = multi.set_model(&model);
            }
            Arc::new(multi)
        }
        ProviderChoice::Opencode
        | ProviderChoice::OpencodeGo
        | ProviderChoice::Zai
        | ProviderChoice::Ai302
        | ProviderChoice::Baseten
        | ProviderChoice::Cortecs
        | ProviderChoice::Comtegra
        | ProviderChoice::Deepseek
        | ProviderChoice::Fpt
        | ProviderChoice::Firmware
        | ProviderChoice::HuggingFace
        | ProviderChoice::MoonshotAi
        | ProviderChoice::Kimi
        | ProviderChoice::Nebius
        | ProviderChoice::Scaleway
        | ProviderChoice::Stackit
        | ProviderChoice::Groq
        | ProviderChoice::Mistral
        | ProviderChoice::Perplexity
        | ProviderChoice::TogetherAi
        | ProviderChoice::Deepinfra
        | ProviderChoice::Fireworks
        | ProviderChoice::Minimax
        | ProviderChoice::Xai
        | ProviderChoice::NvidiaNim
        | ProviderChoice::XiaomiMimo
        | ProviderChoice::Lmstudio
        | ProviderChoice::Ollama
        | ProviderChoice::Chutes
        | ProviderChoice::Cerebras
        | ProviderChoice::AlibabaCodingPlan
        | ProviderChoice::GeminiApi
        | ProviderChoice::OpenaiCompatible => {
            disable_subscription_runtime_mode();
            let profile = profile_for_choice(choice)
                .ok_or_else(|| anyhow::anyhow!("missing provider profile for choice"))?;
            if std::env::var_os("JCODE_NAMED_PROVIDER_PROFILE").is_none() {
                // An explicit `--provider <compatible>` selection should win over
                // any stale active-profile marker inherited from a previous
                // bootstrap/configuration flow. Named provider profiles still take
                // precedence when explicitly configured.
                force_apply_openai_compatible_profile_env(Some(profile));
            }
            let mut runtime_model_hint = None;
            let display_name = if let Ok(named) = std::env::var("JCODE_NAMED_PROVIDER_PROFILE") {
                if let Some(profile) = crate::config::config().providers.get(&named) {
                    runtime_model_hint = profile.default_model.clone();
                }
                named
            } else {
                let resolved = resolve_openai_compatible_profile(profile);
                if resolved.requires_api_key {
                    ensure_external_api_key_auth_allowed_for_explicit_choice(
                        &resolved.api_key_env,
                    )?;
                }
                runtime_model_hint = resolved.default_model.clone();
                resolved.display_name
            };
            init_notice(&format!(
                "Using {} via OpenAI-compatible API (provider locked)",
                display_name
            ));
            crate::provider::activation::apply_openai_compatible_runtime(runtime_model_hint)?;
            if std::env::var_os("JCODE_NAMED_PROVIDER_PROFILE").is_some() {
                let profile_name = std::env::var("JCODE_NAMED_PROVIDER_PROFILE")?;
                let cfg = crate::config::config();
                let profile = cfg.providers.get(&profile_name).ok_or_else(|| {
                    anyhow::anyhow!("Unknown provider profile '{}'", profile_name)
                })?;
                Arc::new(
                    jcode_provider_openrouter_runtime::OpenRouterProvider::new_named_openai_compatible(
                        &profile_name,
                        profile,
                    )?,
                )
            } else {
                Arc::new(jcode_provider_openrouter_runtime::OpenRouterProvider::new()?)
            }
        }
        ProviderChoice::Antigravity => {
            disable_subscription_runtime_mode();
            ensure_antigravity_auth_allowed_for_explicit_choice()?;
            init_notice("Using Antigravity provider (experimental)");
            unlock_model_provider();
            crate::env::set_var("JCODE_ACTIVE_PROVIDER", "antigravity");
            Arc::new(jcode_provider_antigravity_runtime::AntigravityProvider::new())
        }
        ProviderChoice::Google => {
            disable_subscription_runtime_mode();
            init_notice(
                "Note: Google/Gmail is not a model provider. Using auto-detect for model provider.",
            );
            init_notice(
                "Gmail credentials must be mounted under JCODE_HOME before startup; the gmail tool is enabled by default in the full tool profile.",
            );
            unlock_model_provider();
            Arc::new(provider::MultiProvider::new_fast())
        }
        ProviderChoice::Auto => {
            disable_subscription_runtime_mode_preserving_active_provider_profile();
            unlock_model_provider();
            let _ = maybe_enable_config_default_provider_for_auto()?;
            let availability = detect_auto_provider_flags().await;

            if availability.has_any_provider() {
                let multi = provider::MultiProvider::from_auth_status(availability.auth_status);
                init_notice(&format!(
                    "Using {} (use /model to switch models)",
                    multi.name()
                ));
                crate::env::set_var("JCODE_ACTIVE_PROVIDER", multi.name().to_lowercase());
                Arc::new(multi)
            } else {
                anyhow::bail!(
                    "No credentials configured. Mount writable subscription credentials under JCODE_HOME, or provide an API key environment variable/provider profile."
                );
            }
        }
    };

    if let Some(mode) = explicit_credential_mode(choice) {
        provider.set_credential_mode(mode).map_err(|err| {
            anyhow::anyhow!(
                "Failed to select the credential route for --provider {}: {err}",
                choice.as_arg_value()
            )
        })?;
    }

    if std::env::var_os("JCODE_PROVIDER_PROFILE_ACTIVE").is_none()
        && std::env::var_os("JCODE_NAMED_PROVIDER_PROFILE").is_none()
        && model.is_none()
        && let Some(profile) = profile_for_choice(choice)
        && let Some(default_model) = resolved_profile_default_model(profile)
        && provider.set_model(&default_model).is_ok()
    {
        let resolved = resolve_openai_compatible_profile(profile);
        init_notice(&format!(
            "Using default model for {}: {}",
            resolved.display_name, default_model
        ));
    }

    if let Some(model_name) = model {
        if let Err(e) = provider.set_model(model_name) {
            init_notice(&format!(
                "Warning: failed to set model '{}': {}",
                model_name, e
            ));
        } else {
            init_notice(&format!("Using model: {}", model_name));
        }
    }

    Ok(provider)
}

pub async fn init_provider_and_registry(
    choice: &ProviderChoice,
    model: Option<&str>,
) -> Result<(Arc<dyn provider::Provider>, tool::Registry)> {
    let provider = init_provider(choice, model).await?;
    let registry = tool::Registry::new(provider.clone()).await;
    Ok((provider, registry))
}

pub async fn init_provider_and_registry_for_validation(
    choice: &ProviderChoice,
    model: Option<&str>,
) -> Result<(Arc<dyn provider::Provider>, tool::Registry)> {
    let provider = init_provider_for_validation(choice, model).await?;
    let registry = tool::Registry::new(provider.clone()).await;
    Ok((provider, registry))
}

#[cfg(test)]
#[path = "provider_init_tests.rs"]
mod tests;
