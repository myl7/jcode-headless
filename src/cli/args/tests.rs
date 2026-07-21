use super::*;
use crate::cli::provider_init::ProviderChoice;

#[test]
fn test_provider_choice_aliases_parse() {
    let args = Args::try_parse_from(["jcode", "--provider", "z.ai", "run", "smoke"]).unwrap();
    assert_eq!(args.provider, ProviderChoice::Zai);

    let args =
        Args::try_parse_from(["jcode", "--provider", "kimi-for-coding", "run", "smoke"]).unwrap();
    assert_eq!(args.provider, ProviderChoice::Kimi);

    let args =
        Args::try_parse_from(["jcode", "--provider", "cerebrascode", "run", "smoke"]).unwrap();
    assert_eq!(args.provider, ProviderChoice::Cerebras);

    let args = Args::try_parse_from(["jcode", "--provider", "compat", "run", "smoke"]).unwrap();
    assert_eq!(args.provider, ProviderChoice::OpenaiCompatible);

    let args = Args::try_parse_from(["jcode", "--provider", "bailian", "run", "smoke"]).unwrap();
    assert_eq!(args.provider, ProviderChoice::AlibabaCodingPlan);

    let args = Args::try_parse_from(["jcode", "--provider", "together", "run", "smoke"]).unwrap();
    assert_eq!(args.provider, ProviderChoice::TogetherAi);

    let args = Args::try_parse_from(["jcode", "--provider", "grok", "run", "smoke"]).unwrap();
    assert_eq!(args.provider, ProviderChoice::Xai);

    let args = Args::try_parse_from(["jcode", "--provider", "cgc", "run", "smoke"]).unwrap();
    assert_eq!(args.provider, ProviderChoice::Comtegra);
}

