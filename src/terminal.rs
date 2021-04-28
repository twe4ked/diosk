use std::fmt::{self, Display, Formatter};
use std::io::{stdout, Stdout, Write};

use termion::raw::{IntoRawMode, RawTerminal};
use termion::{color, screen};
use url::Url;

use crate::gemini::gemtext::Line;
use crate::gemini::StatusCode;

mod colors;

type Screen = screen::AlternateScreen<RawTerminal<Stdout>>;

#[derive(Debug)]
struct CursorPosition {
    x: u16,
    y: u16,
}

impl CursorPosition {
    fn goto(&self) -> termion::cursor::Goto {
        termion::cursor::Goto(self.x, self.y)
    }
}

pub struct Terminal {
    width: u16,
    height: u16,
    cursor_pos: CursorPosition,
    screen: Screen,
}

impl fmt::Debug for Terminal {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Terminal")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("cursor_pos", &self.cursor_pos)
            .field("screen", &"Screen")
            .finish()
    }
}

enum Render {
    Continue,
    Break,
}

impl Terminal {
    pub fn setup_alternate_screen() -> Self {
        let mut screen = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());

        // Hide the cusor, clear the screen, and set the initial cursor position
        write!(
            screen,
            "{}{}{}",
            termion::cursor::Hide,
            termion::clear::All,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();

        let (width, height) = termion::terminal_size().expect("unable to get terminal size");

        Self {
            width,
            height,
            screen,
            cursor_pos: CursorPosition { x: 1, y: 1 }, // 1-based
        }
    }

    pub fn render_page(
        &mut self,
        current_line: usize,
        content: String,
        url: &Url,
        status_code: StatusCode,
    ) {
        // Move back to the beginning before drawing page
        self.cursor_pos.x = 1;
        self.cursor_pos.y = 1;

        for (i, line) in content.lines().enumerate() {
            let is_active = current_line == i;

            match self.render_line(line, is_active) {
                Render::Continue => {}
                Render::Break => break,
            }
        }

        self.draw_status_line(url, status_code);

        self.screen.flush().unwrap();
    }

    fn render_line(&mut self, line: &str, is_active: bool) -> Render {
        // Highlight the current line
        let bg_color: Box<dyn BgColor> = if is_active {
            Box::new(color::Bg(color::LightBlack))
        } else {
            Box::new(color::Bg(color::Black))
        };

        match Line::parse(line) {
            Line::Normal => {
                for part in textwrap::wrap(line, self.width as usize) {
                    // If we're going to overflow the screen, stop printing
                    if self.cursor_pos.y + 1 > self.height {
                        return Render::Break;
                    }

                    // If we've got a blank line, render a space so we can
                    // see it when it's highlighted
                    if line.is_empty() {
                        write!(
                            self.screen,
                            "{}{}{} ",
                            self.cursor_pos.goto(),
                            color::Fg(color::Reset),
                            bg_color,
                        )
                        .unwrap();
                    } else {
                        write!(
                            self.screen,
                            "{}{}{}{}",
                            self.cursor_pos.goto(),
                            color::Fg(color::Reset),
                            bg_color,
                            part
                        )
                        .unwrap();
                    }

                    self.cursor_pos.x = 1;
                    self.cursor_pos.y += 1;
                }
            }
            Line::Link { url, name } => {
                // If we're going to overflow the screen, stop printing
                if self.cursor_pos.y + 1 > self.height {
                    return Render::Break;
                }

                // TODO: Handle wrapping
                writeln!(
                    self.screen,
                    "{}{}{}=> {}{} {}{}",
                    self.cursor_pos.goto(),
                    bg_color,
                    color::Fg(color::Cyan),
                    color::Fg(color::Reset),
                    name.unwrap_or_else(|| url.clone()),
                    color::Fg(color::LightBlack),
                    url // TODO: Hide if we don't have a name because the URL is already being displayed
                )
                .unwrap();

                self.cursor_pos.x = 1;
                self.cursor_pos.y += 1;
            }
        }

        Render::Continue
    }

    fn draw_status_line(&mut self, url: &Url, status_code: StatusCode) {
        self.cursor_pos.x = 1;
        self.cursor_pos.y = self.height;

        write!(
            self.screen,
            "{cursor_pos}{fg_1}{bg_1} {status_code} {fg_2}{bg_2} {url:width$}",
            cursor_pos = self.cursor_pos.goto(),
            fg_1 = color::Fg(colors::GREEN_SMOKE),
            bg_1 = color::Bg(color::LightBlack),
            fg_2 = color::Fg(colors::FOREGROUND),
            bg_2 = color::Bg(colors::BACKGROUND),
            status_code = status_code.code(),
            url = url,
            width = self.width as usize - 5
        )
        .unwrap();
    }

    pub fn clear_screen(&mut self) {
        write!(
            self.screen,
            "{}{}{}",
            color::Bg(color::Black),
            termion::clear::All,
            termion::cursor::Goto(1, 1),
        )
        .unwrap();
    }

    pub fn flush(&mut self) {
        self.screen.flush().unwrap();
    }

    pub fn show_cursor(&mut self) {
        write!(self.screen, "{}", termion::cursor::Show).unwrap();
    }
}

// This trait erases the color::Color type so we can box the different colors
trait BgColor {
    fn write_to(&self, w: &mut dyn fmt::Write) -> fmt::Result;
}

impl<C> BgColor for color::Bg<C>
where
    C: color::Color,
{
    fn write_to(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "{}", self)
    }
}

impl Display for dyn BgColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        self.write_to(f)
    }
}
