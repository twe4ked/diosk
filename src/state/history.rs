use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};

#[derive(Default)]
pub struct History {
    index: Option<usize>,
    existing: Vec<String>,
    local: Vec<String>,
}

impl History {
    pub fn new(path: &str) -> Self {
        let f = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(path)
            .unwrap();
        let f = BufReader::new(f);

        Self {
            index: None,
            existing: f.lines().map(|s| s.unwrap()).collect(),
            local: Vec::new(),
        }
    }

    pub fn get(&self) -> String {
        self.existing
            .iter()
            .chain(self.local.iter())
            .rev()
            .nth(self.index.expect("must be set"))
            .map_or_else(String::new, |s| s.clone())
    }

    pub fn push(&mut self, item: String) {
        self.local.push(item);
    }

    pub fn up(&mut self) {
        match self.index.as_mut() {
            Some(i) => *i += 1,
            None => self.index = Some(0),
        }
    }

    pub fn down(&mut self) -> bool {
        if self.index == Some(0) {
            return false;
        }

        match self.index.as_mut() {
            Some(i) => *i -= 1,
            None => self.index = Some(0),
        }

        true
    }

    pub fn reset_index(&mut self) {
        self.index = None;
    }

    pub fn flush(&mut self) -> io::Result<()> {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open("target/history.txt")?;
        for line in &self.local {
            writeln!(f, "{}", line)?;
        }
        f.flush()?;

        self.local.clear();

        Ok(())
    }
}
