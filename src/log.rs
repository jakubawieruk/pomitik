use chrono::{DateTime, Datelike, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub name: String,
    pub duration_secs: u64,
    pub completed_at: DateTime<Local>,
}

pub fn log_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tik")
        .join("log.json")
}

pub fn append_entry(entry: &LogEntry) -> std::io::Result<()> {
    let path = log_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let mut json = serde_json::to_string(entry)?;
    json.push('\n');
    file.write_all(json.as_bytes())?;
    Ok(())
}

pub fn read_entries() -> Vec<LogEntry> {
    let path = log_path();
    if !path.exists() {
        return Vec::new();
    }
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    contents
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn format_duration_human(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

pub fn print_summary() {
    let entries = read_entries();
    if entries.is_empty() {
        println!("No sessions logged yet.");
        return;
    }

    let now = Local::now();
    let today = now.date_naive();
    let days_since_monday = now.weekday().num_days_from_monday();
    let week_start = today - chrono::Duration::days(days_since_monday as i64);

    let today_entries: Vec<&LogEntry> = entries
        .iter()
        .filter(|e| e.completed_at.date_naive() == today)
        .collect();

    let week_entries: Vec<&LogEntry> = entries
        .iter()
        .filter(|e| e.completed_at.date_naive() >= week_start)
        .collect();

    print_section("Today", &today_entries);
    println!();
    print_section("This week", &week_entries);
}

fn print_section(title: &str, entries: &[&LogEntry]) {
    let total_secs: u64 = entries.iter().map(|e| e.duration_secs).sum();
    let count = entries.len();

    println!(
        "{title} ({count} session{}, {}):",
        if count == 1 { "" } else { "s" },
        format_duration_human(total_secs)
    );

    if entries.is_empty() {
        println!("  (none)");
        return;
    }

    let mut by_name: HashMap<&str, (usize, u64)> = HashMap::new();
    for e in entries {
        let entry = by_name.entry(e.name.as_str()).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += e.duration_secs;
    }

    let mut names: Vec<_> = by_name.into_iter().collect();
    names.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));

    for (name, (count, secs)) in names {
        if count > 1 {
            println!("  {name:<14} x{count:<4} {}", format_duration_human(secs));
        } else {
            println!("  {name:<14}       {}", format_duration_human(secs));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_log_entry() {
        let entry = LogEntry {
            name: "pomodoro".to_string(),
            duration_secs: 1500,
            completed_at: Local::now(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("pomodoro"));
        assert!(json.contains("1500"));
    }

    #[test]
    fn deserialize_log_entry() {
        let json = r#"{"name":"pomodoro","duration_secs":1500,"completed_at":"2026-02-26T15:30:00+01:00"}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.name, "pomodoro");
        assert_eq!(entry.duration_secs, 1500);
    }

    #[test]
    fn roundtrip_entry() {
        let entry = LogEntry {
            name: "break".to_string(),
            duration_secs: 300,
            completed_at: Local::now(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, entry.name);
        assert_eq!(parsed.duration_secs, entry.duration_secs);
    }

    #[test]
    fn log_path_ends_with_expected() {
        let path = log_path();
        assert!(path.ends_with("tik/log.json"));
    }

    #[test]
    fn format_duration_human_minutes() {
        assert_eq!(format_duration_human(1500), "25m");
    }

    #[test]
    fn format_duration_human_hours_and_minutes() {
        assert_eq!(format_duration_human(5400), "1h 30m");
    }

    #[test]
    fn format_duration_human_hours_only() {
        assert_eq!(format_duration_human(3600), "1h 0m");
    }

    #[test]
    fn format_duration_human_zero() {
        assert_eq!(format_duration_human(0), "0m");
    }
}
