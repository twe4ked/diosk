use std::sync::{Arc, Mutex};

use crossterm::event::{read, Event, KeyCode, KeyModifiers};
use log::info;

use crate::state::{Mode, State};

mod readline;

use readline::Command;

pub fn run(state: Arc<Mutex<State>>) {
    loop {
        match read().unwrap() {
            Event::Key(event) => {
                let mut state = state.lock().expect("poisoned");
                let mode = state.mode.clone();

                match mode {
                    Mode::Normal | Mode::Loading => match event.code {
                        KeyCode::Char('q') => {
                            state.quit();
                            break;
                        }
                        KeyCode::Char(':') => state.input(),
                        KeyCode::Char('j') => state.down(),
                        KeyCode::Char('k') => state.up(),
                        KeyCode::Enter => state.enter(),
                        _ => {}
                    },

                    Mode::Input => match (event.code, event.modifiers) {
                        (KeyCode::Char(c), KeyModifiers::NONE) => state.input_char(c),
                        (KeyCode::Enter, _) => state.enter(),
                        (KeyCode::Esc, _) => state.cancel_input_mode(),
                        (_, _) => match readline::command(event) {
                            Some(command) => match command {
                                Command::DeleteWord => state.delete_word(),
                            },
                            None => {}
                        },
                    },
                }

                state.clear_error_message();

                info!("{:?}", &state);
            }
            Event::Mouse(event) => info!("{:?}", event),
            Event::Resize(width, height) => info!("New size {}x{}", width, height),
        }
    }
}
