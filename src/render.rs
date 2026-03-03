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
    pub todo: Option<&'a crate::todo::TodoSnapshot>,
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
        let (cols, rows) = terminal::size()?;
        let mut stdout = io::stdout();
        execute!(stdout, terminal::Clear(ClearType::All))?;

        if let Some(todo_snap) = params.todo {
            self.draw_with_sidebar(&mut stdout, params, todo_snap, cols, rows)?;
        } else {
            self.draw_centered(&mut stdout, params, cols, rows)?;
        }

        stdout.flush()?;
        Ok(())
    }

    fn draw_centered(&self, stdout: &mut io::Stdout, params: &DrawParams, cols: u16, rows: u16) -> io::Result<()> {
        let remaining_secs = params.remaining_secs;
        let total_secs = params.total_secs;
        let elapsed_secs = params.elapsed_secs;
        let paused = params.paused;

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

        // Color: green -> yellow (last 20%) -> red (last 60s)
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

        // Title -- white, bold, centered
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

        // Round info -- cyan, bold, centered
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

        // Remaining time -- bold, centered
        let time_col = cols.saturating_sub(remaining_str.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(time_col, mid_row.saturating_sub(1)),
            SetAttribute(Attribute::Bold),
            Print(&remaining_str),
            SetAttribute(Attribute::Reset),
        )?;

        // Progress bar -- centered, printed as single strings
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

        // Elapsed or "PAUSED" -- dim, centered
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

        // Hint bar -- dark grey, centered
        let is_last_round = params.round_info.is_some_and(|(cur, total)| cur >= total);
        let hints = match params.context {
            crate::timer::TimerContext::Standalone => {
                "[space] pause  [s] skip  [x] stop".to_string()
            }
            _ if is_last_round => {
                "[space] pause  [a/d] +/-round  [x] stop".to_string()
            }
            _ => {
                "[space] pause  [s] skip  [a/d] +/-round  [x] stop".to_string()
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

        Ok(())
    }

    fn draw_with_sidebar(&self, stdout: &mut io::Stdout, params: &DrawParams, todo: &crate::todo::TodoSnapshot, cols: u16, rows: u16) -> io::Result<()> {
        // Fall back to centered if terminal too narrow
        if cols < 60 {
            return self.draw_centered(stdout, params, cols, rows);
        }

        let sidebar_width: u16 = 32;
        let separator_col = cols.saturating_sub(sidebar_width);
        let left_width = separator_col.saturating_sub(1);
        let mid_row = rows / 2;

        // --- Left side: timer (centered within left_width) ---

        // Current task above title (first non-done item)
        if let Some((_, text, _)) = todo.items.iter().find(|(_, _, done)| !done) {
            let label = format!("> {text}");
            let truncated = if label.len() > left_width as usize - 2 {
                format!("{}...", &label[..left_width as usize - 5])
            } else {
                label
            };
            let col = left_width.saturating_sub(truncated.len() as u16) / 2;
            execute!(
                stdout,
                cursor::MoveTo(col, mid_row.saturating_sub(5)),
                SetForegroundColor(Color::White),
                SetAttribute(Attribute::Bold),
                Print(&truncated),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

        // Title (optional)
        if let Some(title) = params.title {
            let col = left_width.saturating_sub(title.len() as u16) / 2;
            execute!(
                stdout,
                cursor::MoveTo(col, mid_row.saturating_sub(4)),
                SetForegroundColor(Color::White),
                SetAttribute(Attribute::Bold),
                Print(title),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

        // Round info
        if let Some((current, total)) = params.round_info {
            let round_str = format!("Round {current}/{total}");
            let col = left_width.saturating_sub(round_str.len() as u16) / 2;
            execute!(
                stdout,
                cursor::MoveTo(col, mid_row.saturating_sub(3)),
                SetForegroundColor(Color::Cyan),
                SetAttribute(Attribute::Bold),
                Print(&round_str),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

        // Remaining time
        let remaining_str = format_time(params.remaining_secs);
        let time_col = left_width.saturating_sub(remaining_str.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(time_col, mid_row.saturating_sub(1)),
            SetAttribute(Attribute::Bold),
            Print(&remaining_str),
            SetAttribute(Attribute::Reset),
        )?;

        // Progress bar
        let progress = if params.total_secs > 0 {
            1.0 - (params.remaining_secs as f64 / params.total_secs as f64)
        } else { 1.0 };
        let filled = (progress * self.bar_width as f64) as u16;
        let empty = self.bar_width - filled;
        let bar_color = if params.remaining_secs <= 60 {
            Color::Red
        } else if params.remaining_secs as f64 <= params.total_secs as f64 * 0.2 {
            Color::Yellow
        } else {
            Color::Green
        };
        let bar_filled: String = "\u{2588}".repeat(filled as usize);
        let bar_empty: String = "\u{2591}".repeat(empty as usize);
        let bar_col = left_width.saturating_sub(self.bar_width) / 2;
        execute!(
            stdout,
            cursor::MoveTo(bar_col, mid_row + 1),
            SetForegroundColor(bar_color),
            Print(&bar_filled),
            SetForegroundColor(Color::DarkGrey),
            Print(&bar_empty),
            ResetColor,
        )?;

        // Elapsed / PAUSED
        let elapsed_str = format_time(params.elapsed_secs);
        let label = if params.paused { "PAUSED".to_string() } else { format!("{elapsed_str} elapsed") };
        let label_col = left_width.saturating_sub(label.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(label_col, mid_row + 3),
            SetForegroundColor(Color::DarkGrey),
            Print(&label),
            ResetColor,
        )?;

        // Hint bar -- changes based on focus mode
        let hints = if todo.focus {
            "[tab] timer  [\u{2191}\u{2193}] select  [enter] done  [S-\u{2191}\u{2193}] move".to_string()
        } else {
            let is_last_round = params.round_info.is_some_and(|(cur, total)| cur >= total);
            match params.context {
                crate::timer::TimerContext::Standalone => {
                    "[space] pause  [s] skip  [tab] tasks  [x] stop".to_string()
                }
                _ if is_last_round => {
                    "[space] pause  [a/d] +/-round  [tab] tasks  [x] stop".to_string()
                }
                _ => {
                    "[space] pause  [s] skip  [a/d] +/-round  [tab] tasks  [x] stop".to_string()
                }
            }
        };
        let hints_col = left_width.saturating_sub(hints.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(hints_col, mid_row + 5),
            SetForegroundColor(Color::DarkGrey),
            Print(&hints),
            ResetColor,
        )?;

        // --- Vertical separator ---
        for row in 0..rows {
            execute!(
                stdout,
                cursor::MoveTo(separator_col, row),
                SetForegroundColor(Color::DarkGrey),
                Print("\u{2502}"),
                ResetColor,
            )?;
        }

        // --- Right side: todo list ---
        let right_start = separator_col + 2;
        let max_text_width = (sidebar_width - 4) as usize;

        execute!(
            stdout,
            cursor::MoveTo(right_start, 1),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print("Tasks:"),
            SetAttribute(Attribute::Reset),
            ResetColor,
        )?;

        let first_pending_idx = todo.items.iter().position(|(_, _, done)| !done);

        for (i, (_, text, done)) in todo.items.iter().enumerate() {
            let row = 3 + i as u16;
            if row >= rows - 1 { break; } // don't overflow terminal

            let is_selected = todo.focus && i == todo.selected_index;
            let truncated = if text.len() > max_text_width {
                format!("{}...", &text[..max_text_width - 3])
            } else {
                text.clone()
            };

            // Determine prefix and color
            let (prefix, color) = if *done {
                ("\u{2713} ", Color::DarkGrey) // checkmark
            } else if Some(i) == first_pending_idx {
                ("> ", Color::White) // current task marker
            } else {
                ("  ", Color::Grey) // other pending tasks
            };

            let highlight_color = if is_selected { Color::Cyan } else { color };

            execute!(stdout, cursor::MoveTo(right_start, row), SetForegroundColor(highlight_color))?;

            if is_selected {
                execute!(stdout, SetAttribute(Attribute::Bold))?;
            }
            if *done {
                execute!(stdout, SetAttribute(Attribute::CrossedOut))?;
            }

            execute!(
                stdout,
                Print(prefix),
                Print(&truncated),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

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
