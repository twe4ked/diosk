use std::sync::{Arc, Mutex};

use crossterm::event::{read, Event, KeyCode, KeyModifiers};
use log::info;

use crate::state::{Mode, State};

mod edit;

use edit::Command;

pub fn run(state: Arc<Mutex<State>>) {
    loop {
        let event = read().unwrap();
        let mut state = state.lock().expect("poisoned");

        match event {
            Event::Key(event) => {
                let mode = state.mode.clone();

                state.clear_error_message();

                match mode {
                    Mode::Normal | Mode::Loading => match event.code {
                        KeyCode::Char(':') => state.input(),
                        KeyCode::Char('j') => state.down(),
                        KeyCode::Char('k') => state.up(),
                        KeyCode::Enter => {
                            state.enter();
                        }
                        _ => {}
                    },

                    Mode::Input => match (event.code, event.modifiers) {
                        (KeyCode::Char(c), KeyModifiers::NONE) => state.input_char(c),
                        (KeyCode::Enter, _) => state.enter(),
                        (KeyCode::Esc, _) => state.cancel_input_mode(),
                        (_, _) => {
                            if let Some(command) = edit::command(event) {
                                match command {
                                    Command::DeleteWord => state.delete_word(),
                                    Command::DeleteChar => state.delete_char(),
                                }
                            }
                        }
                    },
                }

                info!("{:?}", &state);
            }
            Event::Mouse(event) => info!("{:?}", event),
            Event::Resize(width, height) => state.new_size(width, height),
        }

        if state.terminated() {
            break;
        }
    }
}
