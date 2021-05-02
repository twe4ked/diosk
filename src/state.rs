use std::fmt;
use std::sync::mpsc;

use log::info;
use url::Url;

use crate::gemini::gemtext::Line;
use crate::gemini::StatusCode;
use crate::terminal::Terminal;

#[derive(Debug)]
pub enum Event {
    Navigate(String),
    Terminate,
    Redraw,
}

#[derive(Debug, Clone)]
pub enum Mode {
    Normal,
    Loading,
    Input,
}

pub struct State {
    pub current_line_index: usize,
    pub content: Option<String>,
    pub mode: Mode,
    pub tx: mpsc::Sender<Event>,
    pub current_url: Option<Url>,
    pub last_status_code: Option<StatusCode>,
    pub scroll_offset: u16,
}

impl fmt::Debug for State {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("State")
            .field("current_line_index", &self.current_line_index)
            .field("mode", &self.mode)
            .field("current_url", &self.current_url)
            .field("scroll_offset", &self.scroll_offset)
            .finish()
    }
}

impl State {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        Self {
            current_line_index: 0,
            content: None,
            current_url: None,
            last_status_code: None,
            mode: Mode::Normal,
            tx,
            scroll_offset: 0,
        }
    }

    pub fn request(&mut self, url: String) {
        self.mode = Mode::Loading;
        self.tx.send(Event::Navigate(url)).unwrap();
    }

    fn line(&self, index: usize) -> &str {
        self.content
            .as_ref()
            .unwrap()
            .lines()
            .nth(index as usize)
            .expect("current line not found")
    }

    fn current_line(&self) -> &str {
        self.line(self.current_line_index)
    }

    pub fn down(&mut self) {
        self.current_line_index += 1;

        let terminal = Terminal::new().unwrap();

        let next_line = self.line(self.current_line_index);
        let next_line_rows = terminal.line_wrapped_rows(&next_line);

        if terminal.current_row() + next_line_rows > terminal.page_rows() {
            self.scroll_offset += next_line_rows;
        }

        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn up(&mut self) {
        if self.current_line_index == 0 {
            info!("top of content");
            return;
        }

        self.current_line_index -= 1;

        let terminal = Terminal::new().unwrap();

        let prev_line = self.line(self.current_line_index);
        let prev_line_rows = terminal.line_wrapped_rows(&prev_line);

        if terminal.current_row() - prev_line_rows == 0 {
            self.scroll_offset -= prev_line_rows;
        }

        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn go(&mut self) {
        self.mode = Mode::Input;
        todo!();
    }

    pub fn quit(&mut self) {
        self.tx.send(Event::Terminate).unwrap();
    }

    pub fn enter(&mut self) {
        let line = self.current_line();

        if let Line::Link { url, .. } = Line::parse(line) {
            // Navigate
            self.mode = Mode::Loading;
            self.tx.send(Event::Navigate(url)).unwrap();
        } else {
            // Nothing to do on non-link lines
        }
    }

    pub fn render_page(&mut self) {
        Terminal::render_page(
            self.current_line_index,
            self.content.clone(),
            &self.current_url,
            &self.last_status_code,
            self.scroll_offset,
            &self.mode.clone(),
        )
        .unwrap();
    }
}
