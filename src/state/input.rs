use std::io;

use crate::state::history::History;
use crate::state::Mode;

pub enum InputEnterResult {
    Navigate(String),
    Quit,
    Invalid(String),
}

impl InputEnterResult {
    pub fn from(input: &str) -> Self {
        use InputEnterResult::*;

        if let Some(url) = input.strip_prefix("go ") {
            Navigate(url.to_owned())
        } else if input == "quit" || input == "q" {
            Quit
        } else {
            Invalid(input.to_owned())
        }
    }
}

#[derive(Default)]
pub struct Input {
    pub input: String,
    command_history: History,
    search_history: History,
}

impl Input {
    pub fn new() -> Self {
        Self {
            command_history: History::new("target/command_history.txt"),
            search_history: History::new("target/search_history.txt"),
            ..Self::default()
        }
    }

    pub fn input_char(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn cancel(&mut self) {
        self.input.clear();
    }

    pub fn delete_word(&mut self) {
        let pat = |c: char| !c.is_ascii_alphanumeric() && c != '_';
        let mut split = self.input.split_inclusive(pat);
        let _deleted = split.next_back();
        self.input = split.collect();
    }

    pub fn delete_char(&mut self) {
        let mut chars = self.input.chars();
        chars.next_back();
        self.input = chars.collect();
    }

    pub fn up(&mut self, mode: Mode) {
        self.history(mode).up();
        self.input = self.history(mode).get();
    }

    pub fn down(&mut self, mode: Mode) {
        if self.history(mode).down() {
            self.input = self.history(mode).get();
        }
    }

    pub fn enter(&mut self, mode: Mode) -> InputEnterResult {
        let input = self.input.clone();
        self.input.clear();
        self.history(mode).push(input.clone());
        self.history(mode).reset_index();
        InputEnterResult::from(&input)
    }

    pub fn search(&mut self) {
        self.input.clear();
    }

    pub fn history(&mut self, mode: Mode) -> &mut History {
        match mode {
            Mode::Input => &mut self.command_history,
            Mode::Search => &mut self.search_history,
            _ => panic!("no history for mode: {:?}", mode),
        }
    }

    pub fn flush_history(&mut self) -> io::Result<()> {
        self.search_history.flush()
    }
}
