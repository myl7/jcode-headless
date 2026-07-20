use super::*;

impl Config {
    pub fn display_string(&self) -> String {
        let path = Self::path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let mut disabled_tools: Vec<String> =
            self.tools.selection().disabled_tools.into_iter().collect();
        disabled_tools.sort();

        format!(
            "Headless configuration ({path})\n\n\
             Provider:\n\
             - default provider: {provider}\n\
             - default model: {model}\n\
             - OpenAI transport: {openai_transport}\n\
             - cross-provider failover: {failover}\n\n\
             Tools:\n\
             - profile: {tool_profile}\n\
             - disabled: {disabled_tools}\n\n\
             Agents:\n\
             - swarm model: {swarm_model}\n\
             - memory: {memory}\n\
             - ambient: {ambient}\n\n\
             Notifications:\n\
             - email: {email}\n\
             - ntfy: {ntfy}\n\
             - Telegram: {telegram}\n\
             - Discord: {discord}\n\n\
             Set environment variables or edit the config file to customize the runtime.",
            provider = self.provider.default_provider.as_deref().unwrap_or("auto"),
            model = self
                .provider
                .default_model
                .as_deref()
                .unwrap_or("provider default"),
            openai_transport = self.provider.openai_transport.as_deref().unwrap_or("auto"),
            failover = self.provider.cross_provider_failover.as_str(),
            tool_profile = if self.tools.profile.trim().is_empty() {
                "full"
            } else {
                self.tools.profile.trim()
            },
            disabled_tools = if disabled_tools.is_empty() {
                "none".to_string()
            } else {
                disabled_tools.join(", ")
            },
            swarm_model = self
                .agents
                .swarm_model
                .as_deref()
                .unwrap_or("inherit current session"),
            memory = if self.features.memory {
                "enabled"
            } else {
                "disabled"
            },
            ambient = if self.ambient.enabled {
                "enabled"
            } else {
                "disabled"
            },
            email = if self.safety.email_enabled {
                self.safety.email_to.as_deref().unwrap_or("enabled")
            } else {
                "disabled"
            },
            ntfy = if self.safety.ntfy_topic.is_some() {
                "enabled"
            } else {
                "disabled"
            },
            telegram = if self.safety.telegram_enabled {
                "enabled"
            } else {
                "disabled"
            },
            discord = if self.safety.discord_enabled {
                "enabled"
            } else {
                "disabled"
            },
        )
    }
}
