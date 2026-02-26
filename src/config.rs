use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub presets: HashMap<String, String>,
}

impl Config {
    pub fn load() -> Self {
        let mut presets = Self::defaults();
        let path = Self::config_path();
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(user_config) = toml::from_str::<Config>(&contents) {
                    for (k, v) in user_config.presets {
                        presets.insert(k, v);
                    }
                }
            }
        }
        Config { presets }
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

    pub fn resolve_preset(&self, name: &str) -> Option<&str> {
        self.presets.get(name).map(|s| s.as_str())
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
}
