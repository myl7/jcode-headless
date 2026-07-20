use serde::{Deserialize, Serialize};

/// Compaction mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompactionMode {
    /// Compact when context hits a fixed threshold (default)
    #[default]
    Reactive,
    /// Compact early based on predicted token growth rate
    Proactive,
    /// Compact based on semantic topic shifts and relevance scoring
    Semantic,
}

impl CompactionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reactive => "reactive",
            Self::Proactive => "proactive",
            Self::Semantic => "semantic",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "reactive" => Some(Self::Reactive),
            "proactive" => Some(Self::Proactive),
            "semantic" => Some(Self::Semantic),
            _ => None,
        }
    }
}

/// How to display file diffs from edit/write tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffDisplayMode {
    /// Don't show diffs at all.
    Off,
    /// Show diffs inline in the chat (default).
    #[default]
    Inline,
    /// Show the full inline diff in the chat without preview truncation.
    #[serde(
        rename = "full-inline",
        alias = "full_inline",
        alias = "fullinline",
        alias = "inline-full",
        alias = "inline_full",
        alias = "inlinefull",
        alias = "full"
    )]
    FullInline,
    /// Show diffs in a dedicated pinned pane.
    Pinned,
    /// Show full file with diff highlights in side panel, synced to scroll position.
    File,
}

impl DiffDisplayMode {
    pub fn is_inline(&self) -> bool {
        matches!(self, Self::Inline | Self::FullInline)
    }

    pub fn is_full_inline(&self) -> bool {
        matches!(self, Self::FullInline)
    }

    pub fn is_pinned(&self) -> bool {
        matches!(self, Self::Pinned)
    }

    pub fn is_file(&self) -> bool {
        matches!(self, Self::File)
    }

    pub fn has_side_pane(&self) -> bool {
        matches!(self, Self::Pinned | Self::File)
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Inline,
            Self::Inline => Self::FullInline,
            Self::FullInline => Self::Pinned,
            Self::Pinned => Self::File,
            Self::File => Self::Off,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Inline => "Inline",
            Self::FullInline => "Inline Full",
            Self::Pinned => "Pinned",
            Self::File => "File",
        }
    }
}

/// When to show the overscroll status line (model/provider/context info below
/// the input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverscrollStatusMode {
    /// Never show the status line.
    Off,
    /// Always show the status line below the input.
    On,
    /// Elastic reveal: show it briefly when scrolling past the bottom (default).
    #[default]
    Overscroll,
}

impl OverscrollStatusMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
            Self::Overscroll => "overscroll",
        }
    }
}

/// How to display mermaid diagrams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagramDisplayMode {
    /// Don't show diagrams in dedicated widgets (only inline in messages).
    #[default]
    None,
    /// Show diagrams in info widget margins (opportunistic, if space available).
    Margin,
    /// Show diagrams in a dedicated pinned pane (forces space allocation).
    Pinned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagramPanePosition {
    #[default]
    Side,
    Top,
}

/// How much vertical spacing to use when rendering markdown blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkdownSpacingMode {
    /// Compact chat/TUI-oriented spacing.
    #[default]
    Compact,
    /// Document-style spacing between top-level blocks.
    Document,
}

/// How LaTeX math is rendered in terminal markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LatexRenderingMode {
    /// Preserve the original LaTeX source and delimiters.
    None,
    /// Convert supported notation to terminal-friendly Unicode text.
    Unicode,
    /// Typeset formulas to PNG and display them with the terminal image protocol.
    #[default]
    Image,
}

impl LatexRenderingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Unicode => "unicode",
            Self::Image => "image",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" | "raw" | "off" => Some(Self::None),
            "unicode" | "terminal" | "text" => Some(Self::Unicode),
            "image" | "images" | "png" => Some(Self::Image),
            _ => None,
        }
    }
}

impl MarkdownSpacingMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Document => "Document",
        }
    }
}

/// How to display the model's reasoning/thinking content in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningDisplayMode {
    /// Never display reasoning content.
    #[default]
    Off,
    /// Keep every reasoning trace in the transcript (classic behavior).
    Full,
    /// Show only the *current* reasoning live; collapse it once the model
    /// commits an assistant message or tool call, then show the next one.
    Current,
}

impl ReasoningDisplayMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Full => "Full",
            Self::Current => "Current",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Current,
            Self::Current => Self::Full,
            Self::Full => Self::Off,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "off" | "none" | "false" | "0" | "no" => Some(Self::Off),
            "full" | "all" | "true" | "1" | "yes" | "on" => Some(Self::Full),
            "current" | "live" | "ephemeral" | "collapse" => Some(Self::Current),
            _ => None,
        }
    }
}

/// Update channel: how aggressively to receive updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    /// Only update from tagged GitHub Releases (default).
    #[default]
    Stable,
    /// Update from latest commit on main branch (bleeding edge).
    Main,
}

impl UpdateChannel {
    /// Parse a channel name, returning `None` for unknown values.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "stable" | "release" => Some(Self::Stable),
            "main" | "nightly" | "edge" => Some(Self::Main),
            _ => None,
        }
    }
}

