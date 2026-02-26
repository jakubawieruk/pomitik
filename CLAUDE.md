# CLAUDE.md

## What is this?

**pomitik** is a command-line countdown timer with Pomodoro session support. The package is called `pomitik` but the binary/command users type is `tik`.

## Naming

- Package name: `pomitik` (Cargo.toml, Homebrew, Scoop)
- Binary name: `tik` (what users type in terminal)
- Config dir: `~/.config/pomitik/`
- Log file: `~/.local/share/pomitik/log.json`

## Architecture

Single-binary Rust CLI. Two modes: timer (default) and log subcommand.

```
src/
  main.rs       — clap CLI, resolution order: session → preset → duration
  duration.rs   — parse "25m", "1h30m", "90s" into seconds; format back
  config.rs     — TOML config + built-in presets/sessions, SessionConfig struct
  session.rs    — orchestrates work→break→...→long-break cycles
  timer.rs      — async countdown loop with pause/resume/quit via watch channels
  render.rs     — crossterm alternate screen: centered time, colored progress bar
  notify.rs     — macOS/Windows notifications via notify-rust
  log.rs        — NDJSON append/read, today/week summary display
```

## Key design decisions

- **Resolution order:** `tik pomodoro` checks sessions first, then presets, then raw duration parsing. The built-in `pomodoro` session takes priority over the `pomodoro` preset.
- **Keyboard input:** Runs on a separate OS thread (crossterm events are blocking), communicates with the async timer loop via `tokio::sync::watch` channels.
- **Pause tracking:** Tracks accumulated pause duration separately so only active time counts toward the countdown.
- **Rendering:** Uses crossterm alternate screen. Progress bar is built as strings before printing (single `execute!` call) to avoid flickering. Color transitions: green → yellow (last 20%) → red (last 60s).
- **Notification sound:** Platform-conditional with `#[cfg(target_os = "macos")]` — macOS uses "Glass" sound, Windows uses default toast sound.
- **Session log:** Newline-delimited JSON (one entry per line), easy to append without parsing the whole file.

## Build & test

```bash
cargo build
cargo test          # 28 tests
cargo install --path .
```

## Distribution

- **Homebrew tap:** `jakubawieruk/homebrew-pomitik`
- **Scoop bucket:** `jakubawieruk/scoop-pomitik`
- **CI:** GitHub Actions (`.github/workflows/release.yml`) — push a `v*` tag to build for macOS arm64, macOS x86_64, and Windows x86_64, then create a GitHub release with all three archives.

## Dependencies

clap (derive), tokio, crossterm, notify-rust, serde/serde_json/toml, chrono, dirs, regex
