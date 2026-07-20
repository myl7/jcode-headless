use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v1/userinfo?alt=json";
// OAuth credentials from Google's Antigravity desktop app.
// These are for a desktop OAuth client where the client secret is safe to embed.
// Env vars remain available as optional overrides.
// gitleaks:allow - public desktop OAuth credentials, safe to embed
const ANTIGRAVITY_CLIENT_ID: &str =
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com"; // gitleaks:allow
const ANTIGRAVITY_CLIENT_SECRET: &str = "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"; // gitleaks:allow
const CLIENT_ID_ENV: &str = "JCODE_ANTIGRAVITY_CLIENT_ID";
const CLIENT_SECRET_ENV: &str = "JCODE_ANTIGRAVITY_CLIENT_SECRET";
const VERSION_ENV: &str = "JCODE_ANTIGRAVITY_VERSION";
const ANTIGRAVITY_VERSION: &str = "1.18.3";
const LOAD_ENDPOINTS: &[&str] = &[
    "https://cloudcode-pa.googleapis.com",
    "https://daily-cloudcode-pa.sandbox.googleapis.com",
    "https://autopush-cloudcode-pa.sandbox.googleapis.com",
];
const GOOGLE_OAUTH_USER_AGENT: &str = "google-api-nodejs-client/9.15.1";

fn antigravity_client_id() -> String {
    std::env::var(CLIENT_ID_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ANTIGRAVITY_CLIENT_ID.to_string())
}

fn antigravity_client_secret() -> String {
    std::env::var(CLIENT_SECRET_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ANTIGRAVITY_CLIENT_SECRET.to_string())
}

fn antigravity_version() -> String {
    std::env::var(VERSION_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ANTIGRAVITY_VERSION.to_string())
}

fn metadata_platform() -> &'static str {
    // The Cloud Code backend currently rejects OS-specific string enum values
    // such as MACOS, WINDOWS, and LINUX for ClientMetadata.Platform. Use the
    // string value that is accepted across platforms instead of varying by OS.
    "PLATFORM_UNSPECIFIED"
}

fn user_agent() -> String {
    if cfg!(target_os = "windows") {
        format!("antigravity/{} windows/amd64", antigravity_version())
    } else if cfg!(target_arch = "aarch64") {
        format!("antigravity/{} darwin/arm64", antigravity_version())
    } else {
        format!("antigravity/{} darwin/amd64", antigravity_version())
    }
}