/// Config deserialization is deliberately lenient: an unknown or removed
/// channel name (e.g. a stale `update_channel = "manual"` left in
/// config.toml) falls back to the default channel instead of failing the
/// entire config parse. A strict enum here once made the freshly exec'd
/// server die during the reload handoff, leaving the handoff marker stuck
/// in `starting` and clients re-requesting the reload forever (issue #349).
impl<'de> Deserialize<'de> for UpdateChannel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::parse(&value).unwrap_or_default())
    }
}

impl std::fmt::Display for UpdateChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stable => write!(f, "stable"),
            Self::Main => write!(f, "main"),
        }
    }
}

/// Cross-provider failover behavior when the same input would be resent elsewhere.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CrossProviderFailoverMode {
    /// Show a 3-second cancelable countdown, then resend on another provider.
    #[default]
    Countdown,
    /// Do not resend the prompt to another provider automatically.
    Manual,
}

impl CrossProviderFailoverMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Countdown => "countdown",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "manual" => Some(Self::Manual),
            "countdown" | "auto" | "automatic" => Some(Self::Countdown),
            _ => None,
        }
    }
}

/// Compaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CompactionConfig {
    /// Compaction mode: reactive (default), proactive, or semantic
    pub mode: CompactionMode,

    /// [proactive] Number of turns to look ahead when projecting token growth
    pub lookahead_turns: usize,

    /// [proactive] EWMA alpha for token growth smoothing (0.0-1.0, higher = more recency bias)
    pub ewma_alpha: f32,

    /// [proactive/semantic] Minimum context fill level before any proactive check fires (0.0-1.0)
    pub proactive_floor: f32,

    /// [proactive/semantic] Minimum number of token snapshots needed before proactive check
    pub min_samples: usize,

    /// [proactive/semantic] Number of stable turns (no growth) before suppressing proactive compact
    pub stall_window: usize,

    /// [proactive/semantic] Minimum turns between two compactions (cooldown)
    pub min_turns_between_compactions: usize,

    /// [semantic] Cosine similarity threshold below which a topic shift is detected (0.0-1.0)
    pub topic_shift_threshold: f32,

    /// [semantic] Cosine similarity above which a message is kept verbatim (0.0-1.0)
    pub relevance_keep_threshold: f32,

    /// [semantic] Number of recent turns to look at for building the "current goal" embedding
    pub goal_window_turns: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            mode: CompactionMode::Reactive,
            lookahead_turns: 15,
            ewma_alpha: 0.3,
            proactive_floor: 0.40,
            min_samples: 3,
            stall_window: 5,
            min_turns_between_compactions: 10,
            topic_shift_threshold: 0.45,
            relevance_keep_threshold: 0.65,
            goal_window_turns: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum NamedProviderType {
    #[serde(alias = "openai-compatible", alias = "openai_compatible")]
    #[default]
    OpenAiCompatible,
    OpenRouter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum NamedProviderAuth {
    #[default]
    Bearer,
    Header,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct NamedProviderModelConfig {
    pub id: String,
    #[serde(
        default,
        alias = "context_limit",
        alias = "context-length",
        alias = "context-window",
        alias = "context_length",
        skip_serializing_if = "Option::is_none"
    )]
    pub context_window: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct NamedProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: NamedProviderType,
    pub base_url: String,
    pub api: Option<String>,
    pub auth: NamedProviderAuth,
    pub auth_header: Option<String>,
    pub api_key_env: Option<String>,
    pub api_key: Option<String>,
    pub env_file: Option<String>,
    pub default_model: Option<String>,
    pub requires_api_key: Option<bool>,
    #[serde(default)]
    pub provider_routing: bool,
    #[serde(default)]
    pub model_catalog: bool,
    #[serde(default)]
    pub allow_provider_pinning: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<NamedProviderModelConfig>,
    /// Extra top-level JSON fields merged into every chat/completions request
    /// body sent to this provider. Lets users inject non-standard parameters
    /// some OpenAI-compatible backends require (e.g. NVIDIA NIM DeepSeek-V4
    /// needs `chat_template_kwargs = { thinking = true, reasoning_effort = "high" }`).
    /// Must be a JSON object; keys here override jcode-generated body fields.
    #[serde(default, alias = "extra-body", skip_serializing_if = "Option::is_none")]
    pub extra_body: Option<serde_json::Value>,
    /// Whether this endpoint accepts the DeepSeek-style top-level
    /// `reasoning_effort` request field (`/effort` support). When unset, jcode
    /// auto-detects it from the active model id (DeepSeek-family models
    /// support it regardless of which gateway serves them). Set `false` to
    /// suppress auto-detection for strict-schema endpoints.
    #[serde(
        default,
        alias = "supports-reasoning-effort",
        alias = "reasoning_effort",
        skip_serializing_if = "Option::is_none"
    )]
    pub supports_reasoning_effort: Option<bool>,
}

impl Default for NamedProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: NamedProviderType::OpenAiCompatible,
            base_url: String::new(),
            api: None,
            auth: NamedProviderAuth::Bearer,
            auth_header: None,
            api_key_env: None,
            api_key: None,
            env_file: None,
            default_model: None,
            requires_api_key: None,
            provider_routing: false,
            model_catalog: false,
            allow_provider_pinning: false,
            models: Vec::new(),
            extra_body: None,
            supports_reasoning_effort: None,
        }
    }
}

