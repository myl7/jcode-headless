use super::*;

#[test]
fn oauth_tokens_serialization_roundtrip() -> Result<()> {
    let tokens = OAuthTokens {
        access_token: "access".to_string(),
        refresh_token: "refresh".to_string(),
        expires_at: 123,
        id_token: Some("id".to_string()),
        scopes: vec!["user:inference".to_string()],
    };
    let decoded: OAuthTokens = serde_json::from_str(&serde_json::to_string(&tokens)?)?;
    assert_eq!(decoded.access_token, "access");
    assert_eq!(decoded.refresh_token, "refresh");
    assert_eq!(decoded.id_token.as_deref(), Some("id"));
    assert!(claude_scopes_have_inference(&decoded.scopes));
    Ok(())
}

#[test]
fn save_openai_tokens_uses_jcode_home_sandbox() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let temp = tempfile::tempdir()?;
    let _home = EnvVarGuard::set("JCODE_HOME", temp.path());
    let tokens = OAuthTokens {
        access_token: "access".to_string(),
        refresh_token: "refresh".to_string(),
        expires_at: 123,
        id_token: None,
        scopes: Vec::new(),
    };
    save_openai_tokens(&tokens)?;
    assert!(temp.path().join("openai-auth.json").is_file());
    Ok(())
}

#[test]
fn save_claude_tokens_preserves_existing_account_metadata() -> Result<()> {
    let _lock = crate::storage::lock_test_env();
    let temp = tempfile::tempdir()?;
    let _home = EnvVarGuard::set("JCODE_HOME", temp.path());
    crate::auth::claude::upsert_account(crate::auth::claude::AnthropicAccount {
        label: "default".to_string(),
        access: "old".to_string(),
        refresh: "old-refresh".to_string(),
        expires: 1,
        email: Some("person@example.com".to_string()),
        subscription_type: Some("max".to_string()),
        scopes: vec!["user:inference".to_string()],
    })?;

    save_claude_tokens_for_account(
        &OAuthTokens {
            access_token: "new".to_string(),
            refresh_token: "new-refresh".to_string(),
            expires_at: 2,
            id_token: None,
            scopes: Vec::new(),
        },
        "default",
    )?;

    let account = crate::auth::claude::list_accounts()?
        .into_iter()
        .find(|account| account.label == "default")
        .expect("saved account");
    assert_eq!(account.email.as_deref(), Some("person@example.com"));
    assert_eq!(account.subscription_type.as_deref(), Some("max"));
    assert_eq!(account.scopes, vec!["user:inference"]);
    Ok(())
}
