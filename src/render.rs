use crossterm::{
    cursor,
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{self, Write};

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

    pub fn draw(
        &self,
        remaining_secs: u64,
        total_secs: u64,
        elapsed_secs: u64,
        paused: bool,
    ) -> io::Result<()> {
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

        let mut stdout = io::stdout();

        execute!(stdout, terminal::Clear(ClearType::All))?;

        // Remaining time — bold, centered
        let time_col = cols.saturating_sub(remaining_str.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(time_col, mid_row - 1),
            SetAttribute(Attribute::Bold),
            Print(&remaining_str),
            SetAttribute(Attribute::Reset),
        )?;

        // Progress bar — centered
        let bar_col = cols.saturating_sub(self.bar_width) / 2;
        execute!(stdout, cursor::MoveTo(bar_col, mid_row + 1))?;

        execute!(stdout, SetForegroundColor(bar_color))?;
        for _ in 0..filled {
            execute!(stdout, Print("\u{2588}"))?; // █
        }
        execute!(stdout, SetForegroundColor(Color::DarkGrey))?;
        for _ in 0..empty {
            execute!(stdout, Print("\u{2591}"))?; // ░
        }
        execute!(stdout, ResetColor)?;

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