/// Remembered trust decisions for external auth sources managed by other tools.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AuthConfig {
    /// External auth source ids that the user has approved jcode to read/use.
    pub trusted_external_sources: Vec<String>,
    /// Path-bound approvals for external auth sources managed by other tools.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusted_external_source_paths: Vec<String>,
}

/// Agent-specific model defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentsConfig {
    /// Optional default model override for spawned swarm/subagent sessions.
    ///
    /// Leave unset (or use `"inherit"` / `"coordinator"`) to have spawned swarm
    /// agents inherit the spawning coordinator's model. Set to a concrete model
    /// string only when you deliberately want every swarm worker pinned to a
    /// specific model regardless of which model spawned them.
    pub swarm_model: Option<String>,
    /// Default terminal mode for swarm-created agents.
    pub swarm_spawn_mode: SwarmSpawnMode,
    /// Maximum percentage (1-90) of the chat column height the inline swarm
    /// gallery band may occupy. Leave unset to use the built-in default (40%).
    /// Lower values keep more of the transcript visible; set near the minimum
    /// to effectively collapse the gallery to a thin strip.
    pub swarm_gallery_max_pct: Option<u8>,
    /// Layout of the inline swarm strip above the status line:
    /// `"vertical"` (default) lists one agent per row (session icon + status
    /// glyph + task), capped to a few lines; `"horizontal"` packs all agents
    /// as chips on a single row.
    #[serde(default)]
    pub swarm_strip_layout: SwarmStripLayout,
    /// Optional default model override for the memory sidecar.
    pub memory_model: Option<String>,
    /// Whether memory should use the sidecar for relevance/extraction.
    ///
    /// Defaults to `true`: the LLM precision-judge path is the only memory mode
    /// that is reliably productive (injection precision ~1.0), so memory uses it
    /// by default. Set to `false` only to deliberately opt into the lower-
    /// precision no-LLM hybrid path. When sidecar mode is on but no LLM backend
    /// is reachable, the memory runtime goes dormant instead of degrading to the
    /// no-LLM path.
    #[serde(default = "default_memory_sidecar_enabled")]
    pub memory_sidecar_enabled: bool,
    /// Minimum turns between Mode-2 memory reranks (cadence floor). The
    /// expensive listwise LLM rerank runs at most once per this many turns;
    /// skipped turns fall back to hybrid-ordered surfacing. A topic change or
    /// the first turn always forces a rerank regardless of cadence. 0 or 1 =
    /// rerank every turn (no gating). Default 3.
    #[serde(default = "default_memory_rerank_cadence")]
    pub memory_rerank_cadence: usize,
    /// Number of independent LLM rerank "judges" to run per fired rerank. Their
    /// votes are combined and only memories meeting `memory_rerank_min_agree`
    /// agreement are injected. 1 = single judge (cheapest). 2 = two judges must
    /// agree, which lifts injection precision to ~1.0 with ~100% clean-rate on
    /// no-memory turns (offline adjudication), at 2 LLM calls per fired turn.
    #[serde(default = "default_memory_rerank_votes")]
    pub memory_rerank_votes: usize,
    /// Minimum judge agreement (of `memory_rerank_votes`) required to inject a
    /// memory. Clamped to 1..=votes. Higher = stricter precision, lower recall.
    #[serde(default = "default_memory_rerank_min_agree")]
    pub memory_rerank_min_agree: usize,
    /// Which embedding backend memory dense-retrieval uses: `"local"` (bundled
    /// all-MiniLM-L6-v2 ONNX, default, no network) or `"openai"` (remote
    /// OpenAI/openai-compatible `/v1/embeddings`, opt-in, requires an
    /// `OPENAI_API_KEY`). A keyless `"openai"` setting silently degrades to
    /// local. Env override: `JCODE_MEMORY_EMBEDDING_BACKEND`.
    #[serde(default = "default_memory_embedding_backend")]
    pub memory_embedding_backend: String,
    /// OpenAI embedding model name when `memory_embedding_backend = "openai"`.
    /// Unset = `text-embedding-3-small`. Env: `JCODE_MEMORY_EMBEDDING_MODEL`.
    #[serde(default)]
    pub memory_embedding_model: Option<String>,
    /// Optional override for the embeddings API base URL (no trailing slash),
    /// for OpenAI-compatible gateways. Unset = `https://api.openai.com/v1`.
    /// Env: `JCODE_MEMORY_EMBEDDING_BASE_URL`.
    #[serde(default)]
    pub memory_embedding_base_url: Option<String>,
    /// Optional override for the remote embedding dimensionality (vector-space
    /// metadata / sanity checks). Unset = inferred from the model name.
    #[serde(default)]
    pub memory_embedding_dim: Option<usize>,
    /// Maximum number of live swarm worker agents in one swarm. This is the RAM
    /// safety budget for both recursive ad hoc spawning and deep-mode `run_plan`
    /// parallelism. Completed/stopped workers do not consume slots. Light mode
    /// still uses a smaller fixed fan-out. `0` disables this configurable guard,
    /// leaving only the absolute `MAX_SWARM_MEMBERS` hard cap.
    /// Env override: `JCODE_SWARM_MAX_CONCURRENT_AGENTS`.
    #[serde(default = "default_swarm_max_concurrent_agents")]
    pub swarm_max_concurrent_agents: usize,
}

