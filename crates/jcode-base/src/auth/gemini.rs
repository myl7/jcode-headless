use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const GEMINI_CLI_AUTH_SOURCE_ID: &str = "gemini_cli_oauth_creds";
// OAuth credentials from Google's official Gemini CLI (@google/gemini-cli).
// These are for a "Desktop app" OAuth type where the client secret is safe to embed.
// See: https://developers.google.com/identity/protocols/oauth2#installed
// gitleaks:allow - public desktop OAuth credentials, safe to embed
const GEMINI_CLIENT_ID: &str =
    "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com"; // gitleaks:allow
const GEMINI_CLIENT_SECRET: &str = "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"; // gitleaks:allow
// Env vars can override the hardcoded credentials if needed
const GEMINI_CLIENT_ID_ENV: &str = "GEMINI_CLIENT_ID";
const GEMINI_CLIENT_SECRET_ENV: &str = "GEMINI_CLIENT_SECRET";

/// Environment variable names that hold an official Gemini Developer API key
/// (Google AI Studio). Checked in order; the first non-empty value wins.
pub const GEMINI_API_KEY_ENV_VARS: &[&str] = &["GEMINI_API_KEY", "GOOGLE_API_KEY"];
/// Config file that may persist a saved Gemini Developer API key.
pub const GEMINI_API_KEY_ENV_FILE: &str = "gemini.env";

/// Resolve an official Gemini Developer API key from the environment or the
/// saved `gemini.env` config file.
///
/// Unlike the OAuth Code Assist path (which talks to cloudcode-pa with a free
/// quota tier), an API key authenticates directly against
/// `generativelanguage.googleapis.com` and uses the key's own quota. Preferring
/// the env var keeps it consistent with every other API-key provider in jcode.
pub fn api_key() -> Option<String> {
    for env_key in GEMINI_API_KEY_ENV_VARS {
        if let Some(key) = crate::provider_catalog::load_api_key_from_env_or_config(
            env_key,
            GEMINI_API_KEY_ENV_FILE,
        ) {
            return Some(key);
        }
    }
    None
}

/// True when an official Gemini Developer API key is configured.
pub fn has_api_key() -> bool {
    api_key().is_some()
}

/// Persist a Gemini Developer API key to the `gemini.env` config file under the
/// canonical `GEMINI_API_KEY` name.
pub fn save_api_key(key: &str) -> Result<()> {
    let key = key.trim();
    if key.is_empty() {
        anyhow::bail!("Gemini API key cannot be empty");
    }
    crate::provider_catalog::save_env_value_to_env_file(
        GEMINI_API_KEY_ENV_VARS[0],
        GEMINI_API_KEY_ENV_FILE,
        Some(key),
    )?;
    super::AuthStatus::invalidate_cache();
    Ok(())
}

fn gemini_client_id() -> String {
    std::env::var(GEMINI_CLIENT_ID_ENV).unwrap_or_else(|_| GEMINI_CLIENT_ID.to_string())
}

