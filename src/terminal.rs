use std::borrow::Cow;
use std::io::{stdout, Write};

use crossterm::cursor;
use crossterm::style::{Print, SetBackgroundColor as Bg, SetForegroundColor as Fg};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{ExecutableCommand, QueueableCommand};

use crate::gemini::gemtext::Line;
use crate::state::{Mode, StatusLineContext};

pub mod colors;

const LOGO: &str = r#"
     ,ogggggggg,
    dP"""88""""Y8b,                          ,dPYb,
    Yb,  88     `8b,                         IP'`Yb
     `"  88     `8b'gg                       I8  8I
         88      d8'""    ,ggggg,    ,g,     I8 dP" "8
         88     ,8P 88   dP"  "Y8ggg,8'8,    I8d8bggP"
         88___,dP'_,88_,d8,   ,d8',8'_   8) ,d8    `Yb,
        888888P"  8P""YP"Y8888P"  P' "YY8P8P88P      Y8

                    :go gemini://gemini.circumlunar.space<Enter>
                    :go [URL]<Enter>
                    :quit<Enter> :q<Enter>
"#;

#[derive(Debug)]
struct CursorPosition {
    x: u16,
    y: u16,
}

impl CursorPosition {
    fn move_to(&self) -> cursor::MoveTo {
        cursor::MoveTo(self.x, self.y)
    }
}

#[derive(Debug)]
pub struct Terminal {
    width: u16,
    height: u16,
}

impl Terminal {
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    pub fn render_page(
        &self,
        current_line_index: usize,
        content: Vec<Line>,
        scroll_offset: u16,
        status_line_context: StatusLineContext,
    ) -> crossterm::Result<u16> {
        if status_line_context.url.is_none() {
            self.render_default_page(status_line_context)?;
            stdout().flush()?;
            return Ok(0);
        }

        let start_printing_from_row = scroll_offset + 1;
        let mut row = 0;

        let mut cursor_pos = CursorPosition { x: 0, y: 0 };

        // The return value represents the row that the cursor is on, indexed from the top of the
        // screen
        let mut current_row = None;

        for (i, line) in content.iter().enumerate() {
            let is_active = current_line_index == i;

            let rows = self.render_line(line, is_active)?;
            for row_buffer in rows {
                row += 1;

                // Don't print before we're in view
                if row < start_printing_from_row {
                    // Reset the cursor position because we haven't drawn anything to the screen yet
                    cursor_pos.y = 0;
                    continue;
                }

                // TODO: Move this down once scrolling is row-by-row
                if is_active {
                    current_row = Some(row);
                }

                // If we're going to overflow the screen, stop printing
                if cursor_pos.y >= self.page_rows() {
                    break;
                }

                stdout().queue(&cursor_pos.move_to())?;
                stdout().write_all(&row_buffer).unwrap();

                cursor_pos.x = 0;
                cursor_pos.y += 1;
            }
        }

        self.draw_status_line(status_line_context);

        stdout().flush()?;

        Ok(current_row.expect("no current row"))
    }

    fn render_default_page(&self, status_line_context: StatusLineContext) -> crossterm::Result<()> {
        let logo_height: u16 = LOGO.lines().count() as _;
        let logo_width: u16 = LOGO.lines().map(|l| l.len()).max().expect("infallible") as _;

        let x = (self.width / 2) - (logo_width / 2);
        let y = (self.height / 2) - (logo_height / 2);

        // Move logo to the left slightly as its asymmetrical
        let x = x - 6;

        let mut cursor_pos = CursorPosition { x, y };

        for line in LOGO.lines() {
            print!("{}{}", cursor::MoveTo(cursor_pos.x, cursor_pos.y), line);
            cursor_pos.y += 1;
        }

        self.draw_status_line(status_line_context);

        stdout().flush()?;

        Ok(())
    }

