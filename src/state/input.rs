use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};

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
    history_index: Option<usize>,
    existing_history: Vec<String>,
    local_history: Vec<String>,
}

impl Input {
    pub fn new() -> Self {
        let history = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open("target/history.txt")
            .unwrap();
        let history = BufReader::new(history);
        let history: Vec<String> = history.lines().map(|s| s.unwrap()).collect();

        Self {
            existing_history: history,
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
        match self.history_index.as_mut() {
            Some(i) => *i += 1,
            None => self.history_index = Some(0),
        }
        self.set_input_from_history();
    }

    pub fn down(&mut self) {
        if self.history_index == Some(0) {
            return;
        }

        match self.history_index.as_mut() {
            Some(i) => *i -= 1,
            None => self.history_index = Some(0),
        }
        self.set_input_from_history();
    }

    pub fn set_input_from_history(&mut self) {
        let history = self
            .existing_history
            .iter()
            .chain(self.local_history.iter());

        self.input = history
            .rev()
            .nth(self.history_index.expect("must be set"))
            .map_or_else(String::new, |s| s.clone());
    }

    pub fn enter(&mut self) -> InputEnterResult {
        let input = self.input.clone();
        self.input.clear();
        self.local_history.push(input.clone());
        self.history_index = None;
        InputEnterResult::from(&input)
    }

    pub fn flush_history(&mut self) -> io::Result<()> {
        let mut history = OpenOptions::new()
            .create(true)
            .append(true)
            .open("target/history.txt")?;

        for line in &self.local_history {
            writeln!(history, "{}", line)?;
        }

        history.flush()?;

        self.local_history.clear();
        Ok(())
    }
}
