use std::fs::OpenOptions;
use std::io::Write;

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

pub struct Input {
    pub input: String,
}

impl Input {
    pub fn new() -> Self {
        Self {
            input: String::new(),
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

    pub fn enter(&mut self) -> InputEnterResult {
        let input = self.input.clone();
        self.input.clear();

        let mut history = OpenOptions::new()
            .create(true)
            .append(true)
            .open("target/history.txt")
            .unwrap();
        write!(&mut history, "{}\n", &input).unwrap();

        InputEnterResult::from(&input)
    }
}
