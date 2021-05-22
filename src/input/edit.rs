use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum Command {
    DeleteWord,
    DeleteChar,
    AddChar(char),
    Enter,
    Esc,
}

pub fn command(key_event: KeyEvent) -> Option<Command> {
    use Command::*;

    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => Some(DeleteWord),
        (KeyCode::Backspace, KeyModifiers::NONE) => Some(DeleteChar),
        (KeyCode::Char(c), KeyModifiers::NONE) => Some(AddChar(c)),
        (KeyCode::Enter, _) => Some(Enter),
        (KeyCode::Esc, _) => Some(Esc),

        (key_code, modifiers) => {
            log::info!("{:?} {:?}", key_code, modifiers);
            None
        }
    }
}