fn default_swarm_max_concurrent_agents() -> usize {
    32
}

fn default_memory_embedding_backend() -> String {
    "local".to_string()
}

fn default_memory_sidecar_enabled() -> bool {
    true
}

fn default_memory_rerank_cadence() -> usize {
    3
}

fn default_memory_rerank_votes() -> usize {
    2
}

fn default_memory_rerank_min_agree() -> usize {
    2
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            swarm_model: None,
            swarm_spawn_mode: SwarmSpawnMode::default(),
            swarm_gallery_max_pct: None,
            swarm_strip_layout: SwarmStripLayout::default(),
            memory_model: None,
            memory_sidecar_enabled: default_memory_sidecar_enabled(),
            memory_rerank_cadence: default_memory_rerank_cadence(),
            memory_rerank_votes: default_memory_rerank_votes(),
            memory_rerank_min_agree: default_memory_rerank_min_agree(),
            memory_embedding_backend: default_memory_embedding_backend(),
            memory_embedding_model: None,
            memory_embedding_base_url: None,
            memory_embedding_dim: None,
            swarm_max_concurrent_agents: default_swarm_max_concurrent_agents(),
        }
    }
}

/// How swarm-created agents should be spawned.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SwarmSpawnMode {
    /// Create the worker in-process without opening a terminal window.
    #[default]
    Headless,
}

impl SwarmSpawnMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "headless" => Some(Self::Headless),
            // Accept former UI modes as a migration aid, but always run them
            // in-process without a frontend.
            "visible" | "headed" | "inline" | "auto" => Some(Self::Headless),
            _ => None,
        }
    }

    /// Canonical lowercase string for this mode (matches the config/env values).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Headless => "headless",
        }
    }
}

/// Layout of the inline swarm strip shown above the status line.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SwarmStripLayout {
    /// One agent per row: session icon + status glyph + task label, capped to
    /// a few lines with a `+N more` overflow marker.
    #[default]
    Vertical,
    /// All agents packed as chips on a single row (the historical layout).
    Horizontal,
}

impl SwarmStripLayout {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "vertical" | "list" => Some(Self::Vertical),
            "horizontal" | "chips" | "strip" => Some(Self::Horizontal),
            _ => None,
        }
    }

    /// Canonical lowercase string for this layout (matches config/env values).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        }
    }
}

/// Lifecycle hooks:/// Lifecycle hooks: external commands jcode runs at well-defined points.
///
/// Hook commands are parsed shell-style (quotes work) but executed directly,
/// with `JCODE_HOOK_*` env vars describing the event (`JCODE_HOOK_EVENT`,
/// `JCODE_HOOK_SESSION_ID`, `JCODE_HOOK_CWD`, event-specific fields, and a
/// `JCODE_HOOK_PAYLOAD` JSON mirror). Hook processes get
/// `JCODE_HOOKS_DISABLED=1` so nested jcode invocations don't recurse.
///
/// All hooks except `pre_tool` are observers: detached, fire-and-forget,
/// failures only logged. `pre_tool` is a gate: jcode waits for it and exit
/// code 2 blocks the tool call (stderr becomes the error shown to the model);
/// exit 0 allows; anything else fails open.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HooksConfig {
    /// Runs when an agent turn begins (after the user message is added and
    /// before the model starts generating). Fires before the first `pre_tool`,
    /// so integrations can detect that the agent is actively working even while
    /// it is only thinking/streaming text. Fields: MODEL, SOURCE
    /// ("chat"/"resume"/"ambient"). Env override: JCODE_HOOK_TURN_START.
    pub turn_start: Option<String>,
    /// Runs when an agent turn completes.
    /// Fields: STATUS ("ok"/"error"), DURATION_MS, MODEL, LAST_ASSISTANT_TEXT.
    /// Env override: JCODE_HOOK_TURN_END.
    pub turn_end: Option<String>,
    /// Runs when a session becomes active (created or resumed).
    /// Fields: SOURCE ("create"/"resume").
    /// Env override: JCODE_HOOK_SESSION_START.
    pub session_start: Option<String>,
    /// Runs when a session closes normally.
    /// Env override: JCODE_HOOK_SESSION_END.
    pub session_end: Option<String>,
    /// Gate hook before each tool call. Receives TOOL_NAME and the tool input
    /// JSON on stdin (also truncated in TOOL_INPUT). Exit 0 allows, exit 2
    /// blocks (stderr is fed back to the model), anything else fails open.
    /// Env override: JCODE_HOOK_PRE_TOOL.
    pub pre_tool: Option<String>,
    /// Runs after each tool call completes.
    /// Fields: TOOL_NAME, STATUS ("ok"/"error"), DURATION_MS, OUTPUT_BYTES.
    /// Env override: JCODE_HOOK_POST_TOOL.
    pub post_tool: Option<String>,
    /// Max milliseconds to wait for the pre_tool gate before failing open
    /// (default: 5000). Env override: JCODE_HOOK_PRE_TOOL_TIMEOUT_MS.
    pub pre_tool_timeout_ms: u64,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            turn_start: None,
            turn_end: None,
            session_start: None,
            session_end: None,
            pre_tool: None,
            post_tool: None,
            pre_tool_timeout_ms: 5000,
        }
    }
}

