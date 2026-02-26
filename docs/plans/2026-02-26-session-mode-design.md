# Session Mode Design

## Goal

`tik pomodoro` runs a full Pomodoro cycle automatically: work → break → work → break → ... → work → long-break. Configurable rounds (default 4).

## Resolution order

When user runs `tik <input>`:
1. Session name → run session mode (chained timers)
2. Preset name → run single timer
3. Duration string → run single timer
4. Error

## Config

`~/.config/tik/config.toml` gains a `[sessions]` table:

```toml
[presets]
pomodoro = "25m"
break = "5m"
long-break = "15m"

[sessions]
pomodoro = { work = "pomodoro", break = "break", long_break = "long-break", rounds = 4 }
```

Built-in default: pomodoro session with those values, works with no config file.

## Behavior

For a session with N rounds:
- Rounds 1 through N: run work timer, then break timer
- After round N: run long-break instead of regular break
- Each timer fires a macOS notification on completion
- Between timers: show "Round X/N — preset (duration)" for 2 seconds before countdown starts
- Each completed timer gets its own log entry

## Cancellation

If user quits (q/Ctrl+C) during any timer in the session, the entire session stops. Only completed timers are logged.
