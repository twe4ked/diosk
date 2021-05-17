use std::sync::{Arc, Mutex};

use crossterm::event::{read, Event, KeyCode, KeyModifiers};
use log::info;

use crate::state::{Input, Mode, State};

mod edit;

use edit::Command;

pub fn run(state: Arc<Mutex<State>>) {
    loop {
        match read().unwrap() {
            Event::Key(event) => {
                let mut state = state.lock().expect("poisoned");
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
                        (KeyCode::Enter, _) => match state.enter() {
                            Input::Continue => {}
                            Input::Break => break,
                        },
                        (KeyCode::Esc, _) => state.cancel_input_mode(),
                        (_, _) => match edit::command(event) {
                            Some(command) => match command {
                                Command::DeleteWord => state.delete_word(),
                            },
                            None => {}
                        },
                    },
                }

                info!("{:?}", &state);
            }
            Event::Mouse(event) => info!("{:?}", event),
            Event::Resize(width, height) => info!("New size {}x{}", width, height),
        }
    }
}