/// Automatic end-of-turn code review configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AutoReviewConfig {
    /// Enable autoreview by default for new/resumed sessions (default: false)
    pub enabled: bool,
    /// Optional model override for autoreview reviewer sessions.
    pub model: Option<String>,
}

/// Tool partner discovery configuration.
///
/// Partner discovery makes third-party developer tools discoverable to the
/// agent via a `discover_tools` tool backed by a hosted directory. Some
/// partners may share revenue with Jcode when a referred user becomes a
/// customer, but partnership status never influences recommendations. Each
/// session's first use of `discover_tools` shows a concise disclosure with a
/// learn-more link.
/// See <https://jcode.sh/discovery-tools>.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SponsorsConfig {
    /// Enable tool partner discovery. Enabled by default; set to false to opt
    /// out. When false, no discovery categories are added to the prompt, the
    /// `discover_tools` tool is not registered, and jcode never contacts the
    /// discovery endpoint.
    pub enabled: bool,
    /// Base URL of the discovery endpoint.
    pub endpoint: String,
}

impl Default for SponsorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "https://api.jcode.sh/v1/discovery".to_string(),
        }
    }
}

/// Automatic end-of-turn execution judging configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AutoJudgeConfig {
    /// Enable autojudge by default for new/resumed sessions (default: false)
    pub enabled: bool,
    /// Optional model override for autojudge sessions.
    pub model: Option<String>,
}

/// How to display file diffs from edit/write tools/// How to display file diffs from edit/write tools
/// Display/UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NativeScrollbarConfig {
    /// Show a native terminal scrollbar in the chat viewport (default: true)
    pub chat: bool,
    /// Show a native terminal scrollbar in the side panel (default: true)
    pub side_panel: bool,
}

impl Default for NativeScrollbarConfig {
    fn default() -> Self {
        Self {
            chat: true,
            side_panel: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Display/UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    /// How to display file diffs (off/inline/full-inline/pinned/file, default: inline)
    pub diff_mode: DiffDisplayMode,
    /// Legacy: "show_diffs = true/false" maps to diff_mode inline/off
    #[serde(default)]
    show_diffs: Option<bool>,
    /// Queue mode by default - wait until done before sending (default: false)
    pub queue_mode: bool,
    /// Automatically reload the remote server when a newer server binary is detected (default: true)
    pub auto_server_reload: bool,
    /// Capture mouse events (default: true). Enables scroll wheel but disables terminal selection.
    pub mouse_capture: bool,
    /// Enable debug socket for external control (default: false)
    pub debug_socket: bool,
    /// Center all content (default: false)
    pub centered: bool,
    /// Show thinking/reasoning content by default (default: true)
    pub show_thinking: bool,
    /// How to display reasoning/thinking content (off/full/current).
    /// When unset, falls back to `show_thinking` (true => full, false => off).
    #[serde(default)]
    reasoning_display: Option<ReasoningDisplayMode>,
    /// How to display mermaid diagrams (none/margin/pinned, default: none).
    /// `none` still renders diagrams inline in the transcript via the inline
    /// image pipeline; `margin`/`pinned` add dedicated widget placements.
    pub diagram_mode: DiagramDisplayMode,
    /// Markdown block spacing style (compact/document, default: compact)
    pub markdown_spacing: MarkdownSpacingMode,
    /// LaTeX rendering style (none/unicode/image, default: image)
    pub latex_rendering: LatexRenderingMode,
    /// Pin read images to side pane (default: true)
    pub pin_images: bool,
    /// Show idle animation before first prompt (default: true)
    pub idle_animation: bool,
    /// Briefly animate user prompt line when it enters viewport (default: true)
    pub prompt_entry_animation: bool,
    /// Disable specific animation variants by name (e.g. ["donut", "orbit_rings"])
    pub disabled_animations: Vec<String>,
    /// Wrap long lines in the pinned diff pane (default: true)
    pub diff_line_wrap: bool,
    /// Performance tier override: auto/full/reduced/minimal (default: auto)
    pub performance: String,
    /// FPS for animations (startup, idle donut): 1-120 (default: 60)
    pub animation_fps: u32,
    /// FPS for active redraw (processing, streaming): 1-120 (default: 30)
    pub redraw_fps: u32,
    /// Show a truncated preview of the previous prompt at the top when it scrolls out of view (default: true)
    pub prompt_preview: bool,
    /// Render swarm/file-activity notifications in a compact single-line form
    /// instead of the full multi-line card with diff preview (default: false)
    pub compact_notifications: bool,
    /// Override the Alt/Option label shown in copy badges. Empty = auto (⌥ on macOS, Alt elsewhere).
    pub copy_badge_alt_label: String,
    /// Show the full agentgrep tool output inline in the transcript instead of
    /// just the one-line summary (default: false)
    #[serde(default)]
    pub show_agentgrep_output: bool,
    /// Show the dimmed technical detail (command, path, args) after the
    /// model-provided intent on tool rows (default: false). When off, rows
    /// that have an intent show only the intent; rows without an intent
    /// always fall back to the technical detail.
    #[serde(default)]
    pub tool_call_details: bool,
    /// Native terminal scrollbar configuration for scrollable panes
    pub native_scrollbars: NativeScrollbarConfig,
    /// Surface occasional "learn this keybinding" nudges when the user keeps
    /// performing an action the slow way (slash command) instead of using its
    /// configured shortcut (default: true). Set false to disable all such hints.
    #[serde(default = "default_true")]
    pub keybinding_hints: bool,
    /// Color theme: "auto" (detect terminal background), "dark", or "light".
    /// Auto queries the terminal's background color (OSC 11) at startup and
    /// adapts jcode's palette for light backgrounds. Default: auto.
    #[serde(default)]
    pub theme: String,
    /// Opt-in active sessions manager: pressing Left arrow on an empty input
    /// opens a picker scoped to live (open) sessions, showing which are still
    /// working and which are ready for input (default: false). The `/active`
    /// command works regardless of this setting.
    #[serde(default)]
    pub active_sessions_manager: bool,
    /// When to show the overscroll status line below the input
    /// (off/on/overscroll, default: overscroll). "overscroll" is the elastic
    /// reveal when scrolling past the bottom, "on" keeps it always visible.
    #[serde(default)]
    pub overscroll_status: OverscrollStatusMode,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            diff_mode: DiffDisplayMode::default(),
            show_diffs: None,
            pin_images: true,
            queue_mode: false,
            auto_server_reload: true,
            mouse_capture: true,
            debug_socket: false,
            centered: false,
            show_thinking: true,
            reasoning_display: Some(ReasoningDisplayMode::Current),
            diagram_mode: DiagramDisplayMode::default(),
            markdown_spacing: MarkdownSpacingMode::default(),
            latex_rendering: LatexRenderingMode::default(),
            idle_animation: true,
            prompt_entry_animation: true,
            disabled_animations: Vec::new(),
            diff_line_wrap: true,
            performance: String::new(),
            animation_fps: 60,
            redraw_fps: 60,
            prompt_preview: true,
            compact_notifications: false,
            copy_badge_alt_label: String::new(),
            show_agentgrep_output: false,
            tool_call_details: false,
            native_scrollbars: NativeScrollbarConfig::default(),
            keybinding_hints: true,
            theme: String::new(),
            active_sessions_manager: false,
            overscroll_status: OverscrollStatusMode::default(),
        }
    }
}