    fn render_line(&self, line: &Line, is_active: bool) -> crossterm::Result<Vec<Vec<u8>>> {
        let mut rows = Vec::new();

        // Highlight the current line
        let bg_color = if is_active {
            Bg(colors::REGENT_GREY)
        } else {
            Bg(colors::BACKGROUND)
        };

        match line {
            Line::Normal(content) => {
                for mut part in textwrap::wrap(&content, self.width as usize) {
                    // If we've got a blank line, render a space so we can
                    // see it when it's highlighted
                    if content.is_empty() {
                        part = Cow::from(" ");
                    }

                    let mut row = Vec::new();
                    row.queue(Fg(colors::FOREGROUND))?
                        .queue(bg_color)?
                        .queue(Print(part))?;
                    rows.push(row);
                }
            }
            Line::Link { url, name } => {
                // TODO: Handle wrapping

                let mut row = Vec::new();
                row.queue(bg_color)?
                    .queue(Fg(colors::MANTIS))?
                    .queue(Print("=> "))?
                    .queue(Fg(colors::FOREGROUND))?
                    .queue(Print(name.as_ref().unwrap_or(url)))?
                    .queue(Fg(colors::REGENT_GREY))?
                    .queue(Print(" "))?
                    .queue(Print(url))?; // TODO: Hide if we don't have a name because the URL is already being displayed
                rows.push(row);
            }
            Line::InvalidLink => {
                let mut row = Vec::new();
                row.queue(bg_color)?
                    .queue(Fg(colors::MANTIS))?
                    .queue(Print("=> "))?
                    .queue(Fg(colors::OLD_BRICK))?
                    .queue(Print("[INVALID LINK]"))?;
                rows.push(row);
            }
        }

        Ok(rows)
    }

    fn draw_status_line(&self, status_line_context: StatusLineContext) {
        let cursor_pos = cursor::MoveTo(0, self.height - 1);

        if status_line_context.loading {
            print!(
                "{cursor_pos}{fg_1}{bg_1} Loading... {fg_2}{bg_2}",
                cursor_pos = cursor_pos,
                fg_1 = Fg(colors::GREEN_SMOKE),
                bg_1 = Bg(colors::COSTA_DEL_SOL),
                fg_2 = Fg(colors::FOREGROUND),
                bg_2 = Bg(colors::BACKGROUND),
            );
            return;
        }

        match status_line_context.mode {
            Mode::Normal => {
                let status_code = status_line_context
                    .status_code
                    .map(|s| s.code())
                    .unwrap_or_else(|| "--".to_string());

                let (fg_1, bg_1, message) =
                    if let Some(error_message) = status_line_context.error_message {
                        (Fg(colors::TEMPTRESS), Bg(colors::OLD_BRICK), error_message)
                    } else {
                        let url = status_line_context
                            .url
                            .map(|u| u.to_string())
                            .unwrap_or_else(|| "-".to_string());
                        (Fg(colors::GREEN_SMOKE), Bg(colors::COSTA_DEL_SOL), url)
                    };

                print!(
                    "{cursor_pos}{fg_1}{bg_1} {status_code} {fg_2}{bg_2} {message:width$}",
                    cursor_pos = cursor_pos,
                    fg_1 = fg_1,
                    bg_1 = bg_1,
                    fg_2 = Fg(colors::FOREGROUND),
                    bg_2 = Bg(colors::BACKGROUND),
                    status_code = status_code,
                    message = message,
                    width = self.width as usize - 5
                );
            }

            Mode::Input => {
                let cursor_color = colors::FOREGROUND;

                print!(
                    "{cursor_pos}{fg_1}{bg_1}:{input}{fg_2}{bg_2} {bg_3}",
                    cursor_pos = cursor_pos,
                    fg_1 = Fg(colors::FOREGROUND),
                    bg_1 = Bg(colors::BACKGROUND),
                    bg_2 = Bg(cursor_color),
                    fg_2 = Fg(cursor_color),
                    bg_3 = Bg(colors::BACKGROUND),
                    input = status_line_context.input,
                );
            }
        }
    }

    /// The number of rows a line takes up when wrapped
    pub fn line_wrapped_rows(&self, line: &str) -> u16 {
        textwrap::wrap(line, self.width as usize).len() as _
    }

    pub fn page_rows(&self) -> u16 {
        // -1 for the status row
        self.height - 1
    }
}

pub fn clear_screen() -> crossterm::Result<()> {
    stdout()
        .execute(terminal::Clear(terminal::ClearType::All))?
        .execute(Bg(colors::BACKGROUND))?
        .execute(cursor::MoveTo(1, 1))?;

    Ok(())
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
