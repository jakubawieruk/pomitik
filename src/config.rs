use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SessionConfig {
    pub work: String,
    #[serde(rename = "break")]
    pub break_preset: String,
    pub long_break: String,
    pub rounds: u32,
}

#[derive(Debug, Deserialize, Serialize, Default)]
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
            .join("pomitik")
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

    pub fn show_config(&self) {
        let defaults = Self::defaults();
        let default_rounds: u32 = 4;

        let keys = [("work", "pomodoro"), ("break", "break"), ("long-break", "long-break")];
        for (display_key, preset_name) in &keys {
            let current = self.presets.get(*preset_name).map(|s| s.as_str()).unwrap_or("??");
            let is_default = defaults.get(*preset_name).map(|s| s.as_str()) == Some(current);
            let suffix = if is_default { "  (default)" } else { "" };
            println!("{:<12}{}{}", display_key, current, suffix);
        }

        let session = self.sessions.get("pomodoro");
        let current_rounds = session.map(|s| s.rounds).unwrap_or(default_rounds);
        let is_default = current_rounds == default_rounds;
        let suffix = if is_default { "  (default)" } else { "" };
        println!("{:<12}{}{}", "rounds", current_rounds, suffix);
    }

    pub fn set_value(key: &str, value: &str) -> Result<(), String> {
        if key == "rounds" {
            let rounds: u32 = value.parse().map_err(|_| {
                format!("Invalid rounds value: '{value}'. Must be a positive integer.")
            })?;
            if rounds == 0 {
                return Err("Rounds must be greater than zero.".to_string());
            }
            Self::update_config_file(|config_str| Self::set_toml_rounds(config_str, rounds))?;
            println!("Updated rounds to {rounds}");
            return Ok(());
        }

        let preset_name = config_key_to_preset(key).ok_or_else(|| {
            format!("Unknown config key: '{key}'. Valid keys: work, break, long-break, rounds")
        })?;

        crate::duration::Duration::parse(value)
            .map_err(|e| format!("Invalid duration '{value}': {e}"))?;

        Self::update_config_file(|config_str| {
            Self::set_toml_preset(config_str, preset_name, value)
        })?;
        println!("Updated {key} to {value}");
        Ok(())
    }

    fn update_config_file<F>(updater: F) -> Result<(), String>
    where
        F: FnOnce(&str) -> String,
    {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let updated = updater(&existing);
        std::fs::write(&path, updated).map_err(|e| format!("Failed to write config: {e}"))?;
        Ok(())
    }

    fn set_toml_preset(config_str: &str, key: &str, value: &str) -> String {
        let mut config: toml::Value = config_str
            .parse()
            .unwrap_or(toml::Value::Table(Default::default()));
        let table = config.as_table_mut().unwrap();
        let presets = table
            .entry("presets")
            .or_insert(toml::Value::Table(Default::default()));
        presets
            .as_table_mut()
            .unwrap()
            .insert(key.to_string(), toml::Value::String(value.to_string()));
        toml::to_string_pretty(&config).unwrap_or_default()
    }

    fn set_toml_rounds(config_str: &str, rounds: u32) -> String {
        let mut config: toml::Value = config_str
            .parse()
            .unwrap_or(toml::Value::Table(Default::default()));
        let table = config.as_table_mut().unwrap();
        let sessions = table
            .entry("sessions")
            .or_insert(toml::Value::Table(Default::default()));
        let sessions_table = sessions.as_table_mut().unwrap();
        let pomodoro = sessions_table
            .entry("pomodoro")
            .or_insert(toml::Value::Table(Default::default()));
        let pomodoro_table = pomodoro.as_table_mut().unwrap();

        pomodoro_table
            .entry("work")
            .or_insert(toml::Value::String("pomodoro".to_string()));
        pomodoro_table
            .entry("break")
            .or_insert(toml::Value::String("break".to_string()));
        pomodoro_table
            .entry("long_break")
            .or_insert(toml::Value::String("long-break".to_string()));
        pomodoro_table.insert("rounds".to_string(), toml::Value::Integer(rounds as i64));

        toml::to_string_pretty(&config).unwrap_or_default()
    }
}

pub fn config_key_to_preset(key: &str) -> Option<&'static str> {
    match key {
        "work" => Some("pomodoro"),
        "break" => Some("break"),
        "long-break" => Some("long-break"),
        _ => None,
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

    #[test]
    fn config_key_to_preset_mapping() {
        assert_eq!(super::config_key_to_preset("work"), Some("pomodoro"));
        assert_eq!(super::config_key_to_preset("break"), Some("break"));
        assert_eq!(super::config_key_to_preset("long-break"), Some("long-break"));
        assert_eq!(super::config_key_to_preset("rounds"), None);
        assert_eq!(super::config_key_to_preset("invalid"), None);
    }

    #[test]
    fn set_toml_preset_empty_config() {
        let result = Config::set_toml_preset("", "pomodoro", "30m");
        assert!(result.contains("pomodoro"));
        assert!(result.contains("30m"));
    }

    #[test]
    fn set_toml_preset_existing_config() {
        let existing = "[presets]\npomodoro = \"25m\"\n";
        let result = Config::set_toml_preset(existing, "pomodoro", "30m");
        assert!(result.contains("30m"));
    }

    #[test]
    fn set_toml_rounds_empty_config() {
        let result = Config::set_toml_rounds("", 6);
        assert!(result.contains("rounds = 6"));
    }
}
