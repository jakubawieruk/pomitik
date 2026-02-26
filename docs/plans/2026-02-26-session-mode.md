# Session Mode Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add session mode so `tik pomodoro` runs a full work→break cycle (N rounds, long-break at end) automatically.

**Architecture:** Add a `SessionConfig` struct to the config module, a new `session.rs` module that orchestrates multiple timer runs, and update main.rs resolution order to check sessions first. The existing `timer::run()` gets a small enhancement to show a round header before countdown.

**Tech Stack:** Same crates as existing project (clap, tokio, crossterm, serde/toml, chrono)

---

### Task 1: Extend Config with Sessions

**Files:**
- Modify: `src/config.rs`

**Step 1: Write tests for session config**

Add to the tests module in `src/config.rs`:

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test config`
Expected: Fails — `resolve_session` and `SessionConfig` don't exist.

**Step 3: Implement SessionConfig and update Config**

Add the `SessionConfig` struct and update `Config`:

```rust
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
```

Add default sessions in `defaults()` style — add a `default_sessions()` method:

```rust
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
```

Update `load()` to merge sessions the same way it merges presets:

```rust
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
```

Add `resolve_session`:

```rust
pub fn resolve_session(&self, name: &str) -> Option<&SessionConfig> {
    self.sessions.get(name)
}
```

**Step 4: Run tests**

Run: `cargo test config`
Expected: All 9 tests pass (6 existing + 3 new).

**Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add session config with built-in pomodoro session"
```

---

### Task 2: Add Session Runner

**Files:**
- Create: `src/session.rs`

**Step 1: Implement the session orchestrator**

Create `src/session.rs`:

```rust
use crate::config::{Config, SessionConfig};
use crate::duration::Duration;
use crate::log::LogEntry;
use crate::timer;
use chrono::Local;
use crossterm::{cursor, execute, style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor}, terminal::{self, ClearType}};
use std::io::{self, Write};

pub async fn run_session(session: &SessionConfig, config: &Config, silent: bool) {
    let rounds = session.rounds;

    for round in 1..=rounds {
        // --- Work phase ---
        let work_duration_str = config
            .resolve_preset(&session.work)
            .unwrap_or(&session.work);
        let work_dur = match Duration::parse(work_duration_str) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Invalid work duration '{}': {e}", session.work);
                return;
            }
        };

        show_round_header(round, rounds, &session.work, &work_dur.format_hms());
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let result = timer::run(work_dur.total_secs, &session.work).await;
        if !result.completed {
            println!("Session cancelled.");
            return;
        }

        crate::notify::send_completion(&session.work, &work_dur.format_hms(), silent);
        log_entry(&session.work, work_dur.total_secs);

        // --- Break phase ---
        let (break_name, break_duration_str) = if round == rounds {
            // Last round: long break
            let dur_str = config
                .resolve_preset(&session.long_break)
                .unwrap_or(&session.long_break);
            (&session.long_break, dur_str.to_string())
        } else {
            let dur_str = config
                .resolve_preset(&session.break_preset)
                .unwrap_or(&session.break_preset);
            (&session.break_preset, dur_str.to_string())
        };

        let break_dur = match Duration::parse(&break_duration_str) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Invalid break duration '{break_name}': {e}");
                return;
            }
        };

        show_round_header(round, rounds, break_name, &break_dur.format_hms());
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let result = timer::run(break_dur.total_secs, break_name).await;
        if !result.completed {
            println!("Session cancelled.");
            return;
        }

        crate::notify::send_completion(break_name, &break_dur.format_hms(), silent);
        log_entry(break_name, break_dur.total_secs);
    }

    println!("Session complete! {} rounds finished.", rounds);
}

fn show_round_header(round: u32, total: u32, name: &str, duration: &str) {
    // Use alternate screen briefly to show the header
    let _ = terminal::enable_raw_mode();
    let _ = execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide);

    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let mid_row = rows / 2;

    let line1 = format!("Round {round}/{total}");
    let line2 = format!("{name} ({duration})");

    let col1 = cols.saturating_sub(line1.len() as u16) / 2;
    let col2 = cols.saturating_sub(line2.len() as u16) / 2;

    let _ = execute!(
        io::stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(col1, mid_row.saturating_sub(1)),
        SetForegroundColor(Color::Cyan),
        SetAttribute(Attribute::Bold),
        Print(&line1),
        SetAttribute(Attribute::Reset),
        ResetColor,
        cursor::MoveTo(col2, mid_row + 1),
        SetForegroundColor(Color::DarkGrey),
        Print(&line2),
        ResetColor,
    );
    let _ = io::stdout().flush();
    let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
}

fn log_entry(name: &str, duration_secs: u64) {
    let entry = LogEntry {
        name: name.to_string(),
        duration_secs,
        completed_at: Local::now(),
    };
    if let Err(e) = crate::log::append_entry(&entry) {
        eprintln!("Failed to write log: {e}");
    }
}
```

**Step 2: Add `mod session;` to `src/main.rs`**

Add `mod session;` after the other module declarations.

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles (with warnings about unused `session` module since main.rs doesn't call it yet).

**Step 4: Commit**

```bash
git add src/session.rs src/main.rs
git commit -m "feat: add session runner for chained work/break cycles"
```

---

### Task 3: Update Main Resolution Order

**Files:**
- Modify: `src/main.rs`

**Step 1: Update the main function**

Replace the input resolution logic in `main()`. After `let input = match cli.duration { ... };`, the new logic should be:

```rust
// Resolution order: session → preset → duration
let config = config::Config::load();

// 1. Check if it's a session
if let Some(session_config) = config.resolve_session(&input) {
    let session_config = session_config.clone();
    session::run_session(&session_config, &config, cli.silent).await;
    return;
}

// 2. Try parsing as duration, then as preset (existing logic)
let (name, dur) = match duration::Duration::parse(&input) {
    Ok(d) => (input.clone(), d),
    Err(_) => {
        match config.resolve_preset(&input) {
            Some(preset_duration) => match duration::Duration::parse(preset_duration) {
                Ok(d) => (input.clone(), d),
                Err(e) => {
                    eprintln!("Invalid preset duration for '{input}': {e}");
                    std::process::exit(1);
                }
            },
            None => {
                eprintln!("Unknown duration or preset: '{input}'");
                eprintln!("Valid formats: 25m, 1h30m, 90s");
                eprintln!("Built-in presets: pomodoro, break, long-break");
                std::process::exit(1);
            }
        }
    }
};

// ... rest stays the same (single timer run)
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors.

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass (25 existing + 3 new config tests = 28).

**Step 4: Manual test**

Run: `cargo run -- pomodoro`
Expected: Shows "Round 1/4" header, then starts 25:00 countdown. Press q to cancel.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: update resolution order — sessions take priority over presets"
```
