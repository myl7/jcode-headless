use crate::auth::claude as claude_auth;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Claude Code OAuth configuration
pub mod claude {
    pub const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
    pub const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
    pub const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
    pub const REFRESH_SCOPES: &str =
        "user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";
}

const CLAUDE_TOKEN_TIMEOUT_SECS: u64 = 15;

/// OpenAI Codex OAuth configuration
pub mod openai {
    pub const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
    pub const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
}

fn parse_oauth_scopes(scope: Option<&str>) -> Vec<String> {
    scope
        .unwrap_or_default()
        .split_whitespace()
        .filter(|scope| !scope.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn claude_scopes_have_inference(scopes: &[String]) -> bool {
    scopes.iter().any(|scope| {
        matches!(
            scope.as_str(),
            "user:inference"
                | "user:ccr_inference"
                | "user:voice"
                | "org:service_key_inference"
                | "workspace:developer"
                | "workspace:inference"
        )
    })
}

fn ensure_claude_inference_scope(scopes: &[String], action: &str) -> Result<()> {
    if scopes.is_empty() || claude_scopes_have_inference(scopes) {
        return Ok(());
    }

    anyhow::bail!(
        "Claude OAuth {} returned a token without the required user:inference scope (scopes: {}). Replace the mounted credential with a fresh Claude subscription credential.",
        action,
        scopes.join(" ")
    )
}

/// Save Claude tokens to jcode's credentials file (active account or first numbered account).
pub fn save_claude_tokens(tokens: &OAuthTokens) -> Result<()> {
    let label = claude_auth::login_target_label(None)?;
    save_claude_tokens_for_account(tokens, &label)
}

/// Save Claude tokens for a specific stored account label.
pub fn save_claude_tokens_for_account(tokens: &OAuthTokens, label: &str) -> Result<()> {
    let existing = claude_auth::list_accounts()?
        .into_iter()
        .find(|account| account.label == label);
    let scopes = if tokens.scopes.is_empty() {
        existing
            .as_ref()
            .map(|account| account.scopes.clone())
            .unwrap_or_default()
    } else {
        tokens.scopes.clone()
    };
    let account = claude_auth::AnthropicAccount {
        label: label.to_string(),
        access: tokens.access_token.clone(),
        refresh: tokens.refresh_token.clone(),
        expires: tokens.expires_at,
        email: existing.as_ref().and_then(|account| account.email.clone()),
        subscription_type: existing.and_then(|account| account.subscription_type),
        scopes,
    };
    claude_auth::upsert_account(account)?;
    Ok(())
}

#[derive(Deserialize)]
struct ClaudeProfileResponse {
    #[serde(default)]
    account: ClaudeProfileAccount,
}

#[derive(Deserialize, Default)]
struct ClaudeProfileAccount {
    email: Option<String>,
}

async fn fetch_claude_profile_email_at_url(
    access_token: &str,
    profile_url: &str,
) -> Result<Option<String>> {
    let client = crate::provider::shared_http_client();
    let resp = client
        .get(profile_url)
        .header("Accept", "application/json")
        .header("User-Agent", "claude-cli/1.0.0")
        .header("anthropic-beta", "oauth-2025-04-20,claude-code-20250219")
        .bearer_auth(access_token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = crate::util::http_error_body(resp, "HTTP error").await;
        anyhow::bail!("Profile fetch failed ({}): {}", status, body);
    }

    let profile: ClaudeProfileResponse = resp.json().await?;
    Ok(profile.account.email)
}

/// Fetch profile metadata for a Claude account and persist any discovered fields.
pub async fn update_claude_account_profile(
    label: &str,
    access_token: &str,
) -> Result<Option<String>> {
    let email = fetch_claude_profile_email_at_url(access_token, claude::PROFILE_URL).await?;
    claude_auth::update_account_profile(label, email.clone())?;
    Ok(email)
}

/// Load Claude tokens from jcode's credentials file (active account).
pub fn load_claude_tokens() -> Result<OAuthTokens> {
    if let Ok(creds) = claude_auth::load_credentials() {
        return Ok(OAuthTokens {
            access_token: creds.access_token,
            refresh_token: creds.refresh_token,
            expires_at: creds.expires_at,
            id_token: None,
            scopes: creds.scopes,
        });
    }

    anyhow::bail!("No Claude subscription credentials found in the mounted JCODE_HOME volume.");
}

/// Load Claude tokens for a specific stored account label.
pub fn load_claude_tokens_for_account(label: &str) -> Result<OAuthTokens> {
    let creds = claude_auth::load_credentials_for_account(label)?;
    Ok(OAuthTokens {
        access_token: creds.access_token,
        refresh_token: creds.refresh_token,
        expires_at: creds.expires_at,
        id_token: None,
        scopes: creds.scopes,
    })
}

#[derive(Serialize)]
struct ClaudeRefreshTokenRequest<'a> {
    grant_type: &'static str,
    refresh_token: &'a str,
    client_id: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<&'static str>,
}

#[derive(Deserialize)]
struct ClaudeRefreshTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    scope: Option<String>,
}

