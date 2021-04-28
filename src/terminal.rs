use std::fmt;
use std::io::{stdout, Write};

use crossterm::cursor;
use crossterm::style::{Print, SetBackgroundColor as Bg, SetForegroundColor as Fg};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{ExecutableCommand, QueueableCommand};
use url::Url;

use crate::gemini::gemtext::Line;
use crate::gemini::StatusCode;

mod colors;

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

pub struct Terminal {
    width: u16,
    height: u16,
    cursor_pos: CursorPosition,
}

impl fmt::Debug for Terminal {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Terminal")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("cursor_pos", &self.cursor_pos)
            .finish()
    }
}

enum Render {
    Continue,
    Break,
}

impl Terminal {
    pub fn teardown() -> crossterm::Result<()> {
        stdout().queue(LeaveAlternateScreen)?.queue(cursor::Show)?;
        terminal::disable_raw_mode()?;
        stdout().flush()?;
        Ok(())
    }

    pub fn setup_alternate_screen() -> crossterm::Result<Self> {
        terminal::enable_raw_mode()?;

        stdout()
            .queue(EnterAlternateScreen)?
            // Hide the cusor, clear the screen, and set the initial cursor position
            .queue(cursor::Hide)?
            .queue(Bg(colors::BACKGROUND))?
            .queue(terminal::Clear(terminal::ClearType::All))?
            .queue(cursor::MoveTo(1, 1))?;

        stdout().flush()?;

        let (width, height) = terminal::size()?;

        Ok(Self {
            width,
            height,
            cursor_pos: CursorPosition { x: 1, y: 1 }, // 1-based
        })
    }

    pub fn render_page(
        &mut self,
        current_line: usize,
        content: String,
        url: &Url,
        status_code: StatusCode,
    ) -> crossterm::Result<()> {
        // Move back to the beginning before drawing page
        self.cursor_pos.x = 1;
        self.cursor_pos.y = 1;

        for (i, line) in content.lines().enumerate() {
            let is_active = current_line == i;

            match self.render_line(line, is_active)? {
                Render::Continue => {}
                Render::Break => break,
            }
        }

        self.draw_status_line(url, status_code);

        stdout().flush()?;

        Ok(())
    }

    fn render_line(&mut self, line: &str, is_active: bool) -> crossterm::Result<Render> {
        // Highlight the current line
        let bg_color = if is_active {
            Bg(colors::REGENT_GREY)
        } else {
            Bg(colors::BACKGROUND)
        };

        match Line::parse(line) {
            Line::Normal => {
                for part in textwrap::wrap(line, self.width as usize) {
                    // If we're going to overflow the screen, stop printing
                    if self.cursor_pos.y + 1 > self.height {
                        return Ok(Render::Break);
                    }

                    // If we've got a blank line, render a space so we can
                    // see it when it's highlighted
                    if line.is_empty() {
                        stdout()
                            .queue(self.cursor_pos.move_to())?
                            .queue(Fg(colors::FOREGROUND))?
                            .queue(bg_color)?
                            .queue(Print(" "))?;
                    } else {
                        stdout()
                            .queue(self.cursor_pos.move_to())?
                            .queue(Fg(colors::FOREGROUND))?
                            .queue(bg_color)?
                            .queue(Print(part))?;
                    }

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
                stdout()
                    .queue(self.cursor_pos.move_to())?
                    .queue(bg_color)?
                    .queue(Fg(colors::MANTIS))?
                    .queue(Print("=> "))?
                    .queue(Fg(colors::FOREGROUND))?
                    .queue(Print(name.unwrap_or_else(|| url.clone())))?
                    .queue(Fg(colors::REGENT_GREY))?
                    .queue(Print(" "))?
                    .queue(Print(url))?; // TODO: Hide if we don't have a name because the URL is already being displayed

                self.cursor_pos.x = 1;
                self.cursor_pos.y += 1;
            }
        }

        Ok(Render::Continue)
    }

    fn draw_status_line(&mut self, url: &Url, status_code: StatusCode) {
        self.cursor_pos.x = 1;
        self.cursor_pos.y = self.height;

        write!(
            stdout(),
            "{cursor_pos}{fg_1}{bg_1} {status_code} {fg_2}{bg_2} {url:width$}",
            cursor_pos = self.cursor_pos.move_to(),
            fg_1 = Fg(colors::GREEN_SMOKE),
            bg_1 = Bg(colors::REGENT_GREY),
            fg_2 = Fg(colors::FOREGROUND),
            bg_2 = Bg(colors::BACKGROUND),
            status_code = status_code.code(),
            url = url,
            width = self.width as usize - 5
        )
        .unwrap();
    }

    pub fn clear_screen(&mut self) -> crossterm::Result<()> {
        stdout()
            .execute(terminal::Clear(terminal::ClearType::All))?
            .execute(Bg(colors::BACKGROUND))?
            .execute(cursor::MoveTo(1, 1))?;

        Ok(())
    }

    pub fn flush(&mut self) -> crossterm::Result<()> {
        stdout().flush()?;

        Ok(())
    }
}
