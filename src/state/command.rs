pub enum Command<'a> {
    Navigate(&'a str),
    Quit,
}

pub fn from(input: &str) -> Option<Command> {
    use Command::*;

    if let Some(url) = input.strip_prefix("go ") {
        Some(Navigate(url))
    } else if input == "quit" {
        Some(Quit)
    } else {
        None
    }
}
