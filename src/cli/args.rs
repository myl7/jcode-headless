use clap::{Parser, Subcommand, ValueEnum};

use super::provider_init::ProviderChoice;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum ProviderAuthArg {
    /// Send the API key as Authorization: Bearer <key> (OpenAI-compatible default)
    Bearer,
    /// Send the API key in an API-key header (defaults to api-key)
    ApiKey,
    /// Do not send authentication, useful for localhost model servers
    None,
}

#[derive(Parser, Debug)]
#[command(name = "jcode")]
#[command(version = jcode_build_meta::version())]
#[command(about = "J-Code: A coding agent using Claude Max or ChatGPT Pro subscriptions")]
pub(crate) struct Args {
    /// Provider to use (jcode, claude, openai, openai-api, openrouter, azure, opencode, opencode-go, zai, 302ai, baseten, cortecs, comtegra, deepseek, fpt, firmware, huggingface, moonshotai, nebius, scaleway, stackit, groq, mistral, perplexity, togetherai, deepinfra, xai, nvidia-nim, lmstudio, ollama, chutes, cerebras, alibaba-coding-plan, openai-compatible, cursor, copilot, gemini, antigravity, google, or auto-detect)
    #[arg(short, long, default_value = "auto", global = true)]
    pub(crate) provider: ProviderChoice,

    /// Working directory for the local client process
    #[arg(short = 'C', long, global = true)]
    pub(crate) cwd: Option<String>,

    /// Log tool inputs/outputs and token usage to stderr
    #[arg(long, global = true)]
    pub(crate) trace: bool,

    /// Suppress non-error CLI/status output for scripting and wrappers
    #[arg(long, global = true)]
    pub(crate) quiet: bool,

    /// Resume a session by ID for commands that support session continuation
    #[arg(long, global = true)]
    pub(crate) resume: Option<String>,

    /// Custom socket path for server/client communication
    #[arg(long, global = true)]
    pub(crate) socket: Option<String>,

    /// Enable the local debug socket
    #[arg(long, global = true)]
    pub(crate) debug_socket: bool,

    /// Model to use (e.g., claude-opus-4-6, gpt-5.5)
    #[arg(short, long, global = true)]
    pub(crate) model: Option<String>,

    /// Named provider profile from [providers.<name>] in config.toml.
    /// Implies --provider openai-compatible for OpenAI-compatible profiles.
    #[arg(long, global = true)]
    pub(crate) provider_profile: Option<String>,

    /// Tool profile to expose to the model: full, minimal/lite, or none.
    #[arg(long, global = true)]
    pub(crate) tool_profile: Option<String>,

    /// Comma-separated explicit allow-list of tools to expose, e.g. bash,read,write,apply_patch. Use '*' or 'all' for the unrestricted full toolset.
    #[arg(long, global = true)]
    pub(crate) tools: Option<String>,

    /// Comma-separated list of tools to hide after applying the selected profile.
    #[arg(long, global = true)]
    pub(crate) disabled_tools: Option<String>,

    /// Hide all built-in tools unless --tools or [tools].enabled opts tools back in.
    #[arg(long, global = true)]
    pub(crate) disable_base_tools: bool,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Start the agent server (background daemon)
    Serve {
        /// Internal: mark this server as temporary so it can self-clean when its owner exits.
        #[arg(long, hide = true)]
        temporary_server: bool,

        /// Internal: owning process pid for a temporary server.
        #[arg(long, hide = true)]
        owner_pid: Option<u32>,

        /// Internal: idle shutdown timeout in seconds for a temporary server.
        #[arg(long, hide = true)]
        temp_idle_timeout_secs: Option<u64>,

        /// Stable name used in logs, session metadata, and debug output.
        ///
        /// Useful for long-lived remote runtimes, e.g. `fabian`, `john`, or
        /// `mount-cloud-fabian`. Unsafe characters are normalized before use.
        #[arg(long)]
        server_name: Option<String>,
    },

    /// Run a single message and exit
    Run {
        /// Emit a machine-readable JSON result instead of streaming text
        #[arg(long, conflicts_with = "ndjson")]
        json: bool,

        /// Emit newline-delimited JSON events while the response streams
        #[arg(long, conflicts_with = "json")]
        ndjson: bool,

        /// The message to send
        message: String,
    },

    /// Show build/version information in human or JSON form
    Version {
        /// Emit JSON instead of plain text
        #[arg(long)]
        json: bool,
    },

    /// Show usage limits for connected providers
    Usage {
        /// Emit JSON instead of plain text
        #[arg(long)]
        json: bool,
    },

