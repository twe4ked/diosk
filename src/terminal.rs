use std::borrow::Cow;
use std::io::{stdout, Write};

use crossterm::cursor;
use crossterm::style::{Print, SetBackgroundColor as Bg, SetForegroundColor as Fg};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{ExecutableCommand, QueueableCommand};

use crate::gemini::gemtext::Line;
use crate::state::{self, StatusLineContext};

pub mod colors;

#[derive(Debug)]
struct CursorPosition {
    x: u16,
    y: u16,
}

impl CursorPosition {
    // TODO: We might want to switch to using 0 based indexes,
    // since that's what crossterm deals with
    fn move_to(&self) -> cursor::MoveTo {
        cursor::MoveTo(self.x - 1, self.y - 1)
    }
}

#[derive(Debug)]
pub struct Terminal {
    width: u16,
    height: u16,
    cursor_pos: CursorPosition,
    current_row: u16,
}

enum Render {
    Continue(u16),
    Break,
}

impl Terminal {
    pub fn new() -> crossterm::Result<Self> {
        let (width, height) = terminal::size()?;

        stdout().queue(cursor::MoveTo(1, 1))?;

        Ok(Self {
            width,
            height,
            cursor_pos: CursorPosition { x: 1, y: 1 }, // 1-based
            current_row: 1,
        })
    }

    pub fn render_page(
        current_line_index: usize,
        content: Option<String>,
        scroll_offset: u16,
        mode: &state::Mode,
        status_line_context: StatusLineContext,
    ) -> crossterm::Result<()> {
        let mut terminal = Terminal::new().unwrap();

        let start_printing_from_row = scroll_offset + 1;
        let mut row = 0;

        let content = content.unwrap_or_else(|| match mode {
            state::Mode::Loading => "Loading...".to_string(),
            s => unimplemented!("not content for state: {:?}", s),
        });

        let mut buffer = Vec::new();

        for (i, line) in content.lines().enumerate() {
            let is_active = current_line_index == i;

            match terminal.render_line(&mut buffer, line, is_active)? {
                Render::Continue(r) => {
                    // How many rows the line took up
                    row += r;

                    if row >= start_printing_from_row {
                        stdout().write_all(&buffer).unwrap();
                    }
                }
                Render::Break => break,
            }

            if is_active {
                terminal.current_row = row;
            }

            buffer.clear();
        }

        terminal.draw_status_line(status_line_context);

        stdout().flush()?;

        Ok(())
    }

    fn render_line(
        &mut self,
        buffer: &mut Vec<u8>,
        line: &str,
        is_active: bool,
    ) -> crossterm::Result<Render> {
        let mut rows = 0;

        // Highlight the current line
        let bg_color = if is_active {
            Bg(colors::REGENT_GREY)
        } else {
            Bg(colors::BACKGROUND)
        };

        match Line::parse(line) {
            Line::Normal => {
                for mut part in textwrap::wrap(line, self.width as usize) {
                    // If we're going to overflow the screen, stop printing
                    if self.cursor_pos.y + 1 > self.height {
                        return Ok(Render::Break);
                    }

                    // If we've got a blank line, render a space so we can
                    // see it when it's highlighted
                    if line.is_empty() {
                        part = Cow::from(" ");
                    }

                    buffer
                        .queue(self.cursor_pos.move_to())?
                        .queue(Fg(colors::FOREGROUND))?
                        .queue(bg_color)?
                        .queue(Print(part))?;

                    rows += 1;
                    self.cursor_pos.x = 1;
                    self.cursor_pos.y += 1;
                }
            }
            Line::Link { url, name } => {
                // If we're going to overflow the screen, stop printing
                if self.cursor_pos.y + 1 > self.height {
                    return Ok(Render::Break);
                }

                // TODO: Handle wrapping
                buffer
                    .queue(self.cursor_pos.move_to())?
                    .queue(bg_color)?
                    .queue(Fg(colors::MANTIS))?
                    .queue(Print("=> "))?
                    .queue(Fg(colors::FOREGROUND))?
                    .queue(Print(name.unwrap_or_else(|| url.clone())))?
                    .queue(Fg(colors::REGENT_GREY))?
                    .queue(Print(" "))?
                    .queue(Print(url))?; // TODO: Hide if we don't have a name because the URL is already being displayed

                rows += 1;
                self.cursor_pos.x = 1;
                self.cursor_pos.y += 1;
            }
        }

        Ok(Render::Continue(rows))
    }

    fn draw_status_line(&mut self, status_line_context: StatusLineContext) {
        self.cursor_pos.x = 1;
        self.cursor_pos.y = self.height;

        let status_code = status_line_context
            .status_code
            .map(|s| s.code())
            .unwrap_or_else(|| "--".to_string());

        let url = status_line_context
            .url
            .map(|u| u.to_string())
            .unwrap_or_else(|| "-".to_string());

        print!(
            "{cursor_pos}{fg_1}{bg_1} {status_code} {fg_2}{bg_2} {url:width$}",
            cursor_pos = self.cursor_pos.move_to(),
            fg_1 = Fg(colors::GREEN_SMOKE),
            bg_1 = Bg(colors::COSTA_DEL_SOL),
            fg_2 = Fg(colors::FOREGROUND),
            bg_2 = Bg(colors::BACKGROUND),
            status_code = status_code,
            url = url,
            width = self.width as usize - 5
        );
    }

    /// The number of rows a line takes up when wrapped
    pub fn line_wrapped_rows(&self, line: &str) -> u16 {
        textwrap::wrap(line, self.width as usize).len() as _
    }

    pub fn page_rows(&self) -> u16 {
        // -1 for the status row
        self.height - 1
    }

    /// Represents the row that the cursor is on (indexed from the top of the screen)
    pub fn current_row(&self) -> u16 {
        self.current_row
    }

    pub fn clear_screen() -> crossterm::Result<()> {
        stdout()
            .execute(terminal::Clear(terminal::ClearType::All))?
            .execute(Bg(colors::BACKGROUND))?
            .execute(cursor::MoveTo(1, 1))?;

        Ok(())
    }

    pub fn flush() -> crossterm::Result<()> {
        stdout().flush()?;

        Ok(())
    }
}

pub fn setup_alternate_screen() -> crossterm::Result<()> {
    terminal::enable_raw_mode()?;

    stdout()
        .queue(EnterAlternateScreen)?
        // Hide the cusor, clear the screen, and set the initial cursor position
        .queue(cursor::Hide)?
        .queue(Bg(colors::BACKGROUND))?
        .queue(terminal::Clear(terminal::ClearType::All))?;

    stdout().flush()?;

    Ok(())
}

pub fn teardown() -> crossterm::Result<()> {
    stdout().queue(LeaveAlternateScreen)?.queue(cursor::Show)?;
    terminal::disable_raw_mode()?;
    stdout().flush()?;
    Ok(())
}
