use std::io;

use crate::state::history::History;

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
    // TODO: History needs to be separate for commands and search
    command_history: History,
}

impl Input {
    pub fn new() -> Self {
        Self {
            command_history: History::new("target/history.txt"),
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

    pub fn up(&mut self) {
        self.command_history.up();
        self.input = self.command_history.get();
    }

    pub fn down(&mut self) {
        if self.command_history.down() {
            self.input = self.command_history.get();
        }
    }

    pub fn enter(&mut self) -> InputEnterResult {
        let input = self.input.clone();
        self.input.clear();
        self.command_history.push(input.clone());
        self.command_history.reset_index();
        InputEnterResult::from(&input)
    }

    pub fn search(&mut self) {
        self.input.clear();
    }

    pub fn flush_history(&mut self) -> io::Result<()> {
        self.command_history.flush()
    }
}
