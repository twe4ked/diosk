use std::sync::{Arc, Mutex};

use crossterm::event::{read, Event, KeyCode};
use log::info;

use crate::state::{Mode, State};

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
                        KeyCode::Char('g') => state.go(),
                        KeyCode::Char('j') => state.down(),
                        KeyCode::Char('k') => state.up(),
                        KeyCode::Enter => state.enter(),
                        _ => {}
                    },

                    Mode::Input => todo!(),
                }

                state.clear_error_message();

                info!("{:?}", &state);
            }
            Event::Mouse(event) => info!("{:?}", event),
            Event::Resize(width, height) => info!("New size {}x{}", width, height),
        }
    }
}