fn claude_refresh_error_is_invalid_scope(err: &anyhow::Error) -> bool {
    let text = format!("{err:#}").to_ascii_lowercase();
    text.contains("invalid_scope")
        || text.contains("requested scope is invalid")
        || text.contains("scope is invalid")
}

async fn send_claude_refresh_request(
    refresh_token: &str,
    scope: Option<&'static str>,
) -> Result<ClaudeRefreshTokenResponse> {
    let payload = ClaudeRefreshTokenRequest {
        grant_type: "refresh_token",
        refresh_token,
        client_id: claude::CLIENT_ID,
        scope,
    };

    let client = crate::provider::shared_http_client();
    let resp = client
        .post(claude::TOKEN_URL)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(CLAUDE_TOKEN_TIMEOUT_SECS))
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        let scope_label = scope.unwrap_or("<omitted>");
        anyhow::bail!(
            "Token refresh failed with scope '{}': {}",
            scope_label,
            text
        );
    }

    Ok(resp.json().await?)
}

async fn refresh_claude_tokens_inner(
    refresh_token: &str,
    label: Option<&str>,
) -> Result<OAuthTokens> {
    let scoped_result =
        send_claude_refresh_request(refresh_token, Some(claude::REFRESH_SCOPES)).await;
    let tokens = match scoped_result {
        Ok(tokens) => tokens,
        Err(err) if claude_refresh_error_is_invalid_scope(&err) => {
            crate::logging::warn(
                "Claude token refresh rejected Claude Code scopes; retrying without an explicit scope for legacy token compatibility",
            );
            match send_claude_refresh_request(refresh_token, None).await {
                Ok(tokens) => tokens,
                Err(fallback_err) => {
                    anyhow::bail!(
                        "Claude token refresh fallback without scope failed: {fallback_err:#}; scoped refresh error: {err:#}"
                    );
                }
            }
        }
        Err(err) => return Err(err),
    };

    let expires_at = chrono::Utc::now().timestamp_millis() + (tokens.expires_in * 1000);
    let scopes = parse_oauth_scopes(tokens.scope.as_deref());
    ensure_claude_inference_scope(&scopes, "token refresh")?;
    let oauth_tokens = OAuthTokens {
        access_token: tokens.access_token,
        refresh_token: tokens
            .refresh_token
            .unwrap_or_else(|| refresh_token.to_string()),
        expires_at,
        id_token: None,
        scopes,
    };

    let save_label = label.map(ToString::to_string).unwrap_or_else(|| {
        claude_auth::active_account_label().unwrap_or_else(claude_auth::primary_account_label)
    });
    save_claude_tokens_for_account(&oauth_tokens, &save_label)?;

    Ok(oauth_tokens)
}

/// Refresh Claude OAuth tokens for the active (or primary) stored account.
pub async fn refresh_claude_tokens(refresh_token: &str) -> Result<OAuthTokens> {
    let label =
        claude_auth::active_account_label().unwrap_or_else(claude_auth::primary_account_label);
    refresh_claude_tokens_for_account(refresh_token, &label).await
}

/// Stored Claude tokens for `label`, expressed as [`OAuthTokens`].
fn stored_claude_tokens(label: &str) -> Option<OAuthTokens> {
    let account = claude_auth::list_accounts()
        .ok()?
        .into_iter()
        .find(|account| account.label == label)?;
    Some(OAuthTokens {
        access_token: account.access,
        refresh_token: account.refresh,
        expires_at: account.expires,
        id_token: None,
        scopes: account.scopes,
    })
}

/// Refresh Claude OAuth tokens for a specific account.
///
/// Serialized per account via the refresh coordinator: Anthropic rotates
/// refresh tokens, so two concurrent refreshes can otherwise persist a dead
/// refresh token and break the account.
pub async fn refresh_claude_tokens_for_account(
    refresh_token: &str,
    label: &str,
) -> Result<OAuthTokens> {
    let observed_refresh = refresh_token.to_string();
    let label = label.to_string();
    let result = crate::auth::refresh_coordinator::single_flight(
        format!("claude:{label}"),
        {
            let label = label.clone();
            move || stored_claude_tokens(&label)
        },
        {
            let observed = observed_refresh.clone();
            move |stored: &OAuthTokens| {
                stored.refresh_token != observed
                    && crate::auth::refresh_coordinator::expiry_is_fresh(stored.expires_at)
            }
        },
        move |stored: Option<OAuthTokens>| async move {
            // Prefer the newest stored refresh token over the caller's
            // possibly stale observation.
            let token = stored
                .map(|tokens| tokens.refresh_token)
                .filter(|token| !token.is_empty())
                .unwrap_or(observed_refresh);
            refresh_claude_tokens_inner(&token, Some(&label)).await
        },
    )
    .await;

    match &result {
        Ok(_) => {
            let _ = crate::auth::refresh_state::record_success("claude");
        }
        Err(err) => {
            let _ = crate::auth::refresh_state::record_failure("claude", err.to_string());
        }
    }

    result
}

