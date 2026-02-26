use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::watch;

use crate::render::Renderer;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerContext {
    Standalone,
    Work,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerOutcome {
    Completed,
    Skipped,
    StoppedEarly,
    Quit,
}

pub async fn run(
    total_secs: u64,
    _name: &str,
    context: TimerContext,
    _title: Option<&str>,
    round_info: Option<(u32, Arc<AtomicU32>)>,
) -> TimerOutcome {
    let renderer = Renderer::new();
    if let Err(e) = renderer.setup() {
        eprintln!("Failed to setup terminal: {e}");
        return TimerOutcome::Quit;
    }

    let (pause_tx, pause_rx) = watch::channel(false);
    let (quit_tx, quit_rx) = watch::channel(false);
    let (skip_tx, skip_rx) = watch::channel(false);
    let (stop_tx, stop_rx) = watch::channel(false);

    // Spawn a thread for keyboard input (crossterm events are blocking)
    let pause_tx_clone = pause_tx.clone();
    let quit_tx_clone = quit_tx.clone();
    let skip_tx_clone = skip_tx.clone();
    let stop_tx_clone = stop_tx.clone();
    let round_info_clone = round_info.clone();
    let context_clone = context;
    std::thread::spawn(move || {
        loop {
            if event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    match key {
                        KeyEvent {
                            code: KeyCode::Char(' '),
                            ..
                        } => {
                            let current = *pause_tx_clone.borrow();
                            let _ = pause_tx_clone.send(!current);
                        }
                        KeyEvent {
                            code: KeyCode::Char('c'),
                            modifiers,
                            ..
                        } if modifiers.contains(KeyModifiers::CONTROL) => {
                            let _ = quit_tx_clone.send(true);
                            break;
                        }
                        KeyEvent {
                            code: KeyCode::Char('q'),
                            ..
                        } => {
                            let _ = quit_tx_clone.send(true);
                            break;
                        }
                        KeyEvent {
                            code: KeyCode::Char('s'),
                            ..
                        } => {
                            let _ = skip_tx_clone.send(true);
                            break;
                        }
                        KeyEvent {
                            code: KeyCode::Char('x'),
                            ..
                        } => {
                            let _ = stop_tx_clone.send(true);
                            break;
                        }
                        KeyEvent {
                            code: KeyCode::Char('a'),
                            ..
                        } => {
                            if matches!(context_clone, TimerContext::Work | TimerContext::Break) {
                                if let Some(ref ri) = round_info_clone {
                                    ri.1.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            if *quit_tx_clone.borrow() {
                break;
            }
        }
    });

    let start = Instant::now();
    let mut paused_duration = std::time::Duration::ZERO;
    let mut pause_start: Option<Instant> = None;
    let mut completed = false;

    loop {
        // Check quit
        if *quit_rx.borrow() {
            break;
        }
        if *skip_rx.borrow() {
            let _ = renderer.teardown();
            return TimerOutcome::Skipped;
        }
        if *stop_rx.borrow() {
            let _ = renderer.teardown();
            return TimerOutcome::StoppedEarly;
        }

        let is_paused = *pause_rx.borrow();

        // Track pause duration
        if is_paused {
            if pause_start.is_none() {
                pause_start = Some(Instant::now());
            }
        } else if let Some(ps) = pause_start.take() {
            paused_duration += ps.elapsed();
        }

        let current_pause = pause_start.map_or(std::time::Duration::ZERO, |ps| ps.elapsed());
        let active_elapsed = start.elapsed() - paused_duration - current_pause;

        let elapsed_secs = active_elapsed.as_secs();
        let remaining_secs = total_secs.saturating_sub(elapsed_secs);

        if renderer
            .draw(remaining_secs, total_secs, elapsed_secs, is_paused)
            .is_err()
        {
            break;
        }

        if remaining_secs == 0 {
            completed = true;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

    let _ = renderer.teardown();
    if completed { TimerOutcome::Completed } else { TimerOutcome::Quit }
}
