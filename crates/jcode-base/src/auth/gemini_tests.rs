use super::*;
use crate::storage::lock_test_env;

#[test]
fn parses_env_command_with_args() {
    let resolved =
        resolve_gemini_cli_command_with(Some("npx @google/gemini-cli --proxy test"), |_| false);
    assert_eq!(
        resolved,
        GeminiCliCommand {
            program: "npx".to_string(),
            args: vec![
                "@google/gemini-cli".to_string(),
                "--proxy".to_string(),
                "test".to_string(),
            ],
        }
    );
}

#[test]
fn falls_back_to_gemini_binary_when_available() {
    let resolved = resolve_gemini_cli_command_with(None, |cmd| cmd == "gemini");
    assert_eq!(resolved.program, "gemini");
    assert!(resolved.args.is_empty());
}

#[test]
fn falls_back_to_npx_when_gemini_binary_missing() {
    let resolved = resolve_gemini_cli_command_with(None, |cmd| cmd == "npx");
    assert_eq!(resolved.program, "npx");
    assert_eq!(resolved.args, vec!["@google/gemini-cli"]);
}

#[test]
fn display_includes_args_when_present() {
    let command = GeminiCliCommand {
        program: "npx".to_string(),
        args: vec!["@google/gemini-cli".to_string()],
    };
    assert_eq!(command.display(), "npx @google/gemini-cli");
}

#[test]
fn imports_cli_oauth_tokens_when_native_tokens_missing() {
    let _guard = lock_test_env();
    let temp = tempfile::TempDir::new().expect("tempdir");
    let prev_home = std::env::var_os("JCODE_HOME");
    crate::env::set_var("JCODE_HOME", temp.path());

    let cli_path = gemini_cli_oauth_path().expect("cli path");
    std::fs::create_dir_all(cli_path.parent().unwrap()).expect("create cli dir");
    std::fs::write(
        &cli_path,
        r#"{"access_token":"at-123","refresh_token":"rt-456","expiry_date":4102444800000}"#,
    )
    .expect("write cli token file");
    crate::config::Config::allow_external_auth_source_for_path(
        GEMINI_CLI_AUTH_SOURCE_ID,
        &cli_path,
    )
    .expect("trust cli auth path");

    let tokens = load_tokens().expect("load tokens");
    assert_eq!(tokens.access_token, "at-123");
    assert_eq!(tokens.refresh_token, "rt-456");
    assert_eq!(tokens.expires_at, 4102444800000);

    if let Some(prev_home) = prev_home {
        crate::env::set_var("JCODE_HOME", prev_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

#[cfg(unix)]
#[test]
fn imports_cli_oauth_tokens_without_changing_external_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = lock_test_env();
    let temp = tempfile::TempDir::new().expect("tempdir");
    let prev_home = std::env::var_os("JCODE_HOME");
    crate::env::set_var("JCODE_HOME", temp.path());

    let cli_path = gemini_cli_oauth_path().expect("cli path");
    std::fs::create_dir_all(cli_path.parent().unwrap()).expect("create cli dir");
    std::fs::write(
        &cli_path,
        r#"{"access_token":"at-123","refresh_token":"rt-456","expiry_date":4102444800000}"#,
    )
    .expect("write cli token file");
    std::fs::set_permissions(
        cli_path.parent().unwrap(),
        std::fs::Permissions::from_mode(0o755),
    )
    .expect("set dir perms");
    std::fs::set_permissions(&cli_path, std::fs::Permissions::from_mode(0o644))
        .expect("set file perms");
    crate::config::Config::allow_external_auth_source_for_path(
        GEMINI_CLI_AUTH_SOURCE_ID,
        &cli_path,
    )
    .expect("trust cli auth path");

    let tokens = load_tokens().expect("load tokens");
    assert_eq!(tokens.access_token, "at-123");

    let dir_mode = std::fs::metadata(cli_path.parent().unwrap())
        .expect("stat dir")
        .permissions()
        .mode()
        & 0o777;
    let file_mode = std::fs::metadata(&cli_path)
        .expect("stat file")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(dir_mode, 0o755);
    assert_eq!(file_mode, 0o644);

    if let Some(prev_home) = prev_home {
        crate::env::set_var("JCODE_HOME", prev_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}
