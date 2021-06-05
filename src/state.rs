use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::mpsc;
use std::thread;

use crossterm::terminal::size as terminal_size;
use log::info;
use url::Url;

use crate::gemini::gemtext::Line;
use crate::gemini::status_code::StatusCode;
use crate::gemini::{transaction, Response, TransactionError};
use crate::terminal::{self, Terminal};

mod command;

use command::Command;

#[derive(Debug)]
pub enum Event {
    TerminateWorker,
    TransactionComplete(Response, Url),
    TransactionError(TransactionError),
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Normal,
    Loading,
    Input,
}

pub struct State {
    current_line_index: usize,
    current_row: u16,
    content: Option<String>,
    mode: Mode,
    tx: mpsc::Sender<Event>,
    current_url: Option<Url>,
    last_status_code: Option<StatusCode>,
    scroll_offset: u16,
    error_message: Option<String>,
    input: String,
    width: u16,
    height: u16,
    terminated: bool,
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
    pub fn new() -> (Self, mpsc::Receiver<Event>) {
        // Set up a channel for State to talk to the worker thread
        let (tx, rx) = mpsc::channel();

        (Self::new_with_tx(tx), rx)
    }

    fn new_with_tx(tx: mpsc::Sender<Event>) -> Self {
        let (width, height) = terminal_size().unwrap();

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
            terminated: false,
        }
    }

    fn request(&mut self, url_or_path: &str) {
        let url = self.qualify_url(&url_or_path);
        self.mode = Mode::Loading;
        let tx = self.tx.clone();
        thread::spawn(move || {
            let response = match transaction(&url) {
                Ok(response) => tx.send(Event::TransactionComplete(response, url)),
                Err(e) => tx.send(Event::TransactionError(e)),
            };

            info!("finished navigating");

            response
        });
    }

    pub fn down(&mut self) {
        self.current_line_index += 1;

        // Check if we need to scroll
        let terminal = Terminal::new(self.width, self.height);
        if self.current_row >= terminal.page_rows() {
            self.scroll_offset += 1;
        }

        self.clear_screen_and_render_page();
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

        self.clear_screen_and_render_page();
    }

    pub fn input(&mut self) {
        self.mode = Mode::Input;
        self.clear_screen_and_render_page();
    }

    pub fn input_char(&mut self, c: char) {
        self.input.push(c);
        self.clear_screen_and_render_page();
    }

    pub fn cancel_input_mode(&mut self) {
        self.mode = Mode::Normal;
        self.input.clear();
        self.clear_screen_and_render_page();
    }

    pub fn delete_word(&mut self) {
        let pat = |c: char| !c.is_ascii_alphanumeric() && c != '_';
        let mut split = self.input.split_inclusive(pat);
        let _deleted = split.next_back();
        self.input = split.collect();
        self.clear_screen_and_render_page();
    }

    pub fn delete_char(&mut self) {
        let mut chars = self.input.chars();
        chars.next_back();
        self.input = chars.collect();
        self.clear_screen_and_render_page();
    }

    fn quit(&mut self) {
        self.terminated = true;
        self.tx.send(Event::TerminateWorker).unwrap();
    }

    pub fn enter(&mut self) {
        if matches!(self.mode, Mode::Loading) {
            info!("enter while loading");
            return;
        }

        let line = &self.content()[self.current_line_index];

        if let Line::Link { url, .. } = line {
            self.request(url);
        } else {
            // Nothing to do on non-link lines
        }
    }

    pub fn loading_mode_enter(&mut self) {
        info!("enter while loading");
    }

    pub fn input_mode_enter(&mut self) {
        let input = self.input.clone();

        let mut history = OpenOptions::new()
            .create(true)
            .append(true)
            .open("target/history.txt")
            .unwrap();
        write!(&mut history, "{}\n", &input).unwrap();

        match command::from(&input) {
            Some(command) => match command {
                Command::Navigate(url) => {
                    self.request(url);
                    self.clear_screen_and_render_page();
                }
                Command::Quit => {
                    self.quit();
                }
            },
            None => {
                self.mode = Mode::Normal;
                self.set_error_message(format!("Invalid command: {}", self.input));
                self.clear_screen_and_render_page();
            }
        }

        self.input.clear();
    }

    pub fn terminated(&self) -> bool {
        self.terminated
    }

    fn render_page(&mut self) {
        let status_line_context = StatusLineContext::new_from_state(&self);
        let terminal = Terminal::new(self.width, self.height);

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

    fn set_error_message(&mut self, message: String) {
        self.error_message = Some(message);
    }

    pub fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    pub fn new_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        info!("New size {}x{}", self.width, self.height);
        self.clear_screen_and_render_page();
    }

    pub fn clear_screen_and_render_page(&mut self) {
        // TODO: We don't always need to clear the screen. Only for things like scrolling.
        terminal::clear_screen().unwrap();

        self.render_page();
    }

    pub fn transaction_complete(&mut self, response: Response, url: Url) {
        match response {
            Response::Body {
                content,
                status_code,
            } => {
                // Move the current line back to the top of the page
                self.current_line_index = 0;

                self.content = content;
                self.current_url = Some(url);
                self.last_status_code = Some(status_code);
            }
            Response::RedirectLoop(_url) => todo!("handle redirect loops"),
        }

        terminal::clear_screen().unwrap();
        self.mode = Mode::Normal;
        self.render_page();
    }

    pub fn transaction_error(&mut self, e: TransactionError) {
        info!("transaction error: {}", e);

        self.set_error_message(e.to_string());
        terminal::clear_screen().unwrap();
        self.mode = Mode::Normal;
        self.render_page();
    }

    pub fn mode(&self) -> Mode {
        self.mode
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
