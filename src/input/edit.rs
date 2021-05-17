use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum Command {
    DeleteWord,
}

pub fn command(key_event: KeyEvent) -> Option<Command> {
    use Command::*;

    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => Some(DeleteWord),
        (key_code, modifiers) => {
            log::info!("{:?} {:?}", key_code, modifiers);
            None
        }
    }
}