    /// Debug socket CLI - interact with running jcode server
    Debug {
        /// Debug command to run (list, start, sessions, create_session, message, tool, state, history, etc.)
        #[arg(default_value = "help")]
        command: String,

        /// Optional argument for the command
        #[arg(default_value = "")]
        arg: String,

        /// Target a specific session by ID
        #[arg(short = 'S', long)]
        session: Option<String>,

        /// Connect to specific server socket path
        #[arg(short = 's', long)]
        socket: Option<String>,

        /// Wait for response to complete (for message command)
        #[arg(short, long)]
        wait: bool,
    },

    /// Authentication status and validation helpers
    #[command(subcommand)]
    Auth(AuthCommand),

    /// Provider discovery and selection helpers
    #[command(subcommand)]
    Provider(ProviderCommand),

    /// Memory management commands
    #[command(subcommand)]
    Memory(MemoryCommand),

    /// Session management commands
    #[command(subcommand)]
    Session(SessionCommand),

    /// Model management commands
    #[command(subcommand)]
    Model(ModelCommand),

    /// Show live verification coverage. With no provider/model, prints the full coverage summary.
    #[command(name = "provider-test-coverage", alias = "model-status")]
    ProviderTestCoverage {
        /// Provider to look up. Omit provider and model to print the full coverage summary.
        #[arg(value_name = "PROVIDER")]
        provider_query: Option<String>,

        /// Model to look up. Defaults to the global --model value only when PROVIDER is supplied.
        #[arg(value_name = "MODEL")]
        model_query: Option<String>,

        /// Read coverage from this JSON file instead of the default live-test coverage ledger
        #[arg(long)]
        coverage_file: Option<String>,

        /// Maximum provider/model pairs to list in the full summary (0 = show all)
        #[arg(long, default_value_t = 0)]
        coverage_limit: usize,
    },

    /// Diagnose provider/model routing through catalog, switching, chat, streaming, and tools.
    #[command(name = "provider-doctor", alias = "provider-strict-e2e")]
    ProviderDoctor {
        /// OpenAI-compatible provider id to diagnose (e.g. cerebras, fpt, nvidia-nim)
        #[arg(id = "doctor_provider", value_name = "PROVIDER")]
        provider: String,

        /// How much to exercise: offline (no key/no spend), catalog (key, ~no spend),
        /// or full (key, spends balance: chat + streaming + tools).
        #[arg(long, value_name = "TIER", default_value = "catalog")]
        tier: String,

        /// Emit the report as JSON for scripting
        #[arg(long)]
        json: bool,
    },