fn client_metadata_header() -> String {
    format!(
        "{{\"ideType\":\"ANTIGRAVITY\",\"platform\":\"{}\",\"pluginType\":\"GEMINI\"}}",
        metadata_platform()
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

impl AntigravityTokens {
    pub fn is_expired(&self) -> bool {
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.expires_at <= now_ms + 60_000
    }
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadCodeAssistResponse {
    #[serde(default)]
    cloudaicompanion_project: Option<serde_json::Value>,
}

pub fn tokens_path() -> Result<std::path::PathBuf> {
    Ok(crate::storage::jcode_dir()?.join("antigravity_oauth.json"))
}

pub fn load_tokens() -> Result<AntigravityTokens> {
    let path = tokens_path()?;
    if path.exists() {
        crate::storage::harden_secret_file_permissions(&path);
        return crate::storage::read_json(&path)
            .map_err(|_| anyhow::anyhow!("No Antigravity tokens found in JCODE_HOME."));
    }

    if let Some(tokens) = crate::auth::external::load_antigravity_oauth_tokens() {
        return Ok(AntigravityTokens {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_at: tokens.expires_at,
            email: None,
            project_id: None,
        });
    }

    anyhow::bail!("No Antigravity tokens found in JCODE_HOME.");
}

pub fn save_tokens(tokens: &AntigravityTokens) -> Result<()> {
    let path = tokens_path()?;
    crate::storage::write_json_secret(&path, tokens)
}

pub fn has_cached_auth() -> bool {
    load_tokens().is_ok()
}

pub async fn load_or_refresh_tokens() -> Result<AntigravityTokens> {
    let tokens = load_tokens()?;
    if tokens.is_expired() {
        refresh_tokens(&tokens).await
    } else {
        Ok(tokens)
    }
}

/// Refresh Antigravity OAuth tokens, serialized via the refresh coordinator
/// so concurrent callers do not race the token endpoint and the stored file.
pub async fn refresh_tokens(tokens: &AntigravityTokens) -> Result<AntigravityTokens> {
    crate::auth::refresh_coordinator::single_flight(
        "antigravity".to_string(),
        || load_tokens().ok(),
        |stored: &AntigravityTokens| !stored.is_expired(),
        {
            let observed = tokens.clone();
            move |stored: Option<AntigravityTokens>| async move {
                let source = stored.unwrap_or(observed);
                refresh_tokens_uncoordinated(&source).await
            }
        },
    )
    .await
}

async fn refresh_tokens_uncoordinated(tokens: &AntigravityTokens) -> Result<AntigravityTokens> {
    let result: Result<AntigravityTokens> = async {
        let token = crate::auth::google_oauth::refresh_access_token(
            "Antigravity",
            &antigravity_client_id(),
            &antigravity_client_secret(),
            &tokens.refresh_token,
            Some(GOOGLE_OAUTH_USER_AGENT),
        )
        .await?;

        let mut refreshed = AntigravityTokens {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_at: token.expires_at_ms,
            email: tokens.email.clone(),
            project_id: tokens.project_id.clone(),
        };

        if refreshed.email.is_none() {
            refreshed.email = fetch_email(&refreshed.access_token).await.ok();
        }
        if refreshed.project_id.is_none() {
            refreshed.project_id = fetch_project_id(&refreshed.access_token).await.ok();
        }

        save_tokens(&refreshed)?;
        Ok(refreshed)
    }
    .await;

    match &result {
        Ok(_) => {
            let _ = crate::auth::refresh_state::record_success("antigravity");
        }
        Err(err) => {
            let _ = crate::auth::refresh_state::record_failure("antigravity", err.to_string());
        }
    }

    result
}

pub async fn fetch_email(access_token: &str) -> Result<String> {
    let client = crate::provider::shared_http_client();
    let resp = client
        .get(GOOGLE_USERINFO_URL)
        .header(reqwest::header::USER_AGENT, GOOGLE_OAUTH_USER_AGENT)
        .bearer_auth(access_token)
        .send()
        .await
        .context("Failed to fetch Antigravity Google profile")?;

    if !resp.status().is_success() {
        let body = crate::util::http_error_body(resp, "HTTP error").await;
        anyhow::bail!(
            "Failed to fetch Antigravity Google profile: {}",
            body.trim()
        );
    }

    let profile: GoogleUserInfo = resp
        .json()
        .await
        .context("Failed to parse Antigravity Google profile")?;
    profile
        .email
        .filter(|email| !email.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("Google profile did not include an email address"))
}

pub async fn fetch_project_id(access_token: &str) -> Result<String> {
    let client = crate::provider::shared_http_client();
    let headers = antigravity_headers(access_token)?;
    let body = serde_json::json!({
        "metadata": {
            "ideType": "ANTIGRAVITY",
            "platform": metadata_platform(),
            "pluginType": "GEMINI"
        }
    });
    let mut errors = Vec::new();

    for base_url in LOAD_ENDPOINTS {
        let resp = match client
            .post(format!("{base_url}/v1internal:loadCodeAssist"))
            .headers(headers.clone())
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                errors.push(format!("{base_url}: {err}"));
                continue;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let text = crate::util::http_error_body(resp, "HTTP error").await;
            errors.push(format!("{base_url}: HTTP {status} {}", text.trim()));
            continue;
        }

        let parsed: LoadCodeAssistResponse = resp
            .json()
            .await
            .with_context(|| format!("Failed to parse loadCodeAssist response from {base_url}"))?;
        if let Some(project_id) = extract_project_id(parsed.cloudaicompanion_project) {
            return Ok(project_id);
        }
        errors.push(format!("{base_url}: project id missing from response"));
    }

    anyhow::bail!(
        "Failed to resolve Antigravity project via loadCodeAssist: {}",
        errors.join("; ")
    )
}

fn antigravity_headers(access_token: &str) -> Result<reqwest::header::HeaderMap> {
    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {access_token}"))
            .context("invalid Antigravity authorization header")?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&user_agent()).context("invalid Antigravity user-agent header")?,
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("x-goog-api-client"),
        HeaderValue::from_static("google-cloud-sdk vscode_cloudshelleditor/0.1"),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("client-metadata"),
        HeaderValue::from_str(&client_metadata_header())
            .context("invalid Antigravity client-metadata header")?,
    );
    Ok(headers)
}

fn extract_project_id(value: Option<serde_json::Value>) -> Option<String> {
    match value {
        Some(serde_json::Value::String(project_id)) => {
            let trimmed = project_id.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Some(serde_json::Value::Object(map)) => map
            .get("id")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::lock_test_env;

    #[test]
    fn blank_env_vars_fall_back_to_built_in_credentials() {
        let _guard = lock_test_env();
        crate::env::set_var(CLIENT_ID_ENV, "   ");
        crate::env::set_var(CLIENT_SECRET_ENV, "   ");

        assert_eq!(antigravity_client_id(), ANTIGRAVITY_CLIENT_ID);
        assert_eq!(antigravity_client_secret(), ANTIGRAVITY_CLIENT_SECRET);

        crate::env::remove_var(CLIENT_ID_ENV);
        crate::env::remove_var(CLIENT_SECRET_ENV);
    }

    #[test]
    fn client_metadata_uses_backend_accepted_platform() {
        assert_eq!(metadata_platform(), "PLATFORM_UNSPECIFIED");
        assert!(client_metadata_header().contains("\"platform\":\"PLATFORM_UNSPECIFIED\""));
    }

    #[test]
    fn extract_project_id_supports_string_or_object() {
        assert_eq!(
            extract_project_id(Some(serde_json::Value::String("proj-123".to_string()))),
            Some("proj-123".to_string())
        );
        assert_eq!(
            extract_project_id(Some(serde_json::json!({ "id": "proj-456" }))),
            Some("proj-456".to_string())
        );
        assert_eq!(
            extract_project_id(Some(serde_json::json!({ "id": "   " }))),
            None
        );
    }
}
