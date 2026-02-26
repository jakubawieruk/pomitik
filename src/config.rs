use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct SessionConfig {
    pub work: String,
    #[serde(rename = "break")]
    pub break_preset: String,
    pub long_break: String,
    pub rounds: u32,
}

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub presets: HashMap<String, String>,
    #[serde(default)]
    pub sessions: HashMap<String, SessionConfig>,
}

impl Config {
    pub fn load() -> Self {
        let mut presets = Self::defaults();
        let mut sessions = Self::default_sessions();
        let path = Self::config_path();
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(user_config) = toml::from_str::<Config>(&contents) {
                    for (k, v) in user_config.presets {
                        presets.insert(k, v);
                    }
                    for (k, v) in user_config.sessions {
                        sessions.insert(k, v);
                    }
                }
            }
        }
        Config { presets, sessions }
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tik")
            .join("config.toml")
    }

    fn defaults() -> HashMap<String, String> {
        HashMap::from([
            ("pomodoro".to_string(), "25m".to_string()),
            ("break".to_string(), "5m".to_string()),
            ("long-break".to_string(), "15m".to_string()),
        ])
    }

    fn default_sessions() -> HashMap<String, SessionConfig> {
        HashMap::from([(
            "pomodoro".to_string(),
            SessionConfig {
                work: "pomodoro".to_string(),
                break_preset: "break".to_string(),
                long_break: "long-break".to_string(),
                rounds: 4,
            },
        )])
    }

    pub fn resolve_preset(&self, name: &str) -> Option<&str> {
        self.presets.get(name).map(|s| s.as_str())
    }

    pub fn resolve_session(&self, name: &str) -> Option<&SessionConfig> {
        self.sessions.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_include_pomodoro() {
        let defaults = Config::defaults();
        assert_eq!(defaults.get("pomodoro").unwrap(), "25m");
    }

    #[test]
    fn defaults_include_break() {
        let defaults = Config::defaults();
        assert_eq!(defaults.get("break").unwrap(), "5m");
    }

    #[test]
    fn defaults_include_long_break() {
        let defaults = Config::defaults();
        assert_eq!(defaults.get("long-break").unwrap(), "15m");
    }

    #[test]
    fn parse_toml_config() {
        let toml_str = r#"
[presets]
focus = "50m"
rest = "10m"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.presets.get("focus").unwrap(), "50m");
        assert_eq!(config.presets.get("rest").unwrap(), "10m");
    }

    #[test]
    fn resolve_preset_found() {
        let mut config = Config::default();
        config.presets.insert("pomodoro".to_string(), "25m".to_string());
        assert_eq!(config.resolve_preset("pomodoro"), Some("25m"));
    }

    #[test]
    fn resolve_preset_not_found() {
        let config = Config::default();
        assert_eq!(config.resolve_preset("nonexistent"), None);
    }

    #[test]
    fn default_sessions_include_pomodoro() {
        let config = Config::load();
        let session = config.resolve_session("pomodoro");
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.work, "pomodoro");
        assert_eq!(session.break_preset, "break");
        assert_eq!(session.long_break, "long-break");
        assert_eq!(session.rounds, 4);
    }

    #[test]
    fn parse_toml_session() {
        let toml_str = r#"
[presets]
focus = "50m"
rest = "10m"

[sessions]
deep = { work = "focus", break = "rest", long_break = "rest", rounds = 3 }
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let session = config.sessions.get("deep").unwrap();
        assert_eq!(session.work, "focus");
        assert_eq!(session.rounds, 3);
    }

    #[test]
    fn resolve_session_not_found() {
        let config = Config::default();
        assert!(config.resolve_session("nonexistent").is_none());
    }
}