    /// Test mounted credentials end-to-end: probe, refresh, and provider smoke
    AuthTest {
        /// Test all currently configured supported auth providers instead of just --provider
        #[arg(long)]
        all_configured: bool,

        /// Skip the provider runtime smoke prompt
        #[arg(long)]
        no_smoke: bool,

        /// Skip the tool-enabled runtime smoke prompt (the same request path used during normal chat)
        #[arg(long)]
        no_tool_smoke: bool,

        /// Custom smoke prompt (default asks for AUTH_TEST_OK)
        #[arg(long)]
        prompt: Option<String>,

        /// Emit JSON report instead of human-readable output
        #[arg(long)]
        json: bool,

        /// Write the full auth-test report JSON to a file
        #[arg(long)]
        output: Option<String>,

        /// Show strict live provider/model E2E coverage instead of running auth tests
        #[arg(long, conflicts_with_all = ["all_configured", "no_smoke", "no_tool_smoke", "prompt"])]
        coverage: bool,

        /// Fetch live model catalogs and verify context-window resolution for each model with metadata
        #[arg(long, conflicts_with_all = ["no_smoke", "no_tool_smoke", "prompt", "coverage"])]
        context_audit: bool,

        /// Read coverage from this JSON file instead of the default live-test coverage ledger
        #[arg(long, requires = "coverage")]
        coverage_file: Option<String>,

        /// Maximum uncovered provider/model gaps to show in the text coverage report
        #[arg(long, requires = "coverage", default_value_t = 50)]
        coverage_limit: usize,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ModelCommand {
    /// List model names you can pass to -m/--model
    List {
        /// Emit JSON instead of plain text
        #[arg(long)]
        json: bool,

        /// Show provider/selection summary before the list
        #[arg(long)]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum SessionCommand {
    /// Rename a saved session's human-readable name/title
    Rename {
        /// Session ID or memorable short name, e.g. fox
        session: String,

        /// New session name/title
        #[arg(required_unless_present = "clear")]
        name: Option<String>,

        /// Clear the custom session name/title
        #[arg(long, conflicts_with = "name")]
        clear: bool,

        /// Emit JSON instead of human-readable output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ProviderCommand {
    /// List provider IDs you can pass to -p/--provider
    List {
        /// Emit JSON instead of plain text
        #[arg(long)]
        json: bool,
    },

    /// Show the currently requested and resolved provider selection
    Current {
        /// Emit JSON instead of plain text
        #[arg(long)]
        json: bool,
    },

    /// Add a named OpenAI-compatible API provider profile
    Add {
        /// Profile name used with --provider-profile and config defaults, e.g. my-gateway
        name: String,

        /// OpenAI-compatible API base URL, e.g. https://llm.example.com/v1
        #[arg(long, alias = "api-base")]
        base_url: String,

        /// Default model id for this provider profile
        #[arg(short, long)]
        model: String,

        /// Optional model context window in tokens
        #[arg(long)]
        context_window: Option<usize>,

        /// Environment variable name that contains the API key
        #[arg(long, conflicts_with = "no_api_key")]
        api_key_env: Option<String>,

        /// API key value to store in jcode's private provider env file. Prefer --api-key-stdin for shell history safety.
        #[arg(long, conflicts_with_all = ["api_key_stdin", "no_api_key"])]
        api_key: Option<String>,

        /// Read the API key from stdin and store it in jcode's private provider env file
        #[arg(long, conflicts_with = "no_api_key")]
        api_key_stdin: bool,

        /// Configure the provider with no API key/authentication
        #[arg(long, conflicts_with_all = ["api_key", "api_key_stdin", "api_key_env"])]
        no_api_key: bool,

        /// Authentication style for the API key
        #[arg(long, value_enum)]
        auth: Option<ProviderAuthArg>,

        /// Header name when --auth api-key is used (default: api-key)
        #[arg(long)]
        auth_header: Option<String>,

        /// Private env file name under jcode's app config directory for stored API keys
        #[arg(long)]
        env_file: Option<String>,

        /// Make this profile the startup default provider/model
        #[arg(long, alias = "default")]
        set_default: bool,

        /// Replace an existing profile with the same name
        #[arg(long)]
        overwrite: bool,

        /// Allow provider-routing features for OpenRouter-style gateways
        #[arg(long)]
        provider_routing: bool,

        /// Fetch/list models from the provider's /models endpoint
        #[arg(long)]
        model_catalog: bool,

        /// Emit JSON instead of human-readable setup output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum AuthCommand {
    /// Import an already-authenticated subscription credential into JCODE_HOME
    Import {
        /// Subscription provider: claude or openai
        #[arg(id = "auth_import_provider", value_name = "PROVIDER")]
        provider: String,

        /// Read the authenticated credential JSON from stdin
        #[arg(long)]
        stdin: bool,

        /// Emit JSON instead of plain text
        #[arg(long)]
        json: bool,
    },
    /// Show configured authentication status for model/tool providers
    Status {
        /// Emit JSON instead of plain text
        #[arg(long)]
        json: bool,
    },
    /// Diagnose provider auth issues and suggest next steps
    Doctor {
        /// Optional provider id or alias to focus diagnosis on one provider
        #[arg(id = "auth_provider", value_name = "PROVIDER")]
        provider: Option<String>,

        /// Run live credential/runtime validation for configured providers
        #[arg(long)]
        validate: bool,

        /// Emit JSON instead of plain text
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum MemoryCommand {
    /// List all stored memories
    List {
        /// Filter by scope (project, global, all)
        #[arg(short, long, default_value = "all")]
        scope: String,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },

    /// Search memories by query
    Search {
        /// Search query
        query: String,

        /// Use semantic search (embedding-based) instead of keyword
        #[arg(short, long)]
        semantic: bool,
    },

    /// Export memories to a JSON file
    Export {
        /// Output file path
        output: String,

        /// Export scope (project, global, all)
        #[arg(short, long, default_value = "all")]
        scope: String,
    },

    /// Import memories from a JSON file
    Import {
        /// Input file path
        input: String,

        /// Import scope (project, global)
        #[arg(short, long, default_value = "project")]
        scope: String,

        /// Overwrite existing memories with same ID
        #[arg(long)]
        overwrite: bool,
    },

    /// Show memory statistics
    Stats,

    /// Clear test memory storage (used by debug sessions)
    ClearTest,
}

#[cfg(test)]
mod tests;
