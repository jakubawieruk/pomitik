use crate::config::{Config, SessionConfig};
use crate::duration::Duration;
use crate::log::LogEntry;
use crate::timer;
use chrono::Local;
use crossterm::{
    cursor, execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
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
