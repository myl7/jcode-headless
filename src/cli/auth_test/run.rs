async fn maybe_run_auth_test_smoke(
    report: &mut AuthTestProviderReport,
    kind: AuthTestSmokeKind,
    target: AuthTestTarget,
    model: Option<&str>,
    enabled: bool,
    prompt: &str,
) {
    if enabled && report.success && target.supports_smoke() {
        // Some providers validate chat but cannot run the tool smoke. The
        // Cursor native agent transport is text-only (no tool calls over
        // agent.v1.AgentService/Run), so skip the tool smoke with an
        // explanation instead of hanging waiting for a tool call.
        if matches!(kind, AuthTestSmokeKind::Tool)
            && matches!(target, AuthTestTarget::Cursor)
        {
            report.push_step(
                kind.step_name(),
                true,
                "Skipped: the Cursor native agent transport is text-only in jcode (it does not \
                 expose tool calls over agent.v1.AgentService/Run). Basic provider smoke still \
                 validates chat."
                    .to_string(),
            );
            return;
        }
        match kind.run(target, model, prompt).await {
            Ok(output) => {
                let ok = output.contains("AUTH_TEST_OK");
                kind.set_output(report, output.clone());
                report.push_step(
                    kind.step_name(),
                    ok,
                    if ok {
                        kind.success_detail().to_string()
                    } else {
                        kind.failure_detail(&output)
                    },
                );
            }
            Err(err) => report.push_step(kind.step_name(), false, format!("{err:#}")),
        }
    } else if !target.supports_smoke() {
        report.push_step(kind.step_name(), true, kind.unsupported_detail());
    } else if !enabled {
        report.push_step(kind.step_name(), true, kind.skipped_by_flag_detail());
    }
}

async fn maybe_run_auth_test_smoke_for_choice(
    report: &mut AuthTestProviderReport,
    kind: AuthTestSmokeKind,
    choice: &super::provider_init::ProviderChoice,
    model: Option<&str>,
    enabled: bool,
    prompt: &str,
) {
    if enabled && report.success {
        match auth_test_choice_plan(choice, model).await {
            Ok(AuthTestChoicePlan::Run { model }) => {
                if matches!(kind, AuthTestSmokeKind::Tool)
                    && let Some(detail) =
                        tool_smoke_skip_detail_for_choice(choice, model.as_deref())
                {
                    report.push_step(kind.step_name(), true, detail);
                    return;
                }
                match kind.run_for_choice(choice, model.as_deref(), prompt).await {
                    Ok(output) => {
                        let ok = output.contains("AUTH_TEST_OK");
                        kind.set_output(report, output.clone());
                        report.push_step(
                            kind.step_name(),
                            ok,
                            if ok {
                                kind.success_detail().to_string()
                            } else {
                                kind.failure_detail(&output)
                            },
                        );
                    }
                    Err(err) => report.push_step(kind.step_name(), false, format!("{err:#}")),
                }
            }
            Ok(AuthTestChoicePlan::Skip(detail)) => {
                report.push_step(kind.step_name(), true, detail);
            }
            Err(err) => report.push_step(kind.step_name(), false, format!("{err:#}")),
        }
    } else if !enabled {
        report.push_step(kind.step_name(), true, kind.skipped_by_flag_detail());
    }
}

pub(crate) async fn run_credential_validation_quiet(
    provider: crate::provider_catalog::LoginProviderDescriptor,
) -> Result<()> {
    run_credential_validation_inner(provider, false).await
}

async fn run_credential_validation_inner(
    provider: crate::provider_catalog::LoginProviderDescriptor,
    verbose: bool,
) -> Result<()> {
    let Some(choice) = super::provider_init::choice_for_login_provider(provider) else {
        crate::logging::auth_event(
            "credential_validation_skipped",
            provider.id,
            &[("reason", "no_runtime_provider_choice")],
        );
        if verbose {
            eprintln!(
                "\nSkipping automatic runtime validation for {}. Auto Import can add multiple providers; validate them with `jcode auth doctor --validate`.",
                provider.display_name
            );
        }
        return Ok(());
    };

    super::provider_init::apply_login_provider_profile_env(provider);
    crate::logging::auth_event(
        "credential_validation_started",
        provider.id,
        &[("choice", choice.as_arg_value())],
    );

    if verbose {
        eprintln!(
            "\nValidating {} credentials with live auth/runtime checks...",
            provider.display_name
        );
    }

    let report = if let Some(target) = AuthTestTarget::from_provider_choice(&choice) {
        populate_auth_test_target_report(
            target,
            None,
            true,
            true,
            DEFAULT_AUTH_TEST_PROVIDER_PROMPT,
            DEFAULT_AUTH_TEST_TOOL_PROMPT,
            AuthTestProviderReport::new(target),
        )
        .await
    } else {
        populate_generic_auth_test_report(
            provider,
            choice,
            None,
            true,
            true,
            DEFAULT_AUTH_TEST_PROVIDER_PROMPT,
            DEFAULT_AUTH_TEST_TOOL_PROMPT,
            AuthTestProviderReport::new_generic(
                choice.as_arg_value().to_string(),
                generic_credential_paths_for_provider(provider),
            ),
        )
        .await
    };

    persist_auth_test_report(&report);
    let step_count = report.steps.len().to_string();
    crate::logging::auth_event(
        "credential_validation_completed",
        provider.id,
        &[
            ("choice", choice.as_arg_value()),
            ("success", if report.success { "true" } else { "false" }),
            ("steps", step_count.as_str()),
        ],
    );
    if verbose {
        print_auth_test_reports(std::slice::from_ref(&report));
    }

    if report.success {
        Ok(())
    } else if AuthTestTarget::from_provider_choice(&choice).is_some() {
        anyhow::bail!(
            "Credential validation failed for {}. jcode could not verify runtime readiness. Re-run `jcode auth doctor {} --validate` for details.",
            provider.display_name,
            choice.as_arg_value()
        )
    } else {
        anyhow::bail!(
            "Credential validation failed for {}. Re-test with `jcode --provider {} run \"Reply with exactly AUTH_TEST_OK and nothing else.\"` after fixing the provider/runtime.",
            provider.display_name,
            choice.as_arg_value()
        )
    }
}