impl DisplayConfig {
    pub fn apply_legacy_compat(&mut self) {
        if let Some(show) = self.show_diffs.take() {
            self.diff_mode = if show {
                DiffDisplayMode::Inline
            } else {
                DiffDisplayMode::Off
            };
        }
    }

    /// Resolve the effective reasoning display mode. Prefers the explicit
    /// `reasoning_display` field, falling back to the legacy `show_thinking`
    /// boolean (true => Full, false => Off) when unset.
    pub fn reasoning_display(&self) -> ReasoningDisplayMode {
        self.reasoning_display.unwrap_or(if self.show_thinking {
            ReasoningDisplayMode::Full
        } else {
            ReasoningDisplayMode::Off
        })
    }

    /// Set the reasoning display mode and keep `show_thinking` in sync so the
    /// provider request path (which still keys off `show_thinking`) requests
    /// reasoning whenever any display mode is active.
    pub fn set_reasoning_display(&mut self, mode: ReasoningDisplayMode) {
        self.reasoning_display = Some(mode);
        self.show_thinking = !matches!(mode, ReasoningDisplayMode::Off);
    }

    /// Whether reasoning content should be generated/requested at all.
    pub fn reasoning_enabled(&self) -> bool {
        !matches!(self.reasoning_display(), ReasoningDisplayMode::Off)
    }
}

/// Runtime feature toggles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FeatureConfig {
    /// Enable memory retrieval/extraction features (default: true)
    pub memory: bool,
    /// Enable swarm coordination features (default: true)
    pub swarm: bool,
    /// Enable Mermaid rendering and Mermaid-specific model guidance (default: true)
    pub mermaid: bool,
    /// Inject timestamps into user messages and tool results sent to the model (default: true)
    pub message_timestamps: bool,
    /// Persist auto-recalled memory injections into normal session history instead of sending
    /// them as request-only ephemeral suffix messages (default: false)
    pub persist_memory_injections: bool,
    /// Surface an in-chat system message whenever a request misses the KV cache
    /// for a harness-caused (avoidable) reason: the system prompt, tool set, or
    /// message prefix changed without the conversation legitimately growing.
    /// These should essentially never happen, so the notice acts as a loud alarm
    /// that something in the harness silently invalidated the prefix cache
    /// (default: true).
    pub kv_cache_miss_notices: bool,
    /// Update channel: "stable" (releases only) or "main" (latest commits)
    pub update_channel: UpdateChannel,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            memory: true,
            swarm: true,
            mermaid: true,
            message_timestamps: true,
            persist_memory_injections: false,
            kv_cache_miss_notices: true,
            update_channel: UpdateChannel::default(),
        }
    }
}