/// Save OpenAI tokens to auth file
pub fn save_openai_tokens(tokens: &OAuthTokens) -> Result<()> {
    let label = crate::auth::codex::login_target_label(None)?;
    save_openai_tokens_for_account(tokens, &label)
}

/// Save OpenAI tokens for a specific stored account label.
pub fn save_openai_tokens_for_account(tokens: &OAuthTokens, label: &str) -> Result<()> {
    crate::auth::codex::upsert_account_from_tokens(
        label,
        &tokens.access_token,
        &tokens.refresh_token,
        tokens.id_token.clone(),
        Some(tokens.expires_at),
    )?;
    Ok(())
}

/// Refresh OpenAI/Codex OAuth tokens
pub async fn refresh_openai_tokens(refresh_token: &str) -> Result<OAuthTokens> {
    match crate::auth::codex::active_account_label() {
        Some(label) => refresh_openai_tokens_for_account(refresh_token, &label).await,
        // External token (not stored in jcode auth): nothing on disk to
        // coordinate against, refresh directly.
        None => refresh_openai_tokens_inner(refresh_token, None).await,
    }
}

/// Stored OpenAI tokens for `label`, expressed as [`OAuthTokens`].
fn stored_openai_tokens(label: &str) -> Option<OAuthTokens> {
    let account = crate::auth::codex::list_accounts()
        .ok()?
        .into_iter()
        .find(|account| account.label == label)?;
    Some(OAuthTokens {
        access_token: account.access_token,
        refresh_token: account.refresh_token,
        expires_at: account.expires_at.unwrap_or(0),
        id_token: account.id_token,
        scopes: Vec::new(),
    })
}

/// Refresh OpenAI/Codex OAuth tokens for a specific stored account label.
///
/// Serialized per account via the refresh coordinator: OpenAI rotates
/// refresh tokens, so two concurrent refreshes can otherwise persist a dead
/// refresh token and break the account.
pub async fn refresh_openai_tokens_for_account(
    refresh_token: &str,
    label: &str,
) -> Result<OAuthTokens> {
    let observed_refresh = refresh_token.to_string();
    let label = label.to_string();
    crate::auth::refresh_coordinator::single_flight(
        format!("openai:{label}"),
        {
            let label = label.clone();
            move || stored_openai_tokens(&label)
        },
        {
            let observed = observed_refresh.clone();
            move |stored: &OAuthTokens| {
                stored.refresh_token != observed
                    && crate::auth::refresh_coordinator::expiry_is_fresh(stored.expires_at)
            }
        },
        move |stored: Option<OAuthTokens>| async move {
            // Prefer the newest stored refresh token over the caller's
            // possibly stale observation.
            let token = stored
                .map(|tokens| tokens.refresh_token)
                .filter(|token| !token.is_empty())
                .unwrap_or(observed_refresh);
            refresh_openai_tokens_inner(&token, Some(&label)).await
        },
    )
    .await
}

async fn refresh_openai_tokens_inner(
    refresh_token: &str,
    label: Option<&str>,
) -> Result<OAuthTokens> {
    let result: Result<OAuthTokens> = async {
        let client = crate::provider::shared_http_client();
        let resp = client
            .post(openai::TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!(
                "grant_type=refresh_token&client_id={}&refresh_token={}",
                openai::CLIENT_ID,
                urlencoding::encode(refresh_token)
            ))
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await?;
            anyhow::bail!("OpenAI token refresh failed: {}", text);
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            refresh_token: String,
            expires_in: i64,
            id_token: Option<String>,
        }

        let tokens: TokenResponse = resp.json().await?;
        let expires_at = chrono::Utc::now().timestamp_millis() + (tokens.expires_in * 1000);

        let oauth_tokens = OAuthTokens {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at,
            id_token: tokens.id_token,
            scopes: Vec::new(),
        };

        if let Some(label) = label {
            save_openai_tokens_for_account(&oauth_tokens, label)?;
        } else {
            crate::logging::info(
                "Refreshed OpenAI/Codex tokens from an external source without storing them in jcode auth",
            );
        }
        Ok(oauth_tokens)
    }
    .await;

    match &result {
        Ok(_) => {
            let _ = crate::auth::refresh_state::record_success("openai");
        }
        Err(err) => {
            let _ = crate::auth::refresh_state::record_failure("openai", err.to_string());
        }
    }

    result
}

#[cfg(test)]
#[path = "oauth_tests/mod.rs"]
mod tests;
