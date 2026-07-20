use super::*;

#[tokio::test]
async fn initiative_tool_create_show_and_resume_round_trip() {
    let _guard = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let project = temp.path().join("repo");
    std::fs::create_dir_all(&project).expect("project dir");
    let prev_home = std::env::var_os("JCODE_HOME");
    crate::env::set_var("JCODE_HOME", temp.path());

    let tool = InitiativeTool::new();
    let ctx = ToolContext {
        session_id: "ses_goal_tool".to_string(),
        message_id: "msg1".to_string(),
        tool_call_id: "tool1".to_string(),
        working_dir: Some(project),
        stdin_request_tx: None,
        graceful_shutdown_signal: None,
        execution_mode: crate::tool::ToolExecutionMode::AgentTurn,
    };

    let created = tool
        .execute(
            json!({
                "action": "create",
                "title": "Ship service",
                "scope": "project",
                "next_steps": ["finish reconnect flow"]
            }),
            ctx.clone(),
        )
        .await
        .expect("create goal");
    assert!(created.output.contains("Created initiative"));

    let shown = tool
        .execute(json!({"action": "show", "id": "ship-service"}), ctx.clone())
        .await
        .expect("show goal");
    assert!(shown.output.contains("finish reconnect flow"));

    let resumed = tool
        .execute(json!({"action": "resume"}), ctx)
        .await
        .expect("resume goal");
    assert!(resumed.output.contains("Resumed initiative"));

    if let Some(prev_home) = prev_home {
        crate::env::set_var("JCODE_HOME", prev_home);
    } else {
        crate::env::remove_var("JCODE_HOME");
    }
}

#[test]
fn initiative_schema_milestones_define_items_and_omits_ui_fields() {
    let schema = InitiativeTool::new().parameters_schema();
    let properties = schema["properties"].as_object().expect("properties");
    assert!(properties["milestones"]["items"].is_object());
    assert!(!properties.contains_key("display"));
}
