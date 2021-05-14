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
    current_row: u16,
    pub content: Option<String>,
    pub mode: Mode,
    pub tx: mpsc::Sender<Event>,
    pub current_url: Option<Url>,
    pub last_status_code: Option<StatusCode>,
    pub scroll_offset: u16,
    error_message: Option<String>,
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
        }
    }

    pub fn request(&mut self, url: String) {
        self.mode = Mode::Loading;
        self.tx.send(Event::Navigate(url)).unwrap();
    }

    pub fn down(&mut self) {
        self.current_line_index += 1;

        // Check if we need to scroll
        let terminal = Terminal::new().unwrap();
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

        self.tx.send(Event::Redraw).unwrap();

        unimplemented!();
    }

    pub fn go(&mut self) {
        self.mode = Mode::Input;
        todo!();
    }

    pub fn quit(&mut self) {
        self.tx.send(Event::Terminate).unwrap();
    }

    pub fn enter(&mut self) {
        let line = &self.content()[self.current_line_index];

        if let Line::Link { url, .. } = line {
            // Navigate
            self.mode = Mode::Loading;
            self.tx.send(Event::Navigate(url.to_string())).unwrap();
        } else {
            // Nothing to do on non-link lines
        }
    }

    pub fn render_page(&mut self) {
        let status_line_context = StatusLineContext::new_from_state(&self);
        let terminal = Terminal::new().unwrap();

        self.current_row = terminal
            .render_page(
                self.current_line_index,
                self.content(),
                self.scroll_offset,
                status_line_context,
            )
            .unwrap();
    }

    // TODO: Store parsed lines directly on Self
    fn content(&self) -> Vec<Line> {
        self.content
            .as_ref()
            .map(|c| c.lines().map(Line::parse).collect())
            .unwrap_or_else(|| match &self.mode {
                Mode::Loading => vec![Line::Normal("Loading...".to_string())],
                s => unimplemented!("not content for state: {:?}", s),
            })
    }

    pub fn set_error_message(&mut self, message: String) {
        self.error_message = Some(message);
    }

    pub fn clear_error_message(&mut self) {
        self.error_message = None;
    }
}

pub struct StatusLineContext {
    pub status_code: Option<StatusCode>,
    pub url: Option<Url>,
    pub error_message: Option<String>,
}

impl StatusLineContext {
    fn new_from_state(state: &State) -> Self {
        Self {
            status_code: state.last_status_code.clone(),
            url: state.current_url.clone(),
            error_message: state.error_message.clone(),
        }
    }
}
