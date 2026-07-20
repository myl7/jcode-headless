use anyhow::Result;
use serde::{Deserialize, Serialize};

pub const SCOPE_READONLY: &str = "https://www.googleapis.com/auth/gmail.readonly";
pub const SCOPE_COMPOSE: &str = "https://www.googleapis.com/auth/gmail.compose";
pub const SCOPE_SEND: &str = "https://www.googleapis.com/auth/gmail.send";
pub const SCOPE_MODIFY: &str = "https://www.googleapis.com/auth/gmail.modify";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GmailAccessTier {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "readonly")]
    ReadOnly,
}

impl GmailAccessTier {
    pub fn scopes(&self) -> Vec<&'static str> {
        match self {
            GmailAccessTier::Full => vec![SCOPE_READONLY, SCOPE_COMPOSE, SCOPE_SEND, SCOPE_MODIFY],
            GmailAccessTier::ReadOnly => vec![SCOPE_READONLY, SCOPE_COMPOSE],
        }
    }

    pub fn can_send(&self) -> bool {
        matches!(self, GmailAccessTier::Full)
    }

    pub fn can_delete(&self) -> bool {
        matches!(self, GmailAccessTier::Full)
    }

    pub fn label(&self) -> &'static str {
        match self {
            GmailAccessTier::Full => "Full Access",
            GmailAccessTier::ReadOnly => "Read & Draft Only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleCredentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub tier: GmailAccessTier,
    pub email: Option<String>,
}

impl GoogleTokens {
    pub fn is_expired(&self) -> bool {
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.expires_at <= now_ms + 60_000
    }
}

pub fn credentials_path() -> Result<std::path::PathBuf> {
    Ok(crate::storage::jcode_dir()?.join("google_credentials.json"))
}

pub fn tokens_path() -> Result<std::path::PathBuf> {
    Ok(crate::storage::jcode_dir()?.join("google_oauth.json"))
}

pub fn load_credentials() -> Result<GoogleCredentials> {
    let path = credentials_path()?;
    crate::storage::harden_secret_file_permissions(&path);
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return Err(anyhow::anyhow!("no_credentials")),
    };

    if let Ok(creds) = serde_json::from_str::<GoogleCredentials>(&data) {
        return Ok(creds);
    }

    #[derive(Deserialize)]
    struct GCloudFormat {
        installed: Option<GCloudInstalled>,
        web: Option<GCloudInstalled>,
    }
    #[derive(Deserialize)]
    struct GCloudInstalled {
        client_id: String,
        client_secret: String,
    }

    let gcloud: GCloudFormat = serde_json::from_str(&data)?;
    let inner = gcloud
        .installed
        .or(gcloud.web)
        .ok_or_else(|| anyhow::anyhow!("Invalid Google credentials format"))?;

    Ok(GoogleCredentials {
        client_id: inner.client_id,
        client_secret: inner.client_secret,
    })
}

pub fn save_credentials(creds: &GoogleCredentials) -> Result<()> {
    let path = credentials_path()?;
    crate::storage::write_json_secret(&path, creds)
}

pub fn load_tokens() -> Result<GoogleTokens> {
    let path = tokens_path()?;
    if !path.exists() {
        anyhow::bail!("No Google tokens found in JCODE_HOME.");
    }
    crate::storage::harden_secret_file_permissions(&path);
    crate::storage::read_json(&path)
        .map_err(|_| anyhow::anyhow!("No Google tokens found in JCODE_HOME."))
}

pub fn save_tokens(tokens: &GoogleTokens) -> Result<()> {
    let path = tokens_path()?;
    crate::storage::write_json_secret(&path, tokens)
}

pub fn has_tokens() -> bool {
    tokens_path().map(|path| path.exists()).unwrap_or(false)
}

/// Refresh Google OAuth tokens, serialized via the refresh coordinator so
/// concurrent callers do not race the token endpoint and the stored file.
pub async fn refresh_tokens(tokens: &GoogleTokens) -> Result<GoogleTokens> {
    crate::auth::refresh_coordinator::single_flight(
        "google".to_string(),
        || load_tokens().ok(),
        |stored: &GoogleTokens| !stored.is_expired(),
        {
            let observed = tokens.clone();
            move |stored: Option<GoogleTokens>| async move {
                let source = stored.unwrap_or(observed);
                refresh_tokens_uncoordinated(&source).await
            }
        },
    )
    .await
}

async fn refresh_tokens_uncoordinated(tokens: &GoogleTokens) -> Result<GoogleTokens> {
    let result: Result<GoogleTokens> = async {
        let creds = load_credentials()?;
        let refreshed = crate::auth::google_oauth::refresh_access_token(
            "Google",
            &creds.client_id,
            &creds.client_secret,
            &tokens.refresh_token,
            None,
        )
        .await?;

        let new_tokens = GoogleTokens {
            access_token: refreshed.access_token,
            refresh_token: refreshed.refresh_token,
            expires_at: refreshed.expires_at_ms,
            tier: tokens.tier,
            email: tokens.email.clone(),
        };

        save_tokens(&new_tokens)?;
        Ok(new_tokens)
    }
    .await;

    match &result {
        Ok(_) => {
            let _ = crate::auth::refresh_state::record_success("google");
        }
        Err(err) => {
            let _ = crate::auth::refresh_state::record_failure("google", err.to_string());
        }
    }

    result
}

pub async fn get_valid_token() -> Result<String> {
    let tokens = load_tokens()?;
    if tokens.is_expired() {
        let new_tokens = refresh_tokens(&tokens).await?;
        Ok(new_tokens.access_token)
    } else {
        Ok(tokens.access_token)
    }
}
