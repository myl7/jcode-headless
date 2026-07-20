use anyhow::Result;
use async_trait::async_trait;
use jcode_agent_runtime::InterruptSignal;
use jcode_message_types::ToolDefinition;
use jcode_tool_types::ToolOutput;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const TOOL_INTENT_DESCRIPTION: &str = concat!(
    "Short natural-language label explaining why this tool call is being made. ",
    "Optional metadata used for logs and activity display only; do not use this instead of required tool parameters."
);

pub fn intent_schema_property() -> Value {
    serde_json::json!({
        "type": "string",
        "description": TOOL_INTENT_DESCRIPTION,
    })
}

/// A request for stdin input from a running command.
pub struct StdinInputRequest {
    pub request_id: String,
    pub prompt: String,
    pub is_password: bool,
    pub response_tx: tokio::sync::oneshot::Sender<String>,
}

#[derive(Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub message_id: String,
    pub tool_call_id: String,
    pub working_dir: Option<PathBuf>,
    pub stdin_request_tx: Option<tokio::sync::mpsc::UnboundedSender<StdinInputRequest>>,
    pub graceful_shutdown_signal: Option<InterruptSignal>,
    pub execution_mode: ToolExecutionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolExecutionMode {
    AgentTurn,
    Direct,
}

impl ToolContext {
    pub fn for_subcall(&self, tool_call_id: String) -> Self {
        Self {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            tool_call_id,
            working_dir: self.working_dir.clone(),
            stdin_request_tx: self.stdin_request_tx.clone(),
            graceful_shutdown_signal: self.graceful_shutdown_signal.clone(),
            execution_mode: self.execution_mode,
        }
    }

    pub fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(ref base) = self.working_dir {
            base.join(path)
        } else {
            path.to_path_buf()
        }
    }
}

/// A tool that can be executed by the agent.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (must match what's sent to the API).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// JSON Schema for the input parameters.
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with the given input.
    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolOutput>;

    /// Convert to API tool definition.
    fn to_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: self.parameters_schema(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definition_does_not_inject_intent() {
        struct BareTool;

        #[async_trait]
        impl Tool for BareTool {
            fn name(&self) -> &str {
                "bare"
            }

            fn description(&self) -> &str {
                "bare tool"
            }

            fn parameters_schema(&self) -> Value {
                serde_json::json!({
                    "type": "object",
                    "required": ["command"],
                    "properties": {
                        "command": {"type": "string"}
                    }
                })
            }

            async fn execute(&self, _input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
                Ok(ToolOutput::new("ok"))
            }
        }

        let definition = BareTool.to_definition();
        assert!(definition.input_schema["properties"]["intent"].is_null());
        assert_eq!(
            definition.input_schema["required"],
            serde_json::json!(["command"])
        );
    }

    #[test]
    fn intent_schema_property_is_optional_metadata() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": {"type": "string"},
                "intent": intent_schema_property()
            }
        });
        assert!(schema["properties"]["intent"].is_object());
        let required: Vec<_> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(required.contains(&"command"));
        assert!(!required.contains(&"intent"));
    }
}