/// Search engine used by the websearch tool.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchEngine {
    /// DuckDuckGo HTML search, no API key required.
    #[default]
    Duckduckgo,
    /// Bing search. Uses the Bing API when configured, otherwise Bing HTML search.
    Bing,
    /// SearXNG metasearch instance (JSON API). Requires `searxng_url` (or the
    /// `JCODE_SEARXNG_URL` env var) to point at a SearXNG instance. Useful on
    /// hosts where DuckDuckGo/Bing block the request via TLS fingerprinting.
    Searxng,
}

impl WebSearchEngine {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Duckduckgo => "duckduckgo",
            Self::Bing => "bing",
            Self::Searxng => "searxng",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "duckduckgo" | "ddg" => Some(Self::Duckduckgo),
            "bing" => Some(Self::Bing),
            "searxng" | "searx" => Some(Self::Searxng),
            _ => None,
        }
    }
}

/// Configuration for the websearch tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSearchConfig {
    /// Preferred engine when the tool input does not specify one.
    pub engine: WebSearchEngine,
    /// Keyless HTML engines to try after the preferred engine fails.
    pub fallback_engines: Vec<WebSearchEngine>,
    /// Optional Bing API key for primary Bing searches. Fallback Bing uses keyless HTML search.
    pub bing_api_key: Option<String>,
    /// Environment variable containing the Bing API key.
    pub bing_api_key_env: String,
    /// Bing market, e.g. "en-US" or "zh-CN".
    pub bing_market: String,
    /// Base URL of a SearXNG instance (e.g. "https://searx.example.org"), used
    /// by the `searxng` engine. When empty, the `searxng_url_env` variable is
    /// consulted instead.
    pub searxng_url: Option<String>,
    /// Environment variable containing the SearXNG base URL.
    pub searxng_url_env: String,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            engine: WebSearchEngine::Duckduckgo,
            fallback_engines: vec![WebSearchEngine::Bing],
            bing_api_key: None,
            bing_api_key_env: "JCODE_BING_API_KEY".to_string(),
            bing_market: "en-US".to_string(),
            searxng_url: None,
            searxng_url_env: "JCODE_SEARXNG_URL".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    /// Default model to use (e.g. "claude-opus-4-8", "copilot:claude-opus-4.6")
    pub default_model: Option<String>,
    /// Default provider to use (claude|openai|copilot|openrouter)
    pub default_provider: Option<String>,
    /// Reasoning effort for OpenAI Responses API (none|minimal|low|medium|high|xhigh|max)
    pub openai_reasoning_effort: Option<String>,
    /// Reasoning effort for Anthropic Messages API output_config (none|low|medium|high|xhigh; max aliases to strongest supported)
    pub anthropic_reasoning_effort: Option<String>,
    /// OpenAI transport mode (auto|websocket|https)
    pub openai_transport: Option<String>,
    /// OpenAI service tier override (priority|flex)
    pub openai_service_tier: Option<String>,
    /// OpenAI native compaction mode: "auto", "explicit", or "off".
    pub openai_native_compaction_mode: String,
    /// Token threshold at which OpenAI auto native compaction should trigger.
    pub openai_native_compaction_threshold_tokens: usize,
    /// Preserve provider-native reasoning/thinking items for future-turn context when supported.
    pub preserve_reasoning_context: bool,
    /// How to handle cross-provider failover when the same input would be resent elsewhere.
    pub cross_provider_failover: CrossProviderFailoverMode,
    /// Whether jcode should automatically try another account on the same provider
    /// before falling back to a different provider.
    pub same_provider_account_failover: bool,
    /// Copilot premium request mode: "normal", "one", or "zero"
    /// "zero" means all requests are free (no premium requests consumed)
    pub copilot_premium: Option<String>,
    /// When set (non-empty), /model only lists routes from these providers.
    /// Entries match provider labels ("openai", "anthropic", "copilot",
    /// "openrouter", ...), api methods ("claude-oauth",
    /// "openai-compatible:myprofile", ...), or openai-compatible profile ids
    /// ("myprofile"). The active model's routes always stay visible.
    pub model_picker_providers: Option<Vec<String>>,
    /// Max seconds to wait for streaming data before timing out a request with
    /// no data received. Raise this for slow reasoning models (e.g. DeepSeek)
    /// that think silently for minutes before emitting tokens. Default: 180.
    /// Overridable per-launch via `JCODE_STREAM_IDLE_TIMEOUT_SECS`.
    pub stream_idle_timeout_secs: u64,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            default_model: None,
            default_provider: None,
            openai_reasoning_effort: Some("low".to_string()),
            anthropic_reasoning_effort: None,
            openai_transport: None,
            openai_service_tier: Some("priority".to_string()),
            openai_native_compaction_mode: "auto".to_string(),
            openai_native_compaction_threshold_tokens: 200_000,
            preserve_reasoning_context: true,
            cross_provider_failover: CrossProviderFailoverMode::Countdown,
            same_provider_account_failover: true,
            copilot_premium: None,
            model_picker_providers: None,
            stream_idle_timeout_secs: 180,
        }
    }
}