async fn populate_auth_test_target_report(
    target: AuthTestTarget,
    model: Option<&str>,
    run_smoke: bool,
    run_tool_smoke: bool,
    provider_smoke_prompt: &str,
    tool_smoke_prompt: &str,
    mut report: AuthTestProviderReport,
) -> AuthTestProviderReport {
    match target {
        AuthTestTarget::Claude => probe_claude_auth(&mut report).await,
        AuthTestTarget::Openai => probe_openai_auth(&mut report).await,
        AuthTestTarget::Gemini => probe_gemini_auth(&mut report).await,
        AuthTestTarget::Antigravity => probe_antigravity_auth(&mut report).await,
        AuthTestTarget::Google => probe_google_auth(&mut report).await,
        AuthTestTarget::Copilot => probe_copilot_auth(&mut report).await,
        AuthTestTarget::Cursor => probe_cursor_auth(&mut report).await,
    }

    maybe_run_auth_test_smoke(
        &mut report,
        AuthTestSmokeKind::Provider,
        target,
        model,
        run_smoke,
        provider_smoke_prompt,
    )
    .await;

    maybe_run_auth_test_smoke(
        &mut report,
        AuthTestSmokeKind::Tool,
        target,
        model,
        run_tool_smoke,
        tool_smoke_prompt,
    )
    .await;

    report
}

#[expect(
    clippy::too_many_arguments,
    reason = "Auth-test helper carries explicit smoke and prompt controls until structured options land"
)]
async fn populate_generic_auth_test_report(
    provider: crate::provider_catalog::LoginProviderDescriptor,
    choice: super::provider_init::ProviderChoice,
    model: Option<&str>,
    run_smoke: bool,
    run_tool_smoke: bool,
    provider_smoke_prompt: &str,
    tool_smoke_prompt: &str,
    mut report: AuthTestProviderReport,
) -> AuthTestProviderReport {
    super::provider_init::apply_login_provider_profile_env(provider);
    probe_generic_provider_auth(provider, &mut report);

    maybe_run_auth_test_smoke_for_choice(
        &mut report,
        AuthTestSmokeKind::Provider,
        &choice,
        model,
        run_smoke,
        provider_smoke_prompt,
    )
    .await;

    maybe_run_auth_test_smoke_for_choice(
        &mut report,
        AuthTestSmokeKind::Tool,
        &choice,
        model,
        run_tool_smoke,
        tool_smoke_prompt,
    )
    .await;

    report
}

fn persist_auth_test_report(report: &AuthTestProviderReport) {
    let step_map = report
        .steps
        .iter()
        .map(|step| (step.name.as_str(), step.ok))
        .collect::<HashMap<_, _>>();
    let summary = report
        .steps
        .iter()
        .find(|step| !step.ok)
        .map(|step| format!("{}: {}", step.name, step.detail))
        .or_else(|| {
            report
                .steps
                .last()
                .map(|step| format!("{}: {}", step.name, step.detail))
        })
        .unwrap_or_else(|| "No validation steps recorded.".to_string());

    let record = crate::auth::validation::ProviderValidationRecord {
        checked_at_ms: chrono::Utc::now().timestamp_millis(),
        success: report.success,
        provider_smoke_ok: step_map.get("provider_smoke").copied(),
        tool_smoke_ok: step_map.get("tool_smoke").copied(),
        summary,
    };

    if let Err(err) = crate::auth::validation::save(&report.provider, record) {
        crate::logging::warn(&format!(
            "failed to persist auth validation result for {}: {}",
            report.provider, err
        ));
    }

}