fn gemini_client_secret() -> String {
    std::env::var(GEMINI_CLIENT_SECRET_ENV).unwrap_or_else(|_| GEMINI_CLIENT_SECRET.to_string())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeminiCliCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl GeminiCliCommand {
    pub fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

impl GeminiTokens {
    pub fn is_expired(&self) -> bool {
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.expires_at <= now_ms + 60_000
    }
}

#[derive(Debug, Deserialize)]
struct GeminiCliOAuthCredentials {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expiry_date: Option<i64>,
    expires_at: Option<i64>,
    expires_in: Option<i64>,
}

/// Resolve the Gemini CLI command from the environment or a sensible default.
///
/// Preference order:
/// 1. `JCODE_GEMINI_CLI_PATH` (supports a full command like `npx @google/gemini-cli`)
/// 2. `gemini` on PATH
/// 3. `npx @google/gemini-cli`
pub fn gemini_cli_command() -> GeminiCliCommand {
    resolve_gemini_cli_command_with(
        std::env::var("JCODE_GEMINI_CLI_PATH").ok().as_deref(),
        super::command_exists,
    )
}

/// Resolve just the executable portion for legacy callers.
pub fn gemini_cli_path() -> String {
    gemini_cli_command().program
}

/// Check if a usable Gemini CLI command is available.
pub fn has_gemini_cli() -> bool {
    let resolved = gemini_cli_command();
    super::command_exists(&resolved.program)
}

/// Check if native Gemini OAuth tokens are available (including imported Gemini CLI tokens).
pub fn has_cached_auth() -> bool {
    load_tokens().is_ok()
}

pub fn tokens_path() -> Result<std::path::PathBuf> {
    Ok(crate::storage::jcode_dir()?.join("gemini_oauth.json"))
}

pub fn gemini_cli_oauth_path() -> Result<std::path::PathBuf> {
    crate::storage::user_home_path(".gemini/oauth_creds.json")
}

pub fn gemini_cli_auth_source_exists() -> bool {
    gemini_cli_oauth_path()
        .map(|path| path.exists())
        .unwrap_or(false)
}

pub fn has_unconsented_cli_auth() -> bool {
    gemini_cli_oauth_path()
        .ok()
        .filter(|path| path.exists())
        .map(|path| {
            !crate::config::Config::external_auth_source_allowed_for_path(
                GEMINI_CLI_AUTH_SOURCE_ID,
                &path,
            )
        })
        .unwrap_or(false)
}

pub fn trust_cli_auth_for_future_use() -> Result<()> {
    crate::config::Config::allow_external_auth_source_for_path(
        GEMINI_CLI_AUTH_SOURCE_ID,
        &gemini_cli_oauth_path()?,
    )?;
    super::AuthStatus::invalidate_cache();
    Ok(())
}

pub fn load_tokens() -> Result<GeminiTokens> {
    let native_path = tokens_path()?;
    if native_path.exists() {
        crate::storage::harden_secret_file_permissions(&native_path);
        return crate::storage::read_json(&native_path)
            .with_context(|| format!("Failed to read {}", native_path.display()));
    }

    let cli_path = gemini_cli_oauth_path()?;
    if cli_path.exists()
        && crate::config::Config::external_auth_source_allowed_for_path(
            GEMINI_CLI_AUTH_SOURCE_ID,
            &cli_path,
        )
    {
        let safe_path = crate::storage::validate_external_auth_file(&cli_path)?;
        let raw = std::fs::read_to_string(&safe_path)
            .with_context(|| format!("Failed to read {}", cli_path.display()))?;
        let imported: GeminiCliOAuthCredentials = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse {}", cli_path.display()))?;
        let refresh_token = imported
            .refresh_token
            .filter(|value| !value.trim().is_empty());
        let access_token = imported
            .access_token
            .filter(|value| !value.trim().is_empty());
        if let (Some(refresh_token), Some(access_token)) = (refresh_token, access_token) {
            let expires_at = imported
                .expiry_date
                .or(imported.expires_at)
                .or_else(|| {
                    imported.expires_in.map(|expires_in| {
                        chrono::Utc::now().timestamp_millis() + (expires_in * 1000)
                    })
                })
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() - 1);
            return Ok(GeminiTokens {
                access_token,
                refresh_token,
                expires_at,
                email: None,
            });
        }
    }

    if let Some(tokens) = crate::auth::external::load_gemini_oauth_tokens() {
        return Ok(GeminiTokens {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at: tokens.expires_at,
            email: None,
        });
    }

    anyhow::bail!("No Gemini OAuth tokens found in JCODE_HOME.")
}

pub fn save_tokens(tokens: &GeminiTokens) -> Result<()> {
    let path = tokens_path()?;
    crate::storage::write_json_secret(&path, tokens)?;
    Ok(())
}

pub fn clear_tokens() -> Result<()> {
    let path = tokens_path()?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub async fn load_or_refresh_tokens() -> Result<GeminiTokens> {
    let tokens = load_tokens()?;
    if tokens.is_expired() {
        refresh_tokens(&tokens).await
    } else {
        Ok(tokens)
    }
}

/// Refresh Gemini OAuth tokens, serialized via the refresh coordinator so
/// concurrent callers do not race the token endpoint and the stored file.
pub async fn refresh_tokens(tokens: &GeminiTokens) -> Result<GeminiTokens> {
    crate::auth::refresh_coordinator::single_flight(
        "gemini".to_string(),
        || load_tokens().ok(),
        |stored: &GeminiTokens| !stored.is_expired(),
        {
            let observed = tokens.clone();
            move |stored: Option<GeminiTokens>| async move {
                let source = stored.unwrap_or(observed);
                refresh_tokens_uncoordinated(&source).await
            }
        },
    )
    .await
}

async fn refresh_tokens_uncoordinated(tokens: &GeminiTokens) -> Result<GeminiTokens> {
    let result: Result<GeminiTokens> = async {
        let refreshed = crate::auth::google_oauth::refresh_access_token(
            "Gemini",
            &gemini_client_id(),
            &gemini_client_secret(),
            &tokens.refresh_token,
            None,
        )
        .await?;
        let refreshed = GeminiTokens {
            access_token: refreshed.access_token,
            refresh_token: refreshed.refresh_token,
            expires_at: refreshed.expires_at_ms,
            email: tokens.email.clone(),
        };
        save_tokens(&refreshed)?;
        Ok(refreshed)
    }
    .await;

    match &result {
        Ok(_) => {
            let _ = crate::auth::refresh_state::record_success("gemini");
        }
        Err(err) => {
            let _ = crate::auth::refresh_state::record_failure("gemini", err.to_string());
        }
    }

    result
}

fn resolve_gemini_cli_command_with<F>(env_spec: Option<&str>, command_exists: F) -> GeminiCliCommand
where
    F: Fn(&str) -> bool,
{
    if let Some(spec) = env_spec.and_then(parse_command_spec) {
        return GeminiCliCommand {
            program: spec[0].clone(),
            args: spec[1..].to_vec(),
        };
    }

    if command_exists("gemini") {
        return GeminiCliCommand {
            program: "gemini".to_string(),
            args: Vec::new(),
        };
    }

    if command_exists("npx") {
        return GeminiCliCommand {
            program: "npx".to_string(),
            args: vec!["@google/gemini-cli".to_string()],
        };
    }

    GeminiCliCommand {
        program: "gemini".to_string(),
        args: Vec::new(),
    }
}

fn parse_command_spec(raw: &str) -> Option<Vec<String>> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;

    for ch in raw.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        match ch {
            '\\' if !in_single => escape = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if escape {
        current.push('\\');
    }

    if !current.is_empty() {
        parts.push(current);
    }

    if parts.is_empty() { None } else { Some(parts) }
}

#[cfg(test)]
#[path = "gemini_tests.rs"]
mod tests;