/// Ambient mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AmbientConfig {
    /// Enable ambient mode (default: false)
    pub enabled: bool,
    /// Provider override (default: auto-select)
    pub provider: Option<String>,
    /// Model override (default: provider's strongest)
    pub model: Option<String>,
    /// Allow API key usage (default: false, only OAuth)
    pub allow_api_keys: bool,
    /// Daily token budget when using API keys
    pub api_daily_budget: Option<u64>,
    /// Minimum interval between cycles in minutes (default: 5)
    pub min_interval_minutes: u32,
    /// Maximum interval between cycles in minutes (default: 120)
    pub max_interval_minutes: u32,
    /// Pause ambient when user has active session (default: true)
    pub pause_on_active_session: bool,
    /// Enable proactive work vs garden-only (default: true)
    pub proactive_work: bool,
    /// Proactive work branch prefix (default: "ambient/")
    pub work_branch_prefix: String,
}

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: None,
            model: None,
            allow_api_keys: false,
            api_daily_budget: None,
            min_interval_minutes: 5,
            max_interval_minutes: 120,
            pause_on_active_session: true,
            proactive_work: true,
            work_branch_prefix: "ambient/".to_string(),
        }
    }
}

/// Safety system & notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyConfig {
    /// ntfy.sh topic name (required for push notifications)
    pub ntfy_topic: Option<String>,
    /// ntfy.sh server URL (default: https://ntfy.sh)
    pub ntfy_server: String,
    /// Enable email notifications (default: false)
    pub email_enabled: bool,
    /// Email recipient
    pub email_to: Option<String>,
    /// SMTP host (e.g. smtp.gmail.com)
    pub email_smtp_host: Option<String>,
    /// SMTP port (default: 587)
    pub email_smtp_port: u16,
    /// Email sender address
    pub email_from: Option<String>,
    /// SMTP password (prefer JCODE_SMTP_PASSWORD env var)
    pub email_password: Option<String>,
    /// IMAP host for receiving email replies (e.g. imap.gmail.com)
    pub email_imap_host: Option<String>,
    /// IMAP port (default: 993)
    pub email_imap_port: u16,
    /// Enable email reply → agent directive feature (default: false)
    pub email_reply_enabled: bool,
    /// Enable Telegram notifications (default: false)
    pub telegram_enabled: bool,
    /// Telegram bot token (from @BotFather)
    pub telegram_bot_token: Option<String>,
    /// Telegram chat ID to send messages to
    pub telegram_chat_id: Option<String>,
    /// Enable Telegram reply → agent directive feature (default: false)
    pub telegram_reply_enabled: bool,
    /// Enable Discord notifications (default: false)
    pub discord_enabled: bool,
    /// Discord bot token
    pub discord_bot_token: Option<String>,
    /// Discord channel ID to send messages to
    pub discord_channel_id: Option<String>,
    /// Discord bot user ID (for filtering own messages in polling)
    pub discord_bot_user_id: Option<String>,
    /// Enable Discord reply → agent directive feature (default: false)
    pub discord_reply_enabled: bool,
    /// Enable the Jade cloud relay channel (remote control via cloud mailbox, default: false)
    pub jade_relay_enabled: bool,
    /// Jade relay API base URL (e.g. https://...lambda-url.us-east-1.on.aws/)
    pub jade_relay_api_base: Option<String>,
    /// Jade relay bearer token (prefer JCODE_JADE_RELAY_TOKEN env var)
    pub jade_relay_token: Option<String>,
    /// Jade relay token id header (x-jade-token-id), used for fast token lookup
    pub jade_relay_token_id: Option<String>,
    /// Jade relay user id (channel scope; defaults to the token's user when omitted)
    pub jade_relay_user_id: Option<String>,
    /// Jade relay session id to bind this laptop's listener to (the channel = user_id/session_id)
    pub jade_relay_session_id: Option<String>,
    /// Enable Jade relay prompt → agent directive feature (default: false)
    pub jade_relay_reply_enabled: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            ntfy_topic: None,
            ntfy_server: "https://ntfy.sh".to_string(),
            email_enabled: false,
            email_to: None,
            email_smtp_host: None,
            email_smtp_port: 587,
            email_from: None,
            email_password: None,
            email_imap_host: None,
            email_imap_port: 993,
            email_reply_enabled: false,
            telegram_enabled: false,
            telegram_bot_token: None,
            telegram_chat_id: None,
            telegram_reply_enabled: false,
            discord_enabled: false,
            discord_bot_token: None,
            discord_channel_id: None,
            discord_bot_user_id: None,
            discord_reply_enabled: false,
            jade_relay_enabled: false,
            jade_relay_api_base: None,
            jade_relay_token: None,
            jade_relay_token_id: None,
            jade_relay_user_id: None,
            jade_relay_session_id: None,
            jade_relay_reply_enabled: false,
        }
    }
}

/// Power-management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PowerConfig {
    /// Prevent automatic system sleep while any jcode session is actively
    /// streaming/processing. Linux also asks logind to block lid-switch suspend.
    /// Windows cannot override a user-initiated lid close or power-button action;
    /// those remain controlled by the active Windows power plan. The display is
    /// still allowed to sleep. Default: true.
    ///
    /// Honored by the shared `jcode serve` daemon. The `JCODE_DISABLE_POWER_INHIBIT`
    /// environment variable forces this off regardless of the config value.
    pub prevent_sleep_while_streaming: bool,
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            prevent_sleep_while_streaming: true,
        }
    }
}