#[test]
fn serve_server_name_option_parses() {
    let args =
        Args::try_parse_from(["jcode", "serve", "--server-name", "mount-cloud/fabian"]).unwrap();
    match args.command {
        Some(Command::Serve { server_name, .. }) => {
            assert_eq!(server_name.as_deref(), Some("mount-cloud/fabian"));
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn model_list_subcommand_parses() {
    let args = Args::try_parse_from(["jcode", "model", "list", "--json", "--verbose"]).unwrap();
    match args.command {
        Some(Command::Model(ModelCommand::List { json, verbose })) => {
            assert!(json);
            assert!(verbose);
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn session_rename_subcommand_parses() {
    let args = Args::try_parse_from([
        "jcode",
        "session",
        "rename",
        "fox",
        "release planning",
        "--json",
    ])
    .unwrap();
    match args.command {
        Some(Command::Session(SessionCommand::Rename {
            session,
            name,
            clear,
            json,
        })) => {
            assert_eq!(session, "fox");
            assert_eq!(name.as_deref(), Some("release planning"));
            assert!(!clear);
            assert!(json);
        }
        other => panic!("unexpected command: {:?}", other),
    }

    let args = Args::try_parse_from(["jcode", "session", "rename", "fox", "--clear"]).unwrap();
    match args.command {
        Some(Command::Session(SessionCommand::Rename {
            session,
            name,
            clear,
            json,
        })) => {
            assert_eq!(session, "fox");
            assert!(name.is_none());
            assert!(clear);
            assert!(!json);
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn quiet_global_flag_parses() {
    let args = Args::try_parse_from(["jcode", "--quiet", "model", "list"]).unwrap();
    assert!(args.quiet);
}

#[test]
fn run_json_subcommand_parses() {
    let args = Args::try_parse_from(["jcode", "run", "--json", "hello"]).unwrap();
    match args.command {
        Some(Command::Run {
            json,
            ndjson,
            message,
        }) => {
            assert!(json);
            assert!(!ndjson);
            assert_eq!(message, "hello");
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn run_ndjson_subcommand_parses() {
    let args = Args::try_parse_from(["jcode", "run", "--ndjson", "hello"]).unwrap();
    match args.command {
        Some(Command::Run {
            json,
            ndjson,
            message,
        }) => {
            assert!(!json);
            assert!(ndjson);
            assert_eq!(message, "hello");
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn version_subcommand_parses() {
    let args = Args::try_parse_from(["jcode", "version", "--json"]).unwrap();
    match args.command {
        Some(Command::Version { json }) => assert!(json),
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn usage_subcommand_parses() {
    let args = Args::try_parse_from(["jcode", "usage", "--json"]).unwrap();
    match args.command {
        Some(Command::Usage { json }) => assert!(json),
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn auth_status_subcommand_parses() {
    let args = Args::try_parse_from(["jcode", "auth", "status", "--json"]).unwrap();
    match args.command {
        Some(Command::Auth(AuthCommand::Status { json })) => assert!(json),
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn auth_import_subcommand_parses() {
    let args =
        Args::try_parse_from(["jcode", "auth", "import", "openai", "--stdin", "--json"]).unwrap();
    match args.command {
        Some(Command::Auth(AuthCommand::Import {
            provider,
            stdin,
            json,
        })) => {
            assert_eq!(provider, "openai");
            assert!(stdin);
            assert!(json);
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn auth_doctor_subcommand_parses() {
    let args = Args::try_parse_from(["jcode", "auth", "doctor", "openai", "--validate", "--json"])
        .unwrap();
    match args.command {
        Some(Command::Auth(AuthCommand::Doctor {
            provider,
            validate,
            json,
        })) => {
            assert_eq!(provider.as_deref(), Some("openai"));
            assert!(validate);
            assert!(json);
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn provider_list_subcommand_parses() {
    let args = Args::try_parse_from(["jcode", "provider", "list", "--json"]).unwrap();
    match args.command {
        Some(Command::Provider(ProviderCommand::List { json })) => assert!(json),
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn provider_current_subcommand_parses() {
    let args = Args::try_parse_from(["jcode", "provider", "current", "--json"]).unwrap();
    match args.command {
        Some(Command::Provider(ProviderCommand::Current { json })) => assert!(json),
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn provider_add_subcommand_parses_agent_friendly_flags() {
    let args = Args::try_parse_from([
        "jcode",
        "provider",
        "add",
        "my-api",
        "--base-url",
        "https://llm.example.com/v1",
        "--model",
        "model-a",
        "--context-window",
        "128000",
        "--api-key-stdin",
        "--auth",
        "bearer",
        "--set-default",
        "--json",
    ])
    .unwrap();

    match args.command {
        Some(Command::Provider(ProviderCommand::Add {
            name,
            base_url,
            model,
            context_window,
            api_key_stdin,
            auth,
            set_default,
            json,
            ..
        })) => {
            assert_eq!(name, "my-api");
            assert_eq!(base_url, "https://llm.example.com/v1");
            assert_eq!(model, "model-a");
            assert_eq!(context_window, Some(128000));
            assert!(api_key_stdin);
            assert_eq!(auth, Some(ProviderAuthArg::Bearer));
            assert!(set_default);
            assert!(json);
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

/// Contract test for the onboarding agent-repair brief (see
/// `jcode-tui::tui::app::onboarding_repair::build_repair_brief`). The brief
/// tells a coding agent to run these exact commands to diagnose and fix a
/// failed login. If any flag here stops parsing, the brief would hand the agent
/// a broken command, so this guards the agent-facing CLI contract.
#[test]
fn onboarding_repair_brief_commands_are_valid_cli() {
    // Diagnose.
    Args::try_parse_from(["jcode", "auth", "doctor"]).expect("auth doctor must parse");
    Args::try_parse_from(["jcode", "auth", "doctor", "openai", "--validate", "--json"])
        .expect("auth doctor --validate --json must parse");

    // Fix: custom OpenAI-compatible endpoint via provider add + key on stdin.
    Args::try_parse_from([
        "jcode",
        "provider",
        "add",
        "my-endpoint",
        "--base-url",
        "https://api.example.com/v1",
        "--model",
        "some-model",
        "--api-key-stdin",
    ])
    .expect("provider add --base-url --model --api-key-stdin must parse");
}
