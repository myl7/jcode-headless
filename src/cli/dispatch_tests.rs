use super::*;

#[cfg(target_os = "linux")]
#[test]
fn only_file_controlled_debug_clients_need_parent_lifetime_binding() {
    let _lock = crate::storage::lock_test_env();
    let previous = std::env::var_os("JCODE_DEBUG_CMD_PATH");
    crate::env::remove_var("JCODE_DEBUG_CMD_PATH");
    assert!(!is_file_controlled_debug_client());

    crate::env::set_var("JCODE_DEBUG_CMD_PATH", "/tmp/jcode-test-debug-command");
    assert!(is_file_controlled_debug_client());

    if let Some(previous) = previous {
        crate::env::set_var("JCODE_DEBUG_CMD_PATH", previous);
    } else {
        crate::env::remove_var("JCODE_DEBUG_CMD_PATH");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn init_and_systemd_are_recognized_as_orphan_adopters() {
    assert!(is_orphan_adopter_name("init\n"));
    assert!(is_orphan_adopter_name("systemd\n"));
    assert!(!is_orphan_adopter_name("bash\n"));
    assert!(!is_orphan_adopter_name("jcode\n"));
}

#[test]
fn auth_doctor_provider_focus_uses_global_provider_when_positional_is_absent() {
    assert_eq!(
        auth_doctor_provider_arg(None, &ProviderChoice::Cerebras),
        Some("cerebras")
    );
    assert_eq!(
        auth_doctor_provider_arg(None, &ProviderChoice::Auto),
        None,
        "auto should keep the default doctor behavior of checking configured providers"
    );
}

#[test]
fn auth_doctor_positional_provider_wins_over_global_provider() {
    assert_eq!(
        auth_doctor_provider_arg(Some("openai"), &ProviderChoice::Cerebras),
        Some("openai"),
        "`jcode --provider cerebras auth doctor openai` should diagnose the explicit positional provider"
    );
}

#[test]
fn resolve_resume_id_imports_raw_codex_session_ids() {
    let _guard = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("tempdir");
    crate::env::set_var("JCODE_HOME", temp.path());

    let codex_dir = temp.path().join("external/.codex/sessions/2026/04/16");
    std::fs::create_dir_all(&codex_dir).expect("create codex dir");
    std::fs::write(
            codex_dir.join("rollout.jsonl"),
            concat!(
                "{\"timestamp\":\"2026-04-16T10:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"codex-cli-resume-test\",\"timestamp\":\"2026-04-16T09:59:00Z\",\"cwd\":\"/tmp/codex-cli-resume\"}}\n",
                "{\"timestamp\":\"2026-04-16T10:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"Resume this Codex session\"}]}}\n",
                "{\"timestamp\":\"2026-04-16T10:00:02Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Imported\"}]}}\n"
            ),
        )
        .expect("write codex transcript");

    let resolved = resolve_resume_id("codex-cli-resume-test").expect("resolve codex id");
    let imported_id = crate::import::imported_codex_session_id("codex-cli-resume-test");
    assert_eq!(resolved, imported_id);

    let session = crate::session::Session::load(&resolved).expect("load imported session");
    assert_eq!(session.messages.len(), 2);

    crate::env::remove_var("JCODE_HOME");
}
