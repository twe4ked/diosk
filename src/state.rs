use std::fmt;
use std::sync::mpsc;

use crossterm::terminal;
use log::info;
use url::Url;

use crate::gemini::gemtext::Line;
use crate::gemini::status_code::StatusCode;
use crate::gemini::{Response, TransactionError};
use crate::terminal::Terminal;

mod command;

use command::Command;

#[derive(Debug)]
pub enum Event {
    Navigate(Url),
    Terminate,
    Redraw,
    TransactionComplete(Response, Url),
    TransactionError(TransactionError),
}

#[derive(Debug, Clone)]
pub enum Mode {
    Normal,
    Loading,
    Input,
}

pub enum Input {
    Continue,
    Break,
}

pub struct State {
    pub current_line_index: usize,
    current_row: u16,
    pub content: Option<String>,
    pub mode: Mode,
    pub tx: mpsc::Sender<Event>,
    pub current_url: Option<Url>,
    pub last_status_code: Option<StatusCode>,
    pub scroll_offset: u16,
    error_message: Option<String>,
    input: String,
    width: u16,
    height: u16,
}

impl fmt::Debug for State {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("State")
            .field("current_line_index", &self.current_line_index)
            .field("current_row", &self.current_row)
            .field("mode", &self.mode)
            .field("current_url", &self.current_url)
            .field("scroll_offset", &self.scroll_offset)
            .finish()
    }
}

impl State {
    pub fn new() -> (Self, mpsc::Sender<Event>, mpsc::Receiver<Event>) {
        // Set up a channel for State to talk to the worker thread
        let (tx, rx) = mpsc::channel();

        (Self::new_with_tx(tx.clone()), tx, rx)
    }

    fn new_with_tx(tx: mpsc::Sender<Event>) -> Self {
        let (width, height) = terminal::size().unwrap();

        Self {
            current_line_index: 0,
            current_row: 1,
            content: None,
            current_url: None,
            last_status_code: None,
            mode: Mode::Normal,
            tx,
            scroll_offset: 0,
            error_message: None,
            input: String::new(),
            width,
            height,
        }
    }

    pub fn request(&mut self, url_or_path: String) {
        let url = self.qualify_url(&url_or_path);
        self.mode = Mode::Loading;
        self.tx.send(Event::Navigate(url)).unwrap();
    }

    pub fn down(&mut self) {
        self.current_line_index += 1;

        // Check if we need to scroll
        let terminal = Terminal::new(self.width, self.height).unwrap();
        if self.current_row >= terminal.page_rows() {
            self.scroll_offset += 1;
        }

        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn up(&mut self) {
        if self.current_line_index == 0 {
            info!("top of content");
            return;
        }

        self.current_line_index -= 1;

        // Check if we need to scroll
        if self.current_row == 1 {
            self.scroll_offset -= 1;
        }

        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn input(&mut self) {
        self.mode = Mode::Input;
        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn input_char(&mut self, c: char) {
        self.input.push(c);
        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn cancel_input_mode(&mut self) {
        self.mode = Mode::Normal;
        self.input.clear();
        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn delete_word(&mut self) {
        let pat = |c: char| !c.is_ascii_alphanumeric() && c != '_';
        let mut split = self.input.split_inclusive(pat);
        let _deleted = split.next_back();
        self.input = split.collect();
        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn delete_char(&mut self) {
        let mut chars = self.input.chars();
        chars.next_back();
        self.input = chars.collect();
        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn quit(&mut self) {
        self.tx.send(Event::Terminate).unwrap();
    }

    pub fn enter(&mut self) -> Input {
        match self.mode {
            Mode::Normal => {
                let line = &self.content()[self.current_line_index];

                if let Line::Link { url, .. } = line {
                    // Navigate
                    let url = self.qualify_url(&url);
                    self.mode = Mode::Loading;
                    self.tx.send(Event::Navigate(url)).unwrap();
                } else {
                    // Nothing to do on non-link lines
                }
            }

            Mode::Loading => {
                info!("enter while loading");
            }

            Mode::Input => match command::from(&self.input) {
                Some(command) => match command {
                    Command::Navigate(url) => {
                        let url = self.qualify_url(&url);
                        self.mode = Mode::Loading;
                        self.tx.send(Event::Navigate(url)).unwrap();
                        self.tx.send(Event::Redraw).unwrap();
                    }
                    Command::Quit => {
                        self.quit();
                        return Input::Break;
                    }
                },
                None => {
                    self.mode = Mode::Normal;
                    self.set_error_message(format!("Invalid command: {}", self.input));
                    self.tx.send(Event::Redraw).unwrap();
                }
            },
        }

        self.input.clear();

        Input::Continue
    }

    pub fn render_page(&mut self) {
        let status_line_context = StatusLineContext::new_from_state(&self);
        let terminal = Terminal::new(self.width, self.height).unwrap();

        self.current_row = terminal
            .render_page(
                self.current_line_index,
                self.content(),
                self.scroll_offset,
                status_line_context,
            )
            .unwrap();
    }

    /// Parse the URL to ensure it's valid and check if it has a base path
    fn qualify_url(&self, url_or_path: &str) -> Url {
        match Url::parse(&url_or_path) {
            Ok(url) => url,
            Err(url::ParseError::RelativeUrlWithoutBase) => {
                // If we don't have a URL base, we clear the query/fragment and join
                // on the requested path.
                let mut url = self.current_url.as_ref().unwrap().clone();
                url.set_query(None);
                url.set_fragment(None);
                url.join(&url_or_path).unwrap()
            }
            e => panic!("{:?}", e),
        }
    }

    // TODO: Store parsed lines directly on Self
    fn content(&self) -> Vec<Line> {
        self.content
            .as_ref()
            .map(|c| c.lines().map(Line::parse).collect())
            .unwrap_or_else(|| vec![Line::Normal(String::new())])
    }

    pub fn set_error_message(&mut self, message: String) {
        self.error_message = Some(message);
    }

    pub fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    pub fn send_redraw(&self) {
        self.tx.send(Event::Redraw).unwrap();
    }

    pub fn new_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        info!("New size {}x{}", self.width, self.height);
        self.send_redraw();
    }
}

pub struct StatusLineContext<'a> {
    pub status_code: Option<StatusCode>,
    pub url: Option<Url>,
    pub error_message: Option<String>,
    pub mode: Mode,
    pub input: &'a str,
}

impl<'a> StatusLineContext<'a> {
    fn new_from_state(state: &'a State) -> Self {
        Self {
            status_code: state.last_status_code.clone(),
            url: state.current_url.clone(),
            error_message: state.error_message.clone(),
            mode: state.mode.clone(),
            input: &state.input,
        }
    }
}
