use std::fmt;
use std::sync::mpsc;

use log::info;
use url::Url;

use crate::gemini::gemtext::Line;
use crate::gemini::StatusCode;
use crate::gemini::{transaction, Response};
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
    pub content: String,
    pub mode: Mode,
    pub tx: mpsc::Sender<Event>,
    pub current_url: Url,
    pub last_status_code: StatusCode,
    pub terminal: Terminal,
    pub scroll_offset: u16,
}

impl fmt::Debug for State {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut content = self.content.clone();
        content.truncate(10);

        fmt.debug_struct("State")
            .field("current_line_index", &self.current_line_index)
            .field("mode", &self.mode)
            .field("current_url", &self.current_url.to_string())
            .field("terminal", &self.terminal)
            .field("scroll_offset", &self.scroll_offset)
            .finish()
    }
}

impl State {
    pub fn new(terminal: Terminal, tx: mpsc::Sender<Event>, url: Url) -> Self {
        let (content, last_status_code) =
            match transaction(&url, 0).expect("initial transaction failed") {
                Response::Body {
                    content,
                    status_code,
                } => (content.unwrap(), status_code),
                _ => panic!("initial URL must contain a body"),
            };

        Self {
            current_line_index: 0,
            content,
            current_url: url,
            last_status_code,
            mode: Mode::Normal,
            tx,
            terminal,
            scroll_offset: 0,
        }
    }

    fn line(&self, index: usize) -> &str {
        self.content
            .lines()
            .nth(index as usize)
            .expect("current line not found")
    }

    fn current_line(&self) -> &str {
        self.line(self.current_line_index)
    }

    pub fn down(&mut self) {
        self.current_line_index += 1;

        let next_line = self.line(self.current_line_index);
        let next_line_rows = self.terminal.line_wrapped_rows(&next_line);

        if self.terminal.current_row() + next_line_rows > self.terminal.page_rows() {
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

        let prev_line = self.line(self.current_line_index);
        let prev_line_rows = self.terminal.line_wrapped_rows(&prev_line);

        if self.terminal.current_row() - prev_line_rows == 0 {
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
        let content = self.content.clone();
        let current_line_index = self.current_line_index;
        let current_url = self.current_url.clone();
        let scroll_offset = self.scroll_offset;
        let last_status_code = self.last_status_code.clone();

        self.terminal
            .render_page(
                current_line_index,
                content,
                &current_url,
                last_status_code,
                scroll_offset,
            )
            .unwrap();
    }
}
