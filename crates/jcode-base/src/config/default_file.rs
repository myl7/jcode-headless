use super::*;
use std::path::PathBuf;

impl Config {
    /// Create a default config file with comments
    pub fn create_default_config_file() -> anyhow::Result<PathBuf> {
        let path = Self::path().ok_or_else(|| anyhow::anyhow!("No config path"))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let default_content = r#"# jcode configuration file
# Location: ~/.jcode/config.toml
#
# Environment variables override these settings.
# Run `/config` in jcode to see current settings.

[display]
# Diff display mode: "off", "inline" (default), "full-inline", "pinned" (dedicated pane), or "file"
diff_mode = "inline"

# Center all content by default (default: false)
centered = false

# Pin read images to a side pane (default: true)
pin_images = true

# Wrap long lines in the pinned diff pane (default: true)
# Set to false for horizontal scrolling instead of wrapping
diff_line_wrap = true

# Queue mode: wait until assistant is done before sending next message
queue_mode = false

# Automatically reload the remote server when a newer server binary is detected (default: true)
auto_server_reload = true

# Capture mouse events (enables scroll wheel; disables terminal text selection)
mouse_capture = true

# Enable debug socket for external control/testing (default: false)
debug_socket = false

# Show thinking/reasoning content (default: true)
show_thinking = true

# How to display reasoning/thinking content: "off", "full", or "current".
#   off     - never show reasoning
#   full    - keep every reasoning trace in the transcript
#   current - show only the live reasoning; collapse it once the model commits
#             an assistant message or runs a tool, then show the next one
# When unset, falls back to show_thinking (true => full, false => off).
reasoning_display = "current"

# Markdown spacing style: "compact" (chat/TUI) or "document" (docs-like)
# markdown_spacing = "compact"

# LaTeX rendering: "none" (show source), "unicode" (terminal text), or "image" (typeset PNG, default).
# Image mode uses `latex` + `dvipng`, or `pdflatex` + `pdftocairo` as a fallback.
# When neither toolchain is available, formulas fall back to Unicode.
latex_rendering = "image"

# Show idle animation before first prompt (default: true)
idle_animation = true

# Briefly animate a user prompt line when it enters the viewport (default: true)
prompt_entry_animation = true

# Render swarm/file-activity notifications in a compact single-line form
# instead of the full multi-line card with diff preview (default: false)
# compact_notifications = false

# Show the full agentgrep tool output inline in the transcript instead of just
# the one-line summary (default: false). Useful when you want to read search
# results directly in the chat.
# show_agentgrep_output = false

# Show the dimmed technical detail (command, file path, args) next to the
# model-provided intent on tool rows (default: false). When false, tool rows
# with an intent show just the intent; rows without an intent still show the
# technical detail.
# tool_call_details = false

# Occasionally surface a "learn this keybinding" nudge (in a distinct color)
# when you keep doing something the slow way (e.g. /resume) instead of using
# its configured shortcut. Set false to disable all such hints (default: true).
# keybinding_hints = true

# Active sessions manager: pressing Left arrow on an empty input opens a
# picker scoped to live (open) sessions, showing which are still working on a
# response and which are ready for input (default: false). The /active
# command is always available regardless of this setting.
# active_sessions_manager = false

# Overscroll status line (model/provider/context info below the input):
#   "overscroll" - elastic reveal when scrolling past the bottom (default)
#   "on"         - always visible
#   "off"        - never shown
# overscroll_status = "overscroll"

# Disable specific animation variants by name.
# Examples: ["donut"] or ["donut", "orbit_rings"]
# Legacy aliases such as "three_rings" and "gyroscope" are still accepted.
# disabled_animations = []

# Performance tier: auto/full/reduced/minimal (default: auto)
# auto = detect system load, memory, terminal type, SSH, and apply extra caps for WSL/Windows Terminal
# full = all animations enabled
# reduced = skip idle animations, keep spinners
# minimal = disable all animations, slower redraw rate
# performance = "auto"

# Animation FPS (idle animation): 1-120 (default: 60)
# Runtime policy may cap this lower on slower environments such as WSL/Windows Terminal.
# animation_fps = 60

# Active redraw FPS (processing, streaming, spinners): 1-120 (default: 60)
# Runtime policy may cap this lower on slower environments such as WSL/Windows Terminal.
# redraw_fps = 60

# Label shown for the Alt/Option modifier in copy badges.
# Empty = auto ("⌥" on macOS, "Alt" elsewhere). Examples: "Option", "Alt", "⌥".
# copy_badge_alt_label = ""

[features]
# Memory: retrieval + extraction sidecar features
memory = true
# Swarm: multi-session coordination features
swarm = true
# Mermaid: render Mermaid code blocks and tell the model that diagrams are supported
mermaid = true
# Inject timestamps into user messages and tool results sent to the model
message_timestamps = true
# Persist memory injections into session history instead of sending them as request-only ephemeral context
persist_memory_injections = false
# Show an in-chat warning when a request misses the KV cache for a harness-caused
# (avoidable) reason: system prompt, tool set, or message prefix changed. These
# should essentially never happen and indicate a prefix-cache bug.
kv_cache_miss_notices = true
# Update channel: "stable" (releases only) or "main" (latest commits on push)
# Set to "main" for bleeding edge updates every time code is pushed
update_channel = "stable"

[websearch]
# Preferred websearch engine: "duckduckgo", "bing", or "searxng".
engine = "duckduckgo"
# Keyless HTML engines to try if the preferred engine fails. Default falls back to Bing HTML.
fallback_engines = ["bing"]
# Bring your own Bing Search API key for primary Bing searches. Prefer using an env var.
# Fallback Bing searches intentionally use keyless HTML search.
# bing_api_key_env = "JCODE_BING_API_KEY"
# bing_api_key = ""
# Bing market/region, for example "en-US" or "zh-CN".
bing_market = "en-US"
# SearXNG instance for the "searxng" engine. On some hosts (commonly Linux),
# DuckDuckGo and Bing block scraped requests via TLS fingerprinting / IP
# reputation and return an anti-bot page with no results. Pointing at a SearXNG
# instance (self-hosted or trusted public) with the JSON format enabled avoids
# this. Configure here or via the JCODE_SEARXNG_URL environment variable, then
# set engine = "searxng" or add it to fallback_engines.
# searxng_url = "https://searx.example.org"

[tools]
# Controls which built-in tools are sent to the model.
# Profiles: "full" (default), "acp", "minimal"/"lite", or "none".
# acp keeps core coding tools plus batch for generic ACP clients.
# minimal keeps core coding tools only: bash, read, write, edit, multiedit,
# apply_patch, patch, agentgrep, glob, grep, and ls.
profile = "full"
# Explicit allow-list. When non-empty, only these tools are exposed.
# enabled = ["bash", "read", "write", "apply_patch", "agentgrep", "ls"]
# All built-in tools, including gmail, are exposed by the full profile.
# Use enabled = ["*"] to explicitly select the unrestricted full toolset.
# Hide selected tools after applying the profile/allow-list.
# disabled = ["browser", "gmail", "swarm"]
# Disable all built-in tools unless enabled is set.
disable_base_tools = false

[acp]
# Agent Client Protocol adapter compatibility profile: standard, extended, or full.
# standard emits only spec-compatible ACP messages.
# extended/full additionally emit ignorable _jcode/* extension notifications.
profile = "standard"
# Tool profile requested when `jcode acp` starts the daemon itself.
# Existing daemons keep their current server-wide tool config.
tool_profile = "acp"

[provider]
# Default model (optional, uses provider default if not set)
# Set via /model picker with Ctrl+B to save as default
# default_model = "claude-fable-5"
# Default provider (optional: claude|anthropic-api|openai|openai-api|copilot|openrouter|...)
# When set, this provider is preferred on startup if available.
#   claude        = Claude via OAuth/subscription (token in ~/.jcode/auth.json)
#   anthropic-api = Claude via direct Anthropic API key (ANTHROPIC_API_KEY env
#                   or ~/.config/jcode/anthropic.env). API-key mode does NOT fall
#                   back to OAuth; configure the key first.
# `claude` and `anthropic-api` are distinct providers with distinct credentials.
# See docs/AUTH_CREDENTIAL_SOURCES.md for where each credential lives.
# default_provider = "copilot"
# OpenAI reasoning effort (none|minimal|low|medium|high|xhigh|max)
openai_reasoning_effort = "low"
# Anthropic reasoning effort for Claude reasoning models (none|low|medium|high|xhigh|max)
# xhigh needs Opus 4.7/4.8 or Fable 5; max needs an output_config effort model (Opus/Sonnet 4.6+).
# Defaults to xhigh for Claude Opus 4.7/4.8 (high on older Opus) when unset; other models keep their own default.
# anthropic_reasoning_effort = "medium"
# OpenAI transport mode (auto|websocket|https)
# openai_transport = "auto"
# OpenAI service tier override (priority|flex|off)
# Defaults to `priority` to match Codex /fast behavior for OpenAI OAuth
# (higher speed, higher usage). Set to "off" (or "standard") to disable.
openai_service_tier = "priority"
# Preserve provider-native reasoning/thinking for future-turn context when supported.
# Applies to OpenRouter, Anthropic, and OpenAI native reasoning replay. Display is separate.
preserve_reasoning_context = true
# Cross-provider failover when the same prompt would be resent elsewhere.
# countdown = 3-second countdown before retrying on another provider; press Esc to cancel (default)
# manual = show a notice and let you switch yourself
# cross_provider_failover = "manual"
# Try another account on the same provider before switching providers (default: true)
# same_provider_account_failover = false
cross_provider_failover = "countdown"
# Copilot premium mode: "normal" (default), "one" (first msg only), "zero" (all free)
# Set to "zero" if you have premium Copilot and want free requests
# copilot_premium = "zero"
# Only list these providers in the /model picker (issue #460). Entries match
# provider labels ("openai", "anthropic", "copilot", "openrouter", ...), route
# api methods ("claude-oauth", "openai-compatible:myprofile"), or bare
# openai-compatible profile ids ("myprofile"). The active model's routes always
# stay visible. Unset or empty = show everything.
# model_picker_providers = ["myprofile", "openrouter"]
# Max seconds to wait for streaming data before timing out a request with no
# data received. Raise this for slow reasoning models (e.g. DeepSeek) that think
# silently for minutes before emitting tokens. Default: 180.
# Applies to every streaming provider path (OpenAI native, Anthropic, Copilot,
# OpenRouter/OpenAI-compatible). The TUI's client-side stall guard also extends
# to match this value. Also overridable per-launch via JCODE_STREAM_IDLE_TIMEOUT_SECS.
# stream_idle_timeout_secs = 600

[agents]
# Defaults for spawned helper agents (swarm workers, subagents, sidecars).
# All keys are optional; the values below are the built-in defaults.
#
# Default model for spawned swarm/subagent sessions.
# Leave unset (or "inherit"/"coordinator") so workers inherit the model of the
# session that spawned them. Set a concrete model only to pin every worker to it.
# Env override: JCODE_SWARM_MODEL
# swarm_model = "inherit"
#
# Swarm-created agents always run in-process without a UI.
# Env override: JCODE_SWARM_SPAWN_MODE
swarm_spawn_mode = "headless"
#
# Max live swarm worker agents in one swarm. This RAM-safety budget applies to
# recursive ad hoc spawning and deep-mode run_plan parallelism. Completed/stopped
# workers free their slots. 0 disables this guard and leaves only the absolute
# per-swarm hard cap of 1000. Light mode uses a smaller fixed fan-out.
# Env override: JCODE_SWARM_MAX_CONCURRENT_AGENTS
swarm_max_concurrent_agents = 32
#
# Max percentage (1-90) of the chat height the inline swarm gallery band may use.
# Unset = built-in default (40%). Lower values keep more transcript visible; set
# near the minimum to collapse the gallery to a thin strip.
# swarm_gallery_max_pct = 40
#
# Layout of the inline swarm strip above the status line:
#   "vertical"   - one agent per row (session icon + status + task), capped to
#                  a few rows with a "+N more" overflow marker (default)
#   "horizontal" - all agents packed as chips on a single row
# Env override: JCODE_SWARM_STRIP_LAYOUT
# swarm_strip_layout = "vertical"
#
# Model for the memory sidecar (relevance/extraction). Unset = sidecar auto-select
# (OpenAI defaults to gpt-5.6-luna with reasoning effort "none").
# Env override: JCODE_MEMORY_MODEL
# memory_model = "gpt-5.6-luna"
#
# Whether the memory sidecar (LLM precision judge) handles relevance/extraction.
# Default true: the LLM precision-judge path is the only reliably productive
# memory mode. Set false only to opt into the lower-precision no-LLM hybrid path.
# When this is true but no LLM backend is reachable (logged out), memory goes
# dormant instead of degrading to the no-LLM path. Env: JCODE_MEMORY_SIDECAR_ENABLED
# memory_sidecar_enabled = true
#
# Minimum turns between Mode-2 memory reranks (cadence floor). The expensive
# listwise LLM rerank runs at most once per this many turns; skipped turns fall
# back to hybrid-ordered surfacing. A topic change always forces a rerank. Set 1
# to rerank every turn. Default 3.
# memory_rerank_cadence = 3
#
# High-precision consensus rerank: run N independent LLM judges per fired rerank
# and inject only memories that >= memory_rerank_min_agree of them agree on.
# Default 2 judges / 2 agreement -> injection precision ~1.0 with ~100% clean
# (zero memory) on no-memory turns, at 2 LLM calls per fired turn. Set votes=1
# for the cheaper single-judge path (precision ~0.77).
# memory_rerank_votes = 2
# memory_rerank_min_agree = 2
#
# Embedding backend for memory dense-retrieval. "local" (default) uses the
# bundled all-MiniLM-L6-v2 ONNX model (no network); "openai" uses a remote
# OpenAI / OpenAI-compatible /v1/embeddings endpoint (requires OPENAI_API_KEY;
# silently falls back to local when no key is found). Vectors from different
# models live in separate spaces and are never compared, so switching is safe.
# Env override: JCODE_MEMORY_EMBEDDING_BACKEND
# memory_embedding_backend = "local"
# memory_embedding_model = "text-embedding-3-small"
# memory_embedding_base_url = "https://api.openai.com/v1"
# memory_embedding_dim = 1536

[hooks]
# Lifecycle hooks: external commands jcode runs at well-defined points so other
# programs can observe or gate agent behavior. Commands are parsed shell-style
# (quotes work) but executed directly, with JCODE_HOOK_* env vars describing
# the event:
#   JCODE_HOOK_EVENT       - "turn_start", "turn_end", "session_start",
#                            "session_end", "pre_tool", "post_tool"
#   JCODE_HOOK_SESSION_ID  - the session the event belongs to
#   JCODE_HOOK_CWD         - session working directory (also the hook's cwd)
#   JCODE_HOOK_PAYLOAD     - JSON mirror of all fields
# Hook processes get JCODE_HOOKS_DISABLED=1 so nested jcode calls don't recurse.
#
# All hooks except pre_tool are observers: detached, fire-and-forget, failures
# only logged. Env overrides: JCODE_HOOK_TURN_START, JCODE_HOOK_TURN_END,
# JCODE_HOOK_SESSION_START, JCODE_HOOK_SESSION_END, JCODE_HOOK_PRE_TOOL,
# JCODE_HOOK_POST_TOOL (set empty to disable a config hook).
#
# Runs when an agent turn begins, before the model starts generating and before
# the first pre_tool. Lets integrations detect the agent is working during the
# think/stream window before any tool call. Extra fields: JCODE_HOOK_MODEL,
# JCODE_HOOK_SOURCE ("chat"/"resume"/"ambient").
# turn_start = "~/bin/jcode-turn-start"
#
# Runs when an agent turn completes. Extra fields: JCODE_HOOK_STATUS
# ("ok"/"error"), JCODE_HOOK_DURATION_MS, JCODE_HOOK_MODEL,
# JCODE_HOOK_LAST_ASSISTANT_TEXT (first 4000 chars), JCODE_HOOK_ERROR.
# turn_end = "~/bin/jcode-turn-notify"
#
# Runs when a session becomes active. Extra: JCODE_HOOK_SOURCE
# ("create"/"attach"/"resume").
# session_start = ""
#
# Runs when a session closes normally. Extra: JCODE_HOOK_SOURCE ("close").
# session_end = ""
#
# Gate hook before every tool call. Receives JCODE_HOOK_TOOL_NAME and the tool
# input JSON on stdin (truncated copy in JCODE_HOOK_TOOL_INPUT). Exit 0 allows
# the call; exit 2 blocks it and stderr is shown to the model as the error;
# any other outcome (other exits, timeout, missing binary) fails open.
# pre_tool = "~/bin/jcode-tool-policy"
#
# Max milliseconds to wait for pre_tool before failing open (default: 5000).
# pre_tool_timeout_ms = 5000
#
# Runs after each tool call. Extra fields: JCODE_HOOK_TOOL_NAME,
# JCODE_HOOK_STATUS, JCODE_HOOK_DURATION_MS, JCODE_HOOK_OUTPUT_BYTES,
# JCODE_HOOK_ERROR.
# post_tool = ""

[ambient]
# Ambient mode: background agent that maintains your codebase
# Enable ambient mode (default: false)
enabled = false
# Provider override (default: auto-select based on available credentials)
# provider = "claude"
# Model override (default: provider's strongest)
# model = "claude-sonnet-4-20250514"
# Allow API key usage (default: false, only OAuth to avoid surprise costs)
allow_api_keys = false
# Daily token budget when using API keys (optional)
# api_daily_budget = 100000
# Minimum interval between cycles in minutes
min_interval_minutes = 5
# Maximum interval between cycles in minutes
max_interval_minutes = 120
# Pause ambient when user has active session
pause_on_active_session = true
# Enable proactive work (new features, refactoring) vs garden-only (lint, format, deps)
proactive_work = true
# Branch prefix for proactive work
work_branch_prefix = "ambient/"
[power]
# Prevent automatic system sleep while any jcode session is actively working.
# Linux also blocks lid-switch suspend. Windows still respects explicit lid-close
# and power-button actions from your active power plan. The display may sleep.
# The guard is held only for as long as work is in flight. (default: true)
# Set JCODE_DISABLE_POWER_INHIBIT=1 to force-disable regardless of this setting.
prevent_sleep_while_streaming = true

[safety]
# Notification settings for ambient mode events

# ntfy.sh push notifications (free, phone app: https://ntfy.sh)
# ntfy_topic = "jcode-ambient-your-secret-topic"
# ntfy_server = "https://ntfy.sh"


# Email notifications via SMTP
# email_enabled = false
# email_to = "you@example.com"
# email_from = "jcode@example.com"
# email_smtp_host = "smtp.gmail.com"
# email_smtp_port = 587
# Password via env: JCODE_SMTP_PASSWORD (preferred) or config below
# email_password = ""

# IMAP for email replies (reply to ambient emails to send directives)
# email_reply_enabled = false
# email_imap_host = "imap.gmail.com"
# email_imap_port = 993

# Telegram notifications via Bot API (free, https://telegram.org)
# telegram_enabled = false
# telegram_bot_token = ""  # From @BotFather (prefer JCODE_TELEGRAM_BOT_TOKEN env var)
# telegram_chat_id = ""    # Your user/chat ID
# telegram_reply_enabled = false  # Reply to bot messages to send directives

# Discord notifications via Bot API (https://discord.com/developers)
# discord_enabled = false
# discord_bot_token = ""     # From Discord Developer Portal (prefer JCODE_DISCORD_BOT_TOKEN env var)
# discord_channel_id = ""    # Channel ID to post in
# discord_bot_user_id = ""   # Bot's user ID (for filtering own messages)
# discord_reply_enabled = false  # Messages in channel become agent directives

# Jade cloud relay (outbound-only long polling, disabled by default).
# Prefer environment variables for secrets:
# JCODE_JADE_RELAY_API_BASE, JCODE_JADE_RELAY_TOKEN, JCODE_JADE_RELAY_TOKEN_ID,
# JCODE_JADE_RELAY_USER_ID, JCODE_JADE_RELAY_SESSION_ID.
# jade_relay_enabled = false
# jade_relay_reply_enabled = false   # Deliver cloud prompts to one configured live session.

# [sponsors] # Legacy config section name retained for compatibility.
# Tool partner discovery (enabled by default; set enabled = false to opt out).
# When enabled, the agent gains a `discover_tools` tool listing third-party
# developer tools from Jcode's hosted partner directory. Some partners may
# share revenue with Jcode when a referred user becomes a customer, but
# partnership status never influences recommendations. Each session's first
# use of discover_tools shows a concise disclosure with a learn-more link.
# See https://jcode.sh/discovery-tools
# enabled = true
# endpoint = "https://api.jcode.sh/v1/discovery"
	"#;

        std::fs::write(&path, default_content)?;
        Ok(path)
    }
}
