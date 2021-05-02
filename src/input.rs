use std::sync::{Arc, Mutex};

use crossterm::event::{read, Event as TermEvent, KeyCode};
use log::info;

use crate::state::{Mode, State};
use crate::terminal::Terminal;

pub fn run(state: Arc<Mutex<State>>) {
    loop {
        match read().unwrap() {
            TermEvent::Key(event) => {
                let mut state = state.lock().expect("poisoned");
                let mode = state.mode.clone();

                match mode {
                    Mode::Loading => {
                        if let KeyCode::Char('q') = event.code {
                            state.quit();
                            break;
                        }
                    }

                    Mode::Normal => match event.code {
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

                Terminal::flush().unwrap();

                info!("{:?}", &state);
            }
            TermEvent::Mouse(event) => info!("{:?}", event),
            TermEvent::Resize(width, height) => info!("New size {}x{}", width, height),
        }
    }
}
