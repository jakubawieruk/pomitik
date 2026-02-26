use crossterm::{
    cursor,
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{self, Write};

pub struct DrawParams<'a> {
    pub remaining_secs: u64,
    pub total_secs: u64,
    pub elapsed_secs: u64,
    pub paused: bool,
    pub title: Option<&'a str>,
    pub round_info: Option<(u32, u32)>,  // (current_round, total_rounds)
    pub context: crate::timer::TimerContext,
}

pub struct Renderer {
    bar_width: u16,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer { bar_width: 30 }
    }

    pub fn setup(&self) -> io::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(())
    }

    pub fn teardown(&self) -> io::Result<()> {
        execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    pub fn draw(&self, params: &DrawParams) -> io::Result<()> {
        let remaining_secs = params.remaining_secs;
        let total_secs = params.total_secs;
        let elapsed_secs = params.elapsed_secs;
        let paused = params.paused;

        let (cols, rows) = terminal::size()?;
        let mid_row = rows / 2;

        let remaining_str = format_time(remaining_secs);
        let elapsed_str = format_time(elapsed_secs);
        let progress = if total_secs > 0 {
            1.0 - (remaining_secs as f64 / total_secs as f64)
        } else {
            1.0
        };

        let filled = (progress * self.bar_width as f64) as u16;
        let empty = self.bar_width - filled;

        // Color: green → yellow (last 20%) → red (last 60s)
        let bar_color = if remaining_secs <= 60 {
            Color::Red
        } else if remaining_secs as f64 <= total_secs as f64 * 0.2 {
            Color::Yellow
        } else {
            Color::Green
        };

        // Build progress bar string
        let bar_filled: String = "\u{2588}".repeat(filled as usize);
        let bar_empty: String = "\u{2591}".repeat(empty as usize);

        let mut stdout = io::stdout();

        execute!(stdout, terminal::Clear(ClearType::All))?;

        // Title — white, bold, centered
        if let Some(title) = params.title {
            let title_row = mid_row.saturating_sub(4);
            let title_col = cols.saturating_sub(title.len() as u16) / 2;
            execute!(
                stdout,
                cursor::MoveTo(title_col, title_row),
                SetForegroundColor(Color::White),
                SetAttribute(Attribute::Bold),
                Print(title),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

        // Round info — cyan, bold, centered
        if let Some((current, total)) = params.round_info {
            let round_str = format!("Round {current}/{total}");
            let round_col = cols.saturating_sub(round_str.len() as u16) / 2;
            let round_row = mid_row.saturating_sub(3);
            execute!(
                stdout,
                cursor::MoveTo(round_col, round_row),
                SetForegroundColor(Color::Cyan),
                SetAttribute(Attribute::Bold),
                Print(&round_str),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

        // Remaining time — bold, centered
        let time_col = cols.saturating_sub(remaining_str.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(time_col, mid_row.saturating_sub(1)),
            SetAttribute(Attribute::Bold),
            Print(&remaining_str),
            SetAttribute(Attribute::Reset),
        )?;

        // Progress bar — centered, printed as single strings
        let bar_col = cols.saturating_sub(self.bar_width) / 2;
        execute!(
            stdout,
            cursor::MoveTo(bar_col, mid_row + 1),
            SetForegroundColor(bar_color),
            Print(&bar_filled),
            SetForegroundColor(Color::DarkGrey),
            Print(&bar_empty),
            ResetColor,
        )?;

        // Elapsed or "PAUSED" — dim, centered
        let label = if paused {
            "PAUSED".to_string()
        } else {
            format!("{elapsed_str} elapsed")
        };
        let label_col = cols.saturating_sub(label.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(label_col, mid_row + 3),
            SetForegroundColor(Color::DarkGrey),
            Print(&label),
            ResetColor,
        )?;

        // Hint bar — dark grey, centered
        let hints = match params.context {
            crate::timer::TimerContext::Standalone => {
                "[space] pause  [s] skip  [x] stop"
            }
            crate::timer::TimerContext::Work | crate::timer::TimerContext::Break => {
                "[space] pause  [s] skip  [a] +round  [x] stop"
            }
        };
        let hints_col = cols.saturating_sub(hints.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(hints_col, mid_row + 5),
            SetForegroundColor(Color::DarkGrey),
            Print(hints),
            ResetColor,
        )?;

        stdout.flush()?;
        Ok(())
    }
}

fn format_time(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}
