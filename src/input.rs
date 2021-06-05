use std::sync::{Arc, Mutex};

use crossterm::event::{read, Event, KeyCode, KeyEvent};
use log::info;

use crate::state::{Mode, State};

mod edit;

use edit::Command;

pub fn run(state: Arc<Mutex<State>>) {
    loop {
        let event = read().unwrap();
        let mut state = state.lock().expect("poisoned");

        match event {
            Event::Key(event) => handle_key_event(&mut state, event),
            Event::Mouse(event) => info!("{:?}", event),
            Event::Resize(width, height) => state.new_size(width, height),
        }

        if state.terminated() {
            break;
        }
    }
}

fn handle_key_event(state: &mut State, event: KeyEvent) {
    state.clear_error_message();

    match state.mode() {
        Mode::Normal | Mode::Loading => match event.code {
            KeyCode::Char(':') => state.input(),
            KeyCode::Char('j') => state.down(),
            KeyCode::Char('k') => state.up(),
            KeyCode::Enter => state.enter(),
            _ => {}
        },

        Mode::Input => {
            if let Some(command) = edit::command(event) {
                match command {
                    Command::DeleteWord => state.delete_word(),
                    Command::DeleteChar => state.delete_char(),
                    Command::AddChar(c) => state.input_char(c),
                    Command::Enter => state.enter(),
                    Command::Esc => state.cancel_input_mode(),
                }
            }
        }
    }

    info!("{:?}", &state);
}
